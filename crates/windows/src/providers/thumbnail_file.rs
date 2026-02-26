use std::{
    cell::Cell,
    io::Write, // Add Write trait
};

use space_thumbnails::get_base_path;
// use space_thumbnails::RendererBackend;
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::{E_FAIL, CloseHandle},
        Graphics::Gdi::HBITMAP,
        UI::Shell::{
            IThumbnailProvider_Impl,
            WTSAT_ARGB,
            WTS_ALPHATYPE,
        },
        System::{
            Threading::{OpenProcess, GetExitCodeProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE},
        },
    },
};

const E_PENDING: windows::core::HRESULT = windows::core::HRESULT(0x8000000A_u32 as i32);

fn is_process_running(pid: u32) -> bool {
    const STILL_ACTIVE: u32 = 259;
    unsafe {
        // Based on compiler error, OpenProcess returns HANDLE directly in this environment.
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid);
        
        if !handle.is_invalid() {
             let mut exit_code = 0;
             if GetExitCodeProcess(handle, &mut exit_code).as_bool() {
                 let _ = CloseHandle(handle);
                 return exit_code == STILL_ACTIVE;
             }
             let _ = CloseHandle(handle);
        }
    }
    false
}

use crate::{
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, get_cache_path},
};

use std::process::Command;
use std::os::windows::process::CommandExt;

use super::Provider;

pub struct ThumbnailFileProvider {
    pub clsid: GUID,
    pub file_extension: &'static str,
}

impl ThumbnailFileProvider {
    pub fn new(clsid: GUID, file_extension: &'static str) -> Self {
        Self {
            clsid,
            file_extension,
        }
    }
}

impl Provider for ThumbnailFileProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, true);
        result.push(RegistryKey {
            path: format!(
                "{}\\ShellEx\\{{{:?}}}",
                self.file_extension,
                windows::Win32::UI::Shell::IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
            )],
        });
        result
    }

    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        ThumbnailFileHandler::new(riid, ppv_object)
    }
}


fn log_debug(msg: &str) {
    let log_path = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "[ThumbnailFileProvider] [PID:{}] {}", std::process::id(), msg);
    }
}

#[allow(unused_must_use)]
#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile
)]
pub struct ThumbnailFileHandler {
    filepath: Cell<String>,
}

impl Drop for ThumbnailFileHandler {
    fn drop(&mut self) {
        let temp_log = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
             let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Dropped", std::process::id());
        }
    }
}

impl ThumbnailFileHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        // Logging - DISABLED for performance
        // let temp_log = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
        
        let unknown: IUnknown = ThumbnailFileHandler {
            filepath: Cell::new(String::new()),
        }
        .into();
        
        let result = unsafe { unknown.query(&*riid, ppv_object) };
        
        // if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
        //      let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] new() called for IID: {:?}. Query result: {:?}", std::process::id(), unsafe { &*riid }, result);
        // }
        
        result.ok()
    }
}

impl IThumbnailProvider_Impl for ThumbnailFileHandler {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let cy = cx;

        // Get file path from Cell
        let path_str = self.filepath.take();
        self.filepath.set(path_str.clone()); // put it back immediately

        log_debug(&format!("GetThumbnail called for cx={}, file={:?}", cx, path_str));

        if path_str.is_empty() {
             log_debug("GetThumbnail: No file path");
             return Err(windows::core::Error::from(E_FAIL));
        }

        let file_path = std::path::PathBuf::from(&path_str);
        
        let cache_file = if let Some(path) = get_cache_path(&file_path) {
             path
        } else {
             log_debug("GetThumbnail: Failed to get cache path");
             return Err(windows::core::Error::from(E_FAIL));
        };
        
        // Ensure lock_file is defined as expected by subsequent code
        let lock_file = cache_file.with_extension("lock");
        
        log_debug(&format!("Cache path: {:?}", cache_file));

        if cache_file.exists() {
            log_debug("Cache hit");
            
            // Load from cache
            match image::open(&cache_file) {
                Ok(img) => {
                     // Reuse image loading logic
                     let img = img.resize_exact(cx, cy, image::imageops::FilterType::Lanczos3);
                     let rgba = img.to_rgba8();
                     let buffer = rgba.as_raw();
                     
                     unsafe {
                        let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                        let hbmp = create_argb_bitmap(cx, cy, &mut p_bits);
                        
                        if hbmp.0 != 0 && !p_bits.is_null() {
                            let p_bits_u8 = p_bits as *mut u8;
                            for x in 0..cx {
                                for y in 0..cy {
                                    let index = ((y * cx + x) * 4) as usize;
                                    if index + 3 < buffer.len() {
                                        let r = buffer[index];
                                        let g = buffer[index + 1];
                                        let b = buffer[index + 2];
                                        let a = buffer[index + 3];
                                        
                                        // BGRA
                                        let offset = ((cy - 1 - y) * cx + x) as isize * 4;
                                        *p_bits_u8.offset(offset) = b;
                                        *p_bits_u8.offset(offset + 1) = g;
                                        *p_bits_u8.offset(offset + 2) = r;
                                        *p_bits_u8.offset(offset + 3) = a;
                                    }
                                }
                            }
                            
                            *phbmp = hbmp;
                            *pdwalpha = WTSAT_ARGB;
                            return Ok(());
                        }
                     }
                },
                Err(_e) => {
                    //  if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                    //     let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Failed to load image from cache (corrupt?): {:?}. Deleting...", std::process::id(), e);
                    // }
                    // Delete corrupt file so we can regenerate
                    let _ = std::fs::remove_file(&cache_file);
                }
            }
        }
        
