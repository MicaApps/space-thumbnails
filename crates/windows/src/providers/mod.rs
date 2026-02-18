pub mod thumbnail;
pub mod thumbnail_file;
pub mod psd_provider;
pub mod pdf_provider;
// pub mod ai_provider;

pub use thumbnail::*;
pub use thumbnail_file::*;
pub use psd_provider::*;
pub use pdf_provider::*;
// pub use ai_provider::*; use ai_provider::*;

use crate::registry::RegistryKey;

pub trait Provider {
    fn clsid(&self) -> windows::core::GUID;
    fn register(&self, module_path: &str) -> Vec<RegistryKey>;
    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()>;
}
