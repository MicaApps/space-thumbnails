use std::path::{Path, PathBuf};
use image::{RgbaImage, imageops::FilterType};
use pdfium_render::prelude::*;

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

pub fn render_document(buffer: Option<&[u8]>, filepath: Option<&Path>, width: u32, height: u32, style: PdfRendererStyle) -> Result<Vec<u8>, String> {
    // 1. Initialize PDFium and Load Content (Used for AI, Docx-PDF, Excel-PDF)
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
             return Err("No buffer or filepath provided for document".to_string());
         }
    }.map_err(|e| format!("Failed to load document: {}", e))?;

    // Render first page
    let page = document.pages().get(0).map_err(|e| format!("Failed to get first page: {}", e))?;
    
    // Render at high resolution then downscale for better quality
    // Do NOT auto-rotate landscape pages to portrait (Degrees0) so they stay horizontal
    // Use 4x resolution for better quality downsampling
    
    // NOTE: For best text quality and correct colors (especially subpixel AA),
    // we should render directly onto a WHITE opaque background, not transparent.
    // PDFium's LCD_TEXT flag only works correctly when rendering onto an opaque background.
    
    // Calculate target dimensions
    let target_width = (width * 4) as i32;
    let target_height = (height * 4) as i32;
    
    // Render at high resolution then downscale for better quality
    // Do NOT auto-rotate landscape pages to portrait (Degrees0) so they stay horizontal
    // Use 4x resolution for better quality downsampling
    let render_config = PdfRenderConfig::new()
        .set_target_width((width * 4) as i32)
        .set_maximum_height((height * 4) as i32)
        .rotate_if_landscape(PdfPageRenderRotation::None, true);

    let bitmap = page.render_with_config(&render_config)
        .map_err(|e| format!("Failed to render page: {}", e))?;
    
    let cover = bitmap.as_image(); // Returns DynamicImage
    let cover_rgba = cover.to_rgba8();

    // COMPOSITE ON WHITE MANUALLY (Handling Pre-multiplied Alpha)
    // 1. Gamma Correction: Boost Alpha to make text thicker/bolder.
    // 2. Contrast Enhancement: Darken RGB values to make text blacker.
    
    let mut high_res_paper = RgbaImage::new(cover_rgba.width(), cover_rgba.height());
    
    for (x, y, pixel) in cover_rgba.enumerate_pixels() {
        // Raw Alpha from PDFium (Pre-multiplied)
        let raw_alpha = pixel[3] as f32 / 255.0;
        
        if raw_alpha > 0.0 {
            let r_in = pixel[0] as f32;
            let g_in = pixel[1] as f32;
            let b_in = pixel[2] as f32;

            // 1. Gamma Correction for Alpha (Thicken Text)
            // 0.5 is a strong boost: 0.2 -> 0.45, 0.5 -> 0.71
            let new_alpha = raw_alpha.powf(0.5).min(1.0);
            
            // Calculate scale to maintain color ratio during alpha boost
            // stored_rgb = real_rgb * raw_alpha
            // target_rgb = real_rgb * new_alpha
            // scale = new_alpha / raw_alpha
            let scale = new_alpha / raw_alpha;

            // 2. Apply Boost and Darken (Contrast Enhancement)
            // Multiply by 0.7 to darken the color (make grey text blacker)
            // We only darken if the pixel is NOT white/light.
            // But since we are dealing with pre-multiplied values, it's tricky.
            // Let's just darken everything. White (255) * 0.7 becomes Grey, which is bad for background.
            // Wait, PDFium background is transparent (alpha=0). 
            // The "White" pixels we see are actually transparent pixels mixed with our white background later.
            // The pixels HERE are the content.
            // If content is White text, we shouldn't darken it.
            // If content is Black text, we should darken it.
            
            // Simple approach: Just apply alpha boost. The darkening happens naturally because we mix less White background.
            // If we want to force blacker text, we can scale RGB down.
            // Let's try a mild darkening for non-saturated colors.
            
            let darken_factor = 0.8; 

            let r_boosted = (r_in * scale * darken_factor).min(255.0);
            let g_boosted = (g_in * scale * darken_factor).min(255.0);
            let b_boosted = (b_in * scale * darken_factor).min(255.0);
            
            // 3. Composite over White
            // Final = Src + Dst * (1 - Alpha)
            let r_out = (r_boosted + 255.0 * (1.0 - new_alpha)).min(255.0) as u8;
            let g_out = (g_boosted + 255.0 * (1.0 - new_alpha)).min(255.0) as u8;
            let b_out = (b_boosted + 255.0 * (1.0 - new_alpha)).min(255.0) as u8;
            
            high_res_paper.put_pixel(x, y, image::Rgba([r_out, g_out, b_out, 255]));
        } else {
            // Fully transparent -> White
            high_res_paper.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
        }
    }
    
    // Convert back to DynamicImage for resizing convenience
    let cover_opaque = image::DynamicImage::ImageRgba8(high_res_paper);

    // DEBUG: Save raw high-res render to verify clarity
    // This allows the user to see the pre-downsampled result
    let _ = cover_opaque.save("control-panel/Assets/Previews/excel_debug_1024.png");

    // Calculate margins (20px at 256px resolution)
    let scale_factor = width as f32 / 256.0;
    let margin = (20.0 * scale_factor).round() as u32;
    let border_size = 3;

    // Content constraint: Keep top/bottom margins 20px (content size)
    // The border is added "outside" this content area.
    let max_w = width.saturating_sub(margin * 2);
    let max_h = height.saturating_sub(margin * 2);
    
    // Use Lanczos3 for better downsampling quality
    // Resize the OPAQUE image
    let cover_scaled = cover_opaque.resize(max_w, max_h, FilterType::Lanczos3);
    
    // Add 3px border (#797774)
    let frame_w = cover_scaled.width() + (border_size * 2);
    let frame_h = cover_scaled.height() + (border_size * 2);
    
    // #797774 -> R:121, G:119, B:116
    let mut framed_cover = RgbaImage::from_pixel(frame_w, frame_h, image::Rgba([121, 119, 116, 255]));
    
    image::imageops::overlay(&mut framed_cover, &cover_scaled, border_size as i64, border_size as i64);
    
    // Apply Fold Style if requested
    // This logic mimics the PdfGenerator's fold implementation
    if style == PdfRendererStyle::Folded {
        // Load Fold Asset
        const FOLD_BYTES: &[u8] = include_bytes!("../assets/file_fold_256.png");
        if let Ok(fold_img) = image::load_from_memory(FOLD_BYTES) {
            let mut fold_rgba = fold_img.to_rgba8();
            
            // Scale fold if needed (relative to 256px standard)
            if width != 256 {
                let scale = width as f32 / 256.0;
                let new_w = (fold_rgba.width() as f32 * scale) as u32;
                let new_h = (fold_rgba.height() as f32 * scale) as u32;
                if new_w > 0 && new_h > 0 {
                    fold_rgba = image::imageops::resize(&fold_rgba, new_w, new_h, FilterType::Triangle);
                }
            }

            // Fold alignment: Top-Right of the Frame
            let frame_w_val = framed_cover.width();
            
            // Fold position relative to framed_cover
            let fold_x_rel = frame_w_val.saturating_sub(fold_rgba.width());
            let fold_y_rel = 0; // Top aligned
            
            // Cut logic on framed_cover (make pixels transparent under the fold)
            let fw = fold_rgba.width() as i64;
            let fh = fold_rgba.height() as i64;
            
            for fy in 0..fh {
                for fx in 0..fw {
                    // Diagonal cut
                    if fy * fw < fx * fh {
                        let cx = fold_x_rel as i64 + fx;
                        let cy = fold_y_rel as i64 + fy;
                        
                        if cx >= 0 && cy >= 0 && cx < framed_cover.width() as i64 && cy < framed_cover.height() as i64 {
                            framed_cover.put_pixel(cx as u32, cy as u32, image::Rgba([0, 0, 0, 0]));
                        }
                    }
                }
            }
        }
    }

    let mut canvas = RgbaImage::new(width, height);
    let x = (width - frame_w) / 2;
    let y = (height - frame_h) / 2;
    
    image::imageops::overlay(&mut canvas, &framed_cover, x as i64, y as i64);

    // Overlay Fold Asset on Canvas (for Folded style)
    if style == PdfRendererStyle::Folded {
        const FOLD_BYTES: &[u8] = include_bytes!("../assets/file_fold_256.png");
        if let Ok(fold_img) = image::load_from_memory(FOLD_BYTES) {
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
            
            // Calculate absolute position on canvas
            // Frame x + Frame Width - Fold Width
            // Note: framed_cover.width() should be same as frame_w used in x calculation
            let fold_x = (x + framed_cover.width()).saturating_sub(fold_rgba.width());
            let fold_y = y;
            
            image::imageops::overlay(&mut canvas, &fold_rgba, fold_x as i64, fold_y as i64);
        }
    }

    Ok(canvas.into_raw())
}
