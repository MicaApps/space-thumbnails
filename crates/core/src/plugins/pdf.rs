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
            // Explicitly ignore .ai files even if they have PDF header,
            // so IllustratorGenerator can handle them with its own style.
            if extension.eq_ignore_ascii_case("ai") {
                return false;
            }
            return true;
        }
        extension.eq_ignore_ascii_case("pdf")
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

        // Prepare configuration for rendering
        let render_config = PdfRenderConfig::new()
            .set_target_width((width * 2) as i32)
            .set_maximum_height((height * 2) as i32)
            .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

        // Calculate layout constants
        let scale_factor = width as f32 / 256.0;
        let margin = (20.0 * scale_factor).round() as u32;
        let border_size = 3;
        let max_w = width.saturating_sub(margin * 2);
        let max_h = height.saturating_sub(margin * 2);
        let border_color = image::Rgba([121, 119, 116, 255]); // #797774

        let mut canvas = RgbaImage::new(width, height);

        // Helper to process a page into a framed image
        let process_page = |page_index: u16| -> Result<RgbaImage, String> {
            if let Ok(page) = document.pages().get(page_index) {
                let bitmap = page.render_with_config(&render_config)
                    .map_err(|e| format!("Failed to render page {}: {}", page_index, e))?;
                let cover = bitmap.as_image();
                let cover_scaled = cover.resize(max_w, max_h, FilterType::Triangle);
                
                let frame_w = cover_scaled.width() + (border_size * 2);
                let frame_h = cover_scaled.height() + (border_size * 2);
                let mut framed = RgbaImage::from_pixel(frame_w, frame_h, border_color);
                image::imageops::overlay(&mut framed, &cover_scaled, border_size as i64, border_size as i64);
                Ok(framed)
            } else {
                Err("Page not found".to_string())
            }
        };

        // Render Page 2 (Back) if available
        if document.pages().len() > 1 {
            if let Ok(framed_back) = process_page(1) {
                // Position: Shifted right and up relative to center
                // Let's shift it slightly. For a stack, usually it's offset.
                // Standard center:
                let cx = (width - framed_back.width()) / 2;
                let cy = (height - framed_back.height()) / 2;
                
                // Shift: 0px (Aligned)
                let back_x = cx as i64;
                let back_y = cy as i64;
                
                image::imageops::overlay(&mut canvas, &framed_back, back_x, back_y);
            }
        }

        // Render Page 1 (Front)
        let framed_front = process_page(0)?;
        let x = (width - framed_front.width()) / 2;
        let y = (height - framed_front.height()) / 2;
        
        // We need to cut the top-right corner of the front page *before* overlaying it,
        // or we can cut it after overlaying it on the canvas (but that might cut the back page too if they overlap).
        // Safest is to modify framed_front before overlaying, or careful compositing.
        // However, the fold asset is overlaid on the canvas.
        // The cut logic needs to be relative to the fold asset position.
        
        // Let's place the front page on a temp layer or directly on canvas, then cut, then overlay fold.
        // But if we cut the canvas, we might cut the back page which is visible behind the cut.
        // The user wants the "original first page... removed". This implies the fold reveals what's behind it?
        // Or the fold asset covers it.
        // If the fold asset is transparent in the corner, we see through.
        // If we cut the front page, we see the back page (if it overlaps) or background.
        // This is correct for a "folded corner" revealing the page behind (or the back of the current page).
        
        // Strategy:
        // 1. Draw Back Page on Canvas.
        // 2. Prepare Front Page (Framed).
        // 3. Determine Fold Position relative to Front Page.
        // 4. "Cut" the Front Page (make pixels transparent) where the fold will be.
        // 5. Overlay Front Page on Canvas.
        // 6. Overlay Fold Asset.

        // Load Fold Asset
        const FOLD_BYTES: &[u8] = include_bytes!("../assets/pdf_fold_256.png");
        let fold_img_opt = image::load_from_memory(FOLD_BYTES).ok();
        
        let mut final_front = framed_front.clone();

        if let Some(fold_img) = &fold_img_opt {
             let mut fold_rgba = fold_img.to_rgba8();
             // Scale fold if needed
             if width != 256 {
                let scale = width as f32 / 256.0;
                let new_w = (fold_rgba.width() as f32 * scale) as u32;
                let new_h = (fold_rgba.height() as f32 * scale) as u32;
                if new_w > 0 && new_h > 0 {
                    fold_rgba = image::imageops::resize(&fold_rgba, new_w, new_h, FilterType::Triangle);
                }
             }

             // Fold alignment: Top-Right of the Front Page Frame
             // final_front is the frame.
             let frame_w = final_front.width();
             let frame_h = final_front.height(); // unused but good to know
             
             // Fold position relative to final_front
             let fold_x_rel = frame_w.saturating_sub(fold_rgba.width());
             let fold_y_rel = 0; // Top aligned
             
             // Cut logic on final_front
             let fw = fold_rgba.width() as i64;
             let fh = fold_rgba.height() as i64;
             
             for fy in 0..fh {
                for fx in 0..fw {
                    if fy * fw < fx * fh {
                         let cx = fold_x_rel as i64 + fx;
                         let cy = fold_y_rel as i64 + fy;
                         
                         if cx >= 0 && cy >= 0 && cx < final_front.width() as i64 && cy < final_front.height() as i64 {
                             final_front.put_pixel(cx as u32, cy as u32, image::Rgba([0, 0, 0, 0]));
                         }
                    }
                }
             }
        }

        // Overlay Front Page
        image::imageops::overlay(&mut canvas, &final_front, x as i64, y as i64);

        // Overlay Fold Asset
        if let Some(fold_img) = fold_img_opt {
            let mut fold_rgba = fold_img.to_rgba8();
            if width != 256 {
                let scale = width as f32 / 256.0;
                let new_w = (fold_rgba.width() as f32 * scale) as u32;
                let new_h = (fold_rgba.height() as f32 * scale) as u32;
                if new_w > 0 && new_h > 0 {
                    fold_rgba = image::imageops::resize(&fold_rgba, new_w, new_h, FilterType::Triangle);
                }
            }
            
            // Calculate absolute position on canvas
            // Front Page x + Front Page Width - Fold Width
            let fold_x = (x + final_front.width()).saturating_sub(fold_rgba.width());
            let fold_y = y;
            
            image::imageops::overlay(&mut canvas, &fold_rgba, fold_x as i64, fold_y as i64);
        }

        Ok(canvas.into_raw())
    }
}

