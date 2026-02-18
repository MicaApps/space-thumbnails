use std::path::PathBuf;
use std::io::Write;
use clap::{Parser, ArgEnum};
use space_thumbnails::{SpaceThumbnailsRenderer, RendererBackend};
use image::{ImageBuffer, Rgba};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input file path
    #[clap(value_parser)]
    input: PathBuf,

    /// Output file path
    #[clap(value_parser)]
    output: PathBuf,

    /// Specify the backend API
    #[clap(short, long, arg_enum, default_value_t)]
    api: BackendApi,

    /// Thumbnail width
    #[clap(long, default_value = "256")]
    width: u32,

    /// Thumbnail height
    #[clap(long, default_value = "256")]
    height: u32,

    /// Optional lock file path
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

fn log_to_file(msg: &str) {
    let temp_dir = std::env::temp_dir();
    let log_path = temp_dir.join("space_thumbnails_cli.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let _ = writeln!(file, "[{}] {}", std::process::id(), msg);
    }
}

fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
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
    if renderer.load_asset_from_file(&args.input).is_none() {
        return Err(format!("Failed to load asset: {:?}", args.input).into());
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

    Ok(())
}

struct LockFileGuard {
    path: PathBuf,
}

impl Drop for LockFileGuard {
    fn drop(&mut self) {
        if self.path.exists() {
             let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn main() {
    let args = Args::parse();
    log_to_file(&format!("CLI Started for: {:?}", args.input));

    // RAII guard for lock file
    let mut _lock_guard = if let Some(lock_path) = &args.lock_file {
        Some(LockFileGuard { path: lock_path.clone() })
    } else {
        None
    };
    
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS};
        let _ = SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }

    // Limit concurrency to prevent Explorer freeze (Semaphore with count 4)
    #[cfg(target_os = "windows")]
    let semaphore_handle = unsafe {
        use windows::Win32::System::Threading::{CreateSemaphoreW, WaitForSingleObject};
    // use windows::Win32::Foundation::CloseHandle;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        let name: Vec<u16> = OsStr::new("Local\\SpaceThumbnails_Semaphore").encode_wide().chain(std::iter::once(0)).collect();
        // Allow 4 concurrent processes
        // CreateSemaphoreW returns HANDLE directly, not Result
        let handle = CreateSemaphoreW(std::ptr::null(), 4, 4, windows::core::PCWSTR(name.as_ptr()));
        
        if !handle.is_invalid() {
             WaitForSingleObject(handle, 0xFFFFFFFF); // INFINITE
             Some(handle)
        } else {
             None
        }
    };
    
    let result = run(&args);

    // Release semaphore immediately after work is done
    #[cfg(target_os = "windows")]
    if let Some(handle) = semaphore_handle {
        use windows::Win32::System::Threading::ReleaseSemaphore;
        use windows::Win32::Foundation::CloseHandle;
        unsafe {
            let _ = ReleaseSemaphore(handle, 1, std::ptr::null_mut());
            CloseHandle(handle);
        }
    }

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        log_to_file(&format!("Error: {}", e));
        drop(_lock_guard);
        std::process::exit(1);
    } else {
        log_to_file(&format!("Success. Image saved to {:?}. Deleting lock and notifying shell.", args.output));
        // Success case:
        // 1. Drop the lock guard to delete the .lock file
        drop(_lock_guard);

        // 2. Now notify the Shell that the thumbnail is ready
        // It is CRITICAL that the lock file is gone before this notification,
        // otherwise Explorer might query again, see the lock file, and cache the placeholder again!
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::Shell::{
                SHChangeNotify, SHCNE_UPDATEITEM, SHCNE_UPDATEDIR, SHCNF_IDLIST, SHCNF_FLUSH, 
                ILCreateFromPathW, ILFree
            };
            use std::os::windows::ffi::OsStrExt;
            use std::path::PathBuf;
            use std::ffi::c_void;

            let input = &args.input;
            
            // Clean path: remove \\?\ prefix if present
            let input_str = input.to_string_lossy();
            let clean_path = if input_str.starts_with(r"\\?\") {
                PathBuf::from(&input_str[4..])
            } else {
                input.to_path_buf()
            };
            
            // Convert input path to wide string (null-terminated) for Windows API
            let path_buf: Vec<u16> = clean_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
            
            unsafe {
                // Try to create PIDL from path - this is more robust for Shell notifications
                let pidl = ILCreateFromPathW(windows::core::PCWSTR(path_buf.as_ptr()));
                
                if !pidl.is_null() {
                    log_to_file(&format!("Notifying Shell (via PIDL) for: {:?}", clean_path));
                    
                    // Notify item updated using PIDL
                    SHChangeNotify(
                        SHCNE_UPDATEITEM,
                        SHCNF_IDLIST | SHCNF_FLUSH,
                        pidl as *const c_void,
                        std::ptr::null()
                    );
                    
                    // Notify parent directory update
                    if let Some(parent) = clean_path.parent() {
                        let parent_buf: Vec<u16> = parent.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                        let parent_pidl = ILCreateFromPathW(windows::core::PCWSTR(parent_buf.as_ptr()));
                        
                        if !parent_pidl.is_null() {
                            log_to_file(&format!("Notifying Shell (via PIDL) for parent: {:?}", parent));
                            SHChangeNotify(
                                SHCNE_UPDATEDIR,
                                SHCNF_IDLIST | SHCNF_FLUSH,
                                parent_pidl as *const c_void,
                                std::ptr::null()
                            );
                            ILFree(parent_pidl);
                        }
                    }

                    ILFree(pidl);
                } else {
                    // Fallback to path-based notification if PIDL creation fails
                    use windows::Win32::UI::Shell::SHCNF_PATHW;
                    log_to_file(&format!("PIDL creation failed. Fallback to path notification for: {:?}", clean_path));
                    
                    SHChangeNotify(
                        SHCNE_UPDATEITEM,
                        SHCNF_PATHW | SHCNF_FLUSH,
                        path_buf.as_ptr() as *const c_void,
                        std::ptr::null()
                    );

                    if let Some(parent) = clean_path.parent() {
                        let parent_buf: Vec<u16> = parent.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                        SHChangeNotify(
                            SHCNE_UPDATEDIR,
                            SHCNF_PATHW | SHCNF_FLUSH,
                            parent_buf.as_ptr() as *const c_void,
                            std::ptr::null()
                        );
                    }
                }
            }
        }
    }
}
