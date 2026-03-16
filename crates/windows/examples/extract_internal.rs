use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

fn main() {
    let input_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\Samples\2023年工作总结顺昌县大历镇人民政府.docx";
    let output_path = "internal_thumb.jpeg";

    let file = File::open(input_path).expect("Failed to open file");
    let mut archive = ZipArchive::new(file).expect("Failed to parse zip");

    // Office 标准缩略图路径通常是 docProps/thumbnail.jpeg 或 docProps/thumbnail.wmf
    let internal_paths = ["docProps/thumbnail.jpeg", "docProps/thumbnail.png", "docProps/thumbnail.wmf"];
    
    let mut found = false;
    for path in internal_paths {
        match archive.by_name(path) {
            Ok(mut zip_file) => {
                let mut buffer = Vec::new();
                zip_file.read_to_end(&mut buffer).expect("Failed to read internal file");
                std::fs::write(output_path, buffer).expect("Failed to write output");
                println!("Successfully extracted internal thumbnail from: {}", path);
                found = true;
                break;
            }
            Err(_) => continue,
        }
    }

    if !found {
        println!("No internal thumbnail found in docProps/");
        println!("This means this file was saved without the 'Save Thumbnail' option enabled in Word.");
    }
}
