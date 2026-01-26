use std::path::Path;
use super::ThumbnailGenerator;

pub struct HdriGenerator;

impl ThumbnailGenerator for HdriGenerator {
    fn name(&self) -> &str {
        "HDRI Renderer"
    }

    fn validate(&self, header: &[u8], extension: &str) -> bool {
        if matches!(extension, "hdr" | "exr" | "hdri") {
            return true;
        }
        if header.len() >= 10 && (header.starts_with(b"#?RADIANCE") || header.starts_with(b"#?RGBE")) {
            return true;
        }
        if header.len() >= 4 {
            let exr_magic = [0x76, 0x2f, 0x31, 0x01];
            return header[0..4] == exr_magic;
        }
        false
    }

    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, _extension: &str, filepath: Option<&Path>) -> Result<Vec<u8>, String> {
        let image = match (buffer, filepath) {
            (Some(bytes), _) => image::load_from_memory(bytes).map_err(|e| e.to_string())?,
            (None, Some(path)) => image::open(path).map_err(|e| e.to_string())?,
            _ => return Err("No buffer or filepath provided for HDRI".to_string()),
        };

        let rgba_source = image.to_rgba8();
        let src_width = rgba_source.width() as f32;
        let src_height = rgba_source.height() as f32;

        let mut canvas = image::RgbaImage::new(width, height);
        
        // Sphere properties
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        
        // User requested 20px margin on each side for 256px canvas.
        // This scales proportionally for other resolutions.
        let min_dim = width.min(height) as f32;
        let margin_ratio = 40.0 / 256.0; // 20px * 2 sides / 256px
        let radius = (min_dim * (1.0 - margin_ratio)) / 2.0; 

        // Helper for bilinear sampling
        let sample_bilinear = |u: f32, v: f32| -> image::Rgba<u8> {
            let x = (u * (src_width - 1.0)).clamp(0.0, src_width - 1.0);
            let y = (v * (src_height - 1.0)).clamp(0.0, src_height - 1.0);
            
            let x0 = x.floor() as u32;
            let y0 = y.floor() as u32;
            let x1 = (x0 + 1).min(src_width as u32 - 1);
            let y1 = (y0 + 1).min(src_height as u32 - 1);
            
            let dx = x - x0 as f32;
            let dy = y - y0 as f32;
            
            let p00 = rgba_source.get_pixel(x0, y0);
            let p10 = rgba_source.get_pixel(x1, y0);
            let p01 = rgba_source.get_pixel(x0, y1);
            let p11 = rgba_source.get_pixel(x1, y1);
            
            let mut res = image::Rgba([0, 0, 0, 0]);
            for c in 0..4 {
                let top = p00[c] as f32 * (1.0 - dx) + p10[c] as f32 * dx;
                let bottom = p01[c] as f32 * (1.0 - dx) + p11[c] as f32 * dx;
                res[c] = (top * (1.0 - dy) + bottom * dy) as u8;
            }
            res
        };

        // Iterate over output pixels
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let dist_sq = dx * dx + dy * dy;
                let dist = dist_sq.sqrt();

                // Analytical Anti-Aliasing (SDF based)
                // Calculate alpha based on distance to edge (radius)
                // Smooth transition from 1.0 (inside) to 0.0 (outside) over 1 pixel
                let alpha_mask = (radius + 0.5 - dist).clamp(0.0, 1.0);

                if alpha_mask > 0.0 {
                    // Calculate UV
                    // For pixels partially outside the radius (but within AA range),
                    // clamp distance to radius to avoid NaN in Z calculation and fetch valid texture
                    let effective_dist = dist.min(radius - 0.0001);
                    
                    // Normalize coordinates to -1.0 to 1.0 within the sphere
                    let u_sphere = dx * (effective_dist / dist) / radius;
                    let v_sphere = dy * (effective_dist / dist) / radius;
                    
                    let z_sq = 1.0 - u_sphere * u_sphere - v_sphere * v_sphere;
                    let z_sphere = if z_sq > 0.0 { z_sq.sqrt() } else { 0.0 };

                    let lat = v_sphere.asin();
                    let lon = u_sphere.atan2(z_sphere);

                    let u_src = (lon / std::f32::consts::PI + 1.0) / 2.0;
                    let v_src = (lat / (std::f32::consts::PI / 2.0) + 1.0) / 2.0;

                    let mut pixel = sample_bilinear(u_src, v_src);
                    
                    // Apply Edge Alpha
                    // Note: Pre-multiply alpha if needed, but here we just scale the alpha channel
                    // Assuming non-premultiplied alpha output for PNG
                    pixel[3] = (pixel[3] as f32 * alpha_mask) as u8;
                    
                    canvas.put_pixel(x, y, pixel);
                } else {
                    canvas.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
                }
            }
        }
        
        Ok(canvas.into_raw())
    }
}
