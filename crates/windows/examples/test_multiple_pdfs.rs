use space_thumbnails_windows::providers::pdf_thumbnail;
use std::fs;

fn main() {
    println!("Testing PDF thumbnail provider with multiple PDF files...");
    
    let test_files = vec![
        ("../../test_files/test.pdf", "pdf1_thumbnail.png"),
        ("../../test_files/test2.pdf", "pdf2_thumbnail.png"),
    ];
    
    for (pdf_path, output_name) in test_files {
        println!("\n--- Testing {} ---", pdf_path);
        
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
                        match img.save(output_name) {
                            Ok(_) => println!("✓ Thumbnail saved to: {}", output_name),
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
    
    println!("\n✓ All tests completed! Check the generated PNG files.");
}