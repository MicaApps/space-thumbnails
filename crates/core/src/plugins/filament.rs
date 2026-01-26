use std::path::Path;
use crate::{SpaceThumbnailsRenderer, RendererBackend};
use super::ThumbnailGenerator;

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
            "stp" | "step" | "igs" | "iges" | "x3d" | "x3db"
        )
    }

    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, extension: &str, filepath: Option<&Path>) -> Result<Vec<u8>, String> {
        let mut renderer = SpaceThumbnailsRenderer::new(self.backend, width, height);
        
        // Filament/Assimp wrapper needs either a buffer or a file.
        
        let res = if let Some(path) = filepath {
            renderer.load_asset_from_file(path)
        } else if let Some(buf) = buffer {
            // Construct a dummy filename with the correct extension for Assimp hint
            let hint = format!("file.{}", extension);
            renderer.load_asset_from_memory(buf, &hint)
        } else {
            return Err("FilamentGenerator requires either buffer or filepath".to_string());
        };

        if res.is_none() {
            return Err("Failed to load 3D asset".to_string());
        }

        let mut out = vec![0; renderer.get_screenshot_size_in_byte()];
        renderer.take_screenshot_sync(&mut out);
        Ok(out)
    }
}
