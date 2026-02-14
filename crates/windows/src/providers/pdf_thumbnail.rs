use crate::providers::Provider;
use crate::registry::{register_clsid, RegistryKey, RegistryValue};
use crate::utils::log_debug;
use image;
use windows::core::{GUID, Interface};
use windows::Win32::Foundation::S_OK;

pub struct PdfThumbnailProvider {
    clsid: GUID,
}

impl PdfThumbnailProvider {
    pub fn new() -> Self {
        Self {
            clsid: GUID::from("E3A82405-A21A-4423-B644-93B072E2E7C9"),
        }
    }
}

impl Provider for PdfThumbnailProvider {
    fn clsid(&self) -> GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<RegistryKey> {
        let mut keys = register_clsid(&self.clsid, module_path, false);
        keys.push(RegistryKey {
            path: format!("\\.pdf\\ShellEx\\{{e357fccd-a995-4576-b01f-234630154e96}}"),
            values: vec![RegistryValue(
                "".to_owned(),
                crate::registry::RegistryData::Str(format!("{{{:?}}}", self.clsid)),
            )],
        });
        keys
    }

    fn create_instance(
        &self,
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let handler = crate::providers::thumbnail::ThumbnailHandler::new(".pdf");
        let unknown: windows::core::IUnknown = handler.into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

use std::ffi::{c_void, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

// PDFium FFI declarations
#[link(name = "pdfium")]
extern "C" {
    fn FPDF_InitLibrary();
    fn FPDF_DestroyLibrary();
    fn FPDF_LoadMemDocument(data_buf: *const c_void, size: c_int, password: *const c_char) -> usize;
    fn FPDF_CloseDocument(document: usize);
    fn FPDF_GetPageCount(document: usize) -> c_int;
    fn FPDF_LoadPage(document: usize, page_index: c_int) -> usize;
    fn FPDF_ClosePage(page: usize);
    fn FPDF_GetPageWidthF(page: usize) -> f32;
    fn FPDF_GetPageHeightF(page: usize) -> f32;
    fn FPDF_RenderPageBitmap(
        bitmap: usize,
        page: usize,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    );
    fn FPDFBitmap_Create(width: c_int, height: c_int, alpha: c_int) -> usize;
    fn FPDFBitmap_Destroy(bitmap: usize);
    fn FPDFBitmap_GetBuffer(bitmap: usize) -> *mut c_void;
    fn FPDFBitmap_FillRect(
        bitmap: usize,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        color: u32,
    );
}

const FPDF_LCD_TEXT: c_int = 0x02;
const FPDF_ANNOT: c_int = 0x01;

pub fn render_pdf_to_bitmap(
    pdf_data: &[u8],
    width: i32,
    height: i32,
) -> Option<Vec<u8>> {
    log_debug(&format!("[render_pdf_to_bitmap] Starting PDF rendering, data size: {} bytes, target size: {}x{}", 
        pdf_data.len(), width, height));
    
    unsafe {
        // Initialize PDFium library
        FPDF_InitLibrary();
        log_debug("[render_pdf_to_bitmap] PDFium library initialized");
        
        // Load PDF from memory
        let document = FPDF_LoadMemDocument(
            pdf_data.as_ptr() as *const c_void,
            pdf_data.len() as c_int,
            ptr::null()
        );
        
        if document == 0 {
            log_debug("[render_pdf_to_bitmap] Failed to load PDF document");
            FPDF_DestroyLibrary();
            return None;
        }
        
        log_debug(&format!("[render_pdf_to_bitmap] PDF document loaded successfully, document handle: {}", document));
        
        // Get page count and load first page
        let page_count = FPDF_GetPageCount(document);
        log_debug(&format!("[render_pdf_to_bitmap] PDF has {} pages", page_count));
        
        if page_count == 0 {
            log_debug("[render_pdf_to_bitmap] PDF has no pages");
            FPDF_CloseDocument(document);
            FPDF_DestroyLibrary();
            return None;
        }
        
        let page = FPDF_LoadPage(document, 0);
        if page == 0 {
            log_debug("[render_pdf_to_bitmap] Failed to load first page");
            FPDF_CloseDocument(document);
            FPDF_DestroyLibrary();
            return None;
        }
        
        // Get page dimensions
        let page_width = FPDF_GetPageWidthF(page);
        let page_height = FPDF_GetPageHeightF(page);
        log_debug(&format!("[render_pdf_to_bitmap] Page dimensions: {:.1}x{:.1}", page_width, page_height));
        
        // Create bitmap for rendering
        let bitmap = FPDFBitmap_Create(width, height, 1); // 1 = with alpha channel
        if bitmap == 0 {
            log_debug("[render_pdf_to_bitmap] Failed to create bitmap");
            FPDF_ClosePage(page);
            FPDF_CloseDocument(document);
            FPDF_DestroyLibrary();
            return None;
        }
        
        // Fill bitmap with white background
        FPDFBitmap_FillRect(bitmap, 0, 0, width, height, 0xFFFFFFFF); // White background
        
        // Render PDF page to bitmap
        let flags = FPDF_LCD_TEXT | FPDF_ANNOT;
        FPDF_RenderPageBitmap(bitmap, page, 0, 0, width, height, 0, flags);
        log_debug("[render_pdf_to_bitmap] PDF page rendered to bitmap");
        
        // Get bitmap buffer
        let buffer_ptr = FPDFBitmap_GetBuffer(bitmap);
        if buffer_ptr.is_null() {
            log_debug("[render_pdf_to_bitmap] Failed to get bitmap buffer");
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);
            FPDF_CloseDocument(document);
            FPDF_DestroyLibrary();
            return None;
        }
        
        // Copy bitmap data to Vec<u8>
        let buffer_size = (width * height * 4) as usize; // 4 bytes per pixel (RGBA)
        let mut result = Vec::with_capacity(buffer_size);
        result.set_len(buffer_size);
        
        std::ptr::copy_nonoverlapping(
            buffer_ptr as *const u8,
            result.as_mut_ptr(),
            buffer_size
        );
        
        log_debug(&format!("[render_pdf_to_bitmap] Successfully rendered PDF, buffer size: {} bytes", buffer_size));
        
        // Cleanup
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        FPDF_CloseDocument(document);
        FPDF_DestroyLibrary();
        
        Some(result)
    }
}
