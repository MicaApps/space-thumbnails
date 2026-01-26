use std::path::Path;
use image::{RgbaImage, imageops::FilterType, Rgba, GenericImageView, DynamicImage};
use pdfium_render::prelude::*;
use super::ThumbnailGenerator;

// Embed the Book template asset (reused from EPUB style)
const BOOK_TEMPLATE: &[u8] = include_bytes!("../assets/Book.png");

// Source slice parameters
const SRC_T: u32 = 5;
const SRC_B: u32 = 10;
const SRC_L: u32 = 20;
const SRC_R: u32 = 20;

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
        // 1. Load Template
        let template = image::load_from_memory(BOOK_TEMPLATE)
            .map_err(|e| format!("Failed to load Book template: {}", e))?;

        // 2. Initialize PDFium and Load Content
        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| format!("Failed to bind to PDFium: {}", e))?;
            
        let pdfium = Pdfium::new(bindings);

        let document = if let Some(buf) = buffer {
            pdfium.load_pdf_from_byte_slice(buf, None)
        } else {
             // If we had filepath, we could use load_pdf_from_file, but ThumbnailGenerator trait 
             // often provides buffer. If buffer is None, we might need to read file.
             // But for now let's assume buffer is provided or we fail if not (since trait signature implies it might be None).
             // If buffer is None, we should try filepath if available.
             if let Some(path) = _filepath {
                 pdfium.load_pdf_from_file(path, None)
             } else {
                 return Err("No buffer or filepath provided for PDF".to_string());
             }
        }.map_err(|e| format!("Failed to load PDF: {}", e))?;

        // Render first page
        let page = document.pages().get(0).map_err(|e| format!("Failed to get first page: {}", e))?;
        
        // Render at high resolution then downscale for better quality
        // Target roughly 2x the requested size for the cover part
        let render_config = PdfRenderConfig::new()
            .set_target_width((width * 2) as i32)
            .set_maximum_height((height * 2) as i32)
            .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true); // Auto-rotate landscape pages to fit book cover better? Maybe not.
            // Actually, books are usually portrait. If PDF is landscape, it might look weird. 
            // Let's stick to default orientation but fit within bounds.

        let bitmap = page.render_with_config(&render_config)
            .map_err(|e| format!("Failed to render page: {}", e))?;
        
        let cover = bitmap.as_image(); // Returns DynamicImage

        // 3. Calculate Dimensions & Scale (Same as EPUB)
        let scale_factor = width as f32 / 256.0;

        let dst_t = (SRC_T as f32 * scale_factor) as u32;
        let dst_b = (SRC_B as f32 * scale_factor) as u32;
        let dst_l = (SRC_L as f32 * scale_factor) as u32;
        let dst_r = (SRC_R as f32 * scale_factor) as u32;

        // New Logic: "Body distance from edge max not exceed 20px"
        // Max Physical Limits (Body centered)
        let margin_x = 20.0 * scale_factor;
        let margin_y = 20.0 * scale_factor;

        let max_body_w = (width as f32 - margin_x * 2.0).max(1.0) as u32;
        let max_body_h = (height as f32 - margin_y * 2.0).max(1.0) as u32;

        let mut cover_scaled = cover.resize(max_body_w, max_body_h, FilterType::Triangle);

        // 4. Apply Rounded Corners
        let radius = (3.0 * scale_factor).max(1.0) as u32;
        apply_rounded_corners(&mut cover_scaled, radius);

        // 5. Calculate Final Book Frame Size
        let frame_w = cover_scaled.width() + dst_l + dst_r;
        let frame_h = cover_scaled.height() + dst_t + dst_b;
        
        // 6. Generate 9-Sliced Frame
        let frame_img = generate_nine_slice(
            &template, 
            frame_w, frame_h, 
            SRC_L, SRC_R, SRC_T, SRC_B,
            dst_l, dst_r, dst_t, dst_b
        );

        // 7. Composite
        let mut canvas = RgbaImage::new(width, height);
        
        // Center the COVER on canvas
        let cover_x = (width - cover_scaled.width()) / 2;
        let cover_y = (height - cover_scaled.height()) / 2;
        
        // Frame position relative to Cover
        let frame_x = (cover_x as i64) - (dst_l as i64);
        let frame_y = (cover_y as i64) - (dst_t as i64);
        
        image::imageops::overlay(&mut canvas, &cover_scaled, cover_x as i64, cover_y as i64);
        image::imageops::overlay(&mut canvas, &frame_img, frame_x, frame_y);

        Ok(canvas.into_raw())
    }
}

