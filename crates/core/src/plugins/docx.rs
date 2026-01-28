use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Read, Seek, Cursor};
use std::process::Command;
use zip::ZipArchive;
use image::imageops::FilterType;
use super::{ThumbnailGenerator, TextGenerator};
use super::pdf::PdfGenerator;
use uuid::Uuid;

// Helper trait for Box<dyn Read + Seek>
trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub struct DocxGenerator;

impl DocxGenerator {
    fn extract_text_from_xml(xml: &str) -> String {
        // Very basic XML text extraction: remove tags
        // In a real implementation, we might want to use quick-xml or similar
        // But for now, regex or manual parsing is enough for a thumbnail
        let mut text = String::new();
        let mut in_tag = false;
        for c in xml.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
                text.push(' '); // Add space to separate text nodes
            } else if !in_tag {
                text.push(c);
            }
        }
        // Collapse multiple spaces
        let mut clean_text = String::new();
        let mut last_space = false;
        for c in text.chars() {
            if c.is_whitespace() {
                if !last_space {
                    clean_text.push(' ');
                    last_space = true;
                }
            } else {
                clean_text.push(c);
                last_space = false;
            }
        }
        clean_text
    }

    fn try_office_conversion(buffer: Option<&[u8]>, filepath: Option<&Path>, width: u32, height: u32) -> Option<Vec<u8>> {
        #[cfg(windows)]
        {
            use windows::Win32::System::Com::{
                CoInitializeEx, CoCreateInstance, CoUninitialize, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
                IDispatch, DISPPARAMS, VARIANT, CLSIDFromProgID
            };
            use windows::Win32::Foundation::BSTR;
            use windows::core::{GUID, PCWSTR, PWSTR};
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            // Constants not found in 0.34 modules easily
            const VT_EMPTY: u16 = 0;
            const VT_I4: u16 = 3;
            const VT_BSTR: u16 = 8;
            const VT_DISPATCH: u16 = 9;
            const VT_BOOL: u16 = 11;
            
            const DISPATCH_METHOD: u32 = 1;
            const DISPATCH_PROPERTYGET: u32 = 2;
            const DISPATCH_PROPERTYPUT: u32 = 4;

            // 1. Prepare Input File
            let temp_dir = std::env::temp_dir();
            let run_id = Uuid::new_v4();
            
            let input_path = if let Some(path) = filepath {
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    std::env::current_dir().unwrap_or(PathBuf::from(".")).join(path)
                }
            } else if let Some(buf) = buffer {
                let path = temp_dir.join(format!("input_{}.docx", run_id));
                if fs::write(&path, buf).is_err() {
                    return None;
                }
                path
            } else {
                return None;
            };

            // 2. Prepare Output File
            let output_path = temp_dir.join(format!("output_{}.pdf", run_id));

            // Helper to invoke IDispatch
            unsafe fn invoke(dispatch: &IDispatch, name: &str, flags: u32, args: &mut [VARIANT]) -> Option<VARIANT> {
                let mut dispid = [0i32; 1];
                let mut name_wide: Vec<u16> = OsStr::new(name).encode_wide().chain(std::iter::once(0)).collect();
                let name_ptr = PWSTR(name_wide.as_mut_ptr());
                let names = [name_ptr];
                
                // GetIDsOfNames in windows 0.34 with const generics:
                // fn GetIDsOfNames<const PARAM2: usize>(&self, riid, rgsznames, lcid, rgdispid)
                // cnames is inferred from array size.
                if dispatch.GetIDsOfNames(&GUID::zeroed(), &names, 0, &mut dispid).is_err() {
                    return None;
                }
                
                let target_dispid = dispid[0];

                let mut dp = DISPPARAMS {
                    rgvarg: if args.is_empty() { std::ptr::null_mut() } else { args.as_mut_ptr() },
                    rgdispidNamedArgs: std::ptr::null_mut(),
                    cArgs: args.len() as u32,
                    cNamedArgs: 0,
                };

                // For property put, we need named args
                let mut put_dispid = -3i32; // DISPID_PROPERTYPUT
                if flags == DISPATCH_PROPERTYPUT {
                    dp.rgdispidNamedArgs = &mut put_dispid;
                    dp.cNamedArgs = 1;
                }

                let mut result = VARIANT::default();
                
                // Invoke(dispid, riid, lcid, wflags, pdispparams, pvarresult, pexcepinfo, puargerr)
                let hr = dispatch.Invoke(target_dispid, &GUID::zeroed(), 0, flags as u16, &mut dp, &mut result, std::ptr::null_mut(), std::ptr::null_mut());
                if hr.is_ok() {
                    Some(result)
                } else {
                    if let Err(e) = hr {
                         println!("Invoke failed for {}: HRESULT(0x{:X})", name, e.code().0);
                    }
                    None
                }
            }

            // Helper to create VARIANT
            unsafe fn variant_bool(val: bool) -> VARIANT {
                let mut v = VARIANT::default();
                let inner = &mut v.Anonymous.Anonymous;
                inner.vt = VT_BOOL;
                inner.Anonymous.boolVal = if val { -1 } else { 0 }; // VARIANT_TRUE is -1
                v
            }

            unsafe fn variant_i4(val: i32) -> VARIANT {
                let mut v = VARIANT::default();
                let inner = &mut v.Anonymous.Anonymous;
                inner.vt = VT_I4;
                inner.Anonymous.lVal = val;
                v
            }

            unsafe fn variant_str(val: &str) -> VARIANT {
                let mut v = VARIANT::default();
                let inner = &mut v.Anonymous.Anonymous;
                inner.vt = VT_BSTR;
                inner.Anonymous.bstrVal = std::mem::ManuallyDrop::new(BSTR::from(val)); 
                v
            }

            let mut result_bytes = None;

            unsafe {
                // Initialize COM
                // Use COINIT_APARTMENTTHREADED for Office automation
                let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);

                let prog_id: Vec<u16> = OsStr::new("Word.Application").encode_wide().chain(std::iter::once(0)).collect();
                
                if let Ok(clsid) = CLSIDFromProgID(PCWSTR(prog_id.as_ptr())) {
                     // Try to create instance
                     if let Ok(app) = CoCreateInstance::<_, IDispatch>(&clsid, None, CLSCTX_LOCAL_SERVER) {
                         // app.Visible = False
                         let mut args = [variant_bool(false)];
                         invoke(&app, "Visible", DISPATCH_PROPERTYPUT, &mut args);

                         // Get Documents collection
                         if let Some(docs_var) = invoke(&app, "Documents", DISPATCH_METHOD | DISPATCH_PROPERTYGET, &mut []) {
                             if docs_var.Anonymous.Anonymous.vt == VT_DISPATCH {
                                 // Access ManuallyDrop field
                                 let pdisp = &docs_var.Anonymous.Anonymous.Anonymous.pdispVal;
                                 if let Some(docs_disp) = &**pdisp {
                                    // docs.Open(FileName)
                                    // Pass only FileName to rely on defaults and avoid parameter count mismatch
                                    let mut open_args = [
                                        variant_str(input_path.to_str().unwrap()), // FileName
                                    ];
                                    
                                    if let Some(doc_var) = invoke(docs_disp, "Open", DISPATCH_METHOD, &mut open_args) {
                                        let doc_pdisp = &doc_var.Anonymous.Anonymous.Anonymous.pdispVal;
                                        if let Some(doc_disp) = &**doc_pdisp {
                                            // doc.ExportAsFixedFormat
                                            let mut export_args = [
                                                variant_i4(17), // wdExportFormatPDF
                                                variant_str(output_path.to_str().unwrap()),
                                            ];
                                            
                                            invoke(doc_disp, "ExportAsFixedFormat", DISPATCH_METHOD, &mut export_args);
                                            
                                            // doc.Close
                                            let mut close_args = [variant_i4(0)];
                                            invoke(doc_disp, "Close", DISPATCH_METHOD, &mut close_args);
                                        }
                                    }
                                 }
                             }
                         }
                         
                         // app.Quit
                         invoke(&app, "Quit", DISPATCH_METHOD, &mut []);
                     }
                }
                
                CoUninitialize();
            }

            // 5. Cleanup Script (Not needed anymore)
            
            // Cleanup Input if temporary
            if buffer.is_some() {
                let _ = fs::remove_file(&input_path);
            }

            // 6. Render PDF
            if output_path.exists() {
                 if let Ok(pdf_bytes) = fs::read(&output_path) {
                     if let Ok(rendered) = PdfGenerator::render_pdf(Some(&pdf_bytes), None, width, height) {
                         result_bytes = Some(rendered);
                     }
                 }
                 let _ = fs::remove_file(&output_path);
            }

            result_bytes
        }

        #[cfg(not(windows))]
        {
            None
        }
    }

    fn try_libreoffice_conversion(buffer: Option<&[u8]>, filepath: Option<&Path>, width: u32, height: u32) -> Option<Vec<u8>> {
        // Check for soffice in PATH or standard locations
        let soffice_cmd = if Command::new("soffice").arg("--version").output().is_ok() {
            "soffice".to_string()
        } else if Path::new("C:\\Program Files\\LibreOffice\\program\\soffice.exe").exists() {
            "C:\\Program Files\\LibreOffice\\program\\soffice.exe".to_string()
        } else if Path::new("C:\\Program Files (x86)\\LibreOffice\\program\\soffice.exe").exists() {
            "C:\\Program Files (x86)\\LibreOffice\\program\\soffice.exe".to_string()
        } else {
            // Check for portable version in APPDATA
            let mut found = None;
            if let Ok(appdata) = std::env::var("APPDATA") {
                let portable_path = Path::new(&appdata)
                    .join("SpaceThumbnails")
                    .join("deps")
                    .join("LibreOfficePortable")
                    .join("App")
                    .join("libreoffice")
                    .join("program")
                    .join("soffice.exe");
                if portable_path.exists() {
                    found = Some(portable_path.to_string_lossy().to_string());
                }
            }
            
            if let Some(p) = found {
                p
            } else {
                return None;
            }
        };

        let temp_dir = std::env::temp_dir();
        let run_id = Uuid::new_v4();

        let input_path = if let Some(path) = filepath {
            path.to_path_buf()
        } else if let Some(buf) = buffer {
            let path = temp_dir.join(format!("input_{}.docx", run_id));
            if fs::write(&path, buf).is_err() {
                return None;
            }
            path
        } else {
            return None;
        };

        // LibreOffice --convert-to pdf places output in --outdir with same basename
        // soffice --headless --convert-to pdf --outdir <tmp> <input>
        let output = Command::new(soffice_cmd)
            .arg("--headless")
            .arg("--convert-to")
            .arg("pdf")
            .arg("--outdir")
            .arg(&temp_dir)
            .arg(&input_path)
            .output();
        
        // Cleanup Input if temporary
        if buffer.is_some() {
            let _ = fs::remove_file(&input_path);
        }

        let mut result = None;
        
        if let Ok(output) = output {
             if output.status.success() {
                 // Expected output filename: input filename with .pdf extension
                 let file_stem = input_path.file_stem().unwrap().to_string_lossy();
                 let output_path = temp_dir.join(format!("{}.pdf", file_stem));
                 
                 if output_path.exists() {
                     if let Ok(pdf_bytes) = fs::read(&output_path) {
                         if let Ok(rendered) = PdfGenerator::render_pdf(Some(&pdf_bytes), None, width, height) {
                             result = Some(rendered);
                         }
                     }
                     let _ = fs::remove_file(&output_path);
                 }
             }
        }

        result
    }
}

