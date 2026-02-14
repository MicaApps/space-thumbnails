
use windows::core::GUID;
use crate::registry::RegistryKey;

pub mod pdf_thumbnail;
pub mod thumbnail;
pub mod thumbnail_file;

pub trait Provider {
    fn clsid(&self) -> GUID;
    fn register(&self, module_path: &str) -> Vec<RegistryKey>;
    fn create_instance(
        &self,
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()>;
}
