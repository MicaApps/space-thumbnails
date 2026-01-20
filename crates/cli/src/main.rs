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

fn main() {
    // Lower process priority on Windows to prevent freezing the UI
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS};
        let _ = SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }

    let args = Args::parse();

    // Directly use the input path, the library now handles STEP files internally
    let input = args.input;

    let mut renderer = SpaceThumbnailsRenderer::new(
        match args.api {
            BackendApi::Default => RendererBackend::Default,
            BackendApi::OpenGL => RendererBackend::OpenGL,
            BackendApi::Vulkan => RendererBackend::Vulkan,
            BackendApi::Metal => RendererBackend::Metal,
        },
        args.width,
        args.height,
    );
    
    // Check if loading succeeds
    if renderer.load_asset_from_file(&input).is_none() {
        eprintln!("Failed to load asset: {:?}", input);
        std::process::exit(1);
    }

    let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];
    renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(args.width, args.height, screenshot_buffer).unwrap();
    image.save(args.output).unwrap();

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
}
