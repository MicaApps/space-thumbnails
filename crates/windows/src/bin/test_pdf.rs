use windows::{
    Data::Pdf::{PdfDocument, PdfPageRenderOptions},
    Storage::StorageFile,
    Storage::Streams::InMemoryRandomAccessStream,
};

use image::RgbaImage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Hardcoded path from user
    let path_str = r"D:\Users\Shomn\OneDrive - MSFT\Documents\Apple建议\Apple Support Case 102452405098 7_11_2024_8_29pm.pdf";
    
    // Remove UNC prefix if present
    let clean_path = if path_str.starts_with(r"\\?\") {
        &path_str[4..]
    } else {
        path_str
    };
    
    println!("Loading path: {}", clean_path);
    let file = StorageFile::GetFileFromPathAsync(windows::core::HSTRING::from(clean_path))?.get()?;
    println!("File loaded successfully.");
    
    let doc = PdfDocument::LoadFromFileAsync(&file)?.get()?;
    let page_count = doc.PageCount()?;
    println!("Document loaded. Page count: {}", page_count);

    // Render configuration
    let cx = 256u32;
    let padding_ratio = 40.0 / 256.0;
    let max_side = (cx as f32 * (1.0 - padding_ratio)).max(1.0);
    
    // Helper function to render a page
    let render_page = |page_index: u32| -> Result<RgbaImage, Box<dyn std::error::Error>> {
        if page_index >= page_count {
            return Err("Page index out of bounds".into());
        }
        
        let page = doc.GetPage(page_index)?;
        let src_size = page.Size()?;
        
        let scale = (max_side / src_size.Width).min(max_side / src_size.Height);
        let render_width = (src_size.Width * scale) as u32;
        let render_height = (src_size.Height * scale) as u32;

        let options = PdfPageRenderOptions::new()?;
        options.SetDestinationWidth(render_width)?;
        options.SetDestinationHeight(render_height)?;
        // Set background to white (opaque)
        options.SetBackgroundColor(windows::UI::Color { A: 255, R: 255, G: 255, B: 255 })?;
        
        let stream = InMemoryRandomAccessStream::new()?;
        // Try method with options
        page.RenderWithOptionsToStreamAsync(&stream, &options)?.get()?;
        
        stream.Seek(0)?;
        let size = stream.Size()? as usize;
        let mut buffer = vec![0u8; size];
        
        // Fix DataReader usage
        use windows::Storage::Streams::{DataReader, IInputStream};
        // let reader = DataReader::CreateInputStream(&stream.cast::<IInputStream>()?)?;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        reader.LoadAsync(size as u32)?.get()?;
        reader.ReadBytes(&mut buffer)?;
        
        let mut img = image::load_from_memory(&buffer)?.to_rgba8();
        
        // Ensure exact size constraints (DPI handling)
        let width = img.width();
        let height = img.height();
        
        let scale_w = max_side / width as f32;
        let scale_h = max_side / height as f32;
        let final_scale = scale_w.min(scale_h);
        
        let new_width = (width as f32 * final_scale) as u32;
        let new_height = (height as f32 * final_scale) as u32;

        if new_width != width || new_height != height {
            println!("Resizing page {}: {}x{} -> {}x{}", page_index, width, height, new_width, new_height);
            img = image::imageops::resize(&img, new_width, new_height, image::imageops::FilterType::Lanczos3);
        }
        
        Ok(img)
    };

    // Render pages
    let img0 = render_page(0)?;
    let img1 = if page_count > 1 {
        Some(render_page(1)?)
    } else {
        None
    };

    // Create composition canvas
    let mut canvas = RgbaImage::new(cx, cx);
    // Fill with transparent (already default, but being explicit)
    for pixel in canvas.pixels_mut() {
        *pixel = image::Rgba([0, 0, 0, 0]);
    }

    // Helper to center image
    let center_image = |target: &mut RgbaImage, src: &RgbaImage| {
        let x_offset = (cx as i64 - src.width() as i64) / 2;
        let y_offset = (cx as i64 - src.height() as i64) / 2;
        image::imageops::overlay(target, src, x_offset, y_offset);
    };

    // Calculate stroke width relative to 256px
    let scale_factor = cx as f32 / 256.0;
    let stroke_width = (3.0 * scale_factor).max(1.0) as u32;
    let crop_size = (49.0 * scale_factor).max(1.0) as u32;

    // 1. Draw Page 1 (Bottom) if exists
    if let Some(ref img) = img1 {
        // Page 1 should also have a stroke
        let mut img1_processed = RgbaImage::from_pixel(
            img.width() + 2 * stroke_width,
            img.height() + 2 * stroke_width,
            image::Rgba([117, 116, 113, 255])
        );
        image::imageops::overlay(&mut img1_processed, img, stroke_width as i64, stroke_width as i64);
        
        center_image(&mut canvas, &img1_processed);
        println!("Composed Page 1 (Bottom) with stroke");
    }

    // 2. Process Page 0 (Top) - Crop Top-Right, then Add Stroke
    let mut img0_base = img0.clone();
    
    println!("Applying stroke: {}px, Crop: {}px", stroke_width, crop_size);
    
    // Step 1: Crop the base image
    let width = img0_base.width();
    let height = img0_base.height();
    
    // Crop the top-right corner of the base image
    let start_x = if width >= crop_size { width - crop_size } else { 0 }; 
    let end_x = width;
    
    let start_y = 0;
    let end_y = crop_size.min(height);

    println!("Cropping Base Image: Rect [{}, {}] to [{}, {}]", start_x, start_y, end_x, end_y);

    for y in start_y..end_y {
        for x in start_x..end_x {
             if x < width && y < height {
                img0_base.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
             }
        }
    }
    
    // Step 2: Create bordered background
    let base_width = img0_base.width();
    let base_height = img0_base.height();
    let bordered_width = base_width + 2 * stroke_width;
    let bordered_height = base_height + 2 * stroke_width;
    
    let mut img0_processed = RgbaImage::from_pixel(bordered_width, bordered_height, image::Rgba([117, 116, 113, 255])); // #757471
    
    // Step 3: Crop the background with the SAME logic (relative to its own size)
    // The background is larger, but the crop size is the same, creating a border effect along the cut.
    let bg_width = img0_processed.width();
    let bg_height = img0_processed.height();
    
    let bg_start_x = if bg_width >= crop_size { bg_width - crop_size } else { 0 }; 
    let bg_end_x = bg_width;
    
    let bg_start_y = 0;
    let bg_end_y = crop_size.min(bg_height);

    println!("Cropping Background: Rect [{}, {}] to [{}, {}]", bg_start_x, bg_start_y, bg_end_x, bg_end_y);

    for y in bg_start_y..bg_end_y {
        for x in bg_start_x..bg_end_x {
             if x < bg_width && y < bg_height {
                img0_processed.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
             }
        }
    }

    // Step 4: Overlay the cropped base image onto the cropped background
    image::imageops::overlay(&mut img0_processed, &img0_base, stroke_width as i64, stroke_width as i64);

    // Step 5: Load and Overlay the Fold Image (Top-Right of the bordered composition)
    // Embed the fold image bytes
    const FOLD_BYTES: &[u8] = include_bytes!("../../assets/PDF-Folder.png");
    let fold_img_dynamic = image::load_from_memory(FOLD_BYTES).expect("Failed to load fold image");
    let fold_img = fold_img_dynamic.to_rgba8();
    
    // Resize fold image to match crop_size x crop_size
    let fold_img_resized = image::imageops::resize(&fold_img, crop_size, crop_size, image::imageops::FilterType::Lanczos3);
    
    // Overlay at the top-right corner of the *content* (ignoring the outer stroke)
    // The content ends at (bg_width - stroke_width).
    // The fold image should be placed at (content_right - crop_size)
    // = (bg_width - stroke_width) - crop_size
    // Y position is simply stroke_width (top of content)
    
    let fold_x = (bg_width - stroke_width - crop_size) as i64;
    let fold_y = stroke_width as i64;
    
    println!("Overlaying Fold Image: {}x{} at ({}, {})", crop_size, crop_size, fold_x, fold_y);
    image::imageops::overlay(&mut img0_processed, &fold_img_resized, fold_x, fold_y);
    
    // 3. Draw Page 0 (Top)
    center_image(&mut canvas, &img0_processed);
    println!("Composed Page 0 (Top)");

    // Step 6: Load and Overlay the Binder Image (Left of the content area)
    // Embed the binder image bytes
    const BINDER_BYTES: &[u8] = include_bytes!("../../assets/PDF-binder.png");
    let binder_img_dynamic = image::load_from_memory(BINDER_BYTES).expect("Failed to load binder image");
    let binder_img = binder_img_dynamic.to_rgba8();

    // Determine the total content area height (Page 0 vs Page 1)
    // Since both are centered on the canvas, the vertical extent is determined by the max height.
    // Note: img0_processed has stroke, img1 does not.
    // The user said "height is the total height of overlapping pages (excluding stroke)".
    // So we use max(img0.height(), img1.height()).
    let img1_height = img1.as_ref().map(|i| i.height()).unwrap_or(0);
    let img1_width = img1.as_ref().map(|i| i.width()).unwrap_or(0);
    let content_height = img0.height().max(img1_height);
    let content_width = img0.width().max(img1_width);
    
    // Resize binder to 8px width (scaled) x content_height
    // User said "Width is always 8px". Assuming scaled 8px.
    let binder_width = (8.0 * scale_factor).max(1.0) as u32;
    let binder_img_resized = image::imageops::resize(&binder_img, binder_width, content_height, image::imageops::FilterType::Lanczos3);

    // Calculate position
    // Center of canvas is (cx/2, cx/2).
    // Content starts at (cx - content_width) / 2.
    // Binder should be at this X.
    // Vertical position: (cx - content_height) / 2.
    let binder_x = (cx as i64 - content_width as i64) / 2;
    let binder_y = (cx as i64 - content_height as i64) / 2;

    println!("Overlaying Binder Image: {}x{} at ({}, {})", binder_width, content_height, binder_x, binder_y);
    image::imageops::overlay(&mut canvas, &binder_img_resized, binder_x, binder_y);

    // Save result
    canvas.save("test_dual_page.png")?;
    println!("Saved composition to test_dual_page.png");

    Ok(())
}
