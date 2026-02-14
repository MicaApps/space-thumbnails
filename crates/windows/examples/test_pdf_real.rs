use space_thumbnails_windows::providers::pdf_thumbnail;
use std::fs;

fn main() {
    println!("Testing PDF thumbnail provider with real PDF rendering...");
    
    // Read the test PDF file
    let pdf_path = "../../test_files/test.pdf";
    match fs::read(pdf_path) {
        Ok(pdf_data) => {
            println!("✓ Loaded PDF file: {} ({} bytes)", pdf_path, pdf_data.len());
            
            // Test the render_pdf_to_bitmap function with real PDF data
            let result = pdf_thumbnail::render_pdf_to_bitmap(&pdf_data, 256, 256);
            
            match result {
                Some(buffer) => {
                    println!("✓ Successfully generated thumbnail buffer with {} bytes", buffer.len());
                    
                    // Save the buffer as a PNG file for verification
                    use image::{ImageBuffer, Rgba};
                    let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(256, 256, buffer).unwrap();
                    let output_path = "pdf_thumbnail_real_test.png";
                    match img.save(output_path) {
                        Ok(_) => println!("✓ Thumbnail saved to: {}", output_path),
                        Err(e) => println!("✗ Failed to save thumbnail: {}", e),
                    }
                }
                None => {
                    println!("✗ Failed to generate thumbnail - check debug log for details");
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to load PDF file: {}", e);
        }
    }
}