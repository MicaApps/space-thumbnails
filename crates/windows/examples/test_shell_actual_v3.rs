use windows::{
    core::{PCWSTR, GUID},
    Win32::{
        Foundation::{SIZE},
        Graphics::Gdi::{HBITMAP, BITMAP, GetObjectW, GetDIBits, CreateCompatibleDC, SelectObject, DeleteDC, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB},
        UI::Shell::{SHCreateItemFromParsingName, IShellItem, IExtractImage, IEIFLAG_QUALITY, IEIFLAG_SCREEN, IEIFLAG_OFFLINE},
        System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
    },
};
use image::{RgbaImage, DynamicImage};

fn main() {
    let input_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\Samples\2023年工作总结顺昌县大历镇人民政府.docx";
    let output_path = "shell_thumb_actual_v3.png";
    let cx = 1024;

    unsafe {
        let _ = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED);

        let wide: Vec<u16> = input_path.encode_utf16().chain(std::iter::once(0)).collect();
        let pcwstr = PCWSTR(wide.as_ptr());

        // 1. 获取 IShellItem
        let shell_item: IShellItem = match SHCreateItemFromParsingName(pcwstr, None) {
            Ok(item) => item,
            Err(e) => {
                println!("Error creating shell item: {:?}", e);
                return;
            }
        };

        // 2. 绑定到 IExtractImage
        // IExtractImage GUID: {BB2E617C-0920-11d1-9A0B-00C04FC2D6C1}
        let bhid_extract_image = GUID::from_u128(0xBB2E617C_0920_11d1_9A0B_00C04FC2D6C1);
        let extract_image: IExtractImage = match shell_item.BindToHandler(None, &bhid_extract_image) {
            Ok(p) => p,
            Err(e) => {
                println!("Error binding to IExtractImage: {:?}", e);
                return;
            }
        };

        // 3. 准备获取图片
        let mut path_buffer = [0u16; 260];
        let mut size = SIZE { cx: cx as i32, cy: cx as i32 };
        let mut depth = 32;
        let mut flags = IEIFLAG_QUALITY | IEIFLAG_SCREEN | IEIFLAG_OFFLINE;
        
        match extract_image.GetLocation(PWSTR(path_buffer.as_mut_ptr()), path_buffer.len() as u32, &mut 0, &size, depth, &mut flags) {
            Ok(_) => {
                let mut hbitmap = HBITMAP::default();
                match extract_image.Extract(&mut hbitmap) {
                    Ok(_) => {
                        println!("Successfully extracted HBITMAP via IExtractImage");
                        if let Some(img) = hbitmap_to_dynamic_image_fixed(hbitmap) {
                            img.save(output_path).unwrap();
                            println!("Successfully saved shell thumbnail to {}", output_path);
                        }
                        let _ = DeleteObject(hbitmap);
                    },
                    Err(e) => println!("Error in Extract: {:?}", e),
                }
            },
            Err(e) => println!("Error in GetLocation: {:?}", e),
        }
    }
}

// 补丁代码，因为 windows-rs 可能没直接导出某些结构
#[repr(transparent)]
struct PWSTR(pub *mut u16);

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
