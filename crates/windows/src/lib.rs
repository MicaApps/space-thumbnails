#![allow(unused_must_use)]
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::Win32::Foundation::HINSTANCE;

pub mod providers;
pub mod registry;
pub mod constant;
pub mod utils;

static DLL_REF_COUNT: AtomicUsize = AtomicUsize::new(0);

// Global instance handle
static mut DLL_INSTANCE: HINSTANCE = HINSTANCE(0);

// Helper for logging
pub fn log_msg(msg: &str) {
    let temp_log = std::path::PathBuf::from(r"C:\Users\Public\space_thumbnails_debug.log");
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&temp_log) {
        let _ = writeln!(file, "[DLL] [PID:{}] {}", std::process::id(), msg);
    }
}

pub fn increment_dll_ref_count() {
    DLL_REF_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub fn decrement_dll_ref_count() {
    DLL_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
}

pub fn get_dll_ref_count() -> usize {
    DLL_REF_COUNT.load(Ordering::SeqCst)
}

pub unsafe fn set_dll_instance(instance: HINSTANCE) {
    DLL_INSTANCE = instance;
}

pub unsafe fn get_dll_instance() -> HINSTANCE {
    DLL_INSTANCE
}
