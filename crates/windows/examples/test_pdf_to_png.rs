use std::fs;
use std::path::Path;

fn main() {
    let pdf_path = Path::new("D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\test_files\\test.pdf");
    let output_path = Path::new("D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\test_files\\test.png");

    let pdf_data = match fs::read(pdf_path) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to read PDF file: {}", e);
            return;
        }
    };

    if let Err(e) = space_thumbnails_windows::providers::pdf_thumbnail::render_pdf_to_png(&pdf_data, 256, 256, output_path) {
        println!("Failed to render PDF to PNG: {}", e);
    } else {
        println!("Successfully rendered PDF to PNG: {:?}", output_path);
    }
}
