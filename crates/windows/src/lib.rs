#[macro_use]
extern crate lazy_static;

use windows::{
    core::{implement, IUnknown, Interface, Result, GUID},
    Win32::{
        Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_NOINTERFACE, S_OK},
        System::{
            Com::{IClassFactory, IClassFactory_Impl},
            LibraryLoader::GetModuleFileNameW,
        },
        UI::Shell::PropertiesSystem::{IInitializeWithFile, IInitializeWithStream},
    },
};

pub mod providers;
pub mod registry;
pub mod constant;
pub mod utils;
