use std::{cell::Cell, io::Read, fs::OpenOptions, io::Write};

use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Data::Pdf::PdfDocument,
    Storage::Streams::InMemoryRandomAccessStream,
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::HBITMAP,
        UI::Shell::{IThumbnailProvider_Impl, WTSAT_ARGB, WTS_ALPHATYPE},
    },
};

use crate::{
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, WinStream},
};

use super::Provider;

pub struct PdfThumbnailProvider {
    pub clsid: GUID,
}

impl PdfThumbnailProvider {
    pub fn new(clsid: GUID) -> Self {
        Self { clsid }
    }
}

impl Provider for PdfThumbnailProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, false);
        result.push(RegistryKey {
            path: format!(
                ".pdf\\ShellEx\\{{{:?}}}",
                windows::Win32::UI::Shell::IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
            )],
        });
        result
    }

    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        PdfThumbnailHandler::new(riid, ppv_object)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream
)]
pub struct PdfThumbnailHandler {
    stream: Cell<Option<WinStream>>,
}

impl PdfThumbnailHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = PdfThumbnailHandler {
            stream: Cell::new(None),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream_Impl for PdfThumbnailHandler {
    fn Initialize(
        &self,
        pstream: &Option<windows::Win32::System::Com::IStream>,
        _: u32,
    ) -> windows::core::Result<()> {
        if let Some(stream) = pstream {
            self.stream.set(Some(WinStream::from(stream.clone())));
            Ok(())
        } else {
            Err(windows::core::Error::from(E_FAIL))
        }
    }
}

impl IThumbnailProvider_Impl for PdfThumbnailHandler {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("space_thumbnails_pdf.log"))
            .unwrap_or_else(|_| OpenOptions::new().write(true).open("NUL").unwrap());

        writeln!(log_file, "GetThumbnail called with cx={}", cx).ok();

        // 1. Get the IStream from handler
        let mut win_stream = self
            .stream
            .take()
            .ok_or(windows::core::Error::from(E_FAIL))?;

        writeln!(log_file, "Got stream").ok();

        // 2. Convert Win32 IStream to WinRT RandomAccessStream
        // Note: Windows.Data.Pdf requires IRandomAccessStream.
        // We can create an InMemoryRandomAccessStream and copy data, 
        // or use CreateRandomAccessStreamOverStream if available (requires manual interop).
        // For simplicity and safety, we read the content into memory first.
        
        let mut content = Vec::new();
        win_stream.read_to_end(&mut content).map_err(|e| {
            writeln!(log_file, "Failed to read stream: {:?}", e).ok();
            windows::core::Error::from(E_FAIL)
        })?;
        writeln!(log_file, "Read content: {} bytes", content.len()).ok();

        let mem_stream = InMemoryRandomAccessStream::new()?;
        let data_writer = windows::Storage::Streams::DataWriter::CreateDataWriter(&mem_stream)?;
        data_writer.WriteBytes(&content)?;
        data_writer.StoreAsync()?.get()?;
        data_writer.FlushAsync()?.get()?;
        mem_stream.Seek(0)?;

        writeln!(log_file, "Created InMemoryRandomAccessStream").ok();

        // 3. Load PDF Document
        let pdf_doc = PdfDocument::LoadFromStreamAsync(&mem_stream)?.get().map_err(|e| {
            writeln!(log_file, "Failed to load PDF: {:?}", e).ok();
            e
        })?;
        
        writeln!(log_file, "PDF loaded. Page count check...").ok();
        
        if pdf_doc.PageCount()? == 0 {
            writeln!(log_file, "PDF has 0 pages").ok();
            return Err(windows::core::Error::from(E_FAIL));
        }

        // 4. Render first page
        let page = pdf_doc.GetPage(0)?;
        writeln!(log_file, "Got page 0").ok();
        
        // Calculate size to maintain aspect ratio
        // Default PDF rendering is vector based, so we can render at desired resolution
        // but GetThumbnail expects a bitmap of max size cx * cx
        
        let render_options = windows::Data::Pdf::PdfPageRenderOptions::new()?;
        // Setting destination width/height will handle scaling
        // If we want exact fit:
        let src_size = page.Size()?;
        // User requested proportional padding:
        // Baseline: 256px -> 20px padding per side (40px total)
        // Ratio: 40/256 = 0.15625
        // max_side = cx * (1 - 0.15625)
        let padding_ratio = 40.0 / 256.0;
        let max_side = (cx as f32 * (1.0 - padding_ratio)).max(1.0);
        
        let scale = (max_side / src_size.Width).min(max_side / src_size.Height);
        let dst_width = ((src_size.Width * scale) as u32).max(1);
        let dst_height = ((src_size.Height * scale) as u32).max(1);
        
        writeln!(log_file, "Rendering size: {}x{}", dst_width, dst_height).ok();

