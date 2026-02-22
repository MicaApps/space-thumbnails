use std::cell::Cell;
use std::io::{Read, Seek, SeekFrom, Cursor};
use flate2::read::GzDecoder;
use image::{ImageBuffer, Rgba, ImageOutputFormat, GenericImageView};
use image::imageops;
use windows::core::{implement, IUnknown, Interface, GUID};
use windows::Win32::Foundation::E_FAIL;
use windows::Win32::Graphics::Gdi::HBITMAP;
use windows::Win32::UI::Shell::{IThumbnailProvider_Impl, WTSAT_ARGB, WTS_ALPHATYPE};

use crate::registry::{register_clsid, RegistryData, RegistryKey, RegistryValue};
use crate::utils::{create_argb_bitmap, WinStream};
use super::Provider;

pub struct BlenderThumbnailProvider {
    pub clsid: GUID,
}

impl BlenderThumbnailProvider {
    pub fn new(clsid: GUID) -> Self {
        Self { clsid }
    }
}

impl Provider for BlenderThumbnailProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, false);
        
        let extensions = vec![".blend"];
        
        for ext in extensions {
             // Register under extension
            result.push(RegistryKey {
                path: ext.to_owned(),
                values: vec![
                    RegistryValue(
                        "PerceivedType".to_owned(),
                        RegistryData::Str("Document".to_owned()),
                    ),
                ],
            });

            // Register under extension\ShellEx
            result.push(RegistryKey {
                path: format!(
                    "{}\\ShellEx\\{{{:?}}}",
                    ext,
                    windows::Win32::UI::Shell::IThumbnailProvider::IID
                ),
                values: vec![RegistryValue(
                    "".to_owned(),
                    RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
                )],
            });
            
             // SystemFileAssociations
            result.push(RegistryKey {
                path: format!(
                    "SystemFileAssociations\\{}\\ShellEx\\{{{:?}}}",
                    ext,
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
        BlenderThumbnailHandler::new(riid, ppv_object)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream
)]
pub struct BlenderThumbnailHandler {
    stream: Cell<Option<WinStream>>,
}

impl BlenderThumbnailHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = BlenderThumbnailHandler {
            stream: Cell::new(None),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream_Impl for BlenderThumbnailHandler {
    fn Initialize(
        &self,
        pstream: &Option<windows::Win32::System::Com::IStream>,
        _grfmode: u32,
    ) -> windows::core::Result<()> {
        if let Some(stream) = pstream {
            self.stream.set(Some(WinStream::from(stream.clone())));
            Ok(())
        } else {
            Err(E_FAIL.into())
        }
    }
}

impl IThumbnailProvider_Impl for BlenderThumbnailHandler {
    fn GetThumbnail(
        &self,
        _cx: u32,
        phbmp: *mut HBITMAP,
        alphatype: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let mut stream = self.stream.take().ok_or(windows::core::Error::from(E_FAIL))?;
        
        // Check for Gzip
        let mut magic = [0u8; 2];
        if stream.read_exact(&mut magic).is_err() {
            return Err(E_FAIL.into());
        }
        
        if stream.seek(SeekFrom::Start(0)).is_err() {
            return Err(E_FAIL.into());
        }
        
        let png_data = if magic == [0x1f, 0x8b] {
            // Gzip
            let gz = GzDecoder::new(stream);
            let mut buffer = Vec::new();
            let mut gz_reader = gz;
            if gz_reader.read_to_end(&mut buffer).is_err() {
                return Err(E_FAIL.into());
            }
            let mut cursor = Cursor::new(buffer);
            extract_thumbnail_from_stream(&mut cursor)
        } else {
            // Plain
            extract_thumbnail_from_stream(&mut stream)
        };
        
        let png_data = png_data.ok_or(windows::core::Error::from(E_FAIL))?;
        
        // Decode PNG to bitmap
        let img = image::load_from_memory(&png_data)
            .map_err(|_| windows::core::Error::from(E_FAIL))?;
            
        let (width, height) = img.dimensions();
        // Convert to BGRA for Windows
        let mut rgba = img.to_rgba8().into_raw();
        
        // Convert RGBA to BGRA
        for chunk in rgba.chunks_exact_mut(4) {
            let tmp = chunk[0];
            chunk[0] = chunk[2];
            chunk[2] = tmp;
        }
        
        let hbitmap = unsafe {
            let mut bits: *mut core::ffi::c_void = core::ptr::null_mut();
            let hbmp = create_argb_bitmap(width, height, &mut bits);
            if hbmp.is_invalid() {
                return Err(E_FAIL.into());
            }
            // Copy data
            std::ptr::copy_nonoverlapping(rgba.as_ptr(), bits as *mut u8, rgba.len());
            hbmp
        };
        
        unsafe {
            *phbmp = hbitmap;
            if !alphatype.is_null() {
                *alphatype = WTSAT_ARGB;
            }
        }
        
        Ok(())
    }
}

fn extract_thumbnail_from_stream<R: Read + Seek>(cursor: &mut R) -> Option<Vec<u8>> {
    // Read header (12 bytes)
    let mut head = [0u8; 12];
    if cursor.read_exact(&mut head).is_err() {
        return None;
    }
    
    if &head[0..7] != b"BLENDER" {
        return None;
    }
    
    let is_64_bit = head[7] == b'-';
    let is_big_endian = head[8] == b'V';
    
    let sizeof_bhead = if is_64_bit { 24 } else { 20 };
    
    loop {
        // Read code (4 bytes) and length (4 bytes)
        // We need to read 8 bytes
        let mut bhead_start = [0u8; 8];
        if cursor.read_exact(&mut bhead_start).is_err() {
            break;
        }
        
        let code_bytes = &bhead_start[0..4];
        let length = if is_big_endian {
            u32::from_be_bytes(bhead_start[4..8].try_into().unwrap())
        } else {
            u32::from_le_bytes(bhead_start[4..8].try_into().unwrap())
        };

        // Skip rest of header
        // IMPORTANT: The header size is sizeof_bhead. We read 8 bytes.
        // So we need to skip sizeof_bhead - 8 bytes.
        // BUT we need to check if we found the TEST block BEFORE skipping the data?
        // No, the header is for the block. The data follows the header.
        
        // The loop structure should be:
        // 1. Read Block Header (Code, Len, Pointer, SDNA Index, Count)
        // 2. Decide to read data or skip based on Code.
        
        // My previous logic was:
        // Read 8 bytes (Code + Len).
        // Skip (sizeof_bhead - 8) bytes (Rest of header).
        // Check Code.
        // If interesting, read Data (Length).
        // If not, seek Data (Length).
        
        if cursor.seek(SeekFrom::Current((sizeof_bhead - 8) as i64)).is_err() {
            break;
        }
        
        if code_bytes == b"REND" {
            if cursor.seek(SeekFrom::Current(length as i64)).is_err() {
                break;
            }
        } else if code_bytes == b"TEST" {
             // Found TEST block (thumbnail)
            let mut dims = [0u8; 8];
            if cursor.read_exact(&mut dims).is_err() {
                break;
            }
            
            let (width, height) = if is_big_endian {
                (
                    u32::from_be_bytes(dims[0..4].try_into().unwrap()),
                    u32::from_be_bytes(dims[4..8].try_into().unwrap())
                )
            } else {
                (
                    u32::from_le_bytes(dims[0..4].try_into().unwrap()),
                    u32::from_le_bytes(dims[4..8].try_into().unwrap())
                )
            };
            
            let data_len = length.checked_sub(8)?;
            
            if data_len != width * height * 4 {
                return None;
            }
            
            let mut raw_data = vec![0u8; data_len as usize];
            if cursor.read_exact(&mut raw_data).is_err() {
                return None;
            }
            
            let img_buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, raw_data)?;
            let img = imageops::flip_vertical(&img_buffer);
            
            let mut out = Cursor::new(Vec::new());
            if img.write_to(&mut out, ImageOutputFormat::Png).is_ok() {
                return Some(out.into_inner());
            }
            return None;
        } else {
             // Skip data
            if cursor.seek(SeekFrom::Current(length as i64)).is_err() {
                break;
            }
        }
    }
    None
}
