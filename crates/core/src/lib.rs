use std::{cell::Cell, ffi::OsStr, fs, path::Path, rc::Rc};

#[cfg(target_os = "windows")]
use windows::Win32::System::JobObjects::*;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::*;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;
#[cfg(target_os = "windows")]
use windows::core::PCSTR;
use std::path::PathBuf;

use filament_bindings::{
    assimp::{post_process, AssimpAsset},
    backend::{Backend, PixelBufferDescriptor, PixelDataFormat, PixelDataType},
    filament::{
        self, sRGBColor, Aabb, Camera, ClearOptions, Engine, Fov, IndirectLight,
        IndirectLightBuilder, LightBuilder, Projection, Renderer, Scene, SwapChain,
        SwapChainConfig, Texture, View, Viewport,
    },
    glftio::{
        AssetConfiguration, AssetLoader, MaterialProvider, ResourceConfiguration, ResourceLoader,
    },
    image::{ktx, KtxBundle},
    math::{Float3, Mat3f, Mat4f},
    utils::Entity,
};

// use truck_meshalgo::tessellation::{MeshedShape, RobustMeshableShape};
// use truck_polymesh::PolygonMesh;
// use truck_stepio::r#in::Table;
use rayon::prelude::*;
use std::panic::{self, AssertUnwindSafe};
use std::io::Write;

fn log_debug(msg: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\st_debug.log") {
        let _ = writeln!(file, "[Core] {}", msg);
    }
}

const IDL_TEXTURE_DATA: &'static [u8] = include_bytes!("lightroom_14b_ibl.ktx");

const ASSIMP_FLAGS: u32 = post_process::GEN_SMOOTH_NORMALS
    | post_process::CALC_TANGENT_SPACE
    | post_process::GEN_UV_COORDS
    | post_process::FIND_INSTANCES
    | post_process::OPTIMIZE_MESHES
    | post_process::IMPROVE_CACHE_LOCALITY
    | post_process::SORT_BY_P_TYPE
    | post_process::TRIANGULATE;

pub struct SpaceThumbnailsRenderer {
    // need release
    engine: Engine,
    scene: Scene,
    ibl_texture: Texture,
    ibl: IndirectLight,
    swap_chain: SwapChain,
    renderer: Renderer,
    camera_entity: Entity,
    sunlight_entity: Entity,
    view: View,
    destory_asset: Option<Box<dyn FnOnce(&mut Engine, &mut Scene)>>,

    viewport: Viewport,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RendererBackend {
    Default = 0,
    OpenGL = 1,
    Vulkan = 2,
    Metal = 3,
}

impl SpaceThumbnailsRenderer {
    pub fn new(backend: RendererBackend, width: u32, height: u32) -> Self {
        unsafe {
            let mut engine = Engine::create(match backend {
                RendererBackend::Default => Backend::DEFAULT,
                RendererBackend::OpenGL => Backend::OPENGL,
                RendererBackend::Vulkan => Backend::VULKAN,
                RendererBackend::Metal => Backend::METAL,
            })
            .unwrap();
            let mut scene = engine.create_scene().unwrap();
            let mut swap_chain = engine
                .create_headless_swap_chain(width, height, SwapChainConfig::TRANSPARENT)
                .unwrap();
            let mut renderer = engine.create_renderer().unwrap();
            let mut view = engine.create_view().unwrap();
            let mut entity_manager = engine.get_entity_manager().unwrap();
            let camera_entity = entity_manager.create();
            let mut camera = engine.create_camera(&camera_entity).unwrap();
            let ibl_texture = ktx::create_texture(
                &mut engine,
                KtxBundle::from(IDL_TEXTURE_DATA).unwrap(),
                false,
            )
            .unwrap();

            let mut ibl = IndirectLightBuilder::new()
                .unwrap()
                .reflections(&ibl_texture)
                .intensity(50000.0)
                .rotation(&Mat3f::rotation(-90.0, Float3::new(0.0, 1.0, 0.0)))
                .build(&mut engine)
                .unwrap();
            scene.set_indirect_light(&mut ibl);

            let sunlight_entity = entity_manager.create();
            LightBuilder::new(filament::LightType::SUN)
                .unwrap()
                .color(&sRGBColor(Float3::new(0.98, 0.92, 0.89)).to_linear_fast())
                .intensity(100000.0)
                .direction(&Float3::new(0.6, -1.0, -0.8).normalize())
                .cast_shadows(true)
                .sun_angular_radius(1.0)
                .sun_halo_size(2.0)
                .sun_halo_falloff(80.0)
                .build(&mut engine, &sunlight_entity)
                .unwrap();

            scene.add_entity(&sunlight_entity);

            view.set_camera(&mut camera);
            view.set_scene(&mut scene);
            renderer.set_clear_options(&ClearOptions {
                clear_color: [0.0, 0.0, 0.0, 0.0].into(),
                clear: true,
                discard: false,
            });

            let viewport = Viewport {
                left: 0,
                bottom: 0,
                width,
                height,
            };

            view.set_viewport(&viewport);

            // warming up
            renderer.begin_frame(&mut swap_chain);
            renderer.render(&mut view);
            renderer.end_frame();
            engine.flush_and_wait();

            Self {
                engine,
                scene,
                ibl_texture,
                ibl,
                swap_chain,
                renderer,
                camera_entity,
                sunlight_entity,
                view,
                destory_asset: None,
                viewport,
            }
        }
    }

