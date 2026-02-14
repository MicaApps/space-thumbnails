use image::{GenericImage, ImageBuffer, Rgba};
use std::ffi::{c_void, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

// FFI declarations for PDFium
#[link(name = "pdfium")]
extern "C" {
    fn FPDF_InitLibrary();
    fn FPDF_DestroyLibrary();
    fn FPDF_LoadDocument(path: *const c_char, password: *const c_char) -> usize;
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
const FPDF_REVERSE_BYTE_ORDER: c_int = 0x10;

type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;

// This struct holds the rendered image and the coordinates of the PDF content within it.
struct RenderInfo {
    image: RgbaImage,
    content_x: i32,
    content_y: i32,
    content_width: i32,
    content_height: i32,
}

fn render_pdf_page(doc: usize, page_index: i32) -> Option<RenderInfo> {
    unsafe {
        let page = FPDF_LoadPage(doc, page_index as c_int);
        if page == 0 {
            println!("Failed to load page {}", page_index);
            return None;
        }

        let page_width = FPDF_GetPageWidthF(page);
        let page_height = FPDF_GetPageHeightF(page);

        // Fallback to A4 size if dimensions are invalid
        let (page_width, page_height) = if page_width <= 0.0 || page_height <= 0.0 || page_width > 10000.0 || page_height > 10000.0 {
            println!("Warning: Invalid page dimensions ({}, {}). Using A4 fallback.", page_width, page_height);
            (595.0, 842.0)
        } else {
            (page_width, page_height)
        };

        let target_long_side = 216.0;
        let scale = if page_width > page_height {
            target_long_side / page_width
        } else {
            target_long_side / page_height
        };

        let thumb_width = (page_width * scale) as i32;
        let thumb_height = (page_height * scale) as i32;

        let canvas_size = 256;
        let bitmap = FPDFBitmap_Create(canvas_size, canvas_size, 1);
        if bitmap == 0 {
            println!("Failed to create bitmap for page {}", page_index);
            FPDF_ClosePage(page);
            return None;
        }

        let start_x = (canvas_size - thumb_width) / 2;
        let start_y = (canvas_size - thumb_height) / 2;

        let stroke_thickness = 3;
        let stroke_color = 0xFF797774; // ARGB for grey
        let stroke_x = start_x - stroke_thickness;
        let stroke_y = start_y - stroke_thickness;
        let stroke_width = thumb_width + 2 * stroke_thickness;
        let stroke_height = thumb_height + 2 * stroke_thickness;

        // Draw stroke
        FPDFBitmap_FillRect(
            bitmap,
            stroke_x,
            stroke_y,
            stroke_width,
            stroke_height,
            stroke_color,
        );
        // Draw a white background for PDF content area
        FPDFBitmap_FillRect(bitmap, start_x, start_y, thumb_width, thumb_height, 0xFFFFFFFF); // ARGB for white

        println!("Rendering page {} with dimensions: {}x{}", page_index, page_width, page_height);

        FPDF_RenderPageBitmap(
            bitmap,
            page,
            start_x,
            start_y,
            thumb_width,
            thumb_height,
            0,
            FPDF_LCD_TEXT | FPDF_ANNOT,
        );

        let buffer = FPDFBitmap_GetBuffer(bitmap) as *mut u8;
        let buffer_size = (canvas_size * canvas_size * 4) as usize;
        let bitmap_data = std::slice::from_raw_parts(buffer, buffer_size).to_vec();

        let image =
            ImageBuffer::from_raw(canvas_size as u32, canvas_size as u32, bitmap_data).unwrap();

        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);

        Some(RenderInfo {
            image,
            content_x: start_x,
            content_y: start_y,
            content_width: thumb_width,
            content_height: thumb_height,
        })
    }
}

fn main() {
    let pdf_path_str = "D:\\Users\\Shomn\\OneDrive - MSFT\\Documents\\Apple建议\\Apple Support Case 102452405098 7_11_2024_8_29pm.pdf";
    let c_pdf_path = CString::new(pdf_path_str).unwrap();

    unsafe {
        FPDF_InitLibrary();

        let doc = FPDF_LoadDocument(c_pdf_path.as_ptr(), ptr::null());
        if doc == 0 {
            println!("Failed to load document");
            FPDF_DestroyLibrary();
            return;
        }

        let page_count = FPDF_GetPageCount(doc);
        if page_count < 1 {
            println!("PDF has no pages.");
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return;
        }

        // --- Render Page 1 and Cut Corner ---
        let mut page1_info = match render_pdf_page(doc, 0) {
            Some(info) => info,
            None => {
                FPDF_CloseDocument(doc);
                FPDF_DestroyLibrary();
                return;
            }
        };

        let corner_size = 48;
        // Top-right corner of the *content*
        let content_top_right_x = page1_info.content_x + page1_info.content_width;
        let content_top_y = page1_info.content_y;

        for y in 0..corner_size {
            for x in 0..corner_size {
                // Cut a square corner
                let px = content_top_right_x - corner_size + x;
                let py = content_top_y + y;
                if px < 256 && py < 256 {
                    page1_info.image.put_pixel(px as u32, py as u32, Rgba([0, 0, 0, 0]));
                }
            }
        }


        // --- Render Page 2 ---
        let page2_info = if page_count > 1 {
            render_pdf_page(doc, 1)
        } else {
            None
        };

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();

        // --- Composite Images ---
        let mut final_image = ImageBuffer::new(256, 256);

        // 1. Draw page 2 (if it exists) - perfectly aligned with page 1
        if let Some(p2_info) = page2_info {
            image::imageops::overlay(&mut final_image, &p2_info.image, 0, 0);
        }

        // 2. Draw the modified page 1 on top
        image::imageops::overlay(&mut final_image, &page1_info.image, 0, 0);

        // 3. Load, resize, and overlay the binder icon
        let binder_icon_path = "D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\crates\\windows\\assets\\pdf_binder.png";
        if let Ok(binder_icon) = image::open(binder_icon_path) {
            let binder_height = page1_info.content_height as u32;
            // Stretch the binder to a fixed width of 8px.
            let binder_width = 8;
            
            let binder_icon_resized = image::imageops::resize(&binder_icon, binder_width, binder_height, image::imageops::FilterType::Triangle);

            // Position the binder to overlap the content entirely on the left.
            let binder_x = page1_info.content_x;
            let binder_y = page1_info.content_y;

            image::imageops::overlay(&mut final_image, &binder_icon_resized, binder_x as i64, binder_y as i64);
        } else {
            println!("Could not load binder icon.");
        }

        // 4. Load and overlay the fold icon
        let fold_icon_path = "D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\crates\\windows\\assets\\pdf_fold_256.png";
        if let Ok(fold_icon) = image::open(fold_icon_path) {
            let fold_icon_rgba = fold_icon.to_rgba8();
            
            // Calculate the top-right position of the first page thumbnail (including stroke)
            let stroke_thickness = 3;
            let icon_x = page1_info.content_x + page1_info.content_width + stroke_thickness - fold_icon_rgba.width() as i32;
            let icon_y = page1_info.content_y - stroke_thickness;
            
            // Position the fold icon at the top-right of the first page thumbnail
            image::imageops::overlay(&mut final_image, &fold_icon_rgba, icon_x as i64, icon_y as i64);
        } else {
            println!("Could not load fold icon.");
        }

        // Save the final image
        let output_path = "D:\\Users\\Shomn\\OneDrive - MSFT\\Source\\Repos\\space-thumbnails5\\crates\\windows\\assets\\final_thumbnail.png";
        if let Err(e) = final_image.save(output_path) {
            println!("Failed to save final image: {}", e);
        } else {
            println!("Final thumbnail saved to {}", output_path);
        }
    }
}