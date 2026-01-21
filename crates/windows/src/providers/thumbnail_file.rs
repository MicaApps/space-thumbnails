use std::{
    cell::Cell,
    ffi::OsString,
    fs, io,
    os::windows::prelude::OsStringExt,
    time::{Duration, Instant},
};

use log::info;
use space_thumbnails::RendererBackend;
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::HBITMAP,
        UI::Shell::{
            IThumbnailProvider_Impl, PropertiesSystem::{IInitializeWithFile_Impl, IInitializeWithStream, IInitializeWithStream_Impl}, WTSAT_ARGB,
            WTS_ALPHATYPE,
        },
        System::Com::{IStream, STREAM_SEEK_SET},
    },
};

use crate::{
    constant::{ERROR_256X256_ARGB, TIMEOUT_256X256_ARGB, TOOLARGE_256X256_ARGB, LOADING_256X256_ARGB},
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, run_timeout, get_cache_path},
};

use std::process::Command;
use std::os::windows::process::CommandExt;
use std::path::Path;

use super::Provider;

pub struct ThumbnailFileProvider {
    pub clsid: GUID,
    pub file_extension: &'static str,
    pub backend: RendererBackend,
}

impl ThumbnailFileProvider {
    pub fn new(clsid: GUID, file_extension: &'static str, backend: RendererBackend) -> Self {
        Self {
            clsid,
            file_extension,
            backend,
        }
    }
}

impl Provider for ThumbnailFileProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, true);
        result.append(&mut vec![RegistryKey {
            path: format!(
                "{}\\ShellEx\\{{{:?}}}",
                self.file_extension,
                windows::Win32::UI::Shell::IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
            )],
        }]);
        result
    }

    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        ThumbnailFileHandler::new(riid, ppv_object, self.backend)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream
)]
pub struct ThumbnailFileHandler {
    filepath: Cell<String>,
    backend: RendererBackend,
}

impl ThumbnailFileHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
        backend: RendererBackend,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = ThumbnailFileHandler {
            filepath: Cell::new(String::new()),
            backend,
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl IThumbnailProvider_Impl for ThumbnailFileHandler {
    fn GetThumbnail(
        &self,
        _: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let filepath = self.filepath.take();
        let size = 256;

        if filepath.is_empty() {
            return Err(windows::core::Error::from(E_FAIL));
        }

        // Write logs to file for debugging
        use std::io::Write;
        let log_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\st_debug.log";
        
        let start_time = Instant::now();
        info!(target: "ThumbnailFileProvider", "Getting thumbnail for file: {}", filepath);

        // 1. Check Cache
        let cache_path = get_cache_path(Path::new(&filepath));
        if let Some(path) = &cache_path {
            if path.exists() {
                if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                    let _ = writeln!(file, "Cache hit: {:?} for {}", path, filepath);
                }
                
                if let Ok(img) = image::open(path) {
                    let img = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
                    let rgba = img.to_rgba8();
                    let buffer = rgba.as_raw();

                    unsafe {
                        let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                        let hbmp = create_argb_bitmap(size, size, &mut p_bits);
                        
                        if hbmp.0 != 0 && !p_bits.is_null() {
                            for x in 0..size {
                                for y in 0..size {
                                    // image crate is (x, y) where x is width, y is height.
                                    // buffer is row-major: y * width + x.
                                    // p_bits expect same?
                                    // Let's assume standard layout.
                                    let index = ((y * size + x) * 4) as usize;
                                    // Check bounds
                                    if index + 3 < buffer.len() {
                                        let r = buffer[index];
                                        let g = buffer[index + 1];
                                        let b = buffer[index + 2];
                                        let a = buffer[index + 3];
                                        
                                        // GDI expects BGRA or ARGB? 
                                        // Previous code: (a << 24) | (r << 16) | (g << 8) | b
                                        // This is ARGB in register, so in memory (Little Endian): B G R A
                                        (p_bits.add(index) as *mut u32).write(
                                            (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32,
                                        );
                                    }
                                }
                            }
                            phbmp.write(hbmp);
                            pdwalpha.write(WTSAT_ARGB);
                            return Ok(());
                        }
                    }
                }
            }
        }

        // 2. Cache Miss - Spawn Background Process
        if let Some(path) = &cache_path {
            if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                let _ = writeln!(file, "Cache miss. Spawning background process for: {}", filepath);
            }

            // Determine executable path
            // Priority:
            // 1. D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\target\release\space-thumbnails-cli.exe (Dev env)
            // 2. space-thumbnails-cli.exe (PATH)
            let dev_exe = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\target\release\space-thumbnails-cli.exe";
            let exe = if Path::new(dev_exe).exists() {
                dev_exe
            } else {
                "space-thumbnails-cli.exe"
            };

            let mut cmd = Command::new(exe);
            cmd.arg(path.to_string_lossy().to_string()) // Output first
               .arg("--input")
               .arg(&filepath)
               .arg("--width")
               .arg(size.to_string())
               .arg("--height")
               .arg(size.to_string())
               .arg("--api")
               .arg(match self.backend {
                   RendererBackend::OpenGL => "open-gl",
                   RendererBackend::Vulkan => "vulkan",
                   RendererBackend::Metal => "metal",
                   _ => "default",
               });
            
            // Detach process
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
            
            match cmd.spawn() {
                Ok(_) => {
                    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                        let _ = writeln!(file, "Spawned: {:?} outputting to {:?}", exe, path);
                    }
                },
                Err(e) => {
                    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                        let _ = writeln!(file, "Failed to spawn {:?}: {:?}", exe, e);
                    }
                }
            }
        }

        // 3. Return Placeholder (Loading) immediately
        unsafe {
            let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
            let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
            
            // Use LOADING image as "Processing" placeholder
            std::ptr::copy_nonoverlapping(
                LOADING_256X256_ARGB.as_ptr(),
                p_bits as *mut u8,
                LOADING_256X256_ARGB.len(),
            );
            
            phbmp.write(hbmp);
            pdwalpha.write(WTSAT_ARGB);
        }
        
        Ok(())
    }
}

