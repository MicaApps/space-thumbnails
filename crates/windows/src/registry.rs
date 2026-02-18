use windows::core::{GUID, PCWSTR};
use windows::Win32::System::Registry::{
    HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, KEY_WRITE,
    REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
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

fn log_msg(msg: &str) {
    let temp_log = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
        let _ = writeln!(file, "[REGISTRY] [PID:{}] {}", std::process::id(), msg);
    }
}

pub fn write_registry_keys(keys: &Vec<RegistryKey>) -> windows::core::Result<()> {
    log_msg(&format!("write_registry_keys called with {} keys", keys.len()));
    for key in keys {
        log_msg(&format!("Processing key: {}", key.path));
        
        // Ensure null-terminated wide string for key path
        let mut path_wide: Vec<u16> = key.path.encode_utf16().collect();
        path_wide.push(0);
        
        let mut hkey = HKEY::default();
        
        unsafe {
            let res = RegCreateKeyExW(
                HKEY_CLASSES_ROOT,
                PCWSTR(path_wide.as_ptr()),
                0,
                PCWSTR(std::ptr::null()),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                std::ptr::null(),
                &mut hkey,
                std::ptr::null_mut(),
            );
            
            if res.0 != 0 {
                 log_msg(&format!("RegCreateKeyExW(HKCR) failed for {}: {:?}. Trying HKCU...", key.path, res));
                 
                 // Fallback to HKCU\Software\Classes
                 let hkcu_path = format!("Software\\Classes\\{}", key.path);
                 let mut hkcu_path_wide: Vec<u16> = hkcu_path.encode_utf16().collect();
                 hkcu_path_wide.push(0);
                 
                 let res_hkcu = RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(hkcu_path_wide.as_ptr()),
                    0,
                    PCWSTR(std::ptr::null()),
                    REG_OPTION_NON_VOLATILE,
                    KEY_WRITE,
                    std::ptr::null(),
                    &mut hkey,
                    std::ptr::null_mut(),
                );
                
                if res_hkcu.0 != 0 {
                    log_msg(&format!("RegCreateKeyExW(HKCU) failed for {}: {:?}", hkcu_path, res_hkcu));
                    return Err(windows::core::Error::from(res_hkcu));
                }
            }
            
            for value in &key.values {
                // Ensure null-terminated wide string for value name
                let mut value_name_wide: Vec<u16> = value.0.encode_utf16().collect();
                value_name_wide.push(0);
                
                match &value.1 {
                    RegistryData::Str(s) => {
                        let mut wide_data: Vec<u16> = s.encode_utf16().collect();
                        wide_data.push(0); // Null terminator
                        
                        let ptr = wide_data.as_ptr() as *const u8;
                        let size = (wide_data.len() * 2) as u32;
                        
                        RegSetValueExW(
                            hkey,
                            PCWSTR(value_name_wide.as_ptr()),
                            0,
                            REG_SZ,
                            ptr,
                            size,
                        );
                    },
                    RegistryData::U32(u) => {
                        let bytes = u.to_ne_bytes();
                         RegSetValueExW(
                            hkey,
                            PCWSTR(value_name_wide.as_ptr()),
                            0,
                            REG_DWORD,
                            bytes.as_ptr(),
                            4,
                        );
                    }
                }
            }
            RegCloseKey(hkey);
        }
    }
    Ok(())
}
