
#[macro_use]
extern crate lazy_static;

use crate::utils::log_debug;
use providers::{
    pdf_thumbnail::PdfThumbnailProvider,
    Provider,
};
use registry::{register_server, unregister_server};
use windows::{
    core::{GUID, HRESULT},
    Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    Win32::Foundation::{BOOL, HANDLE, S_OK},
    Win32::System::LibraryLoader::GetModuleFileNameW,
};
use std::ffi::c_void;

pub mod providers;
pub mod registry;
pub mod constant;
pub mod utils;

lazy_static! {
    static ref PROVIDERS: Vec<Box<dyn Provider + Send + Sync>> = vec![
        Box::new(PdfThumbnailProvider::new()),
    ];
}

fn get_module_path() -> String {
    let mut path = [0u16; 260];
    let len = unsafe { GetModuleFileNameW(None, &mut path) };
    String::from_utf16_lossy(&path[..len as usize])
}

