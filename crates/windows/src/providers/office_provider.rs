use std::{cell::Cell, io::{Read, Cursor}, fs::{self, OpenOptions}, io::Write, path::Path, process::Command};
use std::os::windows::ffi::OsStrExt;

use windows::{
    core::{implement, IUnknown, Interface, GUID, PCWSTR, PWSTR},
    Data::Pdf::PdfDocument,
    Storage::Streams::InMemoryRandomAccessStream,
    Win32::{
        Foundation::{E_FAIL, BSTR},
        Graphics::Gdi::HBITMAP,
        System::{
            Com::{
                CoInitializeEx, CoCreateInstance, CoUninitialize, CLSCTX_LOCAL_SERVER, 
                COINIT_APARTMENTTHREADED, IDispatch, DISPPARAMS, VARIANT, CLSIDFromProgID,
                EXCEPINFO
            },
            Ole::{DISPATCH_PROPERTYPUT, DISPATCH_PROPERTYPUTREF, DISPATCH_METHOD, DISPATCH_PROPERTYGET, VT_BOOL, VT_I4, VT_BSTR, VT_DISPATCH, VT_EMPTY},
        },
        UI::Shell::{IThumbnailProvider_Impl, WTSAT_ARGB, WTS_ALPHATYPE},
    },
};

use crate::{
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, WinStream},
};

use super::Provider;
use zip::ZipArchive;
use uuid::Uuid;
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};

pub struct OfficeThumbnailProvider {
    pub clsid: GUID,
    pub extension: String,
    pub progids: Vec<String>,
}

impl OfficeThumbnailProvider {
    pub fn new(clsid: GUID, extension: &str) -> Self {
        Self {
            clsid,
            extension: extension.to_lowercase(),
            progids: Vec::new(),
        }
    }

    pub fn with_progids(clsid: GUID, extension: &str, progids: Vec<&str>) -> Self {
        Self {
            clsid,
            extension: extension.to_lowercase(),
            progids: progids.into_iter().map(|s| s.to_owned()).collect(),
        }
    }
}

impl Provider for OfficeThumbnailProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, false);
        
        // Register for extension
        result.push(RegistryKey {
            path: format!(
                "{}\\ShellEx\\{{{:?}}}",
                self.extension,
                windows::Win32::UI::Shell::IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
            )],
        });

        // Register for ProgIDs
        for progid in &self.progids {
            result.push(RegistryKey {
                path: format!(
                    "{}\\ShellEx\\{{{:?}}}",
                    progid,
                    windows::Win32::UI::Shell::IThumbnailProvider::IID
                ),
                values: vec![RegistryValue(
                    "".to_owned(),
                    RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
                )],
            });
        }
        
        result
    }

    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        OfficeThumbnailHandler::new(riid, ppv_object, &self.extension)
    }
}

#[allow(unused_must_use)]
#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile
)]
pub struct OfficeThumbnailHandler {
    stream: Cell<Option<WinStream>>,
    file_path: Cell<Option<std::path::PathBuf>>,
    extension: String,
}

