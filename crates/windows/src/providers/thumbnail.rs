use std::{
    cell::Cell,
    io,
    path::Path,
    ptr,
    time::{Duration, Instant},
};

use log::{info, warn};
use space_thumbnails::plugins::PluginManager;
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::*,
        System::Com::*,
        UI::Shell::{PropertiesSystem::*, *},
        UI::WindowsAndMessaging::{DestroyIcon, DI_NORMAL, DrawIconEx, HICON},
    },
};

use crate::{
    constant::{ERROR_256X256_ARGB, TIMEOUT_256X256_ARGB, TOOLARGE_256X256_ARGB},
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, get_jumbo_icon, run_timeout, WinStream},
};

use super::Provider;

pub struct ThumbnailProvider {
    pub clsid: GUID,
    pub file_extension: &'static str,
}

impl ThumbnailProvider {
    pub fn new(clsid: GUID, file_extension: &'static str) -> Self {
        Self {
            clsid,
            file_extension,
        }
    }
}

impl Provider for ThumbnailProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, false);
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
        ThumbnailHandler::new(self.file_extension, riid, ppv_object)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream
)]
pub struct ThumbnailHandler {
    filename_hint: &'static str,
    stream: Cell<Option<WinStream>>,
}

impl ThumbnailHandler {
    pub fn new(
        filename_hint: &'static str,
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = ThumbnailHandler {
            filename_hint,
            stream: Cell::new(None),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl IThumbnailProvider_Impl for ThumbnailHandler {
    fn GetThumbnail(
        &self,
        _: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let size = 256;
        let mut stream = self
            .stream
            .take()
            .ok_or(windows::core::Error::from(E_FAIL))?;

        let filesize = stream.size()?;
        if filesize > 300 * 1024 * 1024
        /* 300 MB */
        {
            unsafe {
                let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                std::ptr::copy(
                    TOOLARGE_256X256_ARGB.as_ptr(),
                    p_bits as *mut _,
                    TOOLARGE_256X256_ARGB.len(),
                );
                phbmp.write(hbmp);
                pdwalpha.write(WTSAT_ARGB);
            }
            return Ok(());
        }

        let start_time = Instant::now();
        info!(target: "ThumbnailProvider", "Getting thumbnail from stream [{}], size: {}", self.filename_hint, filesize);

        let mut buffer = Vec::new();
        io::Read::read_to_end(&mut stream, &mut buffer)
            .ok()
            .ok_or(windows::core::Error::from(E_FAIL))?;

        let filename_hint = self.filename_hint;

        let timeout_result = run_timeout(
            move || {
                let manager = PluginManager::new();
                let ext = filename_hint.trim_start_matches('.');
                let header = if buffer.len() > 20 { &buffer[0..20] } else { &buffer };

                if let Some(generator) = manager.get_generator(header, ext) {
                     // Special case for TextGenerator: we want to overlay on default icon
                     if generator.name() == "Text Renderer" {
                         let text_bitmap = generator.generate(Some(buffer.as_slice()), size, size, ext, None).ok();
                         if let Some(tb) = text_bitmap {
                            unsafe {
                                let mut final_buffer = vec![0u8; (size * size * 4) as usize];
                                
                                // 1. Draw Default Icon (Jumbo) to temp buffer
                                if let Some(hicon) = get_jumbo_icon(ext) {
                                     let mut p_icon_bits: *mut core::ffi::c_void = ptr::null_mut();
                                     let h_icon_bmp = create_argb_bitmap(size, size, &mut p_icon_bits);
                                     
                                     let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(windows::Win32::Foundation::HWND(0));
                                     let hdc_mem = CreateCompatibleDC(hdc_screen);
                                     let h_old_obj = SelectObject(hdc_mem, h_icon_bmp);
                                     
                                     DrawIconEx(hdc_mem, 0, 0, hicon, size as i32, size as i32, 0, None, DI_NORMAL);
                                     
                                     // Flush GDI
                                     SelectObject(hdc_mem, h_old_obj);
                                     DeleteDC(hdc_mem);
                                     windows::Win32::Graphics::Gdi::ReleaseDC(windows::Win32::Foundation::HWND(0), hdc_screen);
                                     DestroyIcon(hicon);
                                     
                                     // Read back pixels
                                     let icon_slice = std::slice::from_raw_parts(p_icon_bits as *const u8, final_buffer.len());
                                     final_buffer.copy_from_slice(icon_slice);
                                     
                                     // GDI bitmap is BGRA. Convert to RGBA for blending.
                                     for i in (0..final_buffer.len()).step_by(4) {
                                         let b = final_buffer[i];
                                         let r = final_buffer[i+2];
                                         final_buffer[i] = r;
                                         final_buffer[i+2] = b;
                                     }
                                     
                                     DeleteObject(h_icon_bmp);
                                }
                                
                                // 2. Blend Text (RGBA) over Icon (RGBA)
                                for i in (0..final_buffer.len()).step_by(4) {
                                    let text_a = tb[i+3] as u32;
                                    if text_a > 0 {
                                        let text_r = tb[i] as u32;
                                        let text_g = tb[i+1] as u32;
                                        let text_b = tb[i+2] as u32;
                                        
                                        let bg_r = final_buffer[i] as u32;
                                        let bg_g = final_buffer[i+1] as u32;
                                        let bg_b = final_buffer[i+2] as u32;
                                        let bg_a = final_buffer[i+3] as u32;
                                        
                                        // Standard alpha blending
                                        let out_a = text_a + ((bg_a * (255 - text_a)) / 255);
                                        if out_a > 0 {
                                            final_buffer[i] = ((text_r * text_a + bg_r * bg_a * (255 - text_a) / 255) / out_a) as u8;
                                            final_buffer[i+1] = ((text_g * text_a + bg_g * bg_a * (255 - text_a) / 255) / out_a) as u8;
                                            final_buffer[i+2] = ((text_b * text_a + bg_b * bg_a * (255 - text_a) / 255) / out_a) as u8;
                                            final_buffer[i+3] = out_a as u8;
                                        }
                                    }
                                }
                                
                                return Some(final_buffer);
                            }
                         }
                     }
                     
                     generator.generate(Some(buffer.as_slice()), size, size, ext, None).ok()
                } else {
                     warn!(target: "ThumbnailProvider", "No generator found for extension: {}", ext);
                     None
                }
            },
            Duration::from_secs(5),
        );

        match timeout_result {
            Ok(Some(screenshot_buffer)) => {
                info!(target: "ThumbnailProvider", "Rendering thumbnails success [{}], Elapsed: {:.2?}", self.filename_hint, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(size, size, &mut p_bits);
                    for x in 0..size {
                        for y in 0..size {
                            let index = ((x * size + y) * 4) as usize;
                            let r = screenshot_buffer[index];
                            let g = screenshot_buffer[index + 1];
                            let b = screenshot_buffer[index + 2];
                            let a = screenshot_buffer[index + 3];
                            (p_bits.add(((x * size + y) * 4) as usize) as *mut u32).write(
                                (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32,
                            )
                        }
                    }
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
            Err(err) if err.kind() == io::ErrorKind::TimedOut => {
                warn!(target: "ThumbnailProvider", "Rendering thumbnails timeout [{}], Elapsed: {:.2?}", self.filename_hint, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                    std::ptr::copy(
                        TIMEOUT_256X256_ARGB.as_ptr(),
                        p_bits as *mut _,
                        TIMEOUT_256X256_ARGB.len(),
                    );
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
            Err(_) | Ok(None) => {
                warn!(target: "ThumbnailProvider", "Rendering thumbnails error [{}], Elapsed: {:.2?}", self.filename_hint, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                    std::ptr::copy(
                        ERROR_256X256_ARGB.as_ptr(),
                        p_bits as *mut _,
                        ERROR_256X256_ARGB.len(),
                    );
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
        }
    }
}

impl IInitializeWithStream_Impl for ThumbnailHandler {
    fn Initialize(&self, pstream: &Option<IStream>, _grfmode: u32) -> windows::core::Result<()> {
        if let Some(stream) = pstream {
            self.stream.set(Some(WinStream::from(stream.to_owned())));
            Ok(())
        } else {
            Err(E_FAIL.into())
        }
    }
}
