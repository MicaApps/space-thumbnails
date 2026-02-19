#![allow(unused_must_use)]
#[macro_use]
extern crate lazy_static;

use windows::core::{implement, IUnknown, Interface, Result, GUID, HRESULT};
use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, S_OK, BOOL, HINSTANCE, CLASS_E_NOAGGREGATION, S_FALSE};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
#[allow(unused_imports)]
use windows::Win32::System::Registry::{
    HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY,
};
#[allow(unused_imports)]
use windows::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
#[allow(unused_imports)]
use windows::Win32::UI::Shell::PropertiesSystem::{IInitializeWithFile, IInitializeWithStream};

pub mod providers;
pub mod registry;
pub mod constant;
pub mod utils;

use providers::{ThumbnailFileProvider, ThumbnailProvider, PsdThumbnailProvider, PdfThumbnailProvider, EpubThumbnailProvider, Provider};
// use space_thumbnails::RendererBackend;
use std::sync::atomic::{AtomicUsize, Ordering};

static DLL_REF_COUNT: AtomicUsize = AtomicUsize::new(0);

// Global instance handle
static mut DLL_INSTANCE: HINSTANCE = HINSTANCE(0);

// Helper for logging
pub(crate) fn log_msg(msg: &str) {
    let temp_log = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
        let _ = writeln!(file, "[DLL] [PID:{}] {}", std::process::id(), msg);
    }
}

#[allow(unused_must_use)]
#[implement(windows::Win32::System::Com::IClassFactory)]
struct ClassFactory {
    clsid: GUID,
}

impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(
        &self,
        punkouter: &Option<IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        unsafe {
            let riid_ref = &*riid;
            log_msg(&format!("ClassFactory::CreateInstance called for IID: {:?}", riid_ref));
        }
        if punkouter.is_some() {
            log_msg("ClassFactory::CreateInstance - Aggregation not supported");
            return Err(windows::core::Error::from(CLASS_E_NOAGGREGATION));
        }

        // Check which CLSID is requested
        // .step: {662657D4-0325-4632-9154-116584281360}
        let step_clsid = GUID::from_values(0x662657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x60]);
        // .stp: {552657D4-0325-4632-9154-116584281359}
        let stp_clsid = GUID::from_values(0x552657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x59]);
        // .obj: {650a0a50-3a8c-49ca-ba26-13b31965b8ef}
        let obj_clsid = GUID::from_values(0x650a0a50, 0x3a8c, 0x49ca, [0xba, 0x26, 0x13, 0xb3, 0x19, 0x65, 0xb8, 0xef]);
        // .fbx: {bf2644df-ae9c-4524-8bfd-2d531b837e97}
        let fbx_clsid = GUID::from_values(0xbf2644df, 0xae9c, 0x4524, [0x8b, 0xfd, 0x2d, 0x53, 0x1b, 0x83, 0x7e, 0x97]);
        // .psd: {446593aa-9e7a-4da2-b785-3e2e3b7bd652}
        let psd_clsid = GUID::from_values(0x446593aa, 0x9e7a, 0x4da2, [0xb7, 0x85, 0x3e, 0x2e, 0x3b, 0x7b, 0xd6, 0x52]);
        // .pdf: {102657d4-0325-4632-9154-116584281399}
        let pdf_clsid = GUID::from_values(0x102657d4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x99]);
        // .epub: {772657D4-0325-4632-9154-116584281388}
        let epub_clsid = GUID::from_values(0x772657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x88]);
        // .ai: {556593aa-9e7a-4da2-b785-3e2e3b7bd653}
        // let ai_clsid = GUID::from_values(0x556593aa, 0x9e7a, 0x4da2, [0xb7, 0x85, 0x3e, 0x2e, 0x3b, 0x7b, 0xd6, 0x53]);

        if self.clsid == step_clsid {
             let provider = ThumbnailFileProvider::new(
                self.clsid,
                ".step",
            );
            provider.create_instance(riid, ppvobject)
        } else if self.clsid == stp_clsid {
             let provider = ThumbnailFileProvider::new(
                self.clsid,
                ".stp",
            );
            provider.create_instance(riid, ppvobject)
        } else if self.clsid == obj_clsid {
             let provider = ThumbnailProvider::new(
                self.clsid,
                ".obj",
            );
            provider.create_instance(riid, ppvobject)
        } else if self.clsid == fbx_clsid {
             let provider = ThumbnailProvider::new(
                self.clsid,
                ".fbx",
            );
            provider.create_instance(riid, ppvobject)
        } else if self.clsid == psd_clsid {
             let provider = PsdThumbnailProvider::new(
                self.clsid,
            );
            provider.create_instance(riid, ppvobject)
        } else if self.clsid == pdf_clsid {
             let provider = PdfThumbnailProvider::new(
                self.clsid,
            );
            provider.create_instance(riid, ppvobject)
        } else if self.clsid == epub_clsid {
             let provider = EpubThumbnailProvider::new(
                self.clsid,
            );
            provider.create_instance(riid, ppvobject)
        // } else if self.clsid == ai_clsid {
        //      let provider = AiThumbnailProvider::new(
        //         self.clsid,
        //     );
        //     provider.create_instance(riid, ppvobject)
        } else {
            Err(windows::core::Error::from(CLASS_E_CLASSNOTAVAILABLE))
        }
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            DLL_REF_COUNT.fetch_add(1, Ordering::SeqCst);
        } else {
            DLL_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[no_mangle]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> HRESULT {
    // Log
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(r"C:\Users\Public\space_thumbnails_debug.log") {
         let _ = writeln!(file, "[DllGetClassObject] [PID:{}] Called for CLSID: {:?} IID: {:?}", std::process::id(), unsafe { *rclsid }, unsafe { *riid });
    }

    log_msg("DllGetClassObject called");
    unsafe {
        let rclsid = *rclsid;
        
        let step_clsid = GUID::from_values(0x662657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x60]);
        let stp_clsid = GUID::from_values(0x552657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x59]);
        let obj_clsid = GUID::from_values(0x650a0a50, 0x3a8c, 0x49ca, [0xba, 0x26, 0x13, 0xb3, 0x19, 0x65, 0xb8, 0xef]);
        let fbx_clsid = GUID::from_values(0xbf2644df, 0xae9c, 0x4524, [0x8b, 0xfd, 0x2d, 0x53, 0x1b, 0x83, 0x7e, 0x97]);
        let psd_clsid = GUID::from_values(0x446593aa, 0x9e7a, 0x4da2, [0xb7, 0x85, 0x3e, 0x2e, 0x3b, 0x7b, 0xd6, 0x52]);
        let pdf_clsid = GUID::from_values(0x102657d4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x99]);
        let epub_clsid = GUID::from_values(0x772657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x88]);
        // let ai_clsid = GUID::from_values(0x556593aa, 0x9e7a, 0x4da2, [0xb7, 0x85, 0x3e, 0x2e, 0x3b, 0x7b, 0xd6, 0x53]);

        if rclsid != step_clsid && rclsid != stp_clsid && rclsid != obj_clsid && rclsid != fbx_clsid && rclsid != psd_clsid && rclsid != pdf_clsid && rclsid != epub_clsid {
            log_msg(&format!("DllGetClassObject - Unknown CLSID: {:?}", rclsid));
            return CLASS_E_CLASSNOTAVAILABLE.into();
        }

        let factory = ClassFactory { clsid: rclsid };
        let unknown: IClassFactory = factory.into();
        unknown.query(&*riid, ppv).into()
    }
}

#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REF_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK.into()
    } else {
        S_FALSE.into()
    }
}

