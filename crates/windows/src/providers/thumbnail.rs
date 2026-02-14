use std::{
    cell::Cell,
    fs,
    io,
    time::{Duration, Instant},
};

use log::{info, warn};
use space_thumbnails::{RendererBackend, SpaceThumbnailsRenderer};
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::*,
        System::Com::*,
        UI::Shell::{PropertiesSystem::*, *},
    },
};

use crate::{
    constant::{ERROR_256X256_ARGB, TIMEOUT_256X256_ARGB, TOOLARGE_256X256_ARGB},
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, run_timeout, WinStream},
    providers::pdf_thumbnail,
};

use super::Provider;

pub struct ThumbnailProvider {
    pub clsid: GUID,
    pub file_extension: &'static str,
}

impl ThumbnailProvider {
    pub fn new(clsid: GUID, file_extension: &'static str) -> Self {
        Self {
            clsid,
            file_extension,
        }
    }
}

impl Provider for ThumbnailProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, false);
        result.append(&mut vec![RegistryKey {
            path: format!(
                "{}\\ShellEx\\{{{:?}}}",
                self.file_extension,
                windows::Win32::UI::Shell::IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
            )],
        }]);
        result
    }

    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let handler: IUnknown = ThumbnailHandler::new(self.file_extension).into();
        unsafe { handler.query(&*riid, ppv_object).ok() }
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream
)]
pub struct ThumbnailHandler {
    filename_hint: &'static str,
    stream: Cell<Option<WinStream>>,
}

impl ThumbnailHandler {
    pub fn new(filename_hint: &'static str) -> ThumbnailHandler {
        ThumbnailHandler {
            filename_hint,
            stream: Cell::new(None),
        }
    }
}

impl IThumbnailProvider_Impl for ThumbnailHandler {
    fn GetThumbnail(
        &self,
        _: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let size = 256;
        let mut stream = self
            .stream
            .take()
            .ok_or(windows::core::Error::from(E_FAIL))?;

        let filesize = stream.size()?;

        let log_path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails5\st_debug.log";
        {
            use std::io::Write;
            if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                let _ = writeln!(
                    file,
                    "[{:?}] GetThumbnail called, hint: {}, size: {} bytes",
                    std::time::SystemTime::now(),
                    self.filename_hint,
                    filesize
                );
            }
        }
        if filesize > 300 * 1024 * 1024
        /* 300 MB */
        {
            unsafe {
                let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                std::ptr::copy(
                    TOOLARGE_256X256_ARGB.as_ptr(),
                    p_bits as *mut _,
                    TOOLARGE_256X256_ARGB.len(),
                );
                phbmp.write(hbmp);
                pdwalpha.write(WTSAT_ARGB);
            }
            return Ok(());
        }

        let start_time = Instant::now();
        info!(target: "ThumbnailProvider", "Getting thumbnail from stream [{}], size: {}", self.filename_hint, filesize);

        let mut buffer = Vec::new();
        io::Read::read_to_end(&mut stream, &mut buffer)
            .ok()
            .ok_or(windows::core::Error::from(E_FAIL))?;

        let filename_hint = self.filename_hint;

        let timeout_result = if filename_hint.ends_with(".pdf") {
            {
                use std::io::Write;
                if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                    let _ = writeln!(
                        file,
                        "[{:?}] PDF thumbnail requested, buffer_len: {}",
                        std::time::SystemTime::now(),
                        buffer.len()
                    );
                }
            }
            let result = pdf_thumbnail::render_pdf_to_bitmap(&buffer, size as i32, size as i32);
            {
                use std::io::Write;
                if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
                    let _ = writeln!(
                        file,
                        "[{:?}] PDF render result: {:?}",
                        std::time::SystemTime::now(),
                        result.is_some()
                    );
                }
            }
            match result {
                Some(buffer) => Ok(Some(buffer)),
                None => Ok(None)
            }
        } else {
            run_timeout(
                move || {
                    let mut renderer = SpaceThumbnailsRenderer::new(RendererBackend::Vulkan, size, size);
                    renderer.load_asset_from_memory(
                        buffer.as_slice(),
                        format!("inmemory{}", filename_hint),
                    )?;
                    let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];
                    renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());
                    Some(screenshot_buffer)
                },
                Duration::from_secs(5),
            )
        };

        match timeout_result {
            Ok(Some(screenshot_buffer)) => {
                info!(target: "ThumbnailProvider", "Rendering thumbnails success [{}], Elapsed: {:.2?}", self.filename_hint, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(size, size, &mut p_bits);
                    for x in 0..size {
                        for y in 0..size {
                            let index = ((x * size + y) * 4) as usize;
                            let r = screenshot_buffer[index];
                            let g = screenshot_buffer[index + 1];
                            let b = screenshot_buffer[index + 2];
                            let a = screenshot_buffer[index + 3];
                            (p_bits.add(((x * size + y) * 4) as usize) as *mut u32).write(
                                (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32,
                            )
                        }
                    }
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
            Ok(None) => {
                warn!(target: "ThumbnailProvider", "Rendering thumbnails returned None [{}], Elapsed: {:.2?}", self.filename_hint, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                    std::ptr::copy(
                        ERROR_256X256_ARGB.as_ptr(),
                        p_bits as *mut _,
                        ERROR_256X256_ARGB.len(),
                    );
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
            Err(err) if err.kind() == io::ErrorKind::TimedOut => {
                warn!(target: "ThumbnailProvider", "Rendering thumbnails timeout [{}], Elapsed: {:.2?}", self.filename_hint, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                    std::ptr::copy(
                        TIMEOUT_256X256_ARGB.as_ptr(),
                        p_bits as *mut _,
                        TIMEOUT_256X256_ARGB.len(),
                    );
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
            Err(_) => {
                warn!(target: "ThumbnailProvider", "Rendering thumbnails returned Error [{}], Elapsed: {:.2?}", self.filename_hint, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                    std::ptr::copy(
                        ERROR_256X256_ARGB.as_ptr(),
                        p_bits as *mut _,
                        ERROR_256X256_ARGB.len(),
                    );
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
        }
    }
}

impl IInitializeWithStream_Impl for ThumbnailHandler {
    fn Initialize(&self, pstream: &Option<IStream>, _grfmode: u32) -> windows::core::Result<()> {
        if let Some(stream) = pstream {
            self.stream.set(Some(WinStream::from(stream.to_owned())));
            Ok(())
        } else {
            Err(E_FAIL.into())
        }
    }
}
