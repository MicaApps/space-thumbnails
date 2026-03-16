use windows::{
    core::{PCWSTR, GUID},
    Win32::{
        Foundation::{SIZE},
        Graphics::Gdi::{HBITMAP, BITMAP, GetObjectW, GetDIBits, CreateCompatibleDC, SelectObject, DeleteDC, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB},
        UI::Shell::{SHCreateItemFromParsingName, IShellItem, IThumbnailProvider, WTSAT_ARGB},
        System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
    },
};
use image::{RgbaImage, DynamicImage};

// BHID_ThumbnailHandler: {E357FCCD-A995-4576-B01F-234630154E96}
const BHID_THUMBNAIL_HANDLER: GUID = GUID::from_u128(0xE357FCCD_A995_4576_B01F_234630154E96);

fn main() {
    let input_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\Samples\2023年工作总结顺昌县大历镇人民政府.docx";
    let output_path = "shell_thumb_actual_v2.png";
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

        // 2. 绑定到 IThumbnailProvider
        let thumb_provider: IThumbnailProvider = match shell_item.BindToHandler(None, &BHID_THUMBNAIL_HANDLER) {
            Ok(p) => p,
            Err(e) => {
                println!("Error binding to IThumbnailProvider: {:?}", e);
                println!("This usually means Office's thumbnail handler is not correctly registered for this file type.");
                return;
            }
        };

        // 3. 获取缩略图
        let mut hbitmap = HBITMAP::default();
        let mut alpha = WTSAT_ARGB;
        match thumb_provider.GetThumbnail(cx, &mut hbitmap, &mut alpha) {
            Ok(_) => {
                println!("Successfully got HBITMAP from IThumbnailProvider");
            },
            Err(e) => {
                println!("Error calling GetThumbnail: {:?}", e);
                return;
            }
        }

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
