use std::{cell::Cell, io::Read};
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::HBITMAP,
        UI::Shell::{IThumbnailProvider_Impl, WTSAT_ARGB},
    },
};
use image::GenericImageView;
use zip::ZipArchive;

use crate::{
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, WinStream},
};

use super::Provider;

pub struct IWorkThumbnailProvider {
    pub clsid: GUID,
}

impl IWorkThumbnailProvider {
    pub fn new(clsid: GUID) -> Self {
        Self { clsid }
    }
}

impl Provider for IWorkThumbnailProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, false);
        
        let extensions = vec![".pages", ".numbers", ".key"];
        
        for ext in extensions {
            // Register under extension
            result.push(RegistryKey {
                path: ext.to_owned(),
                values: vec![
                    RegistryValue(
                        "PerceivedType".to_owned(),
                        RegistryData::Str("Document".to_owned()),
                    ),
                    RegistryValue(
                        "Content Type".to_owned(),
                        RegistryData::Str("application/zip".to_owned()),
                    ),
                ],
            });

            // Register under extension\ShellEx
            result.push(RegistryKey {
                path: format!(
                    "{}\\ShellEx\\{{{:?}}}",
                    ext,
                    windows::Win32::UI::Shell::IThumbnailProvider::IID
                ),
                values: vec![RegistryValue(
                    "".to_owned(),
                    RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
                )],
            });

            // Register under SystemFileAssociations\extension to bypass UserChoice/ProgID overrides
            result.push(RegistryKey {
                path: format!(
                    "SystemFileAssociations\\{}\\ShellEx\\{{{:?}}}",
                    ext,
                    windows::Win32::UI::Shell::IThumbnailProvider::IID
                ),
                values: vec![RegistryValue(
                    "".to_owned(),
                    RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
                )],
            });
        }
        
        result
    }

    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        IWorkThumbnailHandler::new(riid, ppv_object)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream
)]
pub struct IWorkThumbnailHandler {
    stream: Cell<Option<WinStream>>,
}

impl IWorkThumbnailHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = IWorkThumbnailHandler {
            stream: Cell::new(None),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream_Impl for IWorkThumbnailHandler {
    fn Initialize(
        &self,
        pstream: &Option<windows::Win32::System::Com::IStream>,
        _: u32,
    ) -> windows::core::Result<()> {
        if let Some(stream) = pstream {
            self.stream.set(Some(WinStream::from(stream.clone())));
            Ok(())
        } else {
            Err(windows::core::Error::from(E_FAIL))
        }
    }
}

impl IThumbnailProvider_Impl for IWorkThumbnailHandler {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut windows::Win32::UI::Shell::WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        crate::log_msg(&format!("iWork GetThumbnail called with cx={}", cx));
        
        // 1. Get the stream
        let mut stream = match self.stream.take() {
            Some(s) => s,
            None => {
                crate::log_msg("iWork Error: Stream is None");
                return Err(windows::core::Error::from(E_FAIL));
            }
        };

        // 2. Open ZIP from stream
        // WinStream implements Read + Seek, so it should work with ZipArchive
        let mut archive = match ZipArchive::new(&mut stream) {
            Ok(a) => a,
            Err(e) => {
                crate::log_msg(&format!("iWork Error: ZipArchive::new failed: {:?}", e));
                return Err(windows::core::Error::from(E_FAIL));
            }
        };

        // 3. Find preview image
        // Priority: preview.jpg > QuickLook/Thumbnail.jpg > preview-web.jpg
        let preview_names = [
            "preview.jpg",
            "QuickLook/Thumbnail.jpg",
            "preview-web.jpg",
        ];

        let mut cover_data = Vec::new();
        let mut found = false;
        let mut found_name = String::new();

        for name in preview_names {
            // Need to handle potential case sensitivity issues or directory structure variations?
            // ZipArchive::by_name is case sensitive.
            if let Ok(mut file) = archive.by_name(name) {
                crate::log_msg(&format!("iWork: Found preview image: {}", name));
                if let Err(e) = file.read_to_end(&mut cover_data) {
                    crate::log_msg(&format!("iWork Error: Failed to read {}: {:?}", name, e));
                    continue;
                }
                found = true;
                found_name = name.to_string();
                break;
            }
        }

        if !found {
            crate::log_msg("iWork Error: No preview image found in ZIP. Checked: preview.jpg, QuickLook/Thumbnail.jpg, preview-web.jpg");
            // List files for debugging
            crate::log_msg("Listing files in archive:");
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    crate::log_msg(&format!("  - {}", file.name()));
                }
            }
            return Err(windows::core::Error::from(E_FAIL));
        }

        crate::log_msg(&format!("iWork Cover extracted from {}, size: {} bytes", found_name, cover_data.len()));

        // 4. Load image
        let img = match image::load_from_memory(&cover_data) {
            Ok(i) => i,
            Err(e) => {
                crate::log_msg(&format!("iWork Error: image::load_from_memory failed: {:?}", e));
                return Err(windows::core::Error::from(E_FAIL));
            }
        };

        // 5. Resize logic
        let (orig_w, orig_h) = (img.width(), img.height());
        
        // User requirement: Long side must be 216px (under 256px context)
        // Ratio = 216 / 256 = 0.84375
        let scale_factor = 216.0 / 256.0;
        let target_long_side = (cx as f32 * scale_factor) as u32;
        let target_long_side = if target_long_side == 0 { 1 } else { target_long_side };

        let (new_w, new_h) = if orig_w > orig_h {
            // Landscape
            let ratio = target_long_side as f32 / orig_w as f32;
            (target_long_side, (orig_h as f32 * ratio) as u32)
        } else {
            // Portrait or square
            let ratio = target_long_side as f32 / orig_h as f32;
            ((orig_w as f32 * ratio) as u32, target_long_side)
        };
        
        let new_w = if new_w == 0 { 1 } else { new_w };
        let new_h = if new_h == 0 { 1 } else { new_h };

        let resized_img = image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Lanczos3);

        // 6. Create canvas (cx x cx)
        let canvas_size = cx;
        let mut canvas = image::RgbaImage::new(canvas_size, canvas_size);

        // Center the image
        let x = (canvas_size - new_w) / 2;
        let y = (canvas_size - new_h) / 2;

        image::imageops::overlay(&mut canvas, &resized_img, x as i64, y as i64);

        // 7. Convert to ARGB for Windows
        let (w, h) = canvas.dimensions();
        
        unsafe {
             let mut p_bits: *mut core::ffi::c_void = std::ptr::null_mut();
             let hbmp = create_argb_bitmap(w, h, &mut p_bits);
             if hbmp.is_invalid() {
                 crate::log_msg("iWork Error: create_argb_bitmap failed");
                 return Err(windows::core::Error::from(E_FAIL));
             }
             
             let dest_slice = std::slice::from_raw_parts_mut(p_bits as *mut u8, (w * h * 4) as usize);
             
             for (i, pixel) in canvas.pixels().enumerate() {
                 let r = pixel[0] as u32;
                 let g = pixel[1] as u32;
                 let b = pixel[2] as u32;
                 let a = pixel[3] as u32;
                 
                 let offset = i * 4;
                 // BGRA
                 dest_slice[offset] = b as u8;
                 dest_slice[offset + 1] = g as u8;
                 dest_slice[offset + 2] = r as u8;
                 dest_slice[offset + 3] = a as u8;
             }
             
             *phbmp = hbmp;
             *pdwalpha = WTSAT_ARGB;
        }

        crate::log_msg("iWork Thumbnail generation successful");
        Ok(())
    }
}
