use std::cell::Cell;
use std::io::Read;
use windows::core::{implement, IUnknown, Interface, GUID, Result};
use windows::Win32::UI::Shell::PropertiesSystem::{IInitializeWithStream, IInitializeWithStream_Impl};
use windows::Win32::UI::Shell::{IThumbnailProvider, IThumbnailProvider_Impl, WTS_ALPHATYPE, WTSAT_ARGB};
use windows::Win32::Graphics::Gdi::HBITMAP;
use crate::registry::{RegistryKey, RegistryValue, RegistryData, register_clsid};
use crate::utils::{WinStream, create_argb_bitmap};
use super::Provider;
use image::GenericImageView;

pub struct PsdThumbnailProvider {
    pub clsid: GUID,
}

impl PsdThumbnailProvider {
    pub fn new(clsid: GUID) -> Self {
        Self { clsid }
    }
}

impl Provider for PsdThumbnailProvider {
    fn clsid(&self) -> GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<RegistryKey> {
        let mut result = register_clsid(&self.clsid, module_path, false);
        result.push(RegistryKey {
            path: format!(
                ".psd\\ShellEx\\{{{:?}}}",
                IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_string(),
                RegistryData::Str(format!("{{{:?}}}", self.clsid)),
            )],
        });
        result
    }

    fn create_instance(
        &self,
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        PsdThumbnailHandler::new(riid, ppv_object)
    }
}

#[implement(IThumbnailProvider, IInitializeWithStream)]
pub struct PsdThumbnailHandler {
    stream: Cell<Option<WinStream>>,
}

impl PsdThumbnailHandler {
    pub fn new(riid: *const GUID, ppv_object: *mut *mut core::ffi::c_void) -> Result<()> {
        let unknown: IUnknown = PsdThumbnailHandler {
            stream: Cell::new(None),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl IInitializeWithStream_Impl for PsdThumbnailHandler {
    fn Initialize(&self, pstream: &Option<windows::Win32::System::Com::IStream>, _grfmode: u32) -> Result<()> {
        if let Some(stream) = pstream {
             self.stream.set(Some(WinStream::from(stream.clone())));
        }
        Ok(())
    }
}

impl IThumbnailProvider_Impl for PsdThumbnailHandler {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        alphatype: *mut WTS_ALPHATYPE,
    ) -> Result<()> {
        let mut stream = match self.stream.take() {
            Some(s) => s,
            None => return Err(windows::core::Error::from(windows::Win32::Foundation::E_FAIL)),
        };

        let mut buffer = Vec::new();
        if stream.read_to_end(&mut buffer).is_err() {
            return Err(windows::core::Error::from(windows::Win32::Foundation::E_FAIL));
        }

        // Parse PSD
        let psd = match psd::Psd::from_bytes(&buffer) {
            Ok(p) => p,
            Err(_) => return Err(windows::core::Error::from(windows::Win32::Foundation::E_FAIL)),
        };

        let width = psd.width();
        let height = psd.height();
        let rgba = psd.rgba(); // Returns Vec<u8> (composite image)

        let img_buffer = match image::RgbaImage::from_raw(width, height, rgba) {
            Some(b) => b,
            None => return Err(windows::core::Error::from(windows::Win32::Foundation::E_FAIL)),
        };

        let dynamic_image = image::DynamicImage::ImageRgba8(img_buffer);
        // Resize to fit in cx * cx
        let resized = dynamic_image.resize(cx, cx, image::imageops::FilterType::Lanczos3);
        
        let (out_width, out_height) = resized.dimensions();
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        
        unsafe {
            let hbitmap = create_argb_bitmap(out_width, out_height, &mut bits);
            if hbitmap.is_invalid() {
                return Err(windows::core::Error::from(windows::Win32::Foundation::E_FAIL));
            }

            // Copy data to HBITMAP buffer (BGRA)
            let slice = std::slice::from_raw_parts_mut(bits as *mut u8, (out_width * out_height * 4) as usize);
            for (x, y, pixel) in resized.to_rgba8().enumerate_pixels() {
                // Windows DIB with negative height is top-down, same as image crate
                let offset = ((y * out_width + x) as usize) * 4;
                
                slice[offset] = pixel[2];     // B
                slice[offset + 1] = pixel[1]; // G
                slice[offset + 2] = pixel[0]; // R
                slice[offset + 3] = pixel[3]; // A
            }
            
            phbmp.write(hbitmap);
            alphatype.write(WTSAT_ARGB);
            Ok(())
        }
    }
}