// Helper functions (duplicated from epub.rs)
fn apply_rounded_corners(img: &mut DynamicImage, radius: u32) {
    let (w, h) = img.dimensions();
    if radius == 0 { return; }
    
    let mut rgba_img = img.to_rgba8();
    let r_sq = (radius as f32).powi(2);

    for y in 0..h {
        for x in 0..w {
            let in_tl = x < radius && y < radius;
            let in_tr = x >= w - radius && y < radius;
            let in_bl = x < radius && y >= h - radius;
            let in_br = x >= w - radius && y >= h - radius;

            if in_tl || in_tr || in_bl || in_br {
                let center_x = if x < radius { radius as f32 - 0.5 } else { w as f32 - radius as f32 + 0.5 };
                let center_y = if y < radius { radius as f32 - 0.5 } else { h as f32 - radius as f32 + 0.5 };
                
                let dist_sq = (x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2);
                
                if dist_sq > r_sq {
                    if dist_sq < r_sq + 2.0 {
                        let alpha = 1.0 - (dist_sq.sqrt() - radius as f32).max(0.0).min(1.0);
                        let p = rgba_img.get_pixel_mut(x, y);
                        let current_alpha = p[3] as f32;
                        p[3] = (current_alpha * alpha) as u8;
                    } else {
                        rgba_img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                    }
                }
            }
        }
    }
    
    *img = DynamicImage::ImageRgba8(rgba_img);
}

fn generate_nine_slice(
    src: &DynamicImage, 
    target_w: u32, target_h: u32, 
    sl: u32, sr: u32, st: u32, sb: u32, 
    tl: u32, tr: u32, tt: u32, tb: u32 
) -> DynamicImage {
    let mut out = RgbaImage::new(target_w, target_h);
    let src_w = src.width();
    let src_h = src.height();

    let draw_chunk = |target: &mut RgbaImage, sx: u32, sy: u32, sw: u32, sh: u32, tx: u32, ty: u32, tw: u32, th: u32| {
        if sw == 0 || sh == 0 || tw == 0 || th == 0 { return; }
        if sx + sw > src_w || sy + sh > src_h { return; }
        
        let chunk = src.view(sx, sy, sw, sh).to_image();
        let resized = image::imageops::resize(&chunk, tw, th, FilterType::Triangle);
        image::imageops::overlay(target, &resized, tx as i64, ty as i64);
    };
    
    let sc_w = src_w.saturating_sub(sl + sr);
    let sc_h = src_h.saturating_sub(st + sb);
    
    let tc_w = target_w.saturating_sub(tl + tr);
    let tc_h = target_h.saturating_sub(tt + tb);

    draw_chunk(&mut out, 0, 0, sl, st, 0, 0, tl, tt);
    draw_chunk(&mut out, sl, 0, sc_w, st, tl, 0, tc_w, tt);
    draw_chunk(&mut out, src_w - sr, 0, sr, st, target_w - tr, 0, tr, tt);
    
    draw_chunk(&mut out, 0, st, sl, sc_h, 0, tt, tl, tc_h);
    draw_chunk(&mut out, sl, st, sc_w, sc_h, tl, tt, tc_w, tc_h);
    draw_chunk(&mut out, src_w - sr, st, sr, sc_h, target_w - tr, tt, tr, tc_h);
    
    draw_chunk(&mut out, 0, src_h - sb, sl, sb, 0, target_h - tb, tl, tb);
    draw_chunk(&mut out, sl, src_h - sb, sc_w, sb, tl, target_h - tb, tc_w, tb);
    draw_chunk(&mut out, src_w - sr, src_h - sb, sr, sb, target_w - tr, target_h - tb, tr, tb);

    DynamicImage::ImageRgba8(out)
}
