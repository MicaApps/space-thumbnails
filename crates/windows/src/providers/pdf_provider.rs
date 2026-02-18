use std::{cell::Cell, io::Cursor, io::Read, fs::OpenOptions, io::Write};

use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Data::Pdf::PdfDocument,
    Storage::Streams::{InMemoryRandomAccessStream, RandomAccessStreamReference},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::HBITMAP,
        System::Com::*,
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
        let scale = (cx as f32 / src_size.Width).min(cx as f32 / src_size.Height);
        let dst_width = (src_size.Width * scale) as u32;
        let dst_height = (src_size.Height * scale) as u32;
        
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
        let img = image::load_from_memory(&png_bytes)
            .map_err(|e| {
                writeln!(log_file, "Image decode failed: {:?}", e).ok();
                windows::core::Error::from(E_FAIL)
            })?
            .to_rgba8();
            
        let width = img.width();
        let height = img.height();
        
        writeln!(log_file, "Decoded image: {}x{}", width, height).ok();

        unsafe {
            let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
            let hbmp = create_argb_bitmap((width as i32).try_into().unwrap(), (height as i32).try_into().unwrap(), &mut p_bits);
            
            // Copy pixels. Note: Windows Bitmap is BGRA (usually), but create_argb_bitmap might expect something else.
            // Let's check create_argb_bitmap impl or standard GDI behavior.
            // 32-bit DIB is typically BGRA or RGBA depending on header.
            // Standard GDI 32bpp is BGRA (0x00BBGGRR).
            // image crate produces RGBA. We need to swap R and B.
            
            let src_data = img.as_raw();
            let dst_slice = std::slice::from_raw_parts_mut(p_bits as *mut u8, (width * height * 4) as usize);
            
            for i in 0..(width * height) as usize {
                let r = src_data[i * 4];
                let g = src_data[i * 4 + 1];
                let b = src_data[i * 4 + 2];
                let a = src_data[i * 4 + 3];
                
                // Pre-multiply alpha if necessary? 
                // Win32 alpha bitmaps usually expect premultiplied alpha.
                // Let's assume yes.
                
                let a_f = a as f32 / 255.0;
                dst_slice[i * 4] = (b as f32 * a_f) as u8;     // B
                dst_slice[i * 4 + 1] = (g as f32 * a_f) as u8; // G
                dst_slice[i * 4 + 2] = (r as f32 * a_f) as u8; // R
                dst_slice[i * 4 + 3] = a;                      // A
            }
            
            phbmp.write(hbmp);
            pdwalpha.write(WTSAT_ARGB);
        }
        
        writeln!(log_file, "Bitmap returned").ok();

        Ok(())
    }
}
