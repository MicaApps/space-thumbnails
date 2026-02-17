use std::{
    cell::Cell,
    ffi::OsString,
    fs, io,
    os::windows::prelude::OsStringExt,
    time::{Duration, Instant},
    io::Write, // Add Write trait
};

use log::info;
use space_thumbnails::{RendererBackend, SpaceThumbnailsRenderer, get_base_path};
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::HBITMAP,
        UI::Shell::{
            IThumbnailProvider_Impl,
            PropertiesSystem::{IInitializeWithFile_Impl, IInitializeWithStream_Impl},
            WTSAT_ARGB,
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


fn log_debug(msg: &str) {
    let log_path = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "[ThumbnailFileProvider] [PID:{}] {}", std::process::id(), msg);
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile
)]
pub struct ThumbnailFileHandler {
    filepath: Cell<String>,
    backend: RendererBackend,
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
        backend: RendererBackend,
    ) -> windows::core::Result<()> {
        // Logging - DISABLED for performance
        // let temp_log = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
        
        let unknown: IUnknown = ThumbnailFileHandler {
            filepath: Cell::new(String::new()),
            backend,
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
                Err(e) => {
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
            // Check if stale (older than 5 mins)
            let mut is_stale = false;
            if let Ok(metadata) = std::fs::metadata(&lock_file) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = std::time::SystemTime::now().duration_since(modified) {
                        if age.as_secs() > 300 {
                            is_stale = true;
                        }
                    }
                }
            } else {
                // Can't read metadata? maybe stale or permission issue. 
                // Let's assume stale if we can't read it to avoid blocking forever.
                is_stale = true;
            }

            if !is_stale {
                // if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                //    let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Generation in progress (Lock exists). Returning E_FAIL (Default Icon).", std::process::id());
                // }
                // Return failure so Explorer shows default icon and moves on.
                // When generation finishes, it will notify Explorer to update.
                return Err(windows::core::Error::from(E_FAIL));
            } else {
                //  if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                //    let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Lock file stale. Removing and regenerating.", std::process::id());
                // }
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
        cmd.arg("--input").arg(&path_str)
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
                log_debug("CLI process spawned successfully");
                
                // Return Loading.png
                // Try to find Loading.png relative to the DLL itself, not CWD
                let mut loading_path = std::path::PathBuf::from("Loading.png"); // fallback
                
                // Get DLL path
                use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
                use windows::Win32::Foundation::HINSTANCE;
                
                unsafe {
                    // We need the HINSTANCE of the current module. 
                    // Since we are in a DLL, we can try to get it via a known symbol or just NULL (which gets EXE path, not what we want).
                    // Better approach: use GetModuleHandleExW with GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    
                    use windows::Win32::System::LibraryLoader::{GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT};
                    
                    let mut module_handle = HINSTANCE(0);
                    // Use a function pointer from this file
                    unsafe extern "system" fn dummy_func() {}
                    let func_ptr = dummy_func as *const ();
                    
                    if GetModuleHandleExW(
                        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                        windows::core::PCWSTR(func_ptr as *const u16),
                        &mut module_handle
                    ).as_bool() {
                        let mut buffer = [0u16; 1024];
                        let len = GetModuleFileNameW(module_handle, &mut buffer);
                        if len > 0 {
                            let path_str = String::from_utf16_lossy(&buffer[..len as usize]);
                            let dll_path = std::path::PathBuf::from(path_str);
                            log_debug(&format!("DLL Path: {:?}", dll_path));
                            if let Some(parent) = dll_path.parent() {
                                loading_path = parent.join("Loading.png");
                            }
                        } else {
                             log_debug("GetModuleFileNameW failed");
                        }
                    } else {
                         log_debug("GetModuleHandleExW failed");
                    }
                }

                // Check if loading_path exists
                if !loading_path.exists() {
                     // Try fallback to hardcoded path
                     loading_path = std::path::PathBuf::from(r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target\release\Loading.png");
                }
                
                log_debug(&format!("Trying to load Loading.png from: {:?}", loading_path));
                
                let mut loaded_bitmap = false;

                if loading_path.exists() {
                     if let Ok(img) = image::open(&loading_path) {
                        log_debug("Loading.png opened successfully");
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
                                            let offset = ((y) * cx + x) as isize * 4;
                                            *p_bits_u8.offset(offset) = b;
                                            *p_bits_u8.offset(offset + 1) = g;
                                            *p_bits_u8.offset(offset + 2) = r;
                                            *p_bits_u8.offset(offset + 3) = a;
                                        }
                                    }
                                }
                                
                                *phbmp = hbmp;
                                *pdwalpha = WTSAT_ARGB;
                                log_debug("Returned Loading.png bitmap successfully");
                                loaded_bitmap = true;
                            } else {
                                 log_debug("create_argb_bitmap failed");
                            }
                        }
                     } else {
                          log_debug("image::open(Loading.png) failed");
                     }
                } else {
                     log_debug("Loading.png not found at any path");
                }
                
                if loaded_bitmap {
                    return Ok(());
                }

                // Fallback to solid color if Loading.png fails
                log_debug("Generating solid color fallback");
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(cx, cy, &mut p_bits);
                    
                    if hbmp.0 != 0 && !p_bits.is_null() {
                        let p_bits_u8 = p_bits as *mut u8;
                        // Fill with semi-transparent blue (ARGB)
                        // A: 128, R: 0, G: 120, B: 215
                        for i in 0..(cx * cy) {
                            let offset = (i * 4) as isize;
                            *p_bits_u8.offset(offset) = 215;     // B
                            *p_bits_u8.offset(offset + 1) = 120; // G
                            *p_bits_u8.offset(offset + 2) = 0;   // R
                            *p_bits_u8.offset(offset + 3) = 128; // A
                        }
                         *phbmp = hbmp;
                         *pdwalpha = WTSAT_ARGB;
                         log_debug("Returned solid color fallback");
                         return Ok(());
                    }
                }

                log_debug("Returning E_FAIL (All fallbacks failed)");
                return Err(windows::core::Error::from(E_FAIL));
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
