use windows::{
    core::{Interface},
    Win32::{
        Graphics::Gdi::{HBITMAP, BITMAP, GetObjectW, GetDIBits, CreateCompatibleDC, SelectObject, DeleteDC, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB},
        System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, CoUninitialize},
        UI::Shell::PropertiesSystem::IInitializeWithFile,
        UI::Shell::{IThumbnailProvider, WTS_ALPHATYPE},
    },
};
use image::{DynamicImage};
use space_thumbnails_windows::providers::OfficeThumbnailHandler;

fn main() {
    let input_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\Samples\Sample.xlsx";
    let output_path = "Sample.png";
    let cx = 1024;

    unsafe {
        let _ = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED);

        // 1. Create handler
        let handler_impl = OfficeThumbnailHandler::new_for_test(".xlsx");
        let handler: IThumbnailProvider = handler_impl.into();

        // 2. Initialize with file
        let init: IInitializeWithFile = handler.cast().unwrap();
        let wide_path: Vec<u16> = input_path.encode_utf16().chain(std::iter::once(0)).collect();
        init.Initialize(windows::core::PCWSTR(wide_path.as_ptr()), 0).unwrap();

        // 3. Get thumbnail
        let mut hbitmap = HBITMAP(0);
        let mut alpha = WTS_ALPHATYPE(0);
        println!("Generating thumbnail for {}...", input_path);
        handler.GetThumbnail(cx, &mut hbitmap, &mut alpha).unwrap();

        if hbitmap.0 != 0 {
            if let Some(img) = hbitmap_to_dynamic_image(hbitmap) {
                img.save(output_path).unwrap();
                println!("Successfully saved thumbnail to {}", output_path);
            } else {
                println!("Failed to convert HBITMAP to image");
            }
            let _ = DeleteObject(hbitmap);
        } else {
            println!("Failed to get HBITMAP");
        }

        CoUninitialize();
    }
}

unsafe fn hbitmap_to_dynamic_image(hbitmap: HBITMAP) -> Option<DynamicImage> {
    let mut bm: BITMAP = std::mem::zeroed();
    if GetObjectW(hbitmap, std::mem::size_of::<BITMAP>() as i32, &mut bm as *mut _ as *mut _) == 0 {
        return None;
    }

    let width = bm.bmWidth;
    let height = bm.bmHeight;

    let hdc = CreateCompatibleDC(None);
    let old_obj = SelectObject(hdc, hbitmap);

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB as u32,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut buffer = vec![0u8; (width * height * 4) as usize];
    if GetDIBits(hdc, hbitmap, 0, height as u32, buffer.as_mut_ptr() as *mut _, &mut bmi, DIB_RGB_COLORS) == 0 {
        SelectObject(hdc, old_obj);
        DeleteDC(hdc);
        return None;
    }

    SelectObject(hdc, old_obj);
    DeleteDC(hdc);

    // Convert BGRA to RGBA
    for i in (0..buffer.len()).step_by(4) {
        let b = buffer[i];
        let r = buffer[i + 2];
        buffer[i] = r;
        buffer[i + 2] = b;
    }

    let img = image::RgbaImage::from_raw(width as u32, height as u32, buffer)?;
    Some(DynamicImage::ImageRgba8(img))
}