    pub fn load_asset_from_file(&mut self, filepath: impl AsRef<Path>) -> Option<&mut Self> {
        eprintln!("DEBUG: load_asset_from_file checking {:?}", filepath.as_ref());
        
        // Try to sniff the file format using magic bytes
        // Only try to open file if we suspect it might be a GLB/STEP masquerading.
        // Or simply, we MUST reopen the file if we want to read it again?
        // Actually, Assimp takes a path, so it will open the file itself.
        // The issue is likely that our 'sniffing' logic returns early or does something that confuses the flow.
        
        let mut is_glb_magic = false;
        let mut is_step_magic = false;

        if let Ok(mut file) = fs::File::open(filepath.as_ref()) {
            let mut header = [0u8; 12]; 
            if let Ok(_) = std::io::Read::read(&mut file, &mut header) {
                if header.starts_with(b"glTF") {
                    is_glb_magic = true;
                }
                if header.starts_with(b"ISO-10303-21") {
                    is_step_magic = true;
                }
            }
        } // file is closed here

        if is_glb_magic {
             eprintln!("DEBUG: Detected GLB magic bytes");
             let data = fs::read(&filepath).ok()?;
             return self.load_gltf_asset(
                 &data,
                 filepath.as_ref().file_name()?,
                 Some(filepath.as_ref()),
             );
        }

        // If is_step_magic is true, it's definitely STEP.
        // If file extension is .step/.stp/.igs/.iges and NOT glTF, it's CAD (fallback).
        let ext = filepath.as_ref().extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase());
        let is_cad = is_step_magic || (!is_glb_magic && matches!(ext.as_deref(), Some("stp") | Some("step") | Some("igs") | Some("iges")));

        if is_cad {
             eprintln!("DEBUG: Detected CAD format (STEP/IGES)");
             return self.load_step_asset(filepath);
        }
        
        // Fallback to Assimp for everything else (OBJ, FBX, etc.)
        eprintln!("DEBUG: Fallback to Assimp for {:?}", filepath.as_ref());
        log_debug(&format!("Fallback to Assimp for {:?}", filepath.as_ref()));
        
        let asset_res = AssimpAsset::from_file_with_flags(&mut self.engine, filepath, ASSIMP_FLAGS);
        match asset_res {
            Ok(asset) => {
                 log_debug("Assimp loaded successfully");
                 self.load_assimp_asset(asset)
            },
            Err(e) => {
                eprintln!("Assimp failed to load file: {:?}", e);
                log_debug(&format!("Assimp failed to load file: {:?}", e));
                None
            }
        }
    }


