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

        // 4. Render Pages
        // Calculate size constraints
        let padding_ratio = 40.0 / 256.0;
        let max_side = (cx as f32 * (1.0 - padding_ratio)).max(1.0);
        
        writeln!(log_file, "Max side: {}", max_side).ok();

        // Helper to render a page
        let render_page_fn = |page_index: u32| -> windows::core::Result<image::RgbaImage> {
            let page = pdf_doc.GetPage(page_index)?;
            let src_size = page.Size()?;
            
            let scale = (max_side / src_size.Width).min(max_side / src_size.Height);
            let render_width = ((src_size.Width * scale) as u32).max(1);
            let render_height = ((src_size.Height * scale) as u32).max(1);
            
            let options = windows::Data::Pdf::PdfPageRenderOptions::new()?;
            options.SetDestinationWidth(render_width)?;
            options.SetDestinationHeight(render_height)?;
            // Set background to white (opaque)
            options.SetBackgroundColor(windows::UI::Color { A: 255, R: 255, G: 255, B: 255 })?;
            
            let stream = InMemoryRandomAccessStream::new()?;
            page.RenderWithOptionsToStreamAsync(&stream, &options)?.get()?;
            
            let reader = windows::Storage::Streams::DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
            let size = stream.Size()? as usize;
            reader.LoadAsync(size as u32)?.get()?;
            let mut buffer = vec![0u8; size];
            reader.ReadBytes(&mut buffer)?;
            
            let mut img = image::load_from_memory(&buffer)
                .map_err(|_| windows::core::Error::from(E_FAIL))?
                .to_rgba8();
                
            // Resize if needed (Lanczos3) to match exact requested dimensions
            let width = img.width();
            let height = img.height();
            if width != render_width || height != render_height {
                 img = image::imageops::resize(&img, render_width, render_height, image::imageops::FilterType::Lanczos3);
            }
            Ok(img)
        };

        // Render Page 0
        let img0 = render_page_fn(0)?;
        writeln!(log_file, "Rendered Page 0").ok();

        // Render Page 1 if exists
        let img1 = if pdf_doc.PageCount()? > 1 {
            match render_page_fn(1) {
                Ok(img) => {
                    writeln!(log_file, "Rendered Page 1").ok();
                    Some(img)
                },
                Err(e) => {
                    writeln!(log_file, "Failed to render Page 1: {:?}", e).ok();
                    None
                }
            }
        } else {
            None
        };

        // 5. Composition and Cropping
        let mut canvas = image::RgbaImage::new(cx, cx);
        // Canvas is initialized with 0 (transparent) by default in image crate, but let's be safe
        for pixel in canvas.pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        let center_image = |target: &mut image::RgbaImage, src: &image::RgbaImage| {
            let x_offset = (cx as i64 - src.width() as i64) / 2;
            let y_offset = (cx as i64 - src.height() as i64) / 2;
            image::imageops::overlay(target, src, x_offset, y_offset);
        };

        // Draw Page 1 (Bottom)
        if let Some(ref img) = img1 {
            center_image(&mut canvas, img);
        }

        // Process Page 0 (Top) - Crop Top-Right, then Add Stroke
        let mut img0_base = img0.clone();
        
        // Calculate stroke width relative to 256px
        let scale_factor = cx as f32 / 256.0;
        let stroke_width = (3.0 * scale_factor).max(1.0) as u32;
        let crop_size = (49.0 * scale_factor).max(1.0) as u32;

        writeln!(log_file, "Applying stroke: {}px, Crop: {}px", stroke_width, crop_size).ok();

        // Step 1: Crop the base image
        let width = img0_base.width();
        let height = img0_base.height();
        
        let start_x = if width >= crop_size { width - crop_size } else { 0 }; 
        let end_x = width;
        
        let start_y = 0;
        let end_y = crop_size.min(height);

        writeln!(log_file, "Cropping Base Image: Rect [{}, {}] to [{}, {}]", start_x, start_y, end_x, end_y).ok();

        for y in start_y..end_y {
            for x in start_x..end_x {
                 if x < width && y < height {
                    img0_base.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
                 }
            }
        }

        // Step 2: Create bordered background
        let base_width = img0_base.width();
        let base_height = img0_base.height();
        let bordered_width = base_width + 2 * stroke_width;
        let bordered_height = base_height + 2 * stroke_width;
        
        let mut img0_processed = image::RgbaImage::from_pixel(bordered_width, bordered_height, image::Rgba([117, 116, 113, 255])); // #757471
        
        // Step 3: Crop the background with the SAME logic
        let bg_width = img0_processed.width();
        let bg_height = img0_processed.height();
        
        let bg_start_x = if bg_width >= crop_size { bg_width - crop_size } else { 0 }; 
        let bg_end_x = bg_width;
        
        let bg_start_y = 0;
        let bg_end_y = crop_size.min(bg_height);

        writeln!(log_file, "Cropping Background: Rect [{}, {}] to [{}, {}]", bg_start_x, bg_start_y, bg_end_x, bg_end_y).ok();

        for y in bg_start_y..bg_end_y {
            for x in bg_start_x..bg_end_x {
                 if x < bg_width && y < bg_height {
                    img0_processed.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
                 }
            }
        }

        // Step 4: Overlay the cropped base image onto the cropped background
        image::imageops::overlay(&mut img0_processed, &img0_base, stroke_width as i64, stroke_width as i64);

        // Step 5: Load and Overlay the Fold Image (Top-Right of the bordered composition)
        const FOLD_BYTES: &[u8] = include_bytes!("../../assets/PDF-Folder.png");
        if let Ok(fold_img_dynamic) = image::load_from_memory(FOLD_BYTES) {
            let fold_img = fold_img_dynamic.to_rgba8();
            // Resize fold image to match crop_size x crop_size
            let fold_img_resized = image::imageops::resize(&fold_img, crop_size, crop_size, image::imageops::FilterType::Lanczos3);
            
            // Overlay at the top-right corner of the *content* (ignoring the outer stroke)
            // The content ends at (bg_width - stroke_width).
            // The fold image should be placed at (content_right - crop_size)
            // = (bg_width - stroke_width) - crop_size
            // Y position is simply stroke_width (top of content)
            
            let fold_x = (bg_width - stroke_width - crop_size) as i64;
            let fold_y = stroke_width as i64;
            
            writeln!(log_file, "Overlaying Fold Image: {}x{} at ({}, {})", crop_size, crop_size, fold_x, fold_y).ok();
            image::imageops::overlay(&mut img0_processed, &fold_img_resized, fold_x, fold_y);
        } else {
             writeln!(log_file, "Failed to load fold image").ok();
        }

        // Draw Page 0 (Top)
        center_image(&mut canvas, &img0_processed);

        // 6. Convert to ARGB Bitmap
        unsafe {
            let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
            let hbmp = create_argb_bitmap(cx, cx, &mut p_bits);
            
            if hbmp.0 == 0 || p_bits.is_null() {
                 writeln!(log_file, "Failed to create bitmap").ok();
                 return Err(windows::core::Error::from(E_FAIL));
            }

            let full_width = cx as usize;
            let full_height = cx as usize;
            let dst_slice = std::slice::from_raw_parts_mut(p_bits as *mut u8, full_width * full_height * 4);

            // Copy canvas to bitmap
            // Canvas is RgbaImage (RGBA), Bitmap needs BGRA + Premultiplied Alpha
            for (i, pixel) in canvas.pixels().enumerate() {
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];
                let a = pixel[3];
                
                let idx = i * 4;
                if idx + 3 < dst_slice.len() {
                    let a_f = a as f32 / 255.0;
                    dst_slice[idx] = (b as f32 * a_f) as u8;     // B
                    dst_slice[idx + 1] = (g as f32 * a_f) as u8; // G
                    dst_slice[idx + 2] = (r as f32 * a_f) as u8; // R
                    dst_slice[idx + 3] = a;                      // A
                }
            }
            
            phbmp.write(hbmp);
            pdwalpha.write(WTSAT_ARGB);
        }
        
        writeln!(log_file, "Bitmap returned").ok();

        Ok(())
    }
}
