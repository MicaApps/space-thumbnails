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
    let args = Args::parse();

    let input = match args
        .input
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("stp") | Some("step") => {
            let mut converted = args.input.clone();
            converted.set_extension("obj");

            let mut cmd = Command::new("cmd");
            cmd.arg("/C")
                .arg("step2obj.bat")
                .env("STEP2OBJ_INPUT", &args.input)
                .env("STEP2OBJ_OUTPUT", &converted);
            
            let status = cmd.status().expect("failed to execute step2obj command");

            if !status.success() {
                eprintln!(
                    "Failed to convert STEP file with step2obj, exit code: {:?}",
                    status.code()
                );
                std::process::exit(1);
            }

            converted
        }
        _ => args.input.clone(),
    };

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
    renderer.load_asset_from_file(&input).expect("Failed to load converted asset");
    let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];
    renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(args.width, args.height, screenshot_buffer).unwrap();
    image.save(args.output).unwrap();
}
