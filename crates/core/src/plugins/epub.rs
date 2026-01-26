use std::path::Path;
use std::fs::File;
use std::io::{Read, Seek};
use image::{RgbaImage, imageops::FilterType, Rgba, GenericImageView, DynamicImage};
use zip::ZipArchive;
use super::ThumbnailGenerator;

// Helper trait for Box<dyn Read + Seek>
trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

// Embed the Book template asset
const BOOK_TEMPLATE: &[u8] = include_bytes!("../assets/Book.png");

// Source slice parameters (based on user spec for the asset)
// "上5，下10，左右20"
const SRC_T: u32 = 5;
const SRC_B: u32 = 10;
const SRC_L: u32 = 20;
const SRC_R: u32 = 20;

pub struct EpubGenerator;

impl ThumbnailGenerator for EpubGenerator {
    fn name(&self) -> &str {
        "EPUB Renderer"
    }

    fn validate(&self, _header: &[u8], extension: &str) -> bool {
        extension.eq_ignore_ascii_case("epub")
    }

    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, _extension: &str, filepath: Option<&Path>) -> Result<Vec<u8>, String> {
        // 1. Load Template
        let template = image::load_from_memory(BOOK_TEMPLATE)
            .map_err(|e| format!("Failed to load Book template: {}", e))?;

        // 2. Load EPUB Content
        let file_buf;
        let reader: Box<dyn ReadSeek> = if let Some(path) = filepath {
            let file = File::open(path).map_err(|e| e.to_string())?;
            Box::new(file)
        } else if let Some(buf) = buffer {
            file_buf = std::io::Cursor::new(buf);
            Box::new(file_buf)
        } else {
            return Err("No buffer or filepath provided for EPUB".to_string());
        };

        let mut archive = ZipArchive::new(reader).map_err(|e| format!("Failed to open EPUB zip: {}", e))?;
        let cover_data = find_cover_image(&mut archive)?;
        let cover = image::load_from_memory(&cover_data).map_err(|e| format!("Failed to load cover image: {}", e))?;

        // 3. Calculate Dimensions & Scale
        // Base resolution is 256px.
        let scale_factor = width as f32 / 256.0;

        // Target Borders (Scaled)
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
        // Removed: Applied to composite instead
        // let radius = (3.0 * scale_factor).max(1.0) as u32;
        // apply_rounded_corners(&mut cover_scaled, radius);

        // 5. Calculate Final Book Frame Size
        // User Requirement: "Mask covers the cover, same size as the cover."
        let frame_w = cover_scaled.width();
        let frame_h = cover_scaled.height();
        
        // 6. Generate 9-Sliced Frame
        let frame_img = generate_nine_slice(
            &template, 
            frame_w, frame_h, 
            SRC_L, SRC_R, SRC_T, SRC_B,
            dst_l, dst_r, dst_t, dst_b
        );

        // 7. Composite Cover + Frame
        let mut book_composite = RgbaImage::new(frame_w, frame_h);
        image::imageops::overlay(&mut book_composite, &cover_scaled, 0, 0);
        image::imageops::overlay(&mut book_composite, &frame_img, 0, 0);

        // 8. Apply Rounded Corners to the Composite
        let mut book_dynamic = DynamicImage::ImageRgba8(book_composite);
        let radius = (3.0 * scale_factor).max(1.0) as u32;
        apply_rounded_corners(&mut book_dynamic, radius);

        // 9. Place on Final Canvas
        let mut canvas = RgbaImage::new(width, height);
        
        // Center the BOOK on canvas
        let book_x = (width - frame_w) / 2;
        let book_y = (height - frame_h) / 2;
        
        image::imageops::overlay(&mut canvas, &book_dynamic, book_x as i64, book_y as i64);

        Ok(canvas.into_raw())
    }
}

fn find_cover_image<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<u8>, String> {
    let common_names = ["cover.jpg", "cover.jpeg", "cover.png", "OEBPS/images/cover.jpg", "OEBPS/images/cover.png", "OEBPS/cover.jpg"];
    for name in common_names {
        if let Ok(mut file) = archive.by_name(name) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            return Ok(buf);
        }
    }

    let mut largest_size = 0;
    let mut largest_file_index = None;
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_lowercase();
            if (name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".png")) && !name.contains("thumb") {
                if file.size() > largest_size {
                    largest_size = file.size();
                    largest_file_index = Some(i);
                }
            }
        }
    }

    if let Some(index) = largest_file_index {
        let mut file = archive.by_index(index).map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        return Ok(buf);
    }

    Err("No cover image found in EPUB".to_string())
}

