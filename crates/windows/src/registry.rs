
use crate::utils::log_debug;
use windows::{
    core::GUID,
    Win32::System::Registry::*,
    Win32::Foundation::ERROR_SUCCESS,
    core::PCWSTR,
};

#[derive(Debug)]
pub enum RegistryData {
    Str(String),
    U32(u32),
}

#[derive(Debug)]
pub struct RegistryValue(pub String, pub RegistryData);

#[derive(Debug)]
pub struct RegistryKey {
    pub path: String,
    pub values: Vec<RegistryValue>,
}

pub fn register_clsid(
    clsid: &GUID,
    module_path: &str,
    disable_process_isolation: bool,
) -> Vec<RegistryKey> {
    vec![
        RegistryKey {
            path: format!("CLSID\\{{{:?}}}", clsid),
            values: vec![
                RegistryValue(
                    "".to_owned(),
                    RegistryData::Str("Model Thumbnail Handler".to_owned()),
                ),
                RegistryValue(
                    "DisableProcessIsolation".to_owned(),
                    RegistryData::U32(if disable_process_isolation { 1 } else { 0 }),
                ),
            ],
        },
        RegistryKey {
            path: format!("CLSID\\{{{:?}}}\\InProcServer32", clsid),
            values: vec![
                RegistryValue("".to_owned(), RegistryData::Str(module_path.to_owned())),
                RegistryValue(
                    "ThreadingModel".to_owned(),
                    RegistryData::Str("Both".to_owned()),
                ),
            ],
        },
    ]
}

pub fn register_server(keys: &[RegistryKey]) -> windows::core::Result<()> {
    log_debug("register_server called");
    for key in keys {
        log_debug(&format!("Registering key: {}", key.path));
        let mut hkey = HKEY::default();
        let h_key_path = windows::core::HSTRING::from(key.path.as_str());
        let result = unsafe {
            RegCreateKeyW(
                HKEY_CLASSES_ROOT,
                PCWSTR(h_key_path.as_wide().as_ptr()),
                &mut hkey,
            )
        };

        if result != ERROR_SUCCESS {
            return Err(windows::core::Error::from_win32());
        }

        for value in &key.values {
            let h_value_name = windows::core::HSTRING::from(value.0.as_str());
            let result = match &value.1 {
                RegistryData::Str(s) => {
                    let h_value = windows::core::HSTRING::from(s.as_str());
                    unsafe {
                        RegSetValueExW(
                            hkey,
                            PCWSTR(h_value_name.as_wide().as_ptr()),
                            0,
                            REG_SZ,
                            h_value.as_wide().as_ptr() as _,
                            (h_value.len() * 2 + 2) as u32,
                        )
                    }
                }
                RegistryData::U32(n) => unsafe {
                    RegSetValueExW(
                        hkey,
                        PCWSTR(h_value_name.as_wide().as_ptr()),
                        0,
                        REG_DWORD,
                        &n.to_le_bytes() as *const _ as _,
                        4,
                    )
                },
            };
            if result != ERROR_SUCCESS {
                return Err(windows::core::Error::from_win32());
            }
        }
        unsafe {
            RegCloseKey(hkey);
        }
    }
    Ok(())
}

pub fn unregister_server(keys: &[RegistryKey]) -> windows::core::Result<()> {
    for key in keys {
        let h_key_path = windows::core::HSTRING::from(key.path.as_str());
        let result = unsafe {
            RegDeleteTreeW(
                HKEY_CLASSES_ROOT,
                PCWSTR(h_key_path.as_wide().as_ptr()),
            )
        };
        if result != ERROR_SUCCESS {
            return Err(windows::core::Error::from_win32());
        }
    }
    Ok(())
}
