#[macro_use]
extern crate lazy_static;

use windows::{
    core::{implement, IUnknown, Interface, Result, GUID},
    Win32::{
        Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_NOINTERFACE, S_OK},
        System::{
            Com::{IClassFactory, IClassFactory_Impl},
            LibraryLoader::GetModuleFileNameW,
            Registry::{
                RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ
            },
        },
        UI::Shell::PropertiesSystem::{IInitializeWithFile, IInitializeWithStream},
    },
};

pub mod providers;
pub mod registry;
pub mod constant;
pub mod utils;
