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
            IThumbnailProvider_Impl, PropertiesSystem::IInitializeWithFile_Impl, WTSAT_ARGB,
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
        // Logging
        let temp_log = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
        use std::io::Write;
        
        let unknown: IUnknown = ThumbnailFileHandler {
            filepath: Cell::new(String::new()),
            backend,
        }
        .into();
        
        let result = unsafe { unknown.query(&*riid, ppv_object) };
        
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
             let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] new() called for IID: {:?}. Query result: {:?}", std::process::id(), unsafe { &*riid }, result);
        }
        
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
        let temp_log = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
        
        // Log start
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
            let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] GetThumbnail called for .step/.stp", std::process::id());
        }

        // Get file path from Cell
        let path_str = self.filepath.take();
        self.filepath.set(path_str.clone()); // put it back immediately

        if path_str.is_empty() {
             return Err(windows::core::Error::from(E_FAIL));
        }

        // Determine CLI path
        let base_path = get_base_path().unwrap_or_else(|| std::path::PathBuf::from("."));
        let cli_path = base_path.join("space-thumbnails-cli.exe");

        if !cli_path.exists() {
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] CLI not found at {:?}", std::process::id(), cli_path);
            }
            // Fallback to loading.png if CLI is missing? No, user wants real conversion.
            return Err(windows::core::Error::from(E_FAIL));
        }

        // Output path
        let output_path = std::env::temp_dir().join(format!("thumb_{}.png", uuid::Uuid::new_v4()));

        // Run CLI
        let mut cmd = Command::new(&cli_path);
        
        // Windows-specific: hide console window
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.arg("--input").arg(&path_str)
               .arg(&output_path) // Positional argument for output
               .arg("--width").arg(cx.to_string())
               .arg("--height").arg(cy.to_string())
               .arg("--api").arg("default");

        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
            let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Running CLI: {:?} input={:?} output={:?}", std::process::id(), cli_path, path_str, output_path);
        }

        // Execute
        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                    let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Failed to execute CLI: {:?}", std::process::id(), e);
                }
                return Err(windows::core::Error::from(E_FAIL));
            }
        };

        if !output.status.success() {
             if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] CLI failed. Exit code: {:?}. Stderr: {}", std::process::id(), output.status.code(), String::from_utf8_lossy(&output.stderr));
            }
            return Err(windows::core::Error::from(E_FAIL));
        }

        // Load image
        if let Ok(img) = image::open(&output_path) {
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Image generated successfully.", std::process::id());
            }
            
            // Resize if needed (CLI should have handled it, but double check or just load)
            // CLI outputs exactly width/height if possible.
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
                                
                                // BGRA for Windows Bitmap
                                (p_bits_u8.add(index) as *mut u32).write(
                                    (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32,
                                );
                            }
                        }
                    }
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                    
                    // Cleanup
                    let _ = std::fs::remove_file(&output_path);

                    return Ok(());
                } else {
                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                        let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Failed to create ARGB bitmap", std::process::id());
                    }
                }
            }
        } else {
             if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
                let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Failed to open generated image at {:?}", std::process::id(), output_path);
            }
        }

        Err(windows::core::Error::from(E_FAIL))
    }
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile_Impl for ThumbnailFileHandler {
    fn Initialize(
        &self,
        pszfilepath: &windows::core::PCWSTR,
        _grfmode: u32,
    ) -> windows::core::Result<()> {
        let filepath = unsafe { 
            let str_p = pszfilepath.0;
            let mut str_len = 0;
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
        
        // Log
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(r"C:\Users\Public\space_thumbnails_debug.log") {
             let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Initialize(File) called with {:?}", std::process::id(), filepath);
        }

        self.filepath.set(filepath);
        Ok(())
    }
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream_Impl for ThumbnailFileHandler {
    fn Initialize(&self, _pstream: &Option<windows::Win32::System::Com::IStream>, _grfmode: u32) -> windows::core::Result<()> {
        // Log stream request
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(r"C:\Users\Public\space_thumbnails_debug.log") {
             let _ = writeln!(file, "[ThumbnailFileHandler] [PID:{}] Initialize(Stream) called (returning E_NOTIMPL)", std::process::id());
        }
        Err(windows::core::Error::from(windows::Win32::Foundation::E_NOTIMPL))
    }
}
