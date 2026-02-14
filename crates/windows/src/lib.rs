
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

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _module: HANDLE,
    call_reason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    match call_reason {
        DLL_PROCESS_ATTACH => (),
        DLL_PROCESS_DETACH => (),
        _ => (),
    }
    true.into()
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    log_debug("DllRegisterServer called");
    let module_path = get_module_path();
    log_debug(&format!("Module path: {}", module_path));
    let keys = PROVIDERS.iter().flat_map(|p| p.register(&module_path)).collect::<Vec<_>>();
    match register_server(&keys) {
        Ok(_) => S_OK,
        Err(e) => e.into(),
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    let module_path = get_module_path();
    let keys = PROVIDERS.iter().flat_map(|p| p.register(&module_path)).collect::<Vec<_>>();
    match unregister_server(&keys) {
        Ok(_) => S_OK,
        Err(e) => e.into(),
    }
}