impl ThumbnailGenerator for DocxGenerator {
    fn name(&self) -> &str {
        "Word (Docx) Extractor"
    }

    fn validate(&self, _header: &[u8], extension: &str) -> bool {
        extension.eq_ignore_ascii_case("docx")
    }

    fn generate(&self, buffer: Option<&[u8]>, width: u32, height: u32, _extension: &str, filepath: Option<&Path>) -> Result<Vec<u8>, String> {
        // 1. Try to find embedded thumbnail (Fastest)
        let mut file_buf;
        let reader: Box<dyn ReadSeek> = if let Some(path) = filepath {
            if let Ok(file) = File::open(path) {
                Box::new(file)
            } else {
                 // If file open fails (e.g. strict lock), maybe we can rely on buffer?
                 if let Some(buf) = buffer {
                     file_buf = Cursor::new(buf);
                     Box::new(file_buf)
                 } else {
                     return Err("Failed to open file and no buffer provided".to_string());
                 }
            }
        } else if let Some(buf) = buffer {
            file_buf = Cursor::new(buf);
            Box::new(file_buf)
        } else {
            return Err("No buffer or filepath provided for Docx".to_string());
        };

        let archive_res = ZipArchive::new(reader);
        
        // If zip open succeeds, try embedded/media
        if let Ok(mut archive) = archive_res {
            // 1.1 Embedded Thumbnail
            let mut thumbnail_data = None;
            let extensions = ["jpeg", "jpg", "png", "bmp", "gif"];
            let mut thumbnail_name = None;
            
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let name = file.name();
                    if name.starts_with("docProps/thumbnail.") {
                        let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                        if extensions.contains(&ext.as_str()) {
                            thumbnail_name = Some(name.to_string());
                            break;
                        }
                    }
                }
            }

