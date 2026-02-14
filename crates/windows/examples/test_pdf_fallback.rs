use space_thumbnails_windows::providers::pdf_thumbnail;

fn main() {
    println!("Testing PDF thumbnail provider with disk-based PNG fallback...");
    
    // Test the render_pdf_to_bitmap function with dummy data
    let dummy_pdf_data = b"dummy pdf data";
    let result = pdf_thumbnail::render_pdf_to_bitmap(dummy_pdf_data, 256, 256);
    
    match result {
        Some(buffer) => {
            println!("✓ Successfully generated thumbnail buffer with {} bytes", buffer.len());
            
            // Save the buffer as a PNG file for verification
            use image::{ImageBuffer, Rgba};
            let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(256, 256, buffer).unwrap();
            let output_path = "pdf_thumbnail_test.png";
            match img.save(output_path) {
                Ok(_) => println!("✓ Thumbnail saved to: {}", output_path),
                Err(e) => println!("✗ Failed to save thumbnail: {}", e),
            }
        }
        None => {
            println!("✗ Failed to generate thumbnail");
        }
    }
}