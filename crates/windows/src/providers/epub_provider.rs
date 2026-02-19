use std::{cell::Cell, io::{Read, Seek}};
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::HBITMAP,
        UI::Shell::{IThumbnailProvider_Impl, WTSAT_ARGB},
    },
};
use image::GenericImageView;
use epub::doc::EpubDoc;

use crate::{
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, WinStream},
};

use super::Provider;

pub struct EpubThumbnailProvider {
    pub clsid: GUID,
}

impl EpubThumbnailProvider {
    pub fn new(clsid: GUID) -> Self {
        Self { clsid }
    }
}

impl Provider for EpubThumbnailProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, false);
        
        // Register under .epub
        result.push(RegistryKey {
            path: ".epub".to_owned(),
            values: vec![
                RegistryValue(
                    "PerceivedType".to_owned(),
                    RegistryData::Str("Document".to_owned()),
                ),
                RegistryValue(
                    "Content Type".to_owned(),
                    RegistryData::Str("application/epub+zip".to_owned()),
                ),
            ],
        });

        // Register under .epub\ShellEx
        result.push(RegistryKey {
            path: format!(
                ".epub\\ShellEx\\{{{:?}}}",
                windows::Win32::UI::Shell::IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
            )],
        });

        // Register under SystemFileAssociations\.epub to bypass UserChoice/ProgID overrides
        result.push(RegistryKey {
            path: format!(
                "SystemFileAssociations\\.epub\\ShellEx\\{{{:?}}}",
                windows::Win32::UI::Shell::IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
            )],
        });

        // Register under the known AppX ProgID for EPUB (Edge/Books) to force override
        // AppXvepbp3z66accmsd0x877zbbxjctkpr6t
        result.push(RegistryKey {
            path: format!(
                "AppXvepbp3z66accmsd0x877zbbxjctkpr6t\\ShellEx\\{{{:?}}}",
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
        EpubThumbnailHandler::new(riid, ppv_object)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream
)]
pub struct EpubThumbnailHandler {
    stream: Cell<Option<WinStream>>,
}

impl EpubThumbnailHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = EpubThumbnailHandler {
            stream: Cell::new(None),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream_Impl for EpubThumbnailHandler {
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

impl IThumbnailProvider_Impl for EpubThumbnailHandler {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut windows::Win32::UI::Shell::WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        crate::log_msg(&format!("EPUB GetThumbnail called with cx={}", cx));
        
        // 1. Get the stream
        let mut stream = match self.stream.take() {
            Some(s) => s,
            None => {
                crate::log_msg("EPUB Error: Stream is None");
                return Err(windows::core::Error::from(E_FAIL));
            }
        };

        // 2. Open EPUB from stream
        // EpubDoc::from_reader requires Read + Seek, which WinStream now implements
        let mut doc = match EpubDoc::from_reader(&mut stream) {
            Ok(d) => d,
            Err(e) => {
                crate::log_msg(&format!("EPUB Error: EpubDoc::from_reader failed: {:?}", e));
                return Err(windows::core::Error::from(E_FAIL));
            }
        };

        // 3. Extract cover
        let cover_data = match doc.get_cover() {
            Ok(data) => data,
            Err(e) => {
                crate::log_msg(&format!("EPUB Error: doc.get_cover() failed: {:?}", e));
                return Err(windows::core::Error::from(E_FAIL));
            }
        };
        crate::log_msg(&format!("EPUB Cover extracted, size: {} bytes", cover_data.len()));

        // 4. Load image
        let img = match image::load_from_memory(&cover_data) {
            Ok(i) => i,
            Err(e) => {
                crate::log_msg(&format!("EPUB Error: image::load_from_memory failed: {:?}", e));
                return Err(windows::core::Error::from(E_FAIL));
            }
        };

        // 5. Resize logic (from test_epub.rs)
        let (orig_w, orig_h) = (img.width(), img.height());
        let target_long_side = 216; // Fixed size based on request

        let (new_w, new_h) = if orig_w > orig_h {
            // Landscape
            let ratio = target_long_side as f32 / orig_w as f32;
            (target_long_side, (orig_h as f32 * ratio) as u32)
        } else {
            // Portrait or square
            let ratio = target_long_side as f32 / orig_h as f32;
            ((orig_w as f32 * ratio) as u32, target_long_side)
        };

        let resized_img = image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Lanczos3);

        // 6. Create canvas (cx x cx)
        // Use requested size `cx` from Windows, usually 256 for Extra Large icons.
        let canvas_size = 256;
        let mut canvas = image::RgbaImage::new(canvas_size, canvas_size);

        // Center the image
        let x = (canvas_size - new_w) / 2;
        let y = (canvas_size - new_h) / 2;

        image::imageops::overlay(&mut canvas, &resized_img, x as i64, y as i64);

        // 7. Apply Book.png overlay
        let book_bytes = include_bytes!("../../assets/Book.png");
        if let Ok(book_img) = image::load_from_memory(book_bytes) {
            let book_rgba = book_img.to_rgba8();
             // 9-slice margins: Top=5, Bottom=10, Left=20, Right=20
            let (slice_l, slice_r, slice_t, slice_b) = (20, 20, 5, 10);
            
            // Target size matches the resized cover size
            let (target_w, target_h) = (new_w, new_h);
            
            let scaled_book = nine_slice_scale(&book_rgba, target_w, target_h, slice_l, slice_r, slice_t, slice_b);
            
            // Overlay ON TOP of the cover
            image::imageops::overlay(&mut canvas, &scaled_book, x as i64, y as i64);
        } else {
             crate::log_msg("EPUB Warning: Failed to load Book.png asset");
        }

        // 8. Convert to ARGB for Windows
        let (w, h) = canvas.dimensions();
        
        unsafe {
             let mut p_bits: *mut core::ffi::c_void = std::ptr::null_mut();
             let hbmp = create_argb_bitmap(w, h, &mut p_bits);
             if hbmp.is_invalid() {
                 crate::log_msg("EPUB Error: create_argb_bitmap failed");
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

        crate::log_msg("EPUB Thumbnail generation successful");
        Ok(())
    }
}

// 9-slice scaling implementation
fn nine_slice_scale(
    src: &image::RgbaImage,
    target_w: u32,
    target_h: u32,
    l: u32,
    r: u32,
    t: u32,
    b: u32,
) -> image::RgbaImage {
    let (src_w, src_h) = src.dimensions();
    let mut dst = image::RgbaImage::new(target_w, target_h);

    // Calculate source regions
    let src_cw = src_w - l - r; // center width
    let src_ch = src_h - t - b; // center height

    // Calculate destination regions
    let dst_cw = target_w - l - r;
    let dst_ch = target_h - t - b;

    // Helper to resize and copy
    let process_region = |
        d: &mut image::RgbaImage, 
        sx: u32, sy: u32, sw: u32, sh: u32, 
        dx: u32, dy: u32, dw: u32, dh: u32
    | {
        if sw == 0 || sh == 0 || dw == 0 || dh == 0 { return; }
        
        let sub = src.view(sx, sy, sw, sh).to_image();
        
        if sw == dw && sh == dh {
            // Direct copy
            image::imageops::overlay(d, &sub, dx as i64, dy as i64);
        } else {
            // Resize (using Triangle/Bilinear for speed on patches, or Lanczos3 for quality)
            let resized = image::imageops::resize(&sub, dw, dh, image::imageops::FilterType::Lanczos3);
            image::imageops::overlay(d, &resized, dx as i64, dy as i64);
        }
    };

    // 1. Top-Left
    process_region(&mut dst, 0, 0, l, t, 0, 0, l, t);
    // 2. Top-Center
    process_region(&mut dst, l, 0, src_cw, t, l, 0, dst_cw, t);
    // 3. Top-Right
    process_region(&mut dst, src_w - r, 0, r, t, target_w - r, 0, r, t);

    // 4. Mid-Left
    process_region(&mut dst, 0, t, l, src_ch, 0, t, l, dst_ch);
    // 5. Center - Don't draw the center if we want transparency?
    // Actually, usually 9-slice includes center. If Book.png center is transparent, it will just copy transparency.
    process_region(&mut dst, l, t, src_cw, src_ch, l, t, dst_cw, dst_ch);
    // 6. Mid-Right
    process_region(&mut dst, src_w - r, t, r, src_ch, target_w - r, t, r, dst_ch);

    // 7. Bot-Left
    process_region(&mut dst, 0, src_h - b, l, b, 0, target_h - b, l, b);
    // 8. Bot-Center
    process_region(&mut dst, l, src_h - b, src_cw, b, l, target_h - b, dst_cw, b);
    // 9. Bot-Right
    process_region(&mut dst, src_w - r, src_h - b, r, b, target_w - r, target_h - b, r, b);

    dst
}
