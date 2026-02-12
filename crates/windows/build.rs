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

    if let Err(e) = setup_pdfium() {
        panic!("Failed to setup pdfium: {}", e);
    }
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

fn setup_pdfium() -> std::io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let pdfium_dir = PathBuf::from(&out_dir).join("pdfium");

    if !pdfium_dir.exists() {
        println!("cargo:warning=Downloading PDFium binaries...");
        
        // Try multiple mirrors and versions - using latest release URLs
        let urls = vec![
            ("latest", "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz"),
            ("stable", "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F5613/pdfium-win-x64.tgz"),
            ("backup", "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F5602/pdfium-win-x64.tgz"),
        ];

        let mut last_error = None;
        
        for (version, url) in urls {
            println!("cargo:warning=Trying PDFium version {} from {}", version, url);
            
            // Download with retry mechanism
            let mut response = None;
            for attempt in 1..=3 {
                println!("cargo:warning=Download attempt {} for PDFium version {}...", attempt, version);
                match reqwest::blocking::get(url) {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            response = Some(resp);
                            break;
                        } else {
                            println!("cargo:warning=Download attempt {} failed with status: {}", attempt, resp.status());
                        }
                    }
                    Err(e) => {
                        println!("cargo:warning=Download attempt {} failed with error: {}", attempt, e);
                    }
                }
                if attempt < 3 {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }

            if let Some(response) = response {
                match response.bytes() {
                    Ok(bytes) => {
                        println!("cargo:warning=Downloaded {} bytes for version {}, extracting...", bytes.len(), version);
                        
                        // Extract with better error handling
                        let tar = flate2::read::GzDecoder::new(bytes.as_ref());
                        let mut archive = tar::Archive::new(tar);
                        
                        match archive.unpack(&pdfium_dir) {
                            Ok(_) => {
                                println!("cargo:warning=Successfully extracted PDFium version {}", version);
                                break;
                            }
                            Err(e) => {
                                println!("cargo:warning=Failed to extract PDFium version {}: {}", version, e);
                                last_error = Some(e);
                                // Clean up partial extraction
                                let _ = fs::remove_dir_all(&pdfium_dir);
                            }
                        }
                    }
                    Err(e) => {
                        println!("cargo:warning=Failed to read response bytes for version {}: {}", version, e);
                    }
                }
            }
        }

        if !pdfium_dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("Failed to download and extract PDFium from all sources. Last error: {:?}", last_error)
            ));
        }
    }

    // Check for lib directory structure
    let lib_path = pdfium_dir.join("lib");
    if !lib_path.exists() {
        // Some PDFium distributions might have different structures
        // Let's check for alternative locations
        let alt_paths = vec![
            pdfium_dir.join("lib"),
            pdfium_dir.clone(),
            pdfium_dir.join("release"),
            pdfium_dir.join("bin"),
        ];

        let mut found_lib = false;
        for path in &alt_paths {
            if path.exists() {
                println!("cargo:warning=Found PDFium libraries at: {:?}", path);
                println!("cargo:rustc-link-search=native={}", path.display());
                found_lib = true;
                break;
            }
        }

        if !found_lib {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("PDFium lib directory not found. Searched in: {:?}", alt_paths)
            ));
        }
    } else {
        println!("cargo:rustc-link-search=native={}", lib_path.display());
    }

    // Check for the correct library file name
    let lib_dir = if lib_path.exists() { lib_path } else { pdfium_dir.clone() };
    
    // PDFium might be named differently depending on the distribution
    let possible_lib_names = vec!["pdfium.dll.lib", "pdfium.lib", "pdfium.dll"];
    let mut found_lib_file = false;
    
    for lib_name in &possible_lib_names {
        let lib_file = lib_dir.join(lib_name);
        if lib_file.exists() {
            println!("cargo:warning=Found PDFium library file: {:?}", lib_file);
            
            // For Windows, when we have pdfium.dll.lib, we need to link against pdfium.dll
            // This is the import library for the DLL
            if lib_name == &"pdfium.dll.lib" {
                println!("cargo:rustc-link-lib=pdfium");
                let lib_file_path = lib_dir.join(lib_name);
                let new_lib_file_path = lib_dir.join("pdfium.lib");
                if let Err(e) = fs::copy(&lib_file_path, &new_lib_file_path) {
                    println!("cargo:warning=Failed to copy pdfium.dll.lib to pdfium.lib: {}", e);
                }
                // Also need to make sure the DLL is available at runtime
                let dll_path = lib_dir.join("pdfium.dll");
                if dll_path.exists() {
                    println!("cargo:warning=Found PDFium DLL: {:?}", dll_path);
                }
            } else if lib_name.ends_with(".lib") {
                println!("cargo:rustc-link-lib=pdfium");
            } else {
                println!("cargo:rustc-link-lib={}", lib_name);
            }
            
            found_lib_file = true;
            break;
        }
    }
    
    if !found_lib_file {
        // List files in the directory to help debug
        if let Ok(entries) = fs::read_dir(&lib_dir) {
            println!("cargo:warning=Files in PDFium directory:");
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("cargo:warning=  {:?}", entry.file_name());
                }
            }
        }
        
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDFium library file not found. Searched for: {:?} in {:?}", possible_lib_names, lib_dir)
        ));
    }
    Ok(())
}