impl IInitializeWithStream_Impl for ThumbnailFileHandler {
    fn Initialize(
        &self,
        pstream: &Option<windows::Win32::System::Com::IStream>,
        _grfmode: u32,
    ) -> windows::core::Result<()> {
        // Write debug log immediately to confirm this method is called
        use std::io::Write;
        let log_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\st_debug.log";
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
             let _ = writeln!(file, "[{:?}] IInitializeWithStream called!", std::time::SystemTime::now());
        }

        if let Some(stream) = pstream {
             // Read stream to temporary file
             // We need a temporary file because our Renderer expects a path
             let temp_dir = std::env::temp_dir();
             let temp_file_path = temp_dir.join(format!("st_temp_{}.step", uuid::Uuid::new_v4()));
             
             // Get stream size
             let mut stat = windows::Win32::System::Com::STATSTG::default();
             unsafe { stream.Stat(&mut stat, 0)?; }
             let size = stat.cbSize;
             
             if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                 let _ = writeln!(file, "Stream size: {}", size);
             }

             if size > 300 * 1024 * 1024 {
                 if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                     let _ = writeln!(file, "Stream too large (>300MB), skipping.");
                 }
                 return Err(windows::core::Error::from(E_FAIL));
             }

             // Seek to beginning
             unsafe {
                 stream.Seek(0, STREAM_SEEK_SET)?;
             }

             // Read content
             let mut bytes = vec![0u8; size as usize];
             let mut bytes_read = 0u32;
             unsafe {
                 stream.Read(bytes.as_mut_ptr() as *mut _, size as u32, &mut bytes_read as *mut _)?;
             }
             
             if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                 let _ = writeln!(file, "Bytes read from stream: {}", bytes_read);
                 if bytes_read > 20 {
                     // Sanitize header for logging to avoid confusing text editors (prevent UTF-16 detection)
                     let header_bytes = &bytes[0..20];
                     let header_hex: Vec<String> = header_bytes.iter().map(|b| format!("{:02X}", b)).collect();
                     let header_safe: String = header_bytes.iter()
                         .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                         .collect();
                     let _ = writeln!(file, "File header (first 20 bytes): {} [{}]", header_safe, header_hex.join(" "));
                 }
             }
             
             // Write to temp file
             if let Err(e) = fs::write(&temp_file_path, &bytes[0..bytes_read as usize]) {
                 if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                     let _ = writeln!(file, "Failed to write temp file: {:?}", e);
                 }
                 return Err(windows::core::Error::from(E_FAIL));
             }

             let path_str = temp_file_path.to_string_lossy().to_string();
             if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                 let _ = writeln!(file, "Stream saved to temp file: {}", path_str);
             }
             
             self.filepath.set(path_str);
             Ok(())
        } else {
             Err(windows::core::Error::from(E_FAIL))
        }
    }
}

impl IInitializeWithFile_Impl for ThumbnailFileHandler {
    fn Initialize(
        &self,
        pszfilepath: &windows::core::PCWSTR,
        _grfmode: u32,
    ) -> windows::core::Result<()> {
        let filepath = unsafe {
            let str_p = pszfilepath.0;
            let mut str_len = 0;
            loop {
                if str_p.add(str_len).read() != 0 {
                    str_len += 1;
                    if str_len > 1024 {
                        return Err(E_FAIL.into());
                    }
                    continue;
                } else {
                    break;
                }
            }
            if str_len > 0 {
                OsString::from_wide(core::slice::from_raw_parts(str_p, str_len))
                    .to_str()
                    .map(|s| s.to_owned())
            } else {
                None
            }
        };
        if let Some(filepath) = filepath {
            self.filepath.set(filepath);
            Ok(())
        } else {
            Err(E_FAIL.into())
        }
    }
}
