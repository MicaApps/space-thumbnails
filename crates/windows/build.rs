use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=assets/error256x256.png");
    png2argb(
        "assets/error256x256.png",
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("error256x256.bin"),
    );

    println!("cargo:rerun-if-changed=assets/timeout256x256.png");
    png2argb(
        "assets/timeout256x256.png",
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("timeout256x256.bin"),
    );

    println!("cargo:rerun-if-changed=assets/toolarge256x256.png");
    png2argb(
        "assets/toolarge256x256.png",
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("toolarge256x256.bin"),
    );

    println!("cargo:rerun-if-changed=assets/loading.png");
    png2argb(
        "assets/loading.png",
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("loading.bin"),
    );
}

fn png2argb(source: impl AsRef<Path>, out: impl AsRef<Path>) {
    let img = image::open(source).unwrap();
    let rgba = img.to_rgba8();
    let mut argb = Vec::with_capacity(rgba.len());

    for (_, _, pixel) in rgba.enumerate_pixels() {
        let alpha = pixel.0[3] as u32;
        let r = (pixel.0[0] as u32 * alpha) / 255;
        let g = (pixel.0[1] as u32 * alpha) / 255;
        let b = (pixel.0[2] as u32 * alpha) / 255;

        argb.push(b as u8);
        argb.push(g as u8);
        argb.push(r as u8);
        argb.push(alpha as u8);
    }

    fs::write(out, argb).unwrap();
}
