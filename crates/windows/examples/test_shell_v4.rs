use windows::{
    core::{PCWSTR, GUID},
    Win32::{
        Foundation::{SIZE},
        Graphics::Gdi::{HBITMAP, BITMAP, GetObjectW, GetDIBits, CreateCompatibleDC, SelectObject, DeleteDC, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB},
        UI::Shell::{SHCreateItemFromParsingName, IShellItemImageFactory, SIIGBF_THUMBNAILONLY, SIIGBF_BIGGERSIZEOK, SIIGBF_RESIZETOFIT, SIIGBF_SCALEUP},
        System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, CoUninitialize},
    },
};
use image::{RgbaImage, DynamicImage};

fn main() {
    let input_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\Samples\2023年工作总结顺昌县大历镇人民政府.docx";
    let output_path = "shell_thumb_actual_v4.png";
    let cx = 1024;

    unsafe {
        // 关键点 1: 必须使用 STA (APARTMENTTHREADED) 才能加载许多 Shell 扩展
        let hr = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            println!("CoInitializeEx failed: {:?}", hr);
        }

        let wide: Vec<u16> = input_path.encode_utf16().chain(std::iter::once(0)).collect();
        let pcwstr = PCWSTR(wide.as_ptr());

        // 2. 获取 IShellItemImageFactory
        let factory: IShellItemImageFactory = match SHCreateItemFromParsingName(pcwstr, None) {
            Ok(f) => f,
            Err(e) => {
                println!("Error creating shell item factory: {:?}", e);
                return;
            }
        };

        let size = SIZE { cx: cx as i32, cy: cx as i32 };

        // 关键点 2: QuickLook 风格的标志组合
        // 我们先尝试最纯净的缩略图 (THUMBNAILONLY)
        // 如果失败，再尝试允许放大 (SCALEUP) 
        let flags = SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK | SIIGBF_SCALEUP;
        
        println!("Attempting GetImage with flags: SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK | SIIGBF_SCALE_UP");
        let hbitmap = match factory.GetImage(size, flags) {
            Ok(h) => h,
            Err(e) => {
                println!("Error with primary flags: {:?}", e);
                println!("Falling back to SIIGBF_RESIZETOFIT...");
                match factory.GetImage(size, SIIGBF_RESIZETOFIT) {
                    Ok(h) => h,
                    Err(e2) => {
                        println!("Failed with all Shell flags: {:?}", e2);
                        return;
                    }
                }
            }
        };

        // 3. 转换并保存
        if let Some(img) = hbitmap_to_dynamic_image_fixed(hbitmap) {
            img.save(output_path).unwrap();
            println!("Successfully saved shell thumbnail to {}", output_path);
        } else {
            println!("Failed to convert HBITMAP to image");
        }

        let _ = DeleteObject(hbitmap);
        CoUninitialize();
    }
}

unsafe fn hbitmap_to_dynamic_image_fixed(hbitmap: HBITMAP) -> Option<DynamicImage> {
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

    // BGRA -> RGBA swap
    for i in (0..buffer.len()).step_by(4) {
        let b = buffer[i];
        let r = buffer[i + 2];
        buffer[i] = r;
        buffer[i + 2] = b;
    }

    let rgba = RgbaImage::from_raw(width as u32, height as u32, buffer)?;
    Some(DynamicImage::ImageRgba8(rgba))
}
