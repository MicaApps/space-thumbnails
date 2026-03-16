use windows::{
    core::{PCWSTR},
    Win32::{
        Foundation::{SIZE},
        Graphics::Gdi::{HBITMAP, BITMAP, GetObjectW, GetDIBits, CreateCompatibleDC, SelectObject, DeleteDC, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB},
        UI::Shell::{SHCreateItemFromParsingName, IShellItemImageFactory, SIIGBF_THUMBNAILONLY, SIIGBF_BIGGERSIZEOK, SIIGBF_RESIZETOFIT},
        System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
    },
};
use image::{RgbaImage, DynamicImage};

fn main() {
    let input_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\Samples\2023年工作总结顺昌县大历镇人民政府.docx";
    let output_path = "shell_thumb_actual.png";
    let cx = 1024; // 尝试更大的尺寸

    println!("Starting CoInitializeEx...");
    unsafe {
        // 使用 APARTMENTTHREADED 有时对 Shell 接口更稳定
        let _ = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED);

        println!("Encoding path...");
        let wide: Vec<u16> = input_path.encode_utf16().chain(std::iter::once(0)).collect();
        let pcwstr = PCWSTR(wide.as_ptr());

        println!("Calling SHCreateItemFromParsingName...");
        let shell_item: IShellItemImageFactory = match SHCreateItemFromParsingName(pcwstr, None) {
            Ok(item) => item,
            Err(e) => {
                println!("Error creating shell item: {:?}", e);
                return;
            }
        };

        let size = SIZE { cx: cx as i32, cy: cx as i32 };

        // 尝试组合标志：THUMBNAILONLY (只要缩略图) | BIGGERSIZEOK | RESIZETOFIT
        let flags = SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK | SIIGBF_RESIZETOFIT;
        
        println!("Calling shell_item.GetImage (THUMBNAILONLY)...");
        let res = shell_item.GetImage(size, flags);
        let hbitmap = match res {
            Ok(h) => {
                println!("Got HBITMAP from shell (THUMBNAILONLY)");
                h
            },
            Err(e) => {
                println!("Error code: 0x{:08X}", e.code().0);
                println!("Falling back to general GETIMAGE...");
                let res2 = shell_item.GetImage(size, SIIGBF_RESIZETOFIT);
                match res2 {
                    Ok(h) => {
                        println!("Got HBITMAP from shell (fallback)");
                        h
                    },
                    Err(e2) => {
                        println!("Failed even with fallback: 0x{:08X}", e2.code().0);
                        return;
                    }
                }
            }
        };

        println!("Converting HBITMAP to DynamicImage...");
        if let Some(img) = hbitmap_to_dynamic_image_fixed(hbitmap) {
            img.save(output_path).unwrap();
            println!("Successfully saved shell thumbnail to {}", output_path);
        } else {
            println!("Failed to convert HBITMAP to image");
        }

        let _ = DeleteObject(hbitmap);
    }
}

unsafe fn hbitmap_to_dynamic_image_fixed(hbitmap: HBITMAP) -> Option<DynamicImage> {
    if hbitmap.is_invalid() {
        println!("HBITMAP is invalid");
        return None;
    }

    let mut bm: BITMAP = std::mem::zeroed();
    if GetObjectW(hbitmap, std::mem::size_of::<BITMAP>() as i32, &mut bm as *mut _ as *mut _) == 0 {
        println!("GetObjectW failed");
        return None;
    }

    let width = bm.bmWidth;
    let height = bm.bmHeight;
    println!("Bitmap dimensions: {}x{}", width, height);

    if width <= 0 || height <= 0 {
        println!("Invalid bitmap dimensions");
        return None;
    }

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

    let buffer_size = (width * height * 4) as usize;
    println!("Allocating buffer of size: {}", buffer_size);
    let mut buffer = vec![0u8; buffer_size];
    
    let result = GetDIBits(hdc, hbitmap, 0, height as u32, buffer.as_mut_ptr() as *mut _, &mut bmi, DIB_RGB_COLORS);
    
    SelectObject(hdc, old_obj);
    DeleteDC(hdc);

    if result == 0 {
        println!("GetDIBits failed");
        return None;
    }

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