    pub fn load_step_asset(&mut self, filepath: impl AsRef<Path>) -> Option<&mut Self> {
        eprintln!("Start reading file: {:?}", filepath.as_ref());
        log_debug(&format!("load_step_asset (FreeCAD+Job): {:?}", filepath.as_ref()));
        let start = std::time::Instant::now();

        // Temporary file for OBJ output
        let out_path = std::env::temp_dir().join(format!("space_thumbnails_{}.obj", uuid::Uuid::new_v4()));
        let out_path_str = match out_path.to_str() {
            Some(s) => s,
            None => {
                log_debug("Failed to convert out_path to string");
                return None;
            }
        };
        let in_path_str = match filepath.as_ref().to_str() {
            Some(s) => s,
            None => {
                 log_debug("Failed to convert in_path to string");
                 return None;
            }
        };

        // Path resolution for portable deployment
        let current_exe = std::env::current_exe().unwrap_or_default();
        let exe_dir = current_exe.parent().unwrap_or(Path::new("."));
        let bat_path = exe_dir.join("tools").join("step2obj.bat");
        let bat_script = if bat_path.exists() {
             bat_path
        } else {
             // Fallback for dev environment
             PathBuf::from(r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\tools\step2obj.bat")
        };
        
        log_debug(&format!("Using conversion script: {:?}", bat_script));
        
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C")
           .arg(&bat_script)
           .env("STEP2OBJ_INPUT", in_path_str)
           .env("STEP2OBJ_OUTPUT", out_path_str);

        // Windows Job Object logic for resource limiting
        #[cfg(target_os = "windows")]
        let status_res = {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);

            unsafe {
                // Wrap in closure to handle Option/Result propagation
                (|| -> Option<(std::process::ExitStatus, HANDLE)> {
                    let job = CreateJobObjectA(std::ptr::null(), PCSTR(std::ptr::null()));
                    if job.is_invalid() {
                         log_debug("Failed to create job object");
                         return None;
                    }
                    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                    info.BasicLimitInformation.LimitFlags = 
                        JOB_OBJECT_LIMIT_PROCESS_MEMORY | 
                        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    info.ProcessMemoryLimit = 2560 * 1024 * 1024; // 2.5 GB (Decimation needs memory)

                    if !SetInformationJobObject(
                        job,
                        JobObjectExtendedLimitInformation,
                        &info as *const _ as *const _,
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    ).as_bool() {
                         log_debug("Failed to set job info");
                         return None;
                    }

                    let mut child = match cmd.spawn() {
                        Ok(c) => c,
                        Err(e) => {
                            log_debug(&format!("Failed to spawn process: {:?}", e));
                            return None;
                        }
                    };
                    
                    let handle = HANDLE(child.as_raw_handle() as isize);
                    if !AssignProcessToJobObject(job, handle).as_bool() {
                        log_debug("Failed to assign process to job");
                        let _ = child.kill();
                        return None;
                    }
                    
                    let s = child.wait().ok()?;
                    Some((s, job)) // Return job to keep it alive until wait finishes
                })()
            }
        };

        #[cfg(not(target_os = "windows"))]
        let status_res = cmd.status().ok().map(|s| (s, ()));

        let status = if let Some((s, _job)) = status_res {
            s
        } else {
            log_debug("Failed to execute conversion command (Job Object setup failed?)");
            return None;
        };

        match status {
            s if s.success() => {
                eprintln!("Conversion successful in {:?}.", start.elapsed());
                log_debug(&format!("Conversion successful in {:?}.", start.elapsed()));
                // Load the generated OBJ
                let obj_bytes = match fs::read(&out_path) {
                    Ok(b) => b,
                    Err(e) => {
                        log_debug(&format!("Failed to read generated OBJ: {:?}", e));
                        return None;
                    }
                };
                let _ = fs::remove_file(&out_path);
                self.load_asset_from_memory(&obj_bytes, "converted.obj")
            }
            s => {
                log_debug(&format!("Conversion failed with exit code: {:?}", s.code()));
                None
            }
        }
    }