fn apply_rounded_corners(img: &mut DynamicImage, radius: u32) {
    let (w, h) = img.dimensions();
    if radius == 0 { return; }
    
    let mut rgba_img = img.to_rgba8();
    let r_sq = (radius as f32).powi(2);

    for y in 0..h {
        for x in 0..w {
            // Check if pixel is in one of the 4 corners
            let in_tl = x < radius && y < radius;
            let in_tr = x >= w - radius && y < radius;
            let in_bl = x < radius && y >= h - radius;
            let in_br = x >= w - radius && y >= h - radius;

            if in_tl || in_tr || in_bl || in_br {
                let center_x = if x < radius { radius as f32 - 0.5 } else { w as f32 - radius as f32 + 0.5 };
                let center_y = if y < radius { radius as f32 - 0.5 } else { h as f32 - radius as f32 + 0.5 };
                
                let dist_sq = (x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2);
                
                if dist_sq > r_sq {
                    // Antialiasing
                    if dist_sq < r_sq + 2.0 {
                        // Edge pixel, some alpha
                        let alpha = 1.0 - (dist_sq.sqrt() - radius as f32).max(0.0).min(1.0);
                        let p = rgba_img.get_pixel_mut(x, y);
                        let current_alpha = p[3] as f32;
                        p[3] = (current_alpha * alpha) as u8;
                    } else {
                        // Fully outside
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
    sl: u32, sr: u32, st: u32, sb: u32, // Source Slices
    tl: u32, tr: u32, tt: u32, tb: u32  // Target Borders
) -> DynamicImage {
    let mut out = RgbaImage::new(target_w, target_h);
    let src_w = src.width();
    let src_h = src.height();

    // Helper to copy/resize a chunk
    // sx, sy, sw, sh: Source rect
    // tx, ty, tw, th: Target rect
    let draw_chunk = |target: &mut RgbaImage, sx: u32, sy: u32, sw: u32, sh: u32, tx: u32, ty: u32, tw: u32, th: u32| {
        if sw == 0 || sh == 0 || tw == 0 || th == 0 { return; }
        // Handle potential out of bounds for source
        if sx + sw > src_w || sy + sh > src_h { return; }
        
        let chunk = src.view(sx, sy, sw, sh).to_image();
        let resized = image::imageops::resize(&chunk, tw, th, FilterType::Triangle);
        image::imageops::overlay(target, &resized, tx as i64, ty as i64);
    };
    
    // Source Center Width/Height
    let sc_w = src_w.saturating_sub(sl + sr);
    let sc_h = src_h.saturating_sub(st + sb);
    
    // Target Center Width/Height
    let tc_w = target_w.saturating_sub(tl + tr);
    let tc_h = target_h.saturating_sub(tt + tb);

    // 1. TL Corner
    draw_chunk(&mut out, 0, 0, sl, st, 0, 0, tl, tt);
    // 2. Top Edge
    draw_chunk(&mut out, sl, 0, sc_w, st, tl, 0, tc_w, tt);
    // 3. TR Corner
    draw_chunk(&mut out, src_w - sr, 0, sr, st, target_w - tr, 0, tr, tt);
    
    // 4. Left Edge
    draw_chunk(&mut out, 0, st, sl, sc_h, 0, tt, tl, tc_h);
    // 5. Center (Usually transparent or background, we stretch it)
    draw_chunk(&mut out, sl, st, sc_w, sc_h, tl, tt, tc_w, tc_h);
    // 6. Right Edge
    draw_chunk(&mut out, src_w - sr, st, sr, sc_h, target_w - tr, tt, tr, tc_h);
    
    // 7. BL Corner
    draw_chunk(&mut out, 0, src_h - sb, sl, sb, 0, target_h - tb, tl, tb);
    // 8. Bottom Edge
    draw_chunk(&mut out, sl, src_h - sb, sc_w, sb, tl, target_h - tb, tc_w, tb);
    // 9. BR Corner
    draw_chunk(&mut out, src_w - sr, src_h - sb, sr, sb, target_w - tr, target_h - tb, tr, tb);

    DynamicImage::ImageRgba8(out)
}
