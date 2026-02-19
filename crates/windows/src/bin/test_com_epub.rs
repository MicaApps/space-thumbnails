use windows::core::{GUID, Interface, HRESULT};
use windows::Win32::System::Com::{CoInitializeEx, CoCreateInstance, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::UI::Shell::IThumbnailProvider;
use windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream;
use windows::Win32::System::Com::IStream;
use windows::Win32::UI::Shell::SHCreateStreamOnFileEx;
use std::path::Path;

fn main() -> windows::core::Result<()> {
    unsafe {
        CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED)?;
    }

    let path_str = r"F:\毛泽东 真实的故事.epub";
    let path = Path::new(path_str);
    println!("Testing COM thumbnail for: {:?}", path);

    // My EPUB CLSID: {772657D4-0325-4632-9154-116584281388}
    let clsid = GUID::from_values(0x772657D4, 0x0325, 0x4632, [0x91, 0x54, 0x11, 0x65, 0x84, 0x28, 0x13, 0x88]);

    println!("Creating instance of CLSID: {:?}", clsid);
    let provider: IThumbnailProvider = unsafe {
        CoCreateInstance(&clsid, None, CLSCTX_INPROC_SERVER)?
    };
    println!("Provider created successfully.");

    // Initialize with stream
    let init_with_stream: IInitializeWithStream = provider.cast()?;
    let path_wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
    
    let stream: IStream = unsafe {
        SHCreateStreamOnFileEx(windows::core::PCWSTR(path_wide.as_ptr()), 0, 0, false, None)?
    };

    unsafe {
        init_with_stream.Initialize(&stream, 0)?;
    }
    println!("Initialize(Stream) called successfully.");

    // Get thumbnail
    let mut bitmap = windows::Win32::Graphics::Gdi::HBITMAP::default();
    let mut alpha = windows::Win32::UI::Shell::WTS_ALPHATYPE::default();
    
    unsafe {
        provider.GetThumbnail(256, &mut bitmap, &mut alpha)?;
    }
    println!("GetThumbnail returned successfully. HBITMAP: {:?}", bitmap);

    Ok(())
}