        render_options.SetDestinationWidth(dst_width)?;
        render_options.SetDestinationHeight(dst_height)?;
        // render_options.SetBitmapEncoderId(windows::core::GUID::from("19e4a5aa-5662-4fc5-a0c0-1758028e1057"))?; // JPEG Encoder
        
        let out_stream = InMemoryRandomAccessStream::new()?;
        page.RenderToStreamAsync(&out_stream)?.get().map_err(|e| {
            writeln!(log_file, "RenderToStreamAsync failed: {:?}", e).ok();
            e
        })?; // Default is PNG
        
        writeln!(log_file, "Rendered to PNG stream").ok();

        // Read PNG data
        let reader = windows::Storage::Streams::DataReader::CreateDataReader(&out_stream.GetInputStreamAt(0)?)?;
        let size = out_stream.Size()? as usize;
        reader.LoadAsync(size as u32)?.get()?;
        let mut png_bytes = vec![0u8; size];
        reader.ReadBytes(&mut png_bytes)?;
        
        writeln!(log_file, "Read PNG bytes: {}", size).ok();

        // Use image crate to decode PNG to BGRA
        let mut img = image::load_from_memory(&png_bytes)
            .map_err(|e| {
                writeln!(log_file, "Image decode failed: {:?}", e).ok();
                windows::core::Error::from(E_FAIL)
            })?
            .to_rgba8();
            
        let mut width = img.width();
        let mut height = img.height();
        
        writeln!(log_file, "Decoded image: {}x{}", width, height).ok();

        // Resize to fit within max_side x max_side (preserving aspect ratio)
        // This ensures the 20px padding on each side (max 216px for 256px container)
        let scale_w = max_side / width as f32;
        let scale_h = max_side / height as f32;
        let final_scale = scale_w.min(scale_h);
        
        let new_width = (width as f32 * final_scale) as u32;
        let new_height = (height as f32 * final_scale) as u32;

        if new_width != width || new_height != height {
            writeln!(log_file, "Resizing image to fit constraints: {}x{} -> {}x{}", width, height, new_width, new_height).ok();
            img = image::imageops::resize(&img, new_width, new_height, image::imageops::FilterType::Lanczos3);
            width = img.width();
            height = img.height();
        }

        unsafe {
            let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
            // Create a bitmap of the full requested size (cx * cx) to include padding
            let hbmp = create_argb_bitmap(cx, cx, &mut p_bits);
            
            if hbmp.0 == 0 || p_bits.is_null() {
                 writeln!(log_file, "Failed to create bitmap").ok();
                 return Err(windows::core::Error::from(E_FAIL));
            }

            let full_width = cx as usize;
            let full_height = cx as usize;
            // Ensure the slice size matches the bitmap size (width * height * 4 bytes)
            let dst_slice = std::slice::from_raw_parts_mut(p_bits as *mut u8, full_width * full_height * 4);

            // Initialize with transparent background (0x00000000)
            dst_slice.fill(0);
            
            let src_data = img.as_raw();
            let img_width = width as usize;
            let img_height = height as usize;

            // Calculate centering offsets (safe subtraction)
            let x_offset = if full_width > img_width { (full_width - img_width) / 2 } else { 0 };
            let y_offset = if full_height > img_height { (full_height - img_height) / 2 } else { 0 };

            writeln!(log_file, "Offsets: x={}, y={}", x_offset, y_offset).ok();

            for y in 0..img_height {
                for x in 0..img_width {
                    let src_idx = (y * img_width + x) * 4;
                    // Calculate destination index with offsets
                    // Ensure we don't go out of bounds (though logic should prevent it)
                    let dst_y = y + y_offset;
                    let dst_x = x + x_offset;
                    
                    if dst_y < full_height && dst_x < full_width {
                        let dst_idx = (dst_y * full_width + dst_x) * 4;
                        
                        if dst_idx + 3 < dst_slice.len() && src_idx + 3 < src_data.len() {
                            let r = src_data[src_idx];
                            let g = src_data[src_idx + 1];
                            let b = src_data[src_idx + 2];
                            let a = src_data[src_idx + 3];
                            
                            // Pre-multiply alpha for ARGB
                            let a_f = a as f32 / 255.0;
                            dst_slice[dst_idx] = (b as f32 * a_f) as u8;     // B
                            dst_slice[dst_idx + 1] = (g as f32 * a_f) as u8; // G
                            dst_slice[dst_idx + 2] = (r as f32 * a_f) as u8; // R
                            dst_slice[dst_idx + 3] = a;                      // A
                        }
                    }
                }
            }
            
            phbmp.write(hbmp);
            pdwalpha.write(WTSAT_ARGB);
        }
        
        writeln!(log_file, "Bitmap returned").ok();

        Ok(())
    }
}