    pub fn load_step_asset_truck(&mut self, filepath: impl AsRef<Path>) -> Option<&mut Self> {
        eprintln!("Start reading file: {:?}", filepath.as_ref());
        log_debug(&format!("load_step_asset (FreeCAD): {:?}", filepath.as_ref()));
        let start = std::time::Instant::now();

        // Temporary file for OBJ output
        let out_path = std::env::temp_dir().join(format!("space_thumbnails_{}.obj", uuid::Uuid::new_v4()));
        let out_path_str = match out_path.to_str() {
            Some(s) => s,
            None => {
                log_debug("Failed to convert out_path to string");
                return None;
            }
        };
        let in_path_str = match filepath.as_ref().to_str() {
            Some(s) => s,
            None => {
                 log_debug("Failed to convert in_path to string");
                 return None;
            }
        };

        // Absolute path to the bat script (HARDCODED for this environment as requested)
        let bat_script = r"d:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\tools\step2obj.bat";
        
        log_debug("Converting STEP to OBJ using FreeCAD...");
        // Use a hidden window creation flag if possible, but std::process doesn't support it directly on Windows easily without extensions.
        // However, since we are running as a background CLI (and lowered priority), it should be fine.
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg(bat_script)
            .env("STEP2OBJ_INPUT", in_path_str)
            .env("STEP2OBJ_OUTPUT", out_path_str)
            .status();

        match status {
            Ok(s) if s.success() => {
                eprintln!("Conversion successful in {:?}.", start.elapsed());
                log_debug(&format!("Conversion successful in {:?}.", start.elapsed()));
                // Load the generated OBJ
                let obj_bytes = match fs::read(&out_path) {
                    Ok(b) => b,
                    Err(e) => {
                        log_debug(&format!("Failed to read generated OBJ: {:?}", e));
                        return None;
                    }
                };
                
                // Cleanup temp file
                let _ = fs::remove_file(&out_path);

                self.load_asset_from_memory(&obj_bytes, "converted.obj")
            }
            Ok(s) => {
                log_debug(&format!("Conversion failed with exit code: {:?}", s.code()));
                None
            }
            Err(e) => {
                log_debug(&format!("Failed to execute conversion script: {:?}", e));
                None
            }
        }
    }

    pub fn load_asset_from_memory(
        &mut self,
        buffer: &[u8],
        filename: impl AsRef<OsStr>,
    ) -> Option<&mut Self> {
        // Sniff buffer content
        if buffer.starts_with(b"glTF") {
             eprintln!("DEBUG: Detected GLB magic bytes in memory buffer");
             return self.load_gltf_asset(buffer, filename.as_ref(), None);
        }

        if matches!(Path::new(filename.as_ref()).extension(), Some(e) if e == "gltf" || e == "glb")
        {
            self.load_gltf_asset(buffer, filename.as_ref(), None)
        } else {
            let asset = AssimpAsset::from_memory_with_flags(
                &mut self.engine,
                buffer,
                filename.as_ref().to_str()?,
                ASSIMP_FLAGS,
            )
            .ok()?;
            self.load_assimp_asset(asset)
        }
    }

    pub fn load_assimp_asset(&mut self, mut asset: AssimpAsset) -> Option<&mut Self> {
        self.destory_opened_asset();

        unsafe {
            let aabb = asset.get_aabb();
            let transform = fit_into_unit_cube(aabb);

            let mut transform_manager = self.engine.get_transform_manager()?;
            let root_entity = asset.get_root_entity();
            let root_transform_instance = transform_manager.get_instance(root_entity)?;
            transform_manager.set_transform_float(&root_transform_instance, &transform);

            self.scene.add_entities(asset.get_renderables());

            self.scene.add_entity(root_entity);

            let mut camera = self
                .engine
                .get_camera_component(&self.camera_entity)
                .unwrap();

            camera.set_exposure_physical(16.0, 1.0 / 125.0, 100.0);

            if let Some(camera_info) = asset.get_main_camera() {
                let aspect = self.viewport.width as f64 / self.viewport.height as f64;
                if camera_info.horizontal_fov != 0.0 {
                    camera.set_projection_fov_direction(
                        camera_info.horizontal_fov,
                        aspect,
                        0.1,
                        f64::INFINITY,
                        Fov::HORIZONTAL,
                    );
                } else {
                    camera.set_projection(
                        Projection::ORTHO,
                        -camera_info.orthographic_width,
                        camera_info.orthographic_width,
                        -camera_info.orthographic_width / aspect,
                        camera_info.orthographic_width / aspect,
                        0.1,
                        100000.0,
                    );
                }
                transform_manager.set_transform_float(
                    &transform_manager.get_instance(&self.camera_entity).unwrap(),
                    &(transform
                        * Mat4f::look_at(
                            &camera_info.position,
                            &camera_info.look_at,
                            &camera_info.up,
                        )),
                )
            } else {
                setup_camera_surround_view(&mut camera, &aabb.transform(transform), &self.viewport);
                // Ensure z_near is small enough and z_far is large enough for large models
                let aspect = self.viewport.width as f64 / self.viewport.height as f64;
                camera.set_lens_projection(28.0, aspect, 0.001, 10000.0);
            }

            self.destory_asset = Some(Box::new(move |engine, scene| {
                scene.remove_entities(asset.get_renderables());
                scene.remove_entity(asset.get_root_entity());
                asset.destory(engine)
            }));
        }

        Some(self)
    }

