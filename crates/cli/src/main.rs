use std::{path::PathBuf, process::Command};

use clap::{ArgEnum, Parser};
use image::{ImageBuffer, Rgba};
use space_thumbnails::{RendererBackend, plugins::PluginManager};

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

    let backend = match args.api {
        BackendApi::Default => RendererBackend::Default,
        BackendApi::OpenGL => RendererBackend::OpenGL,
        BackendApi::Vulkan => RendererBackend::Vulkan,
        BackendApi::Metal => RendererBackend::Metal,
    };

    let manager = PluginManager::with_backend(backend);

    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    
    // Read header for validation
    let mut header = [0u8; 20];
    if let Ok(mut file) = std::fs::File::open(&input) {
        use std::io::Read;
        let _ = file.read_exact(&mut header);
    }

    if let Some(generator) = manager.get_generator(&header, &ext) {
         let buffer = match generator.generate(None, args.width, args.height, &ext, Some(&input)) {
             Ok(b) => b,
             Err(e) => {
                 eprintln!("Failed to generate thumbnail: {}", e);
                 std::process::exit(1);
             }
         };
         
         let image = ImageBuffer::<Rgba<u8>, _>::from_raw(args.width, args.height, buffer).unwrap();
         image.save(args.output).unwrap();
    } else {
         eprintln!("No plugin found for file: {:?}", input);
         std::process::exit(1);
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
}
