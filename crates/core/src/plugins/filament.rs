use std::path::Path;
use crate::{SpaceThumbnailsRenderer, RendererBackend};
use super::ThumbnailGenerator;
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;
use std::fs::File;
use image::RgbaImage;

/// Built-in Filament renderer
pub struct FilamentGenerator {
    backend: RendererBackend,
}

impl FilamentGenerator {
    pub fn new(backend: RendererBackend) -> Self {
        Self { backend }
    }
}

impl ThumbnailGenerator for FilamentGenerator {
    fn name(&self) -> &str {
        "Filament 3D Renderer"
    }

    fn validate(&self, header: &[u8], extension: &str) -> bool {
        if header.starts_with(b"glTF") {
            return true;
        }
        matches!(extension, 
            "glb" | "gltf" | "obj" | "fbx" | "dae" | "ply" | "stl" | "3ds" | 
            "stp" | "step" | "igs" | "iges" | "x3d" | "x3db" | "usdz"
        )
    }

    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, extension: &str, filepath: Option<&Path>) -> Result<Vec<u8>, String> {
        let mut renderer = SpaceThumbnailsRenderer::new(self.backend, width, height);
        
        // Filament/Assimp wrapper needs either a buffer or a file.
        
        let res = if extension == "usdz" {
            if let Some(path) = filepath {
                renderer.load_usdz_asset(path)
            } else {
                // If we only have buffer, we might need to write to temp file for usdz2obj
                // But currently load_usdz_asset takes a path. 
                // For now, fail if no path (CLI always provides path)
                 None
            }
        } else if let Some(path) = filepath {
            renderer.load_asset_from_file(path)
        } else if let Some(buf) = buffer {
            // Construct a dummy filename with the correct extension for Assimp hint
            let hint = format!("file.{}", extension);
            renderer.load_asset_from_memory(buf, &hint)
        } else {
            return Err("FilamentGenerator requires either buffer or filepath".to_string());
        };

        if res.is_none() {
            // Fallback for USDZ: Try to extract image from ZIP
            if extension == "usdz" {
                if let Some(img_data) = try_extract_usdz_image(buffer, filepath) {
                    if let Ok(img) = image::load_from_memory(&img_data) {
                        // Create a transparent canvas
                        let mut canvas = RgbaImage::new(width, height);
                        
                        // Resize image to fit within canvas (preserve aspect ratio)
                        let resized = img.resize(width, height, image::imageops::FilterType::Lanczos3);
                        
                        // Center the image
                        let x = (width - resized.width()) / 2;
                        let y = (height - resized.height()) / 2;
                        
                        image::imageops::overlay(&mut canvas, &resized, x as i64, y as i64);
                        
                        return Ok(canvas.into_raw());
                    }
                }
            }
            return Err("Failed to load 3D asset".to_string());
        }

        let mut out = vec![0; renderer.get_screenshot_size_in_byte()];
        renderer.take_screenshot_sync(&mut out);
        Ok(out)
    }
}

fn try_extract_usdz_image(buffer: Option<&[u8]>, filepath: Option<&Path>) -> Option<Vec<u8>> {
    if let Some(buf) = buffer {
        return extract_from_archive(Cursor::new(buf));
    } else if let Some(path) = filepath {
        if let Ok(file) = File::open(path) {
            return extract_from_archive(file);
        }
    }
    None
}

fn extract_from_archive<R: Read + Seek>(reader: R) -> Option<Vec<u8>> {
    let mut archive = ZipArchive::new(reader).ok()?;

    let mut best_file_index = None;
    let mut best_score = 0;
    let mut max_size = 0;

    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_lowercase();
            let size = file.size();
            
            let is_image = name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg");
            if !is_image { continue; }

            let mut score = 1;
            if name.contains("thumbnail") { score += 10; }
            if name.contains("preview") { score += 5; }
            
            if score > best_score {
                best_score = score;
                best_file_index = Some(i);
                max_size = size;
            } else if score == best_score {
                if size > max_size {
                    max_size = size;
                    best_file_index = Some(i);
                }
            }
        }
    }

    let index = best_file_index?;
    let mut file = archive.by_index(index).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    Some(buf)
}
