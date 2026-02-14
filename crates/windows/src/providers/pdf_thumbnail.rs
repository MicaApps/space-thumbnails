use crate::providers::{
    thumbnail::{IThumbnailProvider, Thumbnail},
    Provider,
};
use crate::registry::{register_clsid, RegistryKey, RegistryValue};
use crate::utils::log_debug;
use image;
use windows::core::{implement, AsImpl, GUID};

pub struct PdfThumbnailProvider {
    clsid: GUID,
}

impl PdfThumbnailProvider {
    pub fn new() -> Self {
        Self {
            clsid: GUID::from("E3A82405-A21A-4423-B644-93B072E2E7C9"),
        }
    }
}

impl Provider for PdfThumbnailProvider {
    fn clsid(&self) -> GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<RegistryKey> {
        let mut keys = register_clsid(&self.clsid, module_path, false);
        keys.push(RegistryKey {
            path: format!("\\.pdf\\ShellEx\\{{e357fccd-a995-4576-b01f-234630154e96}}"),
            values: vec![RegistryValue(
                "".to_owned(),
                crate::registry::RegistryData::Str(format!("{{{:?}}}", self.clsid)),
            )],
        });
        keys
    }

    fn create_instance(
        &self,
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let thumbnail: IThumbnailProvider = Thumbnail::new().into();
        unsafe { thumbnail.query(riid, ppv_object) }
    }
}

pub fn render_pdf_to_bitmap(
    _pdf_data: &[u8],
    width: i32,
    height: i32,
) -> Option<Vec<u8>> {
    let fallback_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails5\test_files\error_fallback.png";
    
    log_debug(&format!("Trying to load fallback PNG from: {}", fallback_path));
    
    match image::open(fallback_path) {
        Ok(img) => {
            log_debug(&format!("Successfully loaded PNG, original size: {}x{}", img.width(), img.height()));
            log_debug(&format!("Resizing to {}x{}", width, height));
            let resized_img = img.resize_to_fill(width as u32, height as u32, image::imageops::FilterType::Triangle);
            let rgba_image = resized_img.to_rgba8();
            let buffer_size = rgba_image.len();
            log_debug(&format!("Converted to RGBA8, buffer size: {} bytes", buffer_size));
            Some(rgba_image.into_raw())
        },
        Err(e) => {
            log_debug(&format!("Failed to load PNG: {}", e));
            None
        }
    }
}
