use std::ffi::{c_void};
use std::os::raw::c_char;

#[link(name = "pdfium")]
extern "C" {
    pub fn FPDF_InitLibrary();
    pub fn FPDF_DestroyLibrary();
    pub fn FPDF_LoadMemDocument(
        data_buf: *const c_void,
        size: i32,
        password: *const c_char,
    ) -> usize;
    pub fn FPDF_CloseDocument(document: usize);
    pub fn FPDF_GetPageCount(document: usize) -> i32;
    pub fn FPDF_LoadPage(document: usize, page_index: i32) -> usize;
    pub fn FPDF_ClosePage(page: usize);
    pub fn FPDF_GetPageWidth(page: usize) -> f64;
    pub fn FPDF_GetPageHeight(page: usize) -> f64;
    pub fn FPDFBitmap_Create(width: i32, height: i32, alpha: i32) -> usize;
    pub fn FPDFBitmap_Destroy(bitmap: usize);
    pub fn FPDFBitmap_GetBuffer(bitmap: usize) -> *mut c_void;
    pub fn FPDF_RenderPageBitmap(
        bitmap: usize,
        page: usize,
        start_x: i32,
        start_y: i32,
        size_x: i32,
        size_y: i32,
        rotate: i32,
        flags: i32,
    );
}

pub fn render_pdf_to_bitmap(
    pdf_data: &[u8],
    width: i32,
    height: i32,
) -> Option<Vec<u8>> {
    unsafe {
        FPDF_InitLibrary();

        let doc = FPDF_LoadMemDocument(
            pdf_data.as_ptr() as *const c_void,
            pdf_data.len() as i32,
            std::ptr::null(),
        );

        if doc == 0 {
            FPDF_DestroyLibrary();
            return None;
        }

        let page_count = FPDF_GetPageCount(doc);
        if page_count == 0 {
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return None;
        }

        let page = FPDF_LoadPage(doc, 0);
        if page == 0 {
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return None;
        }

        let bitmap = FPDFBitmap_Create(width, height, 1);
        if bitmap == 0 {
            FPDF_ClosePage(page);
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return None;
        }

        FPDF_RenderPageBitmap(bitmap, page, 0, 0, width, height, 0, 0);

        let buffer = FPDFBitmap_GetBuffer(bitmap) as *mut u8;
        let buffer_size = (width * height * 4) as usize;
        let screenshot_buffer = std::slice::from_raw_parts(buffer, buffer_size).to_vec();

        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        Some(screenshot_buffer)
    }
}
