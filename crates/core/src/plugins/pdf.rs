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

        Ok(canvas.into_raw())
    }
}

