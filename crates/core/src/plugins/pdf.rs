use std::path::{Path, PathBuf};
use image::{RgbaImage, imageops::FilterType};
use pdfium_render::prelude::*;
use super::ThumbnailGenerator;

fn get_current_dll_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use windows::Win32::System::LibraryLoader::{GetModuleHandleExW, GetModuleFileNameW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT};
        use windows::Win32::Foundation::HINSTANCE;
        
        let mut module = HINSTANCE::default();
        unsafe {
            let lpaddress = get_current_dll_dir as *const std::ffi::c_void;
            if GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                windows::core::PCWSTR(lpaddress as *const u16),
                &mut module
            ).as_bool() {
                 let mut path = [0u16; 1024];
                 let len = GetModuleFileNameW(module, &mut path);
                 if len > 0 {
                     let path_str = String::from_utf16_lossy(&path[..len as usize]);
                     return PathBuf::from(path_str).parent().map(|p| p.to_path_buf());
                 }
            }
        }
    }
    None
}

pub struct PdfGenerator;

impl ThumbnailGenerator for PdfGenerator {
    fn name(&self) -> &str {
        "PDF Renderer"
    }

    fn validate(&self, header: &[u8], extension: &str) -> bool {
        // PDF magic number: %PDF (25 50 44 46)
        if header.len() >= 4 && &header[0..4] == b"%PDF" {
            return true;
        }
        extension.eq_ignore_ascii_case("pdf") || extension.eq_ignore_ascii_case("ai")
    }

    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, _extension: &str, filepath: Option<&Path>) -> Result<Vec<u8>, String> {
        Self::render_pdf(buffer, filepath, width, height)
    }
}

impl PdfGenerator {
    pub fn render_pdf(buffer: Option<&[u8]>, filepath: Option<&Path>, width: u32, height: u32) -> Result<Vec<u8>, String> {
        // 1. Initialize PDFium and Load Content
        let mut bindings_result = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"));
        
        if bindings_result.is_err() {
            if let Some(dll_dir) = get_current_dll_dir() {
                 if let Some(dir_str) = dll_dir.to_str() {
                     bindings_result = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir_str));
                 }
            }
        }

        let bindings = bindings_result
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| format!("Failed to bind to PDFium: {}", e))?;
            
        let pdfium = Pdfium::new(bindings);

        let document = if let Some(buf) = buffer {
            pdfium.load_pdf_from_byte_slice(buf, None)
        } else {
             if let Some(path) = filepath {
                 pdfium.load_pdf_from_file(path, None)
             } else {
                 return Err("No buffer or filepath provided for PDF".to_string());
             }
        }.map_err(|e| format!("Failed to load PDF: {}", e))?;

        // Render first page
        let page = document.pages().get(0).map_err(|e| format!("Failed to get first page: {}", e))?;
        
        // Render at high resolution then downscale for better quality
        let render_config = PdfRenderConfig::new()
            .set_target_width((width * 2) as i32)
            .set_maximum_height((height * 2) as i32)
            .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

        let bitmap = page.render_with_config(&render_config)
            .map_err(|e| format!("Failed to render page: {}", e))?;
        
        let cover = bitmap.as_image(); // Returns DynamicImage

        // Calculate margins (20px at 256px resolution)
        let scale_factor = width as f32 / 256.0;
        let margin = (20.0 * scale_factor).round() as u32;
        let border_size = 3;

        // Content constraint: Keep top/bottom margins 20px (content size)
        // The border is added "outside" this content area.
        let max_w = width.saturating_sub(margin * 2);
        let max_h = height.saturating_sub(margin * 2);
        
        let cover_scaled = cover.resize(max_w, max_h, FilterType::Triangle);
        
        // Add 3px border (#797774)
        let frame_w = cover_scaled.width() + (border_size * 2);
        let frame_h = cover_scaled.height() + (border_size * 2);
        
        // #797774 -> R:121, G:119, B:116
        let mut framed_cover = RgbaImage::from_pixel(frame_w, frame_h, image::Rgba([121, 119, 116, 255]));
        
        image::imageops::overlay(&mut framed_cover, &cover_scaled, border_size as i64, border_size as i64);
        
        let mut canvas = RgbaImage::new(width, height);
        let x = (width - frame_w) / 2;
        let y = (height - frame_h) / 2;
        
        image::imageops::overlay(&mut canvas, &framed_cover, x as i64, y as i64);

        // Overlay Fold Asset
        // Path: crates/core/src/assets/file_fold_256.png
        // Relative to this file (crates/core/src/plugins/pdf.rs): ../assets/file_fold_256.png
        const FOLD_BYTES: &[u8] = include_bytes!("../assets/file_fold_256.png");
        if let Ok(fold_img) = image::load_from_memory(FOLD_BYTES) {
            let mut fold_rgba = fold_img.to_rgba8();
            
            // Scale fold asset if needed (assuming base size is for 256px)
            if width != 256 {
                let scale = width as f32 / 256.0;
                let new_w = (fold_rgba.width() as f32 * scale) as u32;
                let new_h = (fold_rgba.height() as f32 * scale) as u32;
                if new_w > 0 && new_h > 0 {
                    fold_rgba = image::imageops::resize(&fold_rgba, new_w, new_h, FilterType::Triangle);
                }
            }

            // Align Top-Right of fold to Top-Right of frame
            // Frame Top-Right on Canvas = (x + frame_w, y)
            let fold_x = (x + frame_w).saturating_sub(fold_rgba.width());
            let fold_y = y; // Top align
            
            // Crop the top-right corner of the canvas (document + border) to create the "dog-ear" effect.
            // We assume the fold asset represents a diagonal fold from top-left to bottom-right of the asset square.
            // We remove the top-right triangle of the document area covered by the fold asset.
            let fw = fold_rgba.width() as i64;
            let fh = fold_rgba.height() as i64;
            
            for fy in 0..fh {
                for fx in 0..fw {
                    // Define the diagonal from (0,0) to (fw, fh) relative to the fold asset.
                    // We want to erase pixels that are "above/right" of this diagonal (the corner tip).
                    // Condition: y < (fh/fw) * x  =>  y * fw < x * fh
                    if fy * fw < fx * fh {
                         let cx = fold_x as i64 + fx;
                         let cy = fold_y as i64 + fy;
                         
                         if cx >= 0 && cy >= 0 && cx < canvas.width() as i64 && cy < canvas.height() as i64 {
                             canvas.put_pixel(cx as u32, cy as u32, image::Rgba([0, 0, 0, 0]));
                         }
                    }
                }
            }

            image::imageops::overlay(&mut canvas, &fold_rgba, fold_x as i64, fold_y as i64);
        }

        Ok(canvas.into_raw())
    }
}

