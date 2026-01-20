use std::env;
use std::fs::File;
use image::io::Reader as ImageReader;
use image::GenericImageView;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <png_file>", args[0]);
        return;
    }
    
    let path = &args[1];
    let img = ImageReader::open(path).expect("Failed to open file").decode().expect("Failed to decode image");
    let img = img.resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    let rgba = img.to_rgba8();
    
    print!("[");
    for (i, byte) in rgba.as_raw().iter().enumerate() {
        if i % 32 == 0 {
            print!("\n    ");
        }
        print!("{}, ", byte);
    }
    println!("\n];");
}