            if let Some(name) = thumbnail_name {
                if let Ok(mut file) = archive.by_name(&name) {
                    let mut data = Vec::new();
                    if file.read_to_end(&mut data).is_ok() {
                        thumbnail_data = Some(data);
                    }
                }
            }

            if let Some(data) = thumbnail_data {
                if let Ok(img) = image::load_from_memory(&data) {
                    let scaled = img.resize(width, height, FilterType::Triangle);
                    return Ok(scaled.to_rgba8().into_raw());
                }
            }
        }

        // 2. High Quality Conversion Fallbacks (Slower but better than random images)
        
        // 2.1 Microsoft Office Interop
        // Only try if we have a file path or buffer to write to disk
        if let Some(rendered) = Self::try_office_conversion(buffer, filepath, width, height) {
            return Ok(rendered);
        }
        // println!("Office conversion failed");

        // 2.2 LibreOffice
        // println!("Attempting LibreOffice conversion...");
        if let Some(rendered) = Self::try_libreoffice_conversion(buffer, filepath, width, height) {
            // println!("LibreOffice conversion successful");
            return Ok(rendered);
        }
        // println!("LibreOffice conversion failed");

        // 3. Last Resort: Media Image or Text Extraction (from original zip archive)
        // Re-open archive because previous ownership was consumed or complicated
        
        let file_buf_2;
        let reader_2: Box<dyn ReadSeek> = if let Some(path) = filepath {
             if let Ok(file) = File::open(path) {
                Box::new(file)
            } else {
                 if let Some(buf) = buffer {
                     file_buf_2 = Cursor::new(buf);
                     Box::new(file_buf_2)
                 } else {
                     return Err("Failed to re-open for fallback".to_string());
                 }
            }
        } else if let Some(buf) = buffer {
            file_buf_2 = Cursor::new(buf);
            Box::new(file_buf_2)
        } else {
            return Err("Failed to re-open".to_string());
        };

        if let Ok(mut archive) = ZipArchive::new(reader_2) {
            // 3.1 Media Image Fallback
            let mut media_image_data = None;
            let mut max_size = 0;
            let extensions = ["jpeg", "jpg", "png", "bmp", "gif"];
            let mut best_media_name = None;

            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let name = file.name();
                    if name.starts_with("word/media/") {
                        let size = file.size();
                        if size > max_size {
                            let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                            if extensions.contains(&ext.as_str()) {
                                max_size = size;
                                best_media_name = Some(name.to_string());
                            }
                        }
                    }
                }
            }

            if let Some(name) = best_media_name {
                if let Ok(mut file) = archive.by_name(&name) {
                    let mut data = Vec::new();
                    if file.read_to_end(&mut data).is_ok() {
                        media_image_data = Some(data);
                    }
                }
            }

            if let Some(data) = media_image_data {
                 if let Ok(img) = image::load_from_memory(&data) {
                    let scaled = img.resize(width, height, FilterType::Triangle);
                    return Ok(scaled.to_rgba8().into_raw());
                 }
            }

            // 3.2 Text Fallback
            if let Ok(mut file) = archive.by_name("word/document.xml") {
                let mut xml_content = String::new();
                if file.read_to_string(&mut xml_content).is_ok() {
                    let text = Self::extract_text_from_xml(&xml_content);
                    return TextGenerator::render_text(&text, width, height);
                }
            }
        }

        Err("All thumbnail generation methods failed".to_string())
    }
}
