use std::path::Path;
use psd::Psd;
use image::{RgbaImage, imageops::FilterType, Rgba};
use super::ThumbnailGenerator;

pub struct PsdGenerator;

impl ThumbnailGenerator for PsdGenerator {
    fn name(&self) -> &str {
        "PSD Renderer"
    }

    fn validate(&self, header: &[u8], extension: &str) -> bool {
        extension == "psd" || header.starts_with(b"8BPS")
    }

    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, _extension: &str, filepath: Option<&Path>) -> Result<Vec<u8>, String> {
        let file_buf; 
        let bytes = if let Some(b) = buffer {
            b
        } else if let Some(path) = filepath {
            file_buf = std::fs::read(path).map_err(|e| e.to_string())?;
            &file_buf
        } else {
            return Err("No buffer or filepath provided for PSD".to_string());
        };

        let psd = Psd::from_bytes(bytes).map_err(|e| format!("Invalid PSD: {}", e))?;
        let psd_width = psd.width();
        let psd_height = psd.height();
        let raw_rgba = psd.rgba();

        let img_buffer = RgbaImage::from_raw(psd_width, psd_height, raw_rgba)
            .ok_or("Failed to create image buffer from PSD data")?;

        let mut resized = image::imageops::resize(&img_buffer, width, height, FilterType::Triangle);

        // Apply corner fold (Top-Right)
        apply_corner_fold(&mut resized);

        Ok(resized.into_raw())
    }
}

fn apply_corner_fold(img: &mut RgbaImage) {
    let (w, h) = img.dimensions();
    let fold_size = (w.min(h) as f32 * 0.15) as u32; // 15% fold size
    
    if fold_size == 0 { return; }

    let fold_color = Rgba([240, 240, 240, 255]); // White-ish back
    let shadow_color = Rgba([0, 0, 0, 40]);      // Soft shadow

    for y in 0..fold_size {
        for x in (w - fold_size)..w {
            let dx = x - (w - fold_size); // 0 to fold_size
            let dy = y;                   // 0 to fold_size

            // Cut the corner (Top-Right)
            // The cut line is where dx + dy >= fold_size
            // Actually standard fold is usually diagonal: x + y > threshold
            // Let's model a diagonal fold line from (w-fold, 0) to (w, fold)
            
            // Local coords relative to fold box origin (w-fold, 0)
            // We want to cut the triangle where x_local > y_local (if folding from top-right down-left?)
            // No, standard icon fold is usually top-right corner folded down-left.
            // Diagonal line equation: y = -x + C
            
            // Let's use a simpler logic:
            // Cut region: Top-Right triangle
            if dx + dy < fold_size {
                // Inside the main image, keep pixel
            } else {
                // Outside main image (the corner to be removed)
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                
                // Draw the folded flap (mirrored)
                // The flap is the cut part reflected across the fold line
                // Fold line is x + y = w - fold + fold = w? No.
                // Fold line connects (w-fold, 0) and (w, fold).
                // Equation: Y = X - (w-fold). Wait, that's 45 deg.
                
                // Let's render the flap "over" the image manually
                // Flap pixel at (x_dest, y_dest) corresponds to cut pixel (x_src, y_src)
                // For a 45 degree fold, we can just iterate the flap area.
            }
        }
    }

    // Correct approach for 45-degree fold:
    // 1. Clear the Top-Right triangle: x > (w - fold_size + y)
    for y in 0..fold_size {
        for x in (w - fold_size + y)..w {
             img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        }
    }

    // 2. Draw the Folded Flap (White triangle with shadow)
    // It sits on the "bottom-left" side of the fold line.
    // The fold line connects (w-fold, 0) and (w, fold).
    // We want to draw a triangle with vertices: (w-fold, 0), (w-fold, fold), (w, fold).
    // Wait, if we fold top-right corner down, the flap covers the image below it.
    
    for y in 0..fold_size {
        for x in (w - fold_size)..(w - fold_size + y) {
             // Basic white flap
             img.put_pixel(x, y, fold_color);

             // Diagonal shadow on the fold crease
             if x == (w - fold_size + y - 1) {
                 img.put_pixel(x, y, shadow_color);
             }
        }
    }
}