impl OfficeThumbnailHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
        extension: &str,
    ) -> windows::core::Result<()> {
        let riid_val = unsafe { *riid };
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"C:\Users\Public\space_thumbnails_office.log")
            .unwrap_or_else(|_| OpenOptions::new().write(true).open("NUL").unwrap());
        let process_name = std::env::current_exe()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown".to_string());
        writeln!(log_file, "[PID:{}] [{}] OfficeThumbnailHandler::new called for extension {} with IID {:?}", std::process::id(), process_name, extension, riid_val).ok();

        let unknown: IUnknown = OfficeThumbnailHandler {
            stream: Cell::new(None),
            file_path: Cell::new(None),
            extension: extension.to_string(),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream_Impl for OfficeThumbnailHandler {
    fn Initialize(
        &self,
        pstream: &Option<windows::Win32::System::Com::IStream>,
        _: u32,
    ) -> windows::core::Result<()> {
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"C:\Users\Public\space_thumbnails_office.log")
            .unwrap_or_else(|_| OpenOptions::new().write(true).open("NUL").unwrap());
        writeln!(log_file, "[PID:{}] Initialize with stream called", std::process::id()).ok();
        
        if let Some(stream) = pstream {
            self.stream.set(Some(WinStream::from(stream.clone())));
            Ok(())
        } else {
            writeln!(log_file, "[PID:{}] Initialize with stream failed: stream is None", std::process::id()).ok();
            Err(windows::core::Error::from(E_FAIL))
        }
    }
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile_Impl for OfficeThumbnailHandler {
    fn Initialize(
        &self,
        pszfilepath: &windows::core::PCWSTR,
        _: u32,
    ) -> windows::core::Result<()> {
        let mut log_file = OpenOptions::new()
             .create(true)
             .append(true)
             .open(r"C:\Users\Public\space_thumbnails_office.log")
             .unwrap_or_else(|_| OpenOptions::new().write(true).open("NUL").unwrap());
         
         let path_str = if pszfilepath.0.is_null() {
             "unknown".to_owned()
         } else {
             let mut len = 0;
             while unsafe { *pszfilepath.0.add(len) } != 0 {
                 len += 1;
             }
             let slice = unsafe { std::slice::from_raw_parts(pszfilepath.0, len) };
             String::from_utf16_lossy(slice)
         };
         writeln!(log_file, "[PID:{}] Initialize with file called: {}", std::process::id(), path_str).ok();
        
        self.file_path.set(Some(std::path::PathBuf::from(path_str)));
        Ok(())
    }
}

// Constants for COM VARIANT (now imported)
/*
const VT_EMPTY: u16 = 0;
const VT_I4: u16 = 3;
const VT_BSTR: u16 = 8;
const VT_DISPATCH: u16 = 9;
const VT_BOOL: u16 = 11;
const DISPATCH_METHOD: u32 = 1;
const DISPATCH_PROPERTYGET: u32 = 2;
const DISPATCH_PROPERTYPUT: u32 = 4;
*/

impl IThumbnailProvider_Impl for OfficeThumbnailHandler {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"C:\Users\Public\space_thumbnails_office.log")
            .unwrap_or_else(|_| OpenOptions::new().write(true).open("NUL").unwrap());

        writeln!(log_file, "[PID:{}] GetThumbnail called with cx={} for extension {}", std::process::id(), cx, self.extension).ok();

        let content = if let Some(mut win_stream) = self.stream.take() {
            let mut content = Vec::new();
            win_stream.read_to_end(&mut content).map_err(|e| {
                writeln!(log_file, "Failed to read stream: {:?}", e).ok();
                windows::core::Error::from(E_FAIL)
            })?;
            content
        } else if let Some(path) = self.file_path.take() {
            writeln!(log_file, "Reading from file path: {:?}", path).ok();
            fs::read(&path).map_err(|e| {
                writeln!(log_file, "Failed to read file: {:?}", e).ok();
                windows::core::Error::from(E_FAIL)
            })?
        } else {
            writeln!(log_file, "No stream or file path provided").ok();
            return Err(windows::core::Error::from(E_FAIL));
        };

        // 1. Try to extract built-in thumbnail from Zip
        if let Ok(mut archive) = ZipArchive::new(Cursor::new(&content)) {
            let mut thumbnail_data = None;
            
            // Check multiple common thumbnail locations
            // - docProps/thumbnail.png (Office)
            // - QuickLook/Thumbnail.jpg (Apple - newer)
            // - preview.jpg (Apple - older/common)
            // - QuickLook/Preview.png (Apple - alternative)
            let mut thumbnail_index = None;
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let name = file.name().to_lowercase();
                    if name == "docprops/thumbnail.png" || 
                       name == "docprops/thumbnail.jpeg" ||
                       name == "docprops/thumbnail.jpg" ||
                       name == "quicklook/thumbnail.jpg" ||
                       name == "quicklook/thumbnail.png" ||
                       name == "quicklook/preview.jpg" ||
                       name == "quicklook/preview.png" ||
                       name == "preview.jpg" ||
                       name == "preview.png" 
                    {
                        thumbnail_index = Some(i);
                        break;
                    }
                }
            }

            if let Some(idx) = thumbnail_index {
                if let Ok(mut f) = archive.by_index(idx) {
                    let mut data = Vec::new();
                    if f.read_to_end(&mut data).is_ok() {
                        thumbnail_data = Some(data);
                    }
                }
            }

            if let Some(data) = thumbnail_data {
                if let Ok(img) = image::load_from_memory(&data) {
                    writeln!(log_file, "Found built-in thumbnail in zip: index={}", thumbnail_index.unwrap()).ok();
                    return self.return_image(img.to_rgba8(), cx, phbmp, pdwalpha);
                }
            }
        }

        // 2. Try COM Interop conversion
        // We need to write to a temp file because Office COM likes file paths.
        let temp_dir = std::env::temp_dir();
        let run_id = Uuid::new_v4();
        
        let mut is_word = false;
        let mut is_excel = false;
        let mut is_ppt = false;

        // Use the handler's extension as the primary hint
        let ext_hint = self.extension.trim_start_matches('.').to_lowercase();
        if ext_hint.starts_with("doc") { is_word = true; }
        else if ext_hint.starts_with("xls") { is_excel = true; }
        else if ext_hint.starts_with("ppt") { is_ppt = true; }

        // Refine based on content if it's a ZIP (new formats)
        if let Ok(mut archive) = ZipArchive::new(Cursor::new(&content)) {
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let name = file.name();
                    if name.starts_with("word/") { is_word = true; is_excel = false; is_ppt = false; break; }
                    if name.starts_with("xl/") { is_excel = true; is_word = false; is_ppt = false; break; }
                    if name.starts_with("ppt/") { is_ppt = true; is_word = false; is_excel = false; break; }
                }
            }
        }

        let input_ext = if is_word {
            if content.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) { "doc" } else { "docx" }
        } else if is_excel {
            if content.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) { "xls" } else { "xlsx" }
        } else if is_ppt {
            if content.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) { "ppt" } else { "pptx" }
        } else {
            "tmp"
        };
        let input_path = temp_dir.join(format!("input_{}.{}", run_id, input_ext));
        let output_path = temp_dir.join(format!("output_{}.pdf", run_id));

        if fs::write(&input_path, &content).is_ok() {
            writeln!(log_file, "Wrote temp file: {:?}", input_path).ok();
            
            let result = if is_word {
                self.try_word_conversion(&input_path, &output_path)
            } else if is_excel {
                self.try_excel_conversion(&input_path, &output_path)
            } else if is_ppt {
                self.try_ppt_conversion(&input_path, &output_path)
            } else {
                None
            };

            if let Some(pdf_bytes) = result {
                writeln!(log_file, "COM conversion successful, PDF size: {}, rendering PDF", pdf_bytes.len()).ok();
                let _ = fs::remove_file(&input_path);
                let res = self.render_pdf(&pdf_bytes, cx, phbmp, pdwalpha, is_excel);
                match &res {
                    Ok(_) => writeln!(log_file, "render_pdf successful").ok(),
                    Err(e) => writeln!(log_file, "render_pdf failed: {:?}", e).ok(),
                };
                let _ = fs::remove_file(&output_path);
                return res;
            }
            
            // 2.2 Try LibreOffice fallback
            if let Some(pdf_bytes) = self.try_libreoffice_conversion(&input_path) {
                writeln!(log_file, "LibreOffice conversion successful, rendering PDF").ok();
                let _ = fs::remove_file(&input_path);
                let res = self.render_pdf(&pdf_bytes, cx, phbmp, pdwalpha, is_excel);
                return res;
            }

            let _ = fs::remove_file(&input_path);
        }

        // 3. Last Resort: Media Image Fallback
        if let Ok(mut archive) = ZipArchive::new(Cursor::new(&content)) {
            let mut media_image_data = None;
            let mut max_size = 0;
            let extensions = ["jpeg", "jpg", "png", "bmp", "gif"];
            let mut best_media_index = None;

            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let name = file.name();
                    // Check for word/media/, xl/media/, ppt/media/
                    if name.contains("/media/") {
                        let size = file.size();
                        if size > max_size {
                            let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                            if extensions.contains(&ext.as_str()) {
                                max_size = size;
                                best_media_index = Some(i);
                            }
                        }
                    }
                }
            }

            if let Some(idx) = best_media_index {
                if let Ok(mut f) = archive.by_index(idx) {
                    let mut data = Vec::new();
                    if f.read_to_end(&mut data).is_ok() {
                        media_image_data = Some(data);
                    }
                }
            }

            if let Some(data) = media_image_data {
                if let Ok(img) = image::load_from_memory(&data) {
                    writeln!(log_file, "Found media image in zip").ok();
                    return self.return_image(img.to_rgba8(), cx, phbmp, pdwalpha);
                }
            }

            // 4. Text Fallback
            let text_paths = ["word/document.xml", "xl/sharedStrings.xml", "ppt/slides/slide1.xml"];
            for path in text_paths {
                if let Ok(mut file) = archive.by_name(path) {
                    let mut xml_content = String::new();
                    if file.read_to_string(&mut xml_content).is_ok() {
                        let text = self.extract_text_from_xml(&xml_content);
                        if !text.trim().is_empty() {
                            if let Ok(img) = self.render_text(&text, cx, cx) {
                                writeln!(log_file, "Generated text fallback thumbnail").ok();
                                return self.return_image(img, cx, phbmp, pdwalpha);
                            }
                        }
                    }
                }
            }
        }

        writeln!(log_file, "All Office thumbnail generation methods failed").ok();
        Err(windows::core::Error::from(E_FAIL))
    }
}

