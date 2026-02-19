use epub::doc::EpubDoc;
use image::GenericImageView;
use std::path::Path;

fn main() {
    let path_str = r"F:\毛泽东 真实的故事.epub";
    let path = Path::new(path_str);

    println!("Testing EPUB: {:?}", path);

    if !path.exists() {
        println!("Error: File not found!");
        // return; // Let it try anyway in case path is tricky
    }

    // Simulate what the provider does: Open file
    let mut doc = match EpubDoc::new(path) {
        Ok(d) => {
            println!("Success: EpubDoc::new()");
            d
        },
        Err(e) => {
            println!("Error: EpubDoc::new() failed: {:?}", e);
            return;
        }
    };

    // Extract cover
    let cover_data = match doc.get_cover() {
        Ok(data) => {
            println!("Success: doc.get_cover() - {} bytes", data.len());
            data
        },
        Err(e) => {
            println!("Error: doc.get_cover() failed: {:?}", e);
            // List resources to debug
            println!("Available resources:");
            for id in doc.resources.keys() {
                println!(" - {}", id);
            }
            return;
        }
    };

    // Load image
    let img = match image::load_from_memory(&cover_data) {
        Ok(i) => {
            println!("Success: image::load_from_memory() - {}x{}", i.width(), i.height());
            i
        },
        Err(e) => {
            println!("Error: image::load_from_memory() failed: {:?}", e);
            return;
        }
    };

    println!("All simulation steps passed!");
}
