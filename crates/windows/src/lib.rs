
#[macro_use]
extern crate lazy_static;

use std::{
    ffi::c_void,
    sync::atomic::{AtomicU32, Ordering},
};

use providers::{pdf_thumbnail::PdfThumbnailProvider, Provider};
use registry::{register_server, unregister_server};
use windows::{
    core::{implement, GUID, HRESULT, IUnknown, Interface},
    Win32::{
        Foundation::{BOOL, E_POINTER, S_FALSE, S_OK},
        System::{
            Com::{IClassFactory, IClassFactory_Impl},
            LibraryLoader::GetModuleFileNameW,
        },
    },
};

pub mod constant;
pub mod providers;
pub mod registry;
pub mod utils;

static SERVER_LOCKS: AtomicU32 = AtomicU32::new(0);

const CLASS_E_NOAGGREGATION: HRESULT = HRESULT(0x80040110u32 as i32);
const CLASS_E_CLASSNOTAVAILABLE: HRESULT = HRESULT(0x80040111u32 as i32);

lazy_static! {
    static ref PROVIDERS: Vec<Box<dyn Provider + Send + Sync>> = vec![Box::new(
        PdfThumbnailProvider::new()
    ),];
}

fn get_module_path() -> String {
    let mut path = [0u16; 260];
    let len = unsafe { GetModuleFileNameW(None, &mut path) };
    String::from_utf16_lossy(&path[..len as usize])
}

#[implement(IClassFactory)]
struct ClassFactory {
    clsid: GUID,
}

impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(
        &self,
        punkouter: &Option<IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let provider = PROVIDERS
            .iter()
            .find(|p| p.clsid() == self.clsid)
            .ok_or_else(|| windows::core::Error::from(CLASS_E_CLASSNOTAVAILABLE))?;
        provider.create_instance(riid, ppvobject)
    }

    fn LockServer(&self, flock: BOOL) -> windows::core::Result<()> {
        if flock.as_bool() {
            SERVER_LOCKS.fetch_add(1, Ordering::SeqCst);
        } else {
            SERVER_LOCKS.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }

    unsafe {
        *ppv = std::ptr::null_mut();
    }

    let clsid = unsafe { *rclsid };
    let riid = unsafe { *riid };

    if !PROVIDERS.iter().any(|p| p.clsid() == clsid) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    let factory: IUnknown = ClassFactory { clsid }.into();
    unsafe { factory.query(&riid, ppv) }
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if SERVER_LOCKS.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    let module_path = get_module_path();
    let keys: Vec<_> = PROVIDERS
        .iter()
        .flat_map(|p| p.register(&module_path))
        .collect();

    match register_server(&keys) {
        Ok(_) => S_OK,
        Err(e) => e.code(),
    }
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    let module_path = get_module_path();
    let keys: Vec<_> = PROVIDERS
        .iter()
        .flat_map(|p| p.register(&module_path))
        .collect();

    match unregister_server(&keys) {
        Ok(_) => S_OK,
        Err(e) => e.code(),
    }
}
