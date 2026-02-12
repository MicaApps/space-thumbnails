fn main() {
    println!("Testing PDFium integration...");
    
    // Read the test PDF file
    let pdf_data = match std::fs::read("test_pdf.pdf") {
        Ok(data) => {
            println!("Successfully read PDF file: {} bytes", data.len());
            data
        }
        Err(e) => {
            println!("Failed to read PDF file: {}", e);
            return;
        }
    };
    
    // Test PDFium rendering
    println!("Attempting to render PDF to bitmap...");
    match space_thumbnails_windows::providers::pdf_thumbnail::render_pdf_to_bitmap(&pdf_data, 256, 256) {
        Some(bitmap_data) => {
            println!("Successfully rendered PDF to bitmap: {} bytes", bitmap_data.len());
            
            // Save the bitmap as a raw RGBA file for verification
            let output_path = "test_pdf_thumbnail.rgba";
            match std::fs::write(output_path, &bitmap_data) {
                Ok(_) => println!("Saved thumbnail to: {}", output_path),
                Err(e) => println!("Failed to save thumbnail: {}", e),
            }
        }
        None => {
            println!("Failed to render PDF to bitmap");
        }
    }
    
    println!("PDFium integration test completed.");
}