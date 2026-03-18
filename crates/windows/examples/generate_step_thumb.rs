use space_thumbnails::{SpaceThumbnailsRenderer, RendererBackend};
use std::path::Path;
use std::fs;
use image::DynamicImage;

#[cfg(target_os = "windows")]
#[link(name = "advapi32")]
extern "C" {}

fn main() {
    let input_path = r"D:\Users\Shomn\OneDrive - MSFT\Work\同学帮忙\任国维\Keyshot渲染\S2A-04-00送料机.STEP";
    let output_path = "Feeder_Preview_Final.png";
    let width = 1024;
    let height = 1024;

    println!("Initializing renderer...");
    let mut renderer = SpaceThumbnailsRenderer::new(RendererBackend::Default, width, height)
        .expect("Failed to create renderer");

    println!("Loading asset: {}", input_path);
    if renderer.load_asset_from_file(Path::new(input_path)).is_none() {
        panic!("Failed to load STEP asset");
    }

    println!("Rendering...");
    let mut buffer = vec![0u8; renderer.get_screenshot_size_in_byte()];
    renderer.take_screenshot_sync(&mut buffer);

    println!("Saving to {}...", output_path);
    let img = image::RgbaImage::from_raw(width, height, buffer)
        .expect("Failed to create image from buffer");
    
    DynamicImage::ImageRgba8(img).save(output_path).expect("Failed to save image");

    println!("Successfully saved preview to {}", output_path);
}