    pub fn load_gltf_asset(
        &mut self,
        data: &[u8],
        filename: &OsStr,
        filepath: Option<&Path>,
    ) -> Option<&mut Self> {
        self.destory_opened_asset();

        // If data starts with glTF magic bytes, force binary loading regardless of filename
        let binary = data.starts_with(b"glTF") || matches!(Path::new(filename).extension(), Some(e) if e == "glb");

        let filepath_str = filepath.and_then(|p| p.to_str().map(|s| s.to_owned()));

        unsafe {
            let materials = MaterialProvider::create_ubershader_loader(&mut self.engine)?;
            let mut entity_manager = self.engine.get_entity_manager()?;
            let mut transform_manager = self.engine.get_transform_manager()?;
            let mut loader = AssetLoader::create(AssetConfiguration {
                engine: &mut self.engine,
                materials,
                entities: Some(&mut entity_manager),
                default_node_name: None,
            })?;

            let mut asset = if binary {
                loader.create_asset_from_binary(&data)?
            } else {
                loader.create_asset_from_json(&data)?
            };

            let uris = asset.get_resource_uris();
            let has_external_resource = uris
                .map(|uris| uris.into_iter().any(|uri| !is_base64_data_uri(&uri)))
                .unwrap_or(false);
            
            // For binary GLB, we might not have external resources so this check might be too aggressive if filepath is None
            // But let's keep it for now as per original logic, just relaxed for binary
            if filepath_str.is_none() && has_external_resource {
                // If it's a binary GLB loaded from memory (misnamed .step), it usually self-contains textures.
                // If it fails here, it's fine.
                // return None; 
            }

            ResourceLoader::create(ResourceConfiguration {
                engine: &mut self.engine,
                gltf_path: filepath_str,
                normalize_skinning_weights: true,
                recompute_bounding_boxes: false,
                ignore_bind_transform: false,
            })
            .unwrap()
            .load_resources(&mut asset);

            asset.release_source_data();

            let aabb = asset.get_bounding_box();
            let transform = fit_into_unit_cube(&aabb);
            let root_transform_instance = transform_manager.get_instance(&asset.get_root())?;

            transform_manager.set_transform_float(&root_transform_instance, &transform);

            self.scene.add_entities(asset.get_entities());
            
            // Force a light setup that works for this model
            // Re-setup IBL and Sun might be needed if scene was cleared? 
            // The renderer struct keeps them, but let's ensure we are not rendering black.
            
            let mut camera = self
                .engine
                .get_camera_component(&self.camera_entity)
                .unwrap();

            // Increase exposure slightly
            camera.set_exposure_physical(16.0, 1.0 / 125.0, 400.0);

            setup_camera_surround_view(&mut camera, &aabb.transform(transform), &self.viewport);

            // Create a default sunlight if not present (just in case)
            // But we can't easily add entities here without refactoring.
            // Let's rely on IBL being set up correctly by the renderer init.
            
            self.destory_asset = Some(Box::new(move |_engine, scene| {
                scene.remove_entities(asset.get_entities());
                loader.destroy_asset(&asset);
                loader.destroy_materials();
                core::mem::drop(loader);
            }));
        }

        Some(self)
    }

    pub fn take_screenshot_sync(&mut self, output_memory: &mut [u8]) -> usize {
        let byte_count = self.get_screenshot_size_in_byte();

        if output_memory.len() < byte_count {
            panic!("Output memory space is not enough to take screenshot.")
        }

        unsafe {
            let ok: Rc<Cell<bool>> = Rc::new(Cell::new(false));
            let ok_inner = ok.clone();
            let pixel = PixelBufferDescriptor::from_raw_ptr_callback(
                output_memory.as_mut_ptr(),
                output_memory.len(),
                PixelDataFormat::RGBA,
                PixelDataType::UBYTE,
                move |_| ok_inner.set(true),
            );

            self.renderer.begin_frame(&mut self.swap_chain);
            self.renderer.render(&mut self.view);
            self.renderer
                .read_pixels(0, 0, self.viewport.width, self.viewport.height, pixel);
            self.renderer.end_frame();
            self.engine.flush_and_wait();

            if ok.get() == false {
                panic!("Take screenshot failed");
            }
        }

        byte_count
    }

    pub fn get_size(&self) -> (u32, u32) {
        (self.viewport.width, self.viewport.height)
    }

