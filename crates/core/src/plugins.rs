use std::path::Path;
use crate::RendererBackend;

pub mod filament;
pub mod psd;
pub mod text;

pub use filament::FilamentGenerator;
pub use psd::PsdGenerator;
pub use text::TextGenerator;

/// Trait for thumbnail generators.
/// Plugins or built-in renderers must implement this to handle specific file formats.
pub trait ThumbnailGenerator {
    /// Friendly name of the generator (e.g., "Filament 3D Renderer", "PDFium Wrapper")
    fn name(&self) -> &str;

    /// Check if this generator can handle the given file.
    /// `header`: First few bytes of the file (for magic number checks).
    /// `extension`: File extension (lowercase, without dot).
    fn validate(&self, header: &[u8], extension: &str) -> bool;

    /// Generate the thumbnail.
    /// `buffer`: Full file content (optional).
    /// `width`, `height`: Requested thumbnail dimensions.
    /// `extension`: File extension (useful hint if filepath is None).
    /// `filepath`: Path to the file (optional).
    /// Returns: RGBA pixel buffer.
    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, extension: &str, filepath: Option<&Path>) -> Result<Vec<u8>, String>;
}

pub struct PluginManager {
    generators: Vec<Box<dyn ThumbnailGenerator>>,
}

impl PluginManager {
    pub fn new() -> Self {
        // Default to Vulkan if not specified (backward compatibility)
        Self::with_backend(RendererBackend::Vulkan)
    }

    pub fn with_backend(backend: RendererBackend) -> Self {
        Self {
            generators: vec![
                Box::new(FilamentGenerator::new(backend)),
                Box::new(PsdGenerator),
                Box::new(TextGenerator),
            ],
        }
    }

    pub fn get_generator(&self, header: &[u8], extension: &str) -> Option<&dyn ThumbnailGenerator> {
        for gen in &self.generators {
            if gen.validate(header, extension) {
                return Some(gen.as_ref());
            }
        }
        None
    }
}
