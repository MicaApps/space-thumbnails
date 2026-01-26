use std::path::Path;
use image::{RgbaImage, imageops::FilterType, DynamicImage, GenericImageView};
use pdfium_render::prelude::*;
use super::ThumbnailGenerator;

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
        extension.eq_ignore_ascii_case("pdf")
    }

    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, _extension: &str, _filepath: Option<&Path>) -> Result<Vec<u8>, String> {
        // 1. Initialize PDFium and Load Content
        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| format!("Failed to bind to PDFium: {}", e))?;
            
        let pdfium = Pdfium::new(bindings);

        let document = if let Some(buf) = buffer {
            pdfium.load_pdf_from_byte_slice(buf, None)
        } else {
             if let Some(path) = _filepath {
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

        // 2. Resize to fit safe area
        // User Requirement: "At least 20px margin on all sides at 256px resolution"
        let scale_factor = width as f32 / 256.0;
        let margin = 20.0 * scale_factor;
        
        let safe_width = (width as f32 - 2.0 * margin).max(1.0) as u32;
        let safe_height = (height as f32 - 2.0 * margin).max(1.0) as u32;

        let cover_scaled = cover.resize(safe_width, safe_height, FilterType::Triangle);
        
        // 3. Center on Canvas
        let mut canvas = RgbaImage::new(width, height);
        let x = (width - cover_scaled.width()) / 2;
        let y = (height - cover_scaled.height()) / 2;
        
        image::imageops::overlay(&mut canvas, &cover_scaled, x as i64, y as i64);

        Ok(canvas.into_raw())
    }
}

