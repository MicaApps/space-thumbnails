use std::fs;
use std::path::Path;
use png::{Encoder, ColorType, BitDepth};
use std::io::BufWriter;
use std::fs::File;

fn main() {
    let rgba_path = Path::new("D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\test_files\\testpdf2.rgba");
    let png_path = Path::new("D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\test_files\\testpdf2.png");

    let rgba_data = match fs::read(rgba_path) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to read RGBA file: {}", e);
            return;
        }
    };

    // 现在总是256x256画布
    let width = 256u32;
    let height = 256u32;
    
    // 验证文件大小
    let expected_size = (width * height * 4) as usize;
    if rgba_data.len() != expected_size {
        println!("Warning: Expected {} bytes, got {} bytes", expected_size, rgba_data.len());
    }
    
    println!("Image dimensions: {} x {} pixels", width, height);

    let file = match File::create(png_path) {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to create PNG file: {}", e);
            return;
        }
    };

    let ref mut w = BufWriter::new(file);
    let mut encoder = Encoder::new(w, width as u32, height as u32);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    
    let mut writer = match encoder.write_header() {
        Ok(writer) => writer,
        Err(e) => {
            println!("Failed to write PNG header: {}", e);
            return;
        }
    };

    match writer.write_image_data(&rgba_data) {
        Ok(_) => {
            println!("Successfully converted RGBA to PNG: {:?}", png_path);
        },
        Err(e) => {
            println!("Failed to write PNG data: {}", e);
        }
    }
}