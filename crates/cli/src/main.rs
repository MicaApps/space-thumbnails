use std::{path::PathBuf, process::Command};

use clap::{ArgEnum, Parser};
use image::{ImageBuffer, Rgba};
use space_thumbnails::{SpaceThumbnailsRenderer, RendererBackend};

/// A command line tool for generating thumbnails for 3D model files.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The output file
    output: PathBuf,

    // The 3D model file for which you want to generate thumbnail.
    #[clap(short, long)]
    input: PathBuf,

    // Specify the backend API
    #[clap(short, long, arg_enum, default_value_t)]
    api: BackendApi,

    // Generated thumbnail width
    #[clap(short, long, default_value_t = 800)]
    width: u32,

    // Generated thumbnail height
    #[clap(short, long, default_value_t = 800)]
    height: u32,

    // Optional lock file to delete upon completion
    #[clap(long)]
    lock_file: Option<PathBuf>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum BackendApi {
    Default,
    OpenGL,
    Vulkan,
    Metal,
}

impl Default for BackendApi {
    fn default() -> Self {
        Self::Default
    }
}

fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    // Directly use the input path, the library now handles STEP files internally
    let input = &args.input;

    let renderer_opt = SpaceThumbnailsRenderer::new(
        match args.api {
            BackendApi::Default => RendererBackend::Default,
            BackendApi::OpenGL => RendererBackend::OpenGL,
            BackendApi::Vulkan => RendererBackend::Vulkan,
            BackendApi::Metal => RendererBackend::Metal,
        },
        args.width,
        args.height,
    );
    
    if renderer_opt.is_none() {
        return Err("Failed to create renderer backend".into());
    }
    let mut renderer = renderer_opt.unwrap();
    
    // Check if loading succeeds
    if renderer.load_asset_from_file(input).is_none() {
        return Err(format!("Failed to load asset: {:?}", input).into());
    }

    let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];
    renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());

    if let Some(image) = ImageBuffer::<Rgba<u8>, _>::from_raw(args.width, args.height, screenshot_buffer) {
        // Flip image vertically because OpenGL/Vulkan might output bottom-up
        let image = image::imageops::flip_vertical(&image);
        image.save(&args.output)?;
    } else {
        return Err("Failed to create image buffer".into());
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_UPDATEITEM, SHCNF_PATH, SHCNF_FLUSH};
        use std::ffi::CString;
        use std::os::windows::ffi::OsStrExt;

        // Convert input path to wide string (null-terminated) for Windows API
        let path_buf: Vec<u16> = input.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        
        unsafe {
            SHChangeNotify(
                SHCNE_UPDATEITEM,
                SHCNF_PATH | SHCNF_FLUSH,
                path_buf.as_ptr() as *const _,
                std::ptr::null()
            );
        }
    }
    
    Ok(())
}

fn main() {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS};
        let _ = SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }

    // Limit concurrency to prevent Explorer freeze (Semaphore with count 2)
    #[cfg(target_os = "windows")]
    let semaphore_handle = unsafe {
        use windows::Win32::System::Threading::{CreateSemaphoreW, WaitForSingleObject};
        use windows::Win32::Foundation::CloseHandle;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        let name: Vec<u16> = OsStr::new("Local\\SpaceThumbnails_Semaphore").encode_wide().chain(std::iter::once(0)).collect();
        // Allow 2 concurrent processes
        // CreateSemaphoreW returns HANDLE directly, not Result
        let handle = CreateSemaphoreW(std::ptr::null(), 2, 2, windows::core::PCWSTR(name.as_ptr()));
        
        if !handle.is_invalid() {
             WaitForSingleObject(handle, 0xFFFFFFFF); // INFINITE
             Some(handle)
        } else {
             None
        }
    };

    let args = Args::parse();
    
    let result = run(&args);

    // Release semaphore
    #[cfg(target_os = "windows")]
    if let Some(handle) = semaphore_handle {
        use windows::Win32::System::Threading::ReleaseSemaphore;
        use windows::Win32::Foundation::CloseHandle;
        unsafe {
            let _ = ReleaseSemaphore(handle, 1, std::ptr::null_mut());
            CloseHandle(handle);
        }
    }

    // Clean up lock file regardless of success/failure
    if let Some(lock_path) = &args.lock_file {
        if lock_path.exists() {
             // Retry a few times in case of weird file locks? No, just try once.
             let _ = std::fs::remove_file(lock_path);
        }
    }

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
