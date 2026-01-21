use std::{
    io, mem,
    sync::{atomic::AtomicBool, Arc},
    thread,
    time::{Duration, Instant},
    path::{Path, PathBuf},
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
};

use sha2::{Digest, Sha256};
use windows::{
    core::{PCWSTR, HSTRING, Interface},
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{CreateDIBSection, DeleteDC, DeleteObject, SelectObject, CreateCompatibleDC, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP, HDC},
        System::Com::{IStream, STATSTG},
        UI::{
            Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES, SHGFI_SYSICONINDEX, SHIL_JUMBO, SHGetImageList},
            Controls::{IImageList, HIMAGELIST},
            WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON},
        },
    },
};

pub fn get_cache_path(file_path: &Path) -> Option<PathBuf> {
    let cache_dir = std::env::var("LOCALAPPDATA").ok().map(|p| PathBuf::from(p).join("space-thumbnails").join("cache"))?;
    
    if !cache_dir.exists() {
        let _ = std::fs::create_dir_all(&cache_dir);
    }

    let mut hasher = Sha256::new();
    
    // Improved hashing: if file exists, hash content parts. If not, fallback to path.
    // For temp files from IStream, path is useless, we MUST hash content.
    if let Ok(file) = std::fs::File::open(file_path) {
        if let Ok(metadata) = file.metadata() {
            // Include file size
            hasher.update(metadata.len().to_le_bytes());
            
            // Hash first 4KB
            let mut buffer = [0u8; 4096];
            // We need to read from file, but we already have `file`
            // Let's use std::io::Read
            use std::io::{Read, Seek, SeekFrom};
            let mut reader = file; // move file
            
            if let Ok(n) = reader.read(&mut buffer) {
                hasher.update(&buffer[..n]);
            }
            
            // Hash last 4KB if file is large enough
            if metadata.len() > 8192 {
                if reader.seek(SeekFrom::End(-4096)).is_ok() {
                    if let Ok(n) = reader.read(&mut buffer) {
                        hasher.update(&buffer[..n]);
                    }
                }
            }
        } else {
             // Fallback to path if metadata fails
            if let Some(s) = file_path.to_str() {
                hasher.update(s.to_lowercase().as_bytes());
            } else {
                hasher.update(file_path.to_string_lossy().as_bytes());
            }
        }
    } else {
        // Fallback to path if open fails
        if let Some(s) = file_path.to_str() {
            hasher.update(s.to_lowercase().as_bytes());
        } else {
            hasher.update(file_path.to_string_lossy().as_bytes());
        }
    }

    let result = hasher.finalize();
    let filename = hex::encode(result) + ".png";
    Some(cache_dir.join(filename))
}

pub fn run_timeout<T: Send + 'static>(
    func: impl FnOnce() -> T + Send + 'static,
    timeout: Duration,
) -> io::Result<T> {
    let done = Arc::new(AtomicBool::new(false));
    let done_inner = done.clone();

    let start_at = Instant::now();
    let thread_handler = thread::Builder::new().spawn(move || {
        let result = func();

        done_inner.swap(true, std::sync::atomic::Ordering::Relaxed);
        result
    })?;

    // wait for done or timeout
    loop {
        if done.load(std::sync::atomic::Ordering::Relaxed) {
            break match thread_handler.join() {
                Ok(result) => Ok(result),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Thread panic")),
            };
        } else if start_at.elapsed() > timeout {
            break Err(io::Error::new(io::ErrorKind::TimedOut, "Timeout"));
        } else {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
    }
}

pub struct WinStream {
    stream: IStream,
}

impl WinStream {
    pub fn size(&self) -> windows::core::Result<u64> {
        unsafe {
            let mut stats = STATSTG::default();
            self.stream.Stat(&mut stats, 0)?;
            Ok(stats.cbSize)
        }
    }
}

impl From<IStream> for WinStream {
    fn from(stream: IStream) -> Self {
        Self { stream }
    }
}

impl io::Read for WinStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut bytes_read = 0u32;
        unsafe {
            self.stream
                .Read(buf.as_mut_ptr() as _, buf.len() as u32, &mut bytes_read)
        }
        .map_err(|err| {
            std::io::Error::new(
                io::ErrorKind::Other,
                format!("IStream::Read failed: {}", err.code().0),
            )
        })?;
        Ok(bytes_read as usize)
    }
}

pub unsafe fn create_argb_bitmap(
    width: u32,
    height: u32,
    p_bits: &mut *mut core::ffi::c_void,
) -> HBITMAP {
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    CreateDIBSection(
        core::mem::zeroed::<HDC>(),
        &bmi,
        DIB_RGB_COLORS,
        p_bits,
        core::mem::zeroed::<windows::Win32::Foundation::HANDLE>(),
        0,
    )
}

pub unsafe fn get_jumbo_icon(extension: &str) -> Option<HICON> {
    // 1. 获取 Jumbo ImageList (256x256)
    // SHGetImageList expects: (iImageList: i32, riid: REFIID, ppv: *mut *mut c_void)
    let mut image_list: Option<IImageList> = None;
    let riid = IImageList::IID;
    
    let hr = SHGetImageList(SHIL_JUMBO as i32, &riid, &mut image_list as *mut _ as *mut *mut _);
    
    if hr.is_ok() {
        if let Some(image_list) = image_list {
            // 2. 获取图标索引
            let ext = if extension.starts_with('.') { extension.to_string() } else { format!(".{}", extension) };
            let mut ext_wide: Vec<u16> = OsStr::new(&ext).encode_wide().chain(std::iter::once(0)).collect();
            let pcwstr = PCWSTR(ext_wide.as_ptr());
            
            let mut shfi = SHFILEINFOW::default();
            // SHGetFileInfoW signature in this version of windows-rs might be simpler or different?
            // Let's try raw pointer if Option<&mut> fails, but wait, the error said: expected *mut SHFILEINFOW, found Option.
            // This means we should pass `&mut shfi as *mut _` directly WITHOUT wrapping in Some().
            
            let res = SHGetFileInfoW(
                pcwstr,
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0x80), // FILE_ATTRIBUTE_NORMAL
                &mut shfi as *mut SHFILEINFOW,
                mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_SYSICONINDEX | SHGFI_USEFILEATTRIBUTES,
            );
    
            if res != 0 {
                // 3. 提取图标
                let hicon: Result<HICON, _> = image_list.GetIcon(shfi.iIcon, 1);
                if let Ok(icon) = hicon { // ILD_TRANSPARENT = 1
                    return Some(icon);
                }
            }
        }
    }
    None
}