    pub fn get_screenshot_size_in_byte(&self) -> usize {
        (self.viewport.width * self.viewport.height * 4) as usize
    }

    pub fn destory_opened_asset(&mut self) -> &mut Self {
        let destory_asset = self.destory_asset.take();
        if let Some(destory) = destory_asset {
            destory(&mut self.engine, &mut self.scene)
        }

        self
    }
}

impl Drop for SpaceThumbnailsRenderer {
    fn drop(&mut self) {
        unsafe {
            self.destory_opened_asset();
            let mut entity_manager = self.engine.get_entity_manager().unwrap();
            self.engine.destroy_entity_components(&self.camera_entity);
            self.engine.destroy_entity_components(&self.sunlight_entity);
            entity_manager.destory(&mut self.camera_entity);
            entity_manager.destory(&mut self.sunlight_entity);
            self.engine.destroy_texture(&mut self.ibl_texture);
            self.engine.destroy_indirect_light(&mut self.ibl);
            self.engine.destroy_scene(&mut self.scene);
            self.engine.destroy_view(&mut self.view);
            self.engine.destroy_renderer(&mut self.renderer);
            self.engine.destroy_swap_chain(&mut self.swap_chain);
            Engine::destroy(&mut self.engine);
        }
    }
}

unsafe fn setup_camera_surround_view(camera: &mut Camera, aabb: &Aabb, viewport: &Viewport) {
    let aspect = viewport.width as f64 / viewport.height as f64;
    let half_extent = aabb.extent();
    camera.set_lens_projection(28.0, aspect, 0.01, f64::INFINITY);
    camera.look_at_up(
        &(aabb.center()
            + Float3::from(((half_extent[0] + half_extent[2]) / 2.0).max(half_extent[1]))
                * Float3::from([2.5, 1.7, 2.5])),
        &aabb.center(),
        &[0.0, 1.0, 0.0].into(),
    );
}

fn fit_into_unit_cube(bounds: &Aabb) -> Mat4f {
    let min = bounds.min;
    let max = bounds.max;
    let max_extent = f32::max(f32::max(max[0] - min[0], max[1] - min[1]), max[2] - min[2]);
    let scale_factor = 2.0 / max_extent;
    let center = (min + max) / 2.0;
    Mat4f::scaling(Float3::new(scale_factor, scale_factor, scale_factor))
        * Mat4f::translation(center * -1.0)
}

fn is_base64_data_uri(uri: &str) -> bool {
    uri.starts_with("data:") && uri.find(";base64,").is_some()
}

#[cfg(test)]
mod test {
    use std::{fs, io::Cursor, path::PathBuf, str::FromStr, time::Instant};

    use image::{ImageBuffer, ImageOutputFormat, Rgba};

    use crate::{RendererBackend, SpaceThumbnailsRenderer};

    #[test]
    fn render_file_test() {
        let models = fs::read_dir(
            PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
                .unwrap()
                .join("models"),
        )
        .unwrap();

        dbg!(std::env::temp_dir());

        for entry in models {
            let entry = entry.unwrap();

            if entry.file_type().unwrap().is_dir() {
                continue;
            }

            let filepath = entry.path();
            let filename = filepath.file_name().unwrap().to_str().unwrap();

            let now = Instant::now();
            let mut renderer = SpaceThumbnailsRenderer::new(RendererBackend::Vulkan, 800, 800);
            let elapsed = now.elapsed();
            println!("Initialize renderer, Elapsed: {:.2?}", elapsed);

            let now = Instant::now();
            renderer.load_asset_from_file(&filepath).unwrap();
            let elapsed = now.elapsed();
            println!("Load model file {}, Elapsed: {:.2?}", filename, elapsed);

            let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];

            let now = Instant::now();
            renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());
            let elapsed = now.elapsed();
            println!("Render and take screenshot, Elapsed: {:.2?}", elapsed);

            let image = ImageBuffer::<Rgba<u8>, _>::from_raw(800, 800, screenshot_buffer).unwrap();
            let mut encoded = Cursor::new(Vec::new());
            image
                .write_to(&mut encoded, ImageOutputFormat::Png)
                .unwrap();
            test_results::save!(
                format!(
                    "render_file_test/{}-screenshot.png",
                    filepath
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('.', "-")
                )
                .as_str(),
                encoded.get_ref().as_slice()
            )
        }
    }
}