impl OfficeThumbnailHandler {
    fn return_image(&self, img: image::RgbaImage, cx: u32, phbmp: *mut HBITMAP, pdwalpha: *mut WTS_ALPHATYPE) -> windows::core::Result<()> {
        let (src_w, src_h) = (img.width() as f32, img.height() as f32);
        let scale = (cx as f32 / src_w).min(cx as f32 / src_h);
        
        let target_w = ((src_w * scale) as u32).max(1);
        let target_h = ((src_h * scale) as u32).max(1);

        let resized = image::imageops::resize(&img, target_w, target_h, image::imageops::FilterType::Lanczos3);
        
        unsafe {
            let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
            let hbmp = create_argb_bitmap(target_w, target_h, &mut p_bits);
            
            if hbmp.0 == 0 || p_bits.is_null() {
                 return Err(windows::core::Error::from(E_FAIL));
            }

            let dst_slice = std::slice::from_raw_parts_mut(p_bits as *mut u8, target_w as usize * target_h as usize * 4);

            for (i, pixel) in resized.pixels().enumerate() {
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];
                let a = pixel[3];
                
                let idx = i * 4;
                let a_f = a as f32 / 255.0;
                dst_slice[idx] = (b as f32 * a_f) as u8;
                dst_slice[idx + 1] = (g as f32 * a_f) as u8;
                dst_slice[idx + 2] = (r as f32 * a_f) as u8;
                dst_slice[idx + 3] = a;
            }
            
            phbmp.write(hbmp);
            pdwalpha.write(WTSAT_ARGB);
        }
        Ok(())
    }

    fn render_pdf(&self, pdf_bytes: &[u8], cx: u32, phbmp: *mut HBITMAP, pdwalpha: *mut WTS_ALPHATYPE, is_excel: bool) -> windows::core::Result<()> {
        let mut log_file = OpenOptions::new()
            .append(true)
            .open(r"C:\Users\Public\space_thumbnails_office.log")
            .unwrap_or_else(|_| OpenOptions::new().write(true).open("NUL").unwrap());

        let mem_stream = InMemoryRandomAccessStream::new()?;
        let data_writer = windows::Storage::Streams::DataWriter::CreateDataWriter(&mem_stream)?;
        data_writer.WriteBytes(pdf_bytes)?;
        data_writer.StoreAsync()?.get()?;
        data_writer.FlushAsync()?.get()?;
        mem_stream.Seek(0)?;

        let pdf_doc = PdfDocument::LoadFromStreamAsync(&mem_stream)?.get()?;
        if pdf_doc.PageCount()? == 0 {
            writeln!(log_file, "PDF has no pages").ok();
            return Err(windows::core::Error::from(E_FAIL));
        }

        let page = pdf_doc.GetPage(0)?;
        let src_size = page.Size()?;
        writeln!(log_file, "PDF Page 0 size: {}x{}", src_size.Width, src_size.Height).ok();
        
        let scale = (cx as f32 / src_size.Width).min(cx as f32 / src_size.Height);
        let render_width = ((src_size.Width * scale) as u32).max(1);
        let render_height = ((src_size.Height * scale) as u32).max(1);
        
        let options = windows::Data::Pdf::PdfPageRenderOptions::new()?;
        options.SetDestinationWidth(render_width)?;
        options.SetDestinationHeight(render_height)?;
        options.SetBackgroundColor(windows::UI::Color { A: 255, R: 255, G: 255, B: 255 })?;
        
        let stream = InMemoryRandomAccessStream::new()?;
        page.RenderWithOptionsToStreamAsync(&stream, &options)?.get()?;
        
        let reader = windows::Storage::Streams::DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        let size = stream.Size()? as usize;
        writeln!(log_file, "Rendered PDF to stream, size: {}", size).ok();
        reader.LoadAsync(size as u32)?.get()?;
        let mut buffer = vec![0u8; size];
        reader.ReadBytes(&mut buffer)?;
        
        let mut img = image::load_from_memory(&buffer)
            .map_err(|e| {
                writeln!(log_file, "Failed to load image from memory: {:?}", e).ok();
                windows::core::Error::from(E_FAIL)
            })?
            .to_rgba8();
            
        if is_excel {
            writeln!(log_file, "Applying Excel gamma correction").ok();
            let mut corrected_img = image::RgbaImage::new(img.width(), img.height());
            for (x, y, pixel) in img.enumerate_pixels() {
                let raw_alpha = pixel[3] as f32 / 255.0;
                if raw_alpha > 0.0 {
                    let r_in = pixel[0] as f32;
                    let g_in = pixel[1] as f32;
                    let b_in = pixel[2] as f32;
                    
                    // Gamma Correction for Alpha (Thicken text slightly)
                    let new_alpha = raw_alpha.powf(0.6).min(1.0);
                    
                    // Gamma Correction for RGB (Darken midtones, preserve White/Black)
                    let gamma = 1.5; 
                    
                    let r_norm = r_in / 255.0;
                    let g_norm = g_in / 255.0;
                    let b_norm = b_in / 255.0;
                    
                    let r_dark = r_norm.powf(gamma) * 255.0;
                    let g_dark = g_norm.powf(gamma) * 255.0;
                    let b_dark = b_norm.powf(gamma) * 255.0;
                    
                    // Force dark gray text to pure black
                    let max_c = r_dark.max(g_dark).max(b_dark);
                    let min_c = r_dark.min(g_dark).min(b_dark);
                    let saturation = max_c - min_c;
                    let brightness = (r_dark + g_dark + b_dark) / 3.0;

                    let (r_final, g_final, b_final, alpha_final) = if saturation < 50.0 && brightness < 230.0 {
                        (0.0, 0.0, 0.0, 1.0)
                    } else {
                        (r_dark, g_dark, b_dark, new_alpha)
                    };
                    
                    // Standard composite over white
                    let r_out = (r_final * alpha_final + 255.0 * (1.0 - alpha_final)).min(255.0) as u8;
                    let g_out = (g_final * alpha_final + 255.0 * (1.0 - alpha_final)).min(255.0) as u8;
                    let b_out = (b_final * alpha_final + 255.0 * (1.0 - alpha_final)).min(255.0) as u8;
                    
                    corrected_img.put_pixel(x, y, image::Rgba([r_out, g_out, b_out, 255]));
                } else {
                    corrected_img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
                }
            }
            img = corrected_img;
        }

        writeln!(log_file, "Image loaded, size: {}x{}", img.width(), img.height()).ok();
        self.return_image(img, cx, phbmp, pdwalpha)
    }

    fn try_word_conversion(&self, input_path: &Path, output_path: &Path) -> Option<Vec<u8>> {
        let mut log_file = OpenOptions::new().append(true).open(std::env::temp_dir().join("space_thumbnails_office.log")).unwrap_or_else(|_| OpenOptions::new().write(true).open("NUL").unwrap());
        unsafe {
            let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
            let mut result = None;

            if let Some(app) = self.create_app("Word.Application") {
                writeln!(log_file, "Word app created").ok();
                self.invoke(&app, "Visible", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                self.invoke(&app, "DisplayAlerts", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(0)]); // wdAlertsNone = 0
                self.invoke(&app, "ScreenUpdating", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                self.invoke(&app, "WindowState", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(2)]); // wdWindowStateMinimize = 2

                if let Some(docs) = self.invoke(&app, "Documents", DISPATCH_PROPERTYGET, &mut []) {
                    writeln!(log_file, "Word Documents collection retrieved").ok();
                    if let Some(docs_disp) = self.get_dispatch(&docs) {
                        let mut open_args = [
                            self.variant_str(input_path.to_str().unwrap()), // FileName
                            self.variant_bool(false), // ConfirmConversions = False
                            self.variant_bool(true),  // ReadOnly = True
                            self.variant_bool(false), // AddToRecentFiles = False
                        ];
                        if let Some(doc) = self.invoke(&docs_disp, "Open", DISPATCH_METHOD, &mut open_args) {
                            writeln!(log_file, "Word Document opened: {:?}", input_path).ok();
                            if let Some(doc_disp) = self.get_dispatch(&doc) {
                                // Use ExportAsFixedFormat instead of SaveAs for better background support
                                let mut export_args = [
                                    self.variant_str(output_path.to_str().unwrap()), // OutputFileName
                                    self.variant_i4(17),      // ExportFormat = wdExportFormatPDF
                                    self.variant_bool(false), // OpenAfterExport = False
                                    self.variant_i4(0),       // OptimizeFor = wdExportOptimizeForPrint
                                    self.variant_i4(0),       // Range = wdExportAllDocument
                                    self.variant_i4(0),       // From
                                    self.variant_i4(0),       // To
                                    self.variant_i4(0),       // Item = wdExportDocumentContent
                                    self.variant_bool(false), // IncludeDocProps
                                    self.variant_bool(true),  // KeepIRM
                                    self.variant_i4(0),       // CreateBookmarks = wdExportCreateNoBookmarks
                                    self.variant_bool(false), // DocStructureTags
                                    self.variant_bool(false), // BitmapMissingFonts
                                ];
                                
                                writeln!(log_file, "Exporting Word to PDF: {:?}", output_path).ok();
                                self.invoke(&doc_disp, "ExportAsFixedFormat", DISPATCH_METHOD, &mut export_args);

                                self.invoke(&doc_disp, "Close", DISPATCH_METHOD, &mut [self.variant_i4(0)]); // wdDoNotSaveChanges = 0
                                
                                if output_path.exists() {
                                    writeln!(log_file, "Word PDF save successful").ok();
                                    result = fs::read(output_path).ok();
                                } else {
                                    writeln!(log_file, "Word PDF save failed: output file not found").ok();
                                }
                            }
                        } else {
                            writeln!(log_file, "Word Documents.Open failed").ok();
                        }
                    }
                } else {
                    writeln!(log_file, "Word Documents property failed").ok();
                }
                self.invoke(&app, "Quit", DISPATCH_METHOD, &mut [self.variant_i4(0)]); // wdDoNotSaveChanges = 0
            } else {
                writeln!(log_file, "Word.Application creation failed").ok();
            }
            CoUninitialize();
            result
        }
    }

    fn try_excel_conversion(&self, input_path: &Path, output_path: &Path) -> Option<Vec<u8>> {
        unsafe {
            let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
            let mut result = None;

            if let Some(app) = self.create_app("Excel.Application") {
                // Set these BEFORE opening anything to ensure silence
                self.invoke(&app, "Visible", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                self.invoke(&app, "ScreenUpdating", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                self.invoke(&app, "DisplayAlerts", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                self.invoke(&app, "UserControl", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                self.invoke(&app, "Interactive", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                self.invoke(&app, "WindowState", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(-4137)]); // xlMinimized = -4137

                if let Some(workbooks) = self.invoke(&app, "Workbooks", DISPATCH_PROPERTYGET, &mut []) {
                    if let Some(workbooks_disp) = self.get_dispatch(&workbooks) {
                        // Open(FileName, UpdateLinks, ReadOnly, Format, Password, WriteResPassword, IgnoreReadOnlyRecommended, Origin, Delimiter, Editable, Notify, Converter, AddToMru)
                        let mut open_args = [
                            self.variant_str(input_path.to_str().unwrap()), // FileName
                            self.variant_i4(0),                             // UpdateLinks = 0
                            self.variant_bool(true),                        // ReadOnly = True
                            self.variant_empty(),                           // Format
                            self.variant_empty(),                           // Password
                            self.variant_empty(),                           // WriteResPassword
                            self.variant_empty(),                           // IgnoreReadOnlyRecommended
                            self.variant_empty(),                           // Origin
                            self.variant_empty(),                           // Delimiter
                            self.variant_empty(),                           // Editable
                            self.variant_empty(),                           // Notify
                            self.variant_empty(),                           // Converter
                            self.variant_bool(false),                       // AddToMru = False
                        ];
                        if let Some(wb) = self.invoke(&workbooks_disp, "Open", DISPATCH_METHOD, &mut open_args) {
                            if let Some(wb_disp) = self.get_dispatch(&wb) {
                                // --- FIX: Force Default Style to White Background / Black Text ---
                                // This handles cells using default style (Normal)
                                if let Some(sheet_var) = self.invoke(&wb_disp, "ActiveSheet", DISPATCH_PROPERTYGET, &mut []) {
                                    if let Some(sheet_disp) = self.get_dispatch(&sheet_var) {
                                        // 1. Force Default Style Modification (for cells inheriting Normal style)
                                        let mut range_args = [self.variant_str("A1048576")]; // Last row, first col
                                        if let Some(range_var) = self.invoke(&sheet_disp, "Range", DISPATCH_PROPERTYGET, &mut range_args) {
                                            if let Some(range_disp) = self.get_dispatch(&range_var) {
                                                if let Some(style_var) = self.invoke(&range_disp, "Style", DISPATCH_PROPERTYGET, &mut []) {
                                                    if let Some(style_disp) = self.get_dispatch(&style_var) {
                                                        // Set Interior.Color = White (0xFFFFFF = 16777215)
                                                        if let Some(interior_var) = self.invoke(&style_disp, "Interior", DISPATCH_PROPERTYGET, &mut []) {
                                                            if let Some(interior_disp) = self.get_dispatch(&interior_var) {
                                                                let mut color_args = [self.variant_i4(16777215)];
                                                                self.invoke(&interior_disp, "Color", DISPATCH_PROPERTYPUT, &mut color_args);
                                                                let mut pattern_args = [self.variant_i4(1)];
                                                                self.invoke(&interior_disp, "Pattern", DISPATCH_PROPERTYPUT, &mut pattern_args);
                                                            }
                                                        }
                                                        
                                                        // Set Font.Color = Black (0x000000 = 0)
                                                        if let Some(font_var) = self.invoke(&style_disp, "Font", DISPATCH_PROPERTYGET, &mut []) {
                                                            if let Some(font_disp) = self.get_dispatch(&font_var) {
                                                                let mut color_args = [self.variant_i4(0)];
                                                                self.invoke(&font_disp, "Color", DISPATCH_PROPERTYPUT, &mut color_args);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // 2. Force Replace "Automatic" Color with "Black"
                                        // Also replace White text (which might be set by dark themes) with Black
                                        let mut unprotect_args = [];
                                        self.invoke(&sheet_disp, "Unprotect", DISPATCH_METHOD, &mut unprotect_args);

                                        // --- Pass 1: Automatic -> Black ---
                                        if let Some(find_fmt_var) = self.invoke(&app, "FindFormat", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(find_fmt_disp) = self.get_dispatch(&find_fmt_var) {
                                                 self.invoke(&find_fmt_disp, "Clear", DISPATCH_METHOD, &mut []);
                                                 if let Some(font_var) = self.invoke(&find_fmt_disp, "Font", DISPATCH_PROPERTYGET, &mut []) {
                                                     if let Some(font_disp) = self.get_dispatch(&font_var) {
                                                         // ColorIndex = xlColorIndexAutomatic (-4105)
                                                         let mut idx_args = [self.variant_i4(-4105)];
                                                         self.invoke(&font_disp, "ColorIndex", DISPATCH_PROPERTYPUT, &mut idx_args);
                                                     }
                                                 }
                                            }
                                        }
                                        
                                        if let Some(repl_fmt_var) = self.invoke(&app, "ReplaceFormat", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(repl_fmt_disp) = self.get_dispatch(&repl_fmt_var) {
                                                 self.invoke(&repl_fmt_disp, "Clear", DISPATCH_METHOD, &mut []);
                                                 if let Some(font_var) = self.invoke(&repl_fmt_disp, "Font", DISPATCH_PROPERTYGET, &mut []) {
                                                     if let Some(font_disp) = self.get_dispatch(&font_var) {
                                                         // Color = Black (0)
                                                         let mut color_args = [self.variant_i4(0)];
                                                         self.invoke(&font_disp, "Color", DISPATCH_PROPERTYPUT, &mut color_args);
                                                     }
                                                 }
                                            }
                                        }
                                        
                                        if let Some(used_range_var) = self.invoke(&sheet_disp, "UsedRange", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(used_range_disp) = self.get_dispatch(&used_range_var) {
                                                let mut replace_args = [
                                                    self.variant_bool(true), // ReplaceFormat
                                                    self.variant_bool(true), // SearchFormat
                                                    self.variant_bool(false), // MatchByte
                                                    self.variant_bool(false), // MatchCase
                                                    self.variant_i4(1),      // SearchOrder
                                                    self.variant_i4(2),      // LookAt
                                                    self.variant_str(""),    // Replacement
                                                    self.variant_str(""),    // What
                                                ];
                                                self.invoke(&used_range_disp, "Replace", DISPATCH_METHOD, &mut replace_args);
                                            }
                                        }

                                        // --- Pass 2: White ColorIndex (2) -> Black ---
                                        if let Some(find_fmt_var) = self.invoke(&app, "FindFormat", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(find_fmt_disp) = self.get_dispatch(&find_fmt_var) {
                                                 self.invoke(&find_fmt_disp, "Clear", DISPATCH_METHOD, &mut []);
                                                 if let Some(font_var) = self.invoke(&find_fmt_disp, "Font", DISPATCH_PROPERTYGET, &mut []) {
                                                     if let Some(font_disp) = self.get_dispatch(&font_var) {
                                                         // ColorIndex = 2 (White)
                                                         let mut idx_args = [self.variant_i4(2)];
                                                         self.invoke(&font_disp, "ColorIndex", DISPATCH_PROPERTYPUT, &mut idx_args);
                                                     }
                                                 }
                                            }
                                        }
                                        
                                        if let Some(used_range_var) = self.invoke(&sheet_disp, "UsedRange", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(used_range_disp) = self.get_dispatch(&used_range_var) {
                                                let mut replace_args = [
                                                    self.variant_bool(true), self.variant_bool(true), self.variant_bool(false), self.variant_bool(false),
                                                    self.variant_i4(1), self.variant_i4(2), self.variant_str(""), self.variant_str(""),
                                                ];
                                                self.invoke(&used_range_disp, "Replace", DISPATCH_METHOD, &mut replace_args);
                                            }
                                        }

                                        // --- Pass 3: ThemeColor 1 (xlThemeColorDark1) -> Black ---
                                        if let Some(find_fmt_var) = self.invoke(&app, "FindFormat", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(find_fmt_disp) = self.get_dispatch(&find_fmt_var) {
                                                self.invoke(&find_fmt_disp, "Clear", DISPATCH_METHOD, &mut []);
                                                if let Some(font_var) = self.invoke(&find_fmt_disp, "Font", DISPATCH_PROPERTYGET, &mut []) {
                                                    if let Some(font_disp) = self.get_dispatch(&font_var) {
                                                        // ThemeColor = 1
                                                        let mut theme_args = [self.variant_i4(1)];
                                                        self.invoke(&font_disp, "ThemeColor", DISPATCH_PROPERTYPUT, &mut theme_args);
                                                    }
                                                }
                                            }
                                        }

                                        if let Some(used_range_var) = self.invoke(&sheet_disp, "UsedRange", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(used_range_disp) = self.get_dispatch(&used_range_var) {
                                                let mut replace_args = [
                                                    self.variant_bool(true), self.variant_bool(true), self.variant_bool(false), self.variant_bool(false),
                                                    self.variant_i4(1), self.variant_i4(2), self.variant_str(""), self.variant_str(""),
                                                ];
                                                self.invoke(&used_range_disp, "Replace", DISPATCH_METHOD, &mut replace_args);
                                            }
                                        }

                                        // --- Pass 4: ThemeColor 2 (xlThemeColorLight1) -> Black ---
                                        if let Some(find_fmt_var) = self.invoke(&app, "FindFormat", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(find_fmt_disp) = self.get_dispatch(&find_fmt_var) {
                                                self.invoke(&find_fmt_disp, "Clear", DISPATCH_METHOD, &mut []);
                                                if let Some(font_var) = self.invoke(&find_fmt_disp, "Font", DISPATCH_PROPERTYGET, &mut []) {
                                                    if let Some(font_disp) = self.get_dispatch(&font_var) {
                                                        // ThemeColor = 2
                                                        let mut theme_args = [self.variant_i4(2)];
                                                        self.invoke(&font_disp, "ThemeColor", DISPATCH_PROPERTYPUT, &mut theme_args);
                                                    }
                                                }
                                            }
                                        }

                                        if let Some(used_range_var) = self.invoke(&sheet_disp, "UsedRange", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(used_range_disp) = self.get_dispatch(&used_range_var) {
                                                let mut replace_args = [
                                                    self.variant_bool(true), self.variant_bool(true), self.variant_bool(false), self.variant_bool(false),
                                                    self.variant_i4(1), self.variant_i4(2), self.variant_str(""), self.variant_str(""),
                                                ];
                                                self.invoke(&used_range_disp, "Replace", DISPATCH_METHOD, &mut replace_args);
                                            }
                                        }

                                        // 3. PageSetup adjustments
                                        if let Some(setup_var) = self.invoke(&sheet_disp, "PageSetup", DISPATCH_PROPERTYGET, &mut []) {
                                            if let Some(setup_disp) = self.get_dispatch(&setup_var) {
                                                self.invoke(&setup_disp, "BlackAndWhite", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                                                self.invoke(&setup_disp, "Zoom", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                                                self.invoke(&setup_disp, "FitToPagesWide", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(1)]);
                                                self.invoke(&setup_disp, "FitToPagesTall", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(1)]);
                                                self.invoke(&setup_disp, "Orientation", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(2)]); // xlLandscape = 2
                                                self.invoke(&setup_disp, "Draft", DISPATCH_PROPERTYPUT, &mut [self.variant_bool(false)]);
                                            }
                                        }
                                    }
                                }

                                // ExportAsFixedFormat(Type, Filename, Quality, IncludeDocProperties, IgnorePrintAreas, From, To, OpenAfterPublish)
                                // xlTypePDF = 0
                                let mut export_args = [
                                    self.variant_i4(0),                              // Type = xlTypePDF
                                    self.variant_str(output_path.to_str().unwrap()), // Filename
                                    self.variant_i4(0),                              // Quality = xlQualityStandard
                                    self.variant_bool(false),                       // IncludeDocProperties
                                    self.variant_bool(false),                       // IgnorePrintAreas
                                    self.variant_i4(1),                              // From (page 1)
                                    self.variant_i4(1),                              // To (page 1)
                                    self.variant_bool(false),                        // OpenAfterPublish = False
                                ];
                                self.invoke(&wb_disp, "ExportAsFixedFormat", DISPATCH_METHOD, &mut export_args);

                                self.invoke(&wb_disp, "Close", DISPATCH_METHOD, &mut [self.variant_bool(false)]);
                                
                                if output_path.exists() {
                                    result = fs::read(output_path).ok();
                                }
                            }
                        }
                    }
                }
                self.invoke(&app, "Quit", DISPATCH_METHOD, &mut []);
            }
            CoUninitialize();
            result
        }
    }

    fn try_ppt_conversion(&self, input_path: &Path, output_path: &Path) -> Option<Vec<u8>> {
        unsafe {
            let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
            let mut result = None;

            if let Some(app) = self.create_app("PowerPoint.Application") {
                self.invoke(&app, "Visible", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(0)]); // msoFalse = 0
                self.invoke(&app, "DisplayAlerts", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(1)]); // ppAlertsNone = 1
                self.invoke(&app, "WindowState", DISPATCH_PROPERTYPUT, &mut [self.variant_i4(2)]); // ppWindowMinimized = 2
                if let Some(presentations) = self.invoke(&app, "Presentations", DISPATCH_PROPERTYGET, &mut []) {
                    if let Some(presentations_disp) = self.get_dispatch(&presentations) {
                        let mut open_args = [
                            self.variant_str(input_path.to_str().unwrap()), // FileName
                            self.variant_bool(true),  // ReadOnly = True
                            self.variant_bool(false), // Untitled = False
                            self.variant_bool(false), // WithWindow = False
                        ];
                        if let Some(pres) = self.invoke(&presentations_disp, "Open", DISPATCH_METHOD, &mut open_args) {
                            if let Some(pres_disp) = self.get_dispatch(&pres) {
                                // SaveAs(FileName, FileFormat, EmbedTrueTypeFonts)
                                let mut save_args = [
                                    self.variant_str(output_path.to_str().unwrap()), // FileName
                                    self.variant_i4(32), // FileFormat = ppSaveAsPDF
                                ];
                                self.invoke(&pres_disp, "SaveAs", DISPATCH_METHOD, &mut save_args);

                                self.invoke(&pres_disp, "Close", DISPATCH_METHOD, &mut []);
                                
                                if output_path.exists() {
                                    result = fs::read(output_path).ok();
                                }
                            }
                        }
                    }
                }
                self.invoke(&app, "Quit", DISPATCH_METHOD, &mut []);
            }
            CoUninitialize();
            result
        }
    }

    fn try_libreoffice_conversion(&self, input_path: &Path) -> Option<Vec<u8>> {
        let soffice_cmd = if Command::new("soffice").arg("--version").output().is_ok() {
            "soffice".to_string()
        } else if Path::new("C:\\Program Files\\LibreOffice\\program\\soffice.exe").exists() {
            "C:\\Program Files\\LibreOffice\\program\\soffice.exe".to_string()
        } else if Path::new("C:\\Program Files (x86)\\LibreOffice\\program\\soffice.exe").exists() {
            "C:\\Program Files (x86)\\LibreOffice\\program\\soffice.exe".to_string()
        } else {
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

        let temp_dir = input_path.parent().unwrap_or(Path::new("."));
        
        let output = Command::new(soffice_cmd)
            .arg("--headless")
            .arg("--convert-to")
            .arg("pdf")
            .arg("--outdir")
            .arg(temp_dir)
            .arg(input_path)
            .output();

        let actual_output_path = temp_dir.join(input_path.file_stem().unwrap()).with_extension("pdf");
        if output.is_ok() && actual_output_path.exists() {
            let bytes = fs::read(&actual_output_path).ok();
            let _ = fs::remove_file(&actual_output_path);
            return bytes;
        }
        None
    }

    fn extract_text_from_xml(&self, xml: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        for c in xml.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
                text.push(' ');
            } else if !in_tag {
                text.push(c);
            }
        }
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
        clean_text.trim().to_string()
    }

    fn render_text(&self, text: &str, width: u32, height: u32) -> Result<RgbaImage, String> {
        let mut image = RgbaImage::new(width, height);
        let margin_x = (width as f32 * 0.1) as i32;
        let margin_y = (height as f32 * 0.1) as i32;
        let content_width = width as i32 - 2 * margin_x;
        let content_height = height as i32 - 2 * margin_y;

        let font_paths = [
            "C:\\Windows\\Fonts\\consola.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
        ];

        let mut font_data = Vec::new();
        for path in font_paths {
            if let Ok(data) = fs::read(path) {
                font_data = data;
                break;
            }
        }

        if font_data.is_empty() {
            return Ok(image);
        }

        let font = Font::try_from_vec(font_data).ok_or("Error constructing font")?;
        let scale = Scale::uniform(14.0);
        let text_color = Rgba([50, 50, 50, 255]);
        let line_height = 16;
        let max_lines = (content_height / line_height) - 1;
        let max_chars_per_line = (content_width / 8) as usize;

        let mut y = margin_y;
        let x = margin_x;

        for (i, line) in text.lines().enumerate() {
            if i as i32 >= max_lines {
                break;
            }
            let truncated_line = if line.chars().count() > max_chars_per_line {
                line.chars().take(max_chars_per_line).collect::<String>()
            } else {
                line.to_string()
            };

            draw_text_mut(&mut image, text_color, x, y, scale, &font, &truncated_line);
            y += line_height;
        }

        Ok(image)
    }

    unsafe fn create_app(&self, prog_id: &str) -> Option<IDispatch> {
        let prog_id_wide: Vec<u16> = std::ffi::OsStr::new(prog_id).encode_wide().chain(std::iter::once(0)).collect();
        if let Ok(clsid) = CLSIDFromProgID(PCWSTR(prog_id_wide.as_ptr())) {
            return CoCreateInstance::<_, IDispatch>(&clsid, None, CLSCTX_LOCAL_SERVER).ok();
        }
        None
    }

    unsafe fn invoke(&self, dispatch: &IDispatch, name: &str, flags: u32, args: &mut [VARIANT]) -> Option<VARIANT> {
        let mut log_file = OpenOptions::new().append(true).open(std::env::temp_dir().join("space_thumbnails_office.log")).unwrap_or_else(|_| OpenOptions::new().write(true).open("NUL").unwrap());
        let mut dispid = [0i32; 1];
        let mut name_wide: Vec<u16> = std::ffi::OsStr::new(name).encode_wide().chain(std::iter::once(0)).collect();
        let name_ptr = PWSTR(name_wide.as_mut_ptr());
        let names = [name_ptr];

        if dispatch.GetIDsOfNames(&GUID::zeroed(), &names, 0, &mut dispid).is_err() {
            return None;
        }

        // Reverse args for Invoke - COM expectation
        let mut reversed_args = args.to_vec();
        reversed_args.reverse();

        let mut dp = DISPPARAMS {
            rgvarg: if reversed_args.is_empty() { std::ptr::null_mut() } else { reversed_args.as_mut_ptr() },
            rgdispidNamedArgs: std::ptr::null_mut(),
            cArgs: reversed_args.len() as u32,
            cNamedArgs: 0,
        };

        let mut put_dispid = -3i32; // DISPID_PROPERTYPUT
        if flags & DISPATCH_PROPERTYPUT != 0 || flags & DISPATCH_PROPERTYPUTREF != 0 {
            dp.rgdispidNamedArgs = &mut put_dispid;
            dp.cNamedArgs = 1;
        }

        let mut result = VARIANT::default();
        let mut excep_info = EXCEPINFO::default();
        let mut arg_err = 0;
        let res = dispatch.Invoke(
            dispid[0],
            &GUID::zeroed(),
            0,
            flags as u16,
            &mut dp,
            &mut result,
            &mut excep_info,
            &mut arg_err,
        );

        if res.is_ok() {
            Some(result)
        } else {
            writeln!(log_file, "Invoke failed for {}: {:?}, arg_err: {}, code: 0x{:X}", name, res, arg_err, excep_info.wCode).ok();
            None
        }
    }

    unsafe fn get_dispatch(&self, variant: &VARIANT) -> Option<IDispatch> {
        if variant.Anonymous.Anonymous.vt == VT_DISPATCH.0 as u16 {
            let pdisp = &variant.Anonymous.Anonymous.Anonymous.pdispVal;
            return (**pdisp).clone();
        }
        None
    }

    unsafe fn variant_bool(&self, val: bool) -> VARIANT {
        let mut v = VARIANT::default();
        let inner = &mut v.Anonymous.Anonymous;
        inner.vt = VT_BOOL.0 as u16;
        inner.Anonymous.boolVal = if val { -1 } else { 0 };
        v
    }

    unsafe fn variant_empty(&self) -> VARIANT {
        let mut v = VARIANT::default();
        let inner = &mut v.Anonymous.Anonymous;
        inner.vt = VT_EMPTY.0 as u16;
        v
    }

    unsafe fn variant_i4(&self, val: i32) -> VARIANT {
        let mut v = VARIANT::default();
        let inner = &mut v.Anonymous.Anonymous;
        inner.vt = VT_I4.0 as u16;
        inner.Anonymous.lVal = val;
        v
    }

    unsafe fn variant_str(&self, val: &str) -> VARIANT {
        let mut v = VARIANT::default();
        let inner = &mut v.Anonymous.Anonymous;
        inner.vt = VT_BSTR.0 as u16;
        let wide: Vec<u16> = std::ffi::OsStr::new(val).encode_wide().chain(std::iter::once(0)).collect();
        inner.Anonymous.bstrVal = std::mem::ManuallyDrop::new(BSTR::from_wide(&wide));
        v
    }
}
