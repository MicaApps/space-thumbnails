use epub::doc::EpubDoc;
use std::path::Path;
use image::ImageFormat;

use image::GenericImageView;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\test_files\小岛经济学.epub";
    println!("Testing EPUB cover extraction for: {}", path);

    if !Path::new(path).exists() {
        eprintln!("Error: File not found at {}", path);
        return Ok(());
    }

    let mut doc = EpubDoc::new(path)?;
    println!("EPUB opened successfully.");

    // Try to get cover
    match doc.get_cover() {
        Ok(cover_data) => {
            println!("Cover found! Size: {} bytes", cover_data.len());
            
            // Convert to PNG using image crate
            let img = image::load_from_memory(&cover_data)?;
            
            // Resize logic: Long side = 216px
            let (orig_w, orig_h) = (img.width(), img.height());
            let target_long_side = 216;
            
            let (new_w, new_h) = if orig_w > orig_h {
                // Landscape
                let ratio = target_long_side as f32 / orig_w as f32;
                (target_long_side, (orig_h as f32 * ratio) as u32)
            } else {
                // Portrait or square
                let ratio = target_long_side as f32 / orig_h as f32;
                ((orig_w as f32 * ratio) as u32, target_long_side)
            };
            
            println!("Resizing from {}x{} to {}x{}", orig_w, orig_h, new_w, new_h);
            
            let resized_img = image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Lanczos3);
            
            // Place on 256x256 transparent canvas
            let canvas_size = 256;
            let mut canvas = image::RgbaImage::new(canvas_size, canvas_size);
            
            // Center the image
            // "保留一侧20px透明" -> If long side is 216, 256-216=40. So 20px padding on each side.
            // This is naturally achieved by centering.
            let x = (canvas_size - new_w) / 2;
            let y = (canvas_size - new_h) / 2;
            
            image::imageops::overlay(&mut canvas, &resized_img, x as i64, y as i64);
            
            // Load and apply Book.png overlay with 9-slice scaling
            let book_path = "crates/windows/assets/Book.png";
            if Path::new(book_path).exists() {
                println!("Applying Book.png overlay...");
                let book_img = image::open(book_path)?.to_rgba8();
                
                // 9-slice margins: Top=5, Bottom=10, Left=20, Right=20
                let (slice_l, slice_r, slice_t, slice_b) = (20, 20, 5, 10);
                
                // Target size matches the resized cover size
                let (target_w, target_h) = (new_w, new_h);
                
                let scaled_book = nine_slice_scale(&book_img, target_w, target_h, slice_l, slice_r, slice_t, slice_b);
                
                // Overlay ON TOP of the cover
                image::imageops::overlay(&mut canvas, &scaled_book, x as i64, y as i64);
            } else {
                println!("Warning: Book.png not found at {}", book_path);
            }
            
            let output_path = "epub_cover_test.png";
            
            // Save as PNG
            canvas.save_with_format(output_path, ImageFormat::Png)?;
            
            println!("Cover saved to: {}", output_path);
        },
        Err(e) => {
            println!("No cover found or error extracting: {}", e);
        }
    }

    Ok(())
}