#[no_mangle]
extern "system" fn DllMain(
    hinst: HINSTANCE,
    fdwreason: u32,
    _lpvreserved: *const core::ffi::c_void,
) -> BOOL {
    if fdwreason == DLL_PROCESS_ATTACH {
        unsafe { DLL_INSTANCE = hinst; }
        
        log_msg("DllMain attached - BUILD_V29_DEBUG_FALLBACK");

                // Set base path
        let mut buffer = [0u16; 1024];
        unsafe {
            let len = GetModuleFileNameW(hinst, &mut buffer);
            if len > 0 {
                let path = String::from_utf16_lossy(&buffer[..len as usize]);
                let path = std::path::PathBuf::from(path);
                if let Some(parent) = path.parent() {
                     space_thumbnails::set_base_path(parent.to_path_buf());
                     log_msg(&format!("Setting base path to: {:?}", parent));
                }
            }
        }
    }
    BOOL::from(true)
}

#[no_mangle]
extern "system" fn DllRegisterServer() -> HRESULT {
    log_msg("DllRegisterServer called");
    let step_clsid = GUID::from_values(0x662657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x60]);
    let stp_clsid = GUID::from_values(0x552657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x59]);
    let obj_clsid = GUID::from_values(0x650a0a50, 0x3a8c, 0x49ca, [0xba, 0x26, 0x13, 0xb3, 0x19, 0x65, 0xb8, 0xef]);
    let fbx_clsid = GUID::from_values(0xbf2644df, 0xae9c, 0x4524, [0x8b, 0xfd, 0x2d, 0x53, 0x1b, 0x83, 0x7e, 0x97]);
    let psd_clsid = GUID::from_values(0x446593aa, 0x9e7a, 0x4da2, [0xb7, 0x85, 0x3e, 0x2e, 0x3b, 0x7b, 0xd6, 0x52]);
    // let ai_clsid = GUID::from_values(0x556593aa, 0x9e7a, 0x4da2, [0xb7, 0x85, 0x3e, 0x2e, 0x3b, 0x7b, 0xd6, 0x53]);
    let pdf_clsid = GUID::from_values(0x102657d4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x99]);
    let epub_clsid = GUID::from_values(0x772657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x88]);
    
    // Get module path
    let mut buffer = [0u16; 1024];
    let path = unsafe {
        let hinst = DLL_INSTANCE;
        let len = GetModuleFileNameW(hinst, &mut buffer);
        String::from_utf16_lossy(&buffer[..len as usize])
    };

    let p1 = ThumbnailFileProvider::new(step_clsid, ".step");
    let keys = p1.register(&path);
    let _ = registry::write_registry_keys(&keys);

    let p2 = ThumbnailFileProvider::new(stp_clsid, ".stp");
    let keys = p2.register(&path);
    let _ = registry::write_registry_keys(&keys);

    let p3 = ThumbnailProvider::new(obj_clsid, ".obj");
    let keys = p3.register(&path);
    let _ = registry::write_registry_keys(&keys);

    let p4 = ThumbnailProvider::new(fbx_clsid, ".fbx");
    let keys = p4.register(&path);
    let _ = registry::write_registry_keys(&keys);

    let p5 = PsdThumbnailProvider::new(psd_clsid);
    let keys = p5.register(&path);
    let _ = registry::write_registry_keys(&keys);

    // let p6 = AiThumbnailProvider::new(ai_clsid);
    // let keys = p6.register(&path);
    // let _ = registry::write_registry_keys(&keys);

    let p7 = PdfThumbnailProvider::new(pdf_clsid);
    let keys = p7.register(&path);
    let _ = registry::write_registry_keys(&keys);

    let p8 = EpubThumbnailProvider::new(epub_clsid);
    let keys = p8.register(&path);
    let _ = registry::write_registry_keys(&keys);

    S_OK.into()
}

#[no_mangle]
extern "system" fn DllUnregisterServer() -> HRESULT {
    S_OK.into()
}
