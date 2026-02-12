use std::fs;
use std::path::Path;
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
    pub fn FPDFBitmap_FillRect(bitmap: usize, left: i32, top: i32, width: i32, height: i32, color: u32);
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

const FPDF_ANNOT: i32 = 0x01;

fn main() {
    let pdf_path = Path::new("D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\test_files\\testpdf2.pdf");
    let output_path = Path::new("D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\test_files\\testpdf2.png");

    let pdf_data = match fs::read(pdf_path) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to read PDF file: {}", e);
            return;
        }
    };

    unsafe {
        FPDF_InitLibrary();

        let doc = FPDF_LoadMemDocument(
            pdf_data.as_ptr() as *const c_void,
            pdf_data.len() as i32,
            std::ptr::null(),
        );

        if doc == 0 {
            println!("Failed to load PDF document");
            FPDF_DestroyLibrary();
            return;
        }

        let page_count = FPDF_GetPageCount(doc);
        if page_count == 0 {
            println!("PDF has no pages");
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return;
        }

        let page = FPDF_LoadPage(doc, 0);
        if page == 0 {
            println!("Failed to load first page");
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return;
        }

        // 获取页面尺寸
        let page_width = FPDF_GetPageWidth(page);
        let page_height = FPDF_GetPageHeight(page);
        println!("PDF page dimensions: {} x {} points", page_width, page_height);

        // 计算缩略图尺寸，长边始终等于216px
        let target_long_side = 216.0;
        
        // 确定长边并计算缩放比例
        let scale = if page_width > page_height {
            // 宽度是长边，设置为216px
            target_long_side / page_width
        } else {
            // 高度是长边，设置为216px  
            target_long_side / page_height
        };
        
        let thumb_width = (page_width * scale) as i32;
        let thumb_height = (page_height * scale) as i32;
        
        println!("Thumbnail dimensions: {} x {} pixels", thumb_width, thumb_height);

        // 创建256x256的透明背景画布
        let canvas_size = 256;
        let bitmap = FPDFBitmap_Create(canvas_size, canvas_size, 1); // 1 = has alpha
        if bitmap == 0 {
            println!("Failed to create bitmap");
            FPDF_ClosePage(page);
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return;
        }

        // 计算居中位置
        let start_x = ((canvas_size - thumb_width) / 2) as i32;
        let start_y = ((canvas_size - thumb_height) / 2) as i32;
        
        println!("Rendering at position: ({}, {})", start_x, start_y);

        // 先在页面区域填充白色背景
        // 颜色格式是 ARGB，所以 0xFFFFFFFF 是白色
        FPDFBitmap_FillRect(bitmap, start_x, start_y, thumb_width, thumb_height, 0xFFFFFFFF);

        // 然后将PDF页面渲染到这个白色背景之上
        let render_flags = FPDF_ANNOT;
        FPDF_RenderPageBitmap(bitmap, page, start_x, start_y, thumb_width, thumb_height, 0, render_flags);

        // 获取最终的位图数据
        let buffer = FPDFBitmap_GetBuffer(bitmap) as *mut u8;
        let buffer_size = (canvas_size * canvas_size * 4) as usize;
        let bitmap_data = std::slice::from_raw_parts(buffer, buffer_size).to_vec();

        // 保存为 RGBA 数据
        if let Err(e) = fs::write(output_path.with_extension("rgba"), &bitmap_data) {
            println!("Failed to write bitmap data: {}", e);
        } else {
            println!("Successfully rendered PDF to RGBA bitmap: {:?}", output_path.with_extension("rgba"));
            println!("Bitmap size: {} bytes", bitmap_data.len());
        }

        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
    }
}