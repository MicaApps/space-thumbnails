use std::env;
use std::path::PathBuf;

fn main() {
    println!("Testing PDFium integration...");
    
    // Set up environment to find PDFium DLL
    let out_dir = env::var("OUT_DIR").unwrap_or_else(|_| {
        // Try to find the build output directory
        let current_dir = env::current_dir().unwrap();
        let target_dir = current_dir.join("target").join("debug").join("build");
        
        if let Ok(entries) = std::fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.file_name().unwrap_or_default().to_string_lossy().contains("space-thumbnails-windows") {
                    let out_path = path.join("out").join("pdfium");
                    if out_path.exists() {
                        return out_path.to_string_lossy().to_string();
                    }
                }
            }
        }
        
        // Fallback to a reasonable default
        target_dir.to_string_lossy().to_string()
    });
    
    println!("Looking for PDFium in: {}", out_dir);
    
    // Add PDFium directory to PATH for Windows to find the DLL
    let pdfium_path = PathBuf::from(&out_dir);
    if pdfium_path.exists() {
        env::set_var("PATH", format!("{};{}", pdfium_path.display(), env::var("PATH").unwrap_or_default()));
        println!("Added PDFium directory to PATH");
        
        // List contents to see what we have
        if let Ok(entries) = std::fs::read_dir(&pdfium_path) {
            println!("PDFium directory contents:");
            for entry in entries.flatten() {
                println!("  - {}", entry.file_name().to_string_lossy());
            }
        }
    }
    
    println!("PDFium integration test setup completed!");
    println!("Note: Full PDFium functionality requires the PDFium DLL to be available at runtime.");
}