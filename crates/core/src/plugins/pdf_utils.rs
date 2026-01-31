use std::path::{Path, PathBuf};
use image::DynamicImage;
use pdfium_render::prelude::*;

fn get_current_dll_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use windows::Win32::System::LibraryLoader::{GetModuleHandleExW, GetModuleFileNameW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT};
        use windows::Win32::Foundation::HINSTANCE;
        
        let mut module = HINSTANCE::default();
        unsafe {
            let lpaddress = get_current_dll_dir as *const std::ffi::c_void;
            if GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                windows::core::PCWSTR(lpaddress as *const u16),
                &mut module
            ).as_bool() {
                 let mut path = [0u16; 1024];
                 let len = GetModuleFileNameW(module, &mut path);
                 if len > 0 {
                     let path_str = String::from_utf16_lossy(&path[..len as usize]);
                     return PathBuf::from(path_str).parent().map(|p| p.to_path_buf());
                 }
            }
        }
    }
    None
}

/// Renders the first page of a PDF-compatible document (PDF, AI) to a DynamicImage.
/// 
/// - Loads PDFium from local or system.
/// - Renders at 4x resolution for high quality.
/// - Does NOT apply any specific styling (border, fold, gamma).
pub fn render_pdf_page_to_image(buffer: Option<&[u8]>, filepath: Option<&Path>, width: u32, height: u32) -> Result<DynamicImage, String> {
    // 1. Initialize PDFium
    let mut bindings_result = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"));
    
    if bindings_result.is_err() {
        if let Some(dll_dir) = get_current_dll_dir() {
             if let Some(dir_str) = dll_dir.to_str() {
                 bindings_result = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir_str));
             }
        }
    }

    let bindings = bindings_result
        .or_else(|_| Pdfium::bind_to_system_library())
        .map_err(|e| format!("Failed to bind to PDFium: {}", e))?;
        
    let pdfium = Pdfium::new(bindings);

    // 2. Load Document
    let document = if let Some(buf) = buffer {
        pdfium.load_pdf_from_byte_slice(buf, None)
    } else {
         if let Some(path) = filepath {
             pdfium.load_pdf_from_file(path, None)
         } else {
             return Err("No buffer or filepath provided for document".to_string());
         }
    }.map_err(|e| format!("Failed to load document: {}", e))?;

    // 3. Render first page
    let page = document.pages().get(0).map_err(|e| format!("Failed to get first page: {}", e))?;
    
    // Render at 4x resolution for better quality
    let render_config = PdfRenderConfig::new()
        .set_target_width((width * 4) as i32)
        .set_maximum_height((height * 4) as i32)
        .rotate_if_landscape(PdfPageRenderRotation::None, true);

    let bitmap = page.render_with_config(&render_config)
        .map_err(|e| format!("Failed to render page: {}", e))?;
    
    Ok(bitmap.as_image())
}