        // === Async Generation Logic ===
        
        // Check if lock file exists (prevent spawn storm)
        if lock_file.exists() {
            let mut is_stale = false;
            let mut process_check_done = false;

            // Try to read PID from lock file to check if process is actually running
            if let Ok(content) = std::fs::read_to_string(&lock_file) {
                // Format is "PID: <number>"
                // We handle potential extra whitespace or newlines
                let content_trim = content.trim();
                if let Some(pid_str) = content_trim.strip_prefix("PID: ") {
                    if let Ok(pid) = pid_str.trim().parse::<u32>() {
                        process_check_done = true;
                        if is_process_running(pid) {
                            // Process is running, so it's definitely NOT stale.
                            // We return E_PENDING to tell OS "not ready yet, don't cache failure/placeholder".
                            log_debug(&format!("Generation in progress (PID: {} running). returning E_PENDING.", pid));
                            return Err(windows::core::Error::from(E_PENDING));
                        } else {
                            // Process is NOT running (dead), so lock IS stale.
                            log_debug(&format!("Lock file exists but PID {} is dead. Treating as stale.", pid));
                            is_stale = true;
                        }
                    }
                }
            }
            
            if !process_check_done {
                // Fallback to time-based check if we couldn't read PID
                // (e.g. old lock file format or read error)
                if let Ok(metadata) = std::fs::metadata(&lock_file) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = std::time::SystemTime::now().duration_since(modified) {
                            // If we can't check PID, we must rely on timeout.
                            // Timeout reduced to 120 seconds (2 minutes) for faster recovery from stale locks
                            if age.as_secs() > 120 {
                                log_debug("Lock file too old (timeout fallback). Treating as stale.");
                                is_stale = true;
                            }
                        }
                    }
                } else {
                    // Can't read metadata? maybe stale or permission issue. 
                    is_stale = true;
                }
            }

            if !is_stale {
                // Generation in progress (and verified running via PID or within timeout).
                log_debug("Generation in progress (Lock exists). returning E_PENDING.");
                return Err(windows::core::Error::from(E_PENDING));
            } else {
                log_debug("Lock file stale (dead PID or timeout). Removing and regenerating.");
                let _ = std::fs::remove_file(&lock_file);
            }
        }

        // Create lock file
        if let Ok(mut f) = std::fs::File::create(&lock_file) {
            let _ = write!(f, "PID: {}", std::process::id());
        }

        // Determine CLI path
        let base_path = get_base_path().unwrap_or_else(|| std::path::PathBuf::from("."));
        let cli_path = base_path.join("space-thumbnails-cli.exe");

        if !cli_path.exists() {
            // ... log error ...
            // if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
            //    let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] CLI not found at {:?}", std::process::id(), cli_path);
            // }
            return Err(windows::core::Error::from(E_FAIL));
        }

        let mut cmd = Command::new(&cli_path);
        
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);

        // Run CLI to write DIRECTLY to cache_file
        cmd.arg(&path_str)
           .arg(&cache_file) 
           .arg("--width").arg("256")
           .arg("--height").arg("256")
           .arg("--api").arg("default")
           .arg("--lock-file").arg(&lock_file);

        // if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
        //    let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Spawning Async CLI: {:?} -> {:?}", std::process::id(), cli_path, cache_file);
        // }

        // Spawn async
        match cmd.spawn() {
            Ok(_) => {
                log_debug("CLI process spawned successfully. Returning E_PENDING.");
                return Err(windows::core::Error::from(E_PENDING));
            },
            Err(e) => {
                log_debug(&format!("Failed to spawn CLI: {:?}", e));
                // Cleanup lock
                let _ = std::fs::remove_file(&lock_file);
                return Err(windows::core::Error::from(E_FAIL));
            }
        }

    }
}

impl ThumbnailFileHandler {
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile_Impl for ThumbnailFileHandler {
    fn Initialize(
        &self,
        pszfilepath: &windows::core::PCWSTR,
        _grfmode: u32,
    ) -> windows::core::Result<()> {
        log_debug("Initialize(File) called");
        
        let filepath = unsafe {
            let mut str_len = 0;
            let str_p = pszfilepath.0;
            loop {
                if *str_p.add(str_len) != 0 {
                    str_len += 1;
                    if str_len > 1024 { break; }
                } else {
                    break;
                }
            }
            if str_len > 0 {
                String::from_utf16_lossy(core::slice::from_raw_parts(str_p, str_len))
            } else {
                String::new()
            }
        };
        
        log_debug(&format!("Initialize(File) called with {:?}", filepath));

        self.filepath.set(filepath);
        Ok(())
    }
}
