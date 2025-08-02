use crate::camera::CameraHistory;
use crate::compute_pass::ReverseProj;
use crate::egui_tools::EguiRenderer;
use crate::headless::HeadlessImage;
use crate::light_factor::LFBuffers;
use crate::raytracer::RayTraceInfo;
use crate::save::{ImageCache, Save, SaveManager};
use crate::scene::Scene;
use crate::shape::Quad;
use crate::stereoscope::StereoscopeBuffer;
use crate::utils::DrawUI;
use crate::vertex;
use crate::FileWatcher;
use crevice::std140::AsStd140;
use egui::ahash::HashSet;
use egui_notify::Toasts;
use egui_wgpu::wgpu::SurfaceError;
use egui_wgpu::{wgpu, ScreenDescriptor};
use image::{DynamicImage, GenericImageView};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use vec_utilities::maths::Statistics;
use wgpu::util::DeviceExt;
use wgpu::{Adapter, Backends, RenderPipeline};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

// Struct to deal with all the drawing. This is where all the setting live
pub struct AppState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: Option<wgpu::SurfaceConfiguration>,
    pub surface: Option<wgpu::Surface<'static>>,
    pub scale_factor: f32,
    pub egui_renderer: Option<EguiRenderer>,
    pub render_pipe: Option<RenderPipeline>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_index: u32,
    pub scene: Scene,
    pub factorizer: LFBuffers,
    pub stereoscope: StereoscopeBuffer,
    pub rev_proj: ReverseProj,
    pub camera_history: CameraHistory,
    pub image_cache: ImageCache,
    pub save_manager: SaveManager,
    displaying_panel_textures: bool,
    distort_rays: bool,
    pub headless: HeadlessImage,
}

impl AppState {
    async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: Option<&Window>,
        width: u32,
        height: u32,
    ) -> Self {
        let power_pref = wgpu::PowerPreference::HighPerformance;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_pref,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let features = wgpu::Features::VERTEX_WRITABLE_STORAGE
            | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::BGRA8UNORM_STORAGE
            | wgpu::Features::MULTI_DRAW_INDIRECT
            | wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let limites = adapter.limits();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: features,
                    required_limits: limites,
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let camera_history = CameraHistory::new(&device);
        let camera = &camera_history.current_camera;

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .expect("failed to select proper surface texture format!");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        let image_height = surface_config.height;
        let image_width = surface_config.width;
        let rt_test = RayTraceInfo::test(camera, image_height, image_width);
        let scene = Scene::new(rt_test, camera, &device, &queue);

        let factorizer = LFBuffers::set_up(&device);
        let stereoscope = StereoscopeBuffer::set_up(&device);
        let headless = HeadlessImage::new(&device, &queue, image_width, image_height);

        let egui_renderer = {
            if let Some(window) = window {
                Some(EguiRenderer::new(
                    &device,
                    surface_config.format,
                    None,
                    1,
                    window,
                ))
            } else {
                None
            }
        };

        let scale_factor = 1.0;

        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/simple.wgsl").into()),
        });
        if let Some(error) = pollster::block_on(device.pop_error_scope()) {
            println!("Could not validate shader!: {error}");
        }
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &scene.target_binds.bind_layout,
                    &scene.texture_binds.bind_layout,
                    &scene.panel_binds.bind_layout,
                    &factorizer.bind_group_layout,
                    //&headless.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // 1. Entry points to the shader, call the main
                // function to make things easier!
                buffers: &[vertex::Vertex::desc()], // 2. //Passing things to the shader
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                // 3. // Fragment is optional
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        // 4. // What color outputs it should set up!
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // Multitarget output
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // Types of Primitives
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // Colling mode
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5. === USEFUL FOR RENDERING TO ARRAY TEXTURES ====
            cache: None,     // 6.
        });

        // Surface quad:
        let surface_quad = Quad::screen_quad();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: surface_quad.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_list: [i16; 6] = [0, 1, 3, 1, 3, 2];

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_list),
            usage: wgpu::BufferUsages::INDEX,
        });
        let rev_proj = ReverseProj::new(
            &device,
            &queue,
            &scene,
            &factorizer,
            &stereoscope,
            &camera_history,
        );
        let mut image_cache = ImageCache::default();
        let target = scene.world.texture.load_texture();
        image_cache.target_image = target;

        let save_manager = SaveManager::boot();

        Self {
            displaying_panel_textures: false,
            distort_rays: true,
            save_manager,
            image_cache,
            device,
            queue,
            surface: Some(surface),
            surface_config: Some(surface_config),
            egui_renderer,
            scale_factor,
            render_pipe: Some(render_pipe),
            vertex_buffer: Some(vertex_buffer),
            index_buffer: Some(index_buffer),
            num_index: index_list.len() as u32,
            scene,
            factorizer,
            stereoscope,
            rev_proj,
            camera_history,
            headless,
        }
    }
    async fn request_adapter(instance: &wgpu::Instance) -> Adapter {
        let power_pref = wgpu::PowerPreference::HighPerformance;
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_pref,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find an appropriate adapter")
    }

    fn headless_app(instance: &wgpu::Instance) -> Self {
        let adapter = pollster::block_on(Self::request_adapter(instance));

        let features = wgpu::Features::VERTEX_WRITABLE_STORAGE
            | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::BGRA8UNORM_STORAGE
            | wgpu::Features::MULTI_DRAW_INDIRECT
            | wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let limites = adapter.limits();
        let request = async {
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: features,
                        required_limits: limites,
                        ..Default::default()
                    },
                    None,
                )
                .await
                .expect("Failed to create device")
        };

        let (device, queue) = pollster::block_on(request);

        let camera_history = CameraHistory::new(&device);
        let camera = &camera_history.current_camera;

        let image_height = 1;
        let image_width = 1;
        let rt_test = RayTraceInfo::test(camera, image_height, image_width);
        let scene = Scene::new(rt_test, camera, &device, &queue);

        let factorizer = LFBuffers::set_up(&device);
        let stereoscope = StereoscopeBuffer::set_up(&device);
        let headless = HeadlessImage::new(&device, &queue, image_width, image_height);

        let rev_proj = ReverseProj::new(
            &device,
            &queue,
            &scene,
            &factorizer,
            &stereoscope,
            &camera_history,
        );
        let mut image_cache = ImageCache::default();
        let target = scene.world.texture.load_texture();
        image_cache.target_image = target;

        let save_manager = SaveManager::boot();
        Self {
            displaying_panel_textures: false,
            distort_rays: true,
            save_manager,
            image_cache,
            device,
            queue,
            surface: None,
            surface_config: None,
            egui_renderer: None,
            scale_factor: 0.0,
            render_pipe: None,
            vertex_buffer: None,
            index_buffer: None,
            num_index: 0u32,
            scene,
            factorizer,
            stereoscope,
            rev_proj,
            camera_history,
            headless,
        }
    }

    fn build_pipeline(&mut self) {
        let device = &self.device;
        let scene = &self.scene;

        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let Ok(shader_string) = fs::read_to_string("./shaders/simple.wgsl") else {
            return;
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_string.into()),
        });
        if let Some(_error) = pollster::block_on(device.pop_error_scope()) {
            println!("Could not validate shader!");
        } else {
            let render_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &scene.target_binds.bind_layout,
                        &scene.texture_binds.bind_layout,
                        &scene.panel_binds.bind_layout,
                        &self.factorizer.bind_group_layout,
                        //&self.headless.bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

            let render_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"), // 1. Entry points to the shader, call the main
                    // function to make things easier!
                    buffers: &[vertex::Vertex::desc()], // 2. //Passing things to the shader
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    // 3. // Fragment is optional
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            // 4. // What color outputs it should set up!
                            format: self.surface_config.as_ref().unwrap().format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        // Multitarget output
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, // Types of Primitives
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw, // Colling mode
                    cull_mode: None,
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None, // 1.
                multisample: wgpu::MultisampleState {
                    count: 1,                         // 2.
                    mask: !0,                         // 3.
                    alpha_to_coverage_enabled: false, // 4.
                },
                multiview: None, // 5. === USEFUL FOR RENDERING TO ARRAY TEXTURES ====
                cache: None,     // 6.
            });
            self.render_pipe = Some(render_pipe);
        }
    }
    pub fn compute_pass(&mut self) {
        // Update the world first!
        self.camera_history.update_buffer(&self.queue);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            self.rev_proj.compute_pass(
                &mut encoder,
                &self.scene,
                &self.factorizer,
                &self.camera_history,
                &self.stereoscope,
            );
        }
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::MaintainBase::Wait);
        self.rev_proj.time_taken(&self.device, &self.queue);
    }
    /// Function runs compute pipeline 100 times. Collects Data on run time
    /// Uses the current state of the world ()
    fn benchmark_stereo_pass(&mut self, iterations: usize) -> Vec<f32> {
        self.camera_history.update_buffer(&self.queue);
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            for index in 0..iterations {
                self.rev_proj.stereo_pass(
                    &mut encoder,
                    &self.scene,
                    index,
                    &self.camera_history,
                    &self.factorizer,
                    &self.stereoscope,
                );
            }
            {
                self.rev_proj.resolve_queries(&mut encoder, iterations);
            }
            self.queue.submit(Some(encoder.finish()));
        }
        self.device.poll(wgpu::MaintainBase::Wait);
        self.rev_proj
            .benchmark_time_total(&self.device, &self.queue, iterations)
    }

    fn benchmark_sep_pass(&mut self, iterations: usize) -> Vec<f32> {
        self.camera_history.update_buffer(&self.queue);
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            for index in 0..iterations {
                self.rev_proj.sep_pass(
                    &mut encoder,
                    &self.scene,
                    index,
                    &self.camera_history,
                    &self.factorizer,
                    &self.stereoscope,
                );
            }
            {
                self.rev_proj.resolve_queries(&mut encoder, iterations);
            }
            self.queue.submit(Some(encoder.finish()));
        }
        self.device.poll(wgpu::MaintainBase::Wait);
        self.rev_proj
            .benchmark_time_total(&self.device, &self.queue, iterations)
    }
    fn run_benchmark(&mut self) {
        let iterations = 100;
        self.rev_proj.update_query_set(&self.device, iterations + 1);
        let stereo_data = self.benchmark_stereo_pass(iterations);
        let stereo_average = stereo_data.iter().mean().unwrap();

        self.rev_proj.update_query_set(&self.device, iterations + 1);
        let sep_data = self.benchmark_sep_pass(iterations);
        let sep_mean = sep_data.iter().mean().unwrap();

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("./BenchSample")
            .unwrap();

        let title_line = format!(
            "BENCH: Size {:?}, Kernel: {} , ViewPoints: {}  ",
            self.scene.world.pixel_count,
            self.camera_history.kernel,
            self.camera_history.len()
        );
        println!("Sep: {:?}", Duration::from_nanos(sep_mean as u64));
        println!("Stereo: {:?}", Duration::from_nanos(stereo_average as u64));
        writeln!(file, "{title_line}").unwrap();
        let stereo_line = format!("STEREO: {stereo_average}");
        writeln!(file, "{stereo_line}").unwrap();
        let sep_line = format!("SEP: {sep_mean}");
        writeln!(file, "{sep_line}",).unwrap();
    }
    fn run_through_curated(&mut self) {
        let path = PathBuf::from("./resources/textures/Curated/");
        let dir = fs::read_dir(path).unwrap();
        for file in dir {
            println!("File {file:?}");
            let new_target = image::open(file.unwrap().path()).unwrap();
            self.update_target(new_target);
            self.run_benchmark();
        }
    }
    pub fn update_target(&mut self, image: DynamicImage) {
        self.image_cache.target_image = image;
        self.update_target_texture();
    }

    fn verify_m_a(&mut self) {
        println!("Length of history is: {}", self.camera_history.len());
        self.compute_pass();
        let image_shape = self.scene.world.pixel_count;
        println!("Target Pixel Count is: {image_shape:?}");
        let _rays_cast = image_shape.x * image_shape.y * self.camera_history.history.len() as u32;

        //self.stereoscope.verify_m_a(&self.device, rays_cast);
    }
    pub fn sample_stereo(&mut self) {
        let c_t = &self.image_cache.target_image;
        let pixel_count_a = self.scene.panels[0].panel.pixel_count.yx();
        let pixel_count_b = self.scene.panels[1].panel.pixel_count.yx();

        let target_size = (
            self.scene.world.pixel_count.y,
            self.scene.world.pixel_count.x,
        );

        let number_of_view_points = self.camera_history.len() as u32;

        self.stereoscope.sample_light_field(
            &self.device,
            pixel_count_a,
            pixel_count_b,
            target_size,
            number_of_view_points,
        );
    }
    pub fn sample_sep(&mut self) {
        let c_t = &self.image_cache.target_image;
        let pixel_count_a = self.scene.panels[0].panel.pixel_count.yx();
        let pixel_count_b = self.scene.panels[1].panel.pixel_count.yx();

        let target_size = (
            self.scene.world.pixel_count.y,
            self.scene.world.pixel_count.x,
        );

        let number_of_view_points = self.camera_history.len() as u32;

        self.factorizer.sample_light_field(
            &self.device,
            pixel_count_a,
            pixel_count_b,
            c_t,
            target_size,
            number_of_view_points,
        );
    }

    fn sample_both(&mut self) {
        self.compute_pass();

        self.sample_sep();
        self.sample_stereo();
    }

    pub fn solve_stereo(&mut self) {
        let imgs = self.stereoscope.factorize_stereo();
        self.image_cache.cache_output(true, imgs);
        self.update_panel(0);
        self.update_panel(1);
    }

    pub fn solver_light_field(&mut self) {
        self.compute_pass();

        // Y here maps to additional rows and X to additional Columns
        let images = self.factorizer.alternative_factorization();

        self.image_cache.cache_output(false, images);
        self.update_panel(0);
        self.update_panel(1);
    }

    fn update_panel(&self, panel_entry: usize) {
        let image = &self.image_cache.panels[panel_entry];
        let dimensions = image.dimensions();

        let copy = &self.scene.panel_binds.panel_texture.texture;
        if dimensions.0 > copy.width() || dimensions.1 > copy.height() {
            println!(
                "Selected texture {:?} is larger than allocated buffer {:?}",
                dimensions,
                (copy.width(), copy.height())
            );
            return;
        }
        // Pad the image to deal with no squares
        if dimensions.0 != dimensions.1 {
            println!("IMage is not a square, they may be distortions.")
        }

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: copy,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: panel_entry as u32,
                },
            },
            &image.to_rgba8(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
        );
    }
    fn update_target_texture(&mut self) {
        let img = &self.image_cache.target_image;
        if let Ok(_ok) = self
            .scene
            .texture_binds
            .update_target_texture(img, &self.queue)
        {
            self.scene.world.update_pixel_count(img.dimensions());
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.as_mut().unwrap().width = width;
        self.surface_config.as_mut().unwrap().height = height;
        self.surface
            .as_ref()
            .unwrap()
            .configure(&self.device, &self.surface_config.as_ref().unwrap());
        self.headless
            .handle_resize(width, height, &self.device, &self.queue);
    }
    fn print_compute(&self) {
        self.rev_proj.print_image(&self.device);
    }
    fn save_pop_up(&mut self) {
        self.save_manager.save_open = true;
    }
    fn save(&mut self) {
        if self.save_manager.name_inserted {
            let name = &self.save_manager.current_save_name;
            let save = Save::from_cache(
                &self.camera_history.history,
                name,
                &self.image_cache,
                &self.scene,
            );
            self.save_manager.add_save(save);
        }
    }

    fn load_next_save(&mut self) -> Option<String> {
        let next_save = self.save_manager.next_save();
        if let Some(save) = next_save {
            let name = save.name.clone();
            let new_cache = save.to_cache();
            let cameras = save.cameras.clone();
            save.update_scene(&mut self.scene);
            self.image_cache = new_cache;
            self.update_target_texture();
            self.update_panel(0);
            self.update_panel(1);
            self.displaying_panel_textures = true;
            self.camera_history.update_history(cameras);
            Some(name)
        } else {
            None
        }
    }
    fn load_previous_save(&mut self) -> Option<String> {
        let next_save = self.save_manager.previous_save();
        if let Some(save) = next_save {
            let name = save.name.clone();
            let new_cache = save.to_cache();
            let cameras = save.cameras.clone();

            save.update_scene(&mut self.scene);
            self.image_cache = new_cache;

            self.camera_history.update_history(cameras);
            self.update_target_texture();
            self.update_panel(0);
            self.update_panel(1);
            self.displaying_panel_textures = true;

            Some(name)
        } else {
            None
        }
    }
}

// Handles the drawing and the app logic
pub struct App {
    instance: wgpu::Instance,
    pub state: Option<AppState>,
    window: Option<Arc<Window>>,
    previous_draw: Instant,
    mouse_press: bool,
    mouse_on_ui: bool,
    disable_controls: bool,
    cache_stereo: bool,
    pressed_keys: HashSet<KeyCode>,
    toasts: Toasts,
}

impl App {
    pub fn new(headless: bool) -> Self {
        if headless {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: Backends::VULKAN,
                ..Default::default()
            });

            let state = AppState::headless_app(&instance);
            let mut output = Self {
                instance,
                toasts: Toasts::default(),
                mouse_press: false,
                mouse_on_ui: false,
                disable_controls: false,
                cache_stereo: false,
                state: None,
                window: None,
                previous_draw: Instant::now(),
                pressed_keys: HashSet::default(),
            };
            output.state = Some(state);
            return output;
        }
        // Fore opengl backend?
        let instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::VULKAN,
            ..Default::default()
        });
        Self {
            instance,
            toasts: Toasts::default(),
            mouse_press: false,
            mouse_on_ui: false,
            disable_controls: false,
            cache_stereo: false,
            state: None,
            window: None,
            previous_draw: Instant::now(),
            pressed_keys: HashSet::default(),
        }
    }

    pub fn process_keyboard(&mut self, event: &KeyEvent) {
        if !(event.state == ElementState::Pressed) || self.mouse_on_ui {
            return;
        }
        if let PhysicalKey::Code(code) = event.physical_key {
            if event.state.is_pressed() {
                self.pressed_keys.insert(code);
            } else {
                self.pressed_keys.remove(&code);
            }
        }
        // Only check if controls have been renabled
        if self.disable_controls {
            if let PhysicalKey::Code(KeyCode::Minus) = event.physical_key {
                self.disable_controls = !self.disable_controls;

                if self.disable_controls {
                    self.toasts.info("Controls Disabled");
                } else {
                    self.toasts.info("Controls Enabled");
                }
            }
            return;
        }

        match event.physical_key {
            PhysicalKey::Code(KeyCode::Minus) => {
                self.disable_controls = !self.disable_controls;

                if self.disable_controls {
                    self.toasts.info("Controls Disabled");
                } else {
                    self.toasts.info("Controls Enabled");
                }
            }
            PhysicalKey::Code(KeyCode::Digit1) => {
                if let Some(state) = self.state.as_mut() {
                    state.sample_both();
                }
            }
            PhysicalKey::Code(KeyCode::Backslash) => {
                self.toasts.info("Saving Scene!");
                self.disable_controls = true;
                if let Some(state) = self.state.as_mut() {
                    state.save_pop_up();
                }
            }
            PhysicalKey::Code(KeyCode::Enter) => {
                self.toasts.info("Saving image");
                if let Some(state) = self.state.as_mut() {
                    state.headless.retrieve_image = true;
                }
            }

            PhysicalKey::Code(KeyCode::BracketRight) => {
                if self.pressed_keys.contains(&KeyCode::ShiftLeft)
                    || self.pressed_keys.contains(&KeyCode::ShiftRight)
                {
                    if let Some(state) = self.state.as_mut() {
                        let name = state.load_next_save();
                        self.toasts.info(format!("Loading next save {name:?}"));
                        self.next_camera();
                    }
                }
            }
            PhysicalKey::Code(KeyCode::Digit2) => {
                if let Some(state) = self.state.as_mut() {
                    state.run_through_curated();
                }
            }

            PhysicalKey::Code(KeyCode::BracketLeft) => {
                if self.pressed_keys.contains(&KeyCode::ShiftLeft)
                    || self.pressed_keys.contains(&KeyCode::ShiftRight)
                {
                    if let Some(state) = self.state.as_mut() {
                        let name = state.load_previous_save();

                        self.toasts.info(format!("Previous next save{name:?}"));

                        self.next_camera();
                    }
                }
            }
            PhysicalKey::Code(KeyCode::Comma) => {
                if self.pressed_keys.contains(&KeyCode::ShiftLeft)
                    || self.pressed_keys.contains(&KeyCode::ShiftRight)
                {
                    self.previous_camera();
                }
            }

            PhysicalKey::Code(KeyCode::Period) => {
                if self.pressed_keys.contains(&KeyCode::ShiftLeft)
                    || self.pressed_keys.contains(&KeyCode::ShiftRight)
                {
                    self.next_camera();
                }
            }

            PhysicalKey::Code(KeyCode::KeyP) => {
                if let Some(state) = self.state.as_mut() {
                    self.cache_stereo = !self.cache_stereo;

                    if state.image_cache.load_output(self.cache_stereo).is_ok() {
                        if self.cache_stereo {
                            self.toasts.info("Displaying stereo output");
                            state.update_panel(0);
                            state.update_panel(1);
                        } else {
                            self.toasts.info("Displaying Separable Output");

                            state.update_panel(0);
                            state.update_panel(1);
                        }
                    } else {
                        self.toasts.warning("No output found in cache");

                        self.cache_stereo = !self.cache_stereo;
                    }
                }
            }
            PhysicalKey::Code(KeyCode::KeyM) => {
                self.update_panel_texture();
                if let Some(state) = self.state.as_mut() {
                    state.displaying_panel_textures = !state.displaying_panel_textures;
                    if state.displaying_panel_textures {
                        self.toasts.info("Displaying Textures on Panels");
                    } else {
                        self.toasts.info("Panels are transparent");
                    }
                }
            }

            PhysicalKey::Code(KeyCode::KeyN) => {
                if let Some(state) = self.state.as_mut() {
                    state.distort_rays = !state.distort_rays;

                    if state.distort_rays {
                        self.toasts.info("Applying nearest neighbour approximation");
                    } else {
                        self.toasts.info("No NN approximation");
                    }
                }
            }

            PhysicalKey::Code(KeyCode::KeyO) => {
                self.print_compute();
            }
            _ => (),
        }
    }
    pub fn print_compute(&self) {
        if let Some(state) = &self.state {
            state.print_compute();
        }
    }

    pub fn next_camera(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.camera_history.next_save().unwrap();
        }
    }
    pub fn play_animation(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.camera_history.animate_camera();
        }
    }
    pub fn previous_camera(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.camera_history.previous_save();
        }
    }

    // Take the new texture and queue an update
    pub fn update_texture(&mut self) {
        let state = self.state.as_mut().unwrap();

        let target = &mut state.scene.world;
        if !target.texture.change_file {
            return;
        }

        target.texture.change_file = false;

        let img = target.texture.load_texture();
        state.image_cache.load_world(img);
        state.update_target_texture();

        //self.nmf_solver.reset();
    }

    /// This is highly inefficient, as it uses the os as the between layer. We can probably do
    /// better, but will leave it be for now
    pub fn update_panel_texture(&mut self) {
        let state = self.state.as_mut().unwrap();

        for x in 0..=1 {
            let panel = &state.scene.panels[x];
            if !panel.texture.change_file {
                continue;
            }
            let path = &panel.texture.texture_file;

            let file = File::open(path).unwrap();
            if file.metadata().unwrap().is_file() {
                let img = image::ImageReader::open(path).unwrap().decode().unwrap();
                state.image_cache.cache_panel(x, img);
                state.update_panel(x);
            }
            state.scene.panels[x].texture.change_file = false;
        }
    }

    async fn set_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 2000;
        let initial_height = 2000;

        let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));

        let surface = self
            .instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");

        let mut state = AppState::new(
            &self.instance,
            surface,
            Some(&window),
            initial_width,
            initial_width,
        )
        .await;

        state.camera_history.save_point();
        // Have one compute pass ready to go.
        state.verify_m_a();
        //state.rev_proj.print_image(&state.device);
        state.camera_history.reset();

        self.window.get_or_insert(window);
        self.state.get_or_insert(state);
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.state.as_mut().unwrap().resize_surface(width, height);
        }
    }

    fn handle_redraw(&mut self) {
        // Attempt to handle minimizing window
        if let Some(window) = self.window.as_ref() {
            if let Some(min) = window.is_minimized() {
                if min {
                    println!("Window is minimized");
                    return;
                }
            }
        }

        self.update_texture();
        self.play_animation();

        //self.update_panel_texture();
        //

        let state = self.state.as_mut().unwrap();
        state.save();

        if state.factorizer.will_solve() {
            state.solver_light_field();
            state.displaying_panel_textures = true;
            state.factorizer.has_solved();
        }

        if state.stereoscope.will_solve() {
            state.solve_stereo();
            state.displaying_panel_textures = true;
            state.stereoscope.has_solved();
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [
                state.surface_config.as_ref().unwrap().width,
                state.surface_config.as_ref().unwrap().height,
            ],
            pixels_per_point: self.window.as_ref().unwrap().scale_factor() as f32
                * state.scale_factor,
        };

        let surface_texture = state.surface.as_ref().unwrap().get_current_texture();

        match surface_texture {
            Err(SurfaceError::Outdated) => {
                // Ignoring outdated to allow resizing and minimization
                println!("wgpu surface outdated");
                return;
            }
            Err(_) => {
                surface_texture.expect("Failed to acquire next swap chain texture");
                return;
            }
            Ok(_) => {}
        };

        let surface_texture = surface_texture.unwrap();

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let window = self.window.as_ref().unwrap();

        // Order of passes matters!
        // The render pass
        {
            state.scene.update_rt_info(
                &state.camera_history.current_camera,
                state.surface_config.as_ref().unwrap().height,
                state.surface_config.as_ref().unwrap().width,
            );
            state.scene.update_draw(
                &state.queue,
                &state.camera_history.current_camera,
                state.displaying_panel_textures,
                state.distort_rays,
            );

            state.camera_history.update_buffer(&state.queue);

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &state.headless.texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&state.render_pipe.as_ref().unwrap());
            // Pass uniform!
            state.scene.render_pass(&mut render_pass);
            render_pass.set_bind_group(3, &state.factorizer.bind_group, &[]);
            //render_pass.set_bind_group(4, &state.headless.bind_group, &[]);
            // Takes 2 params, as you might pass multiple vertex buffers
            render_pass.set_vertex_buffer(0, state.vertex_buffer.as_ref().unwrap().slice(..));
            render_pass.set_index_buffer(
                state.index_buffer.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..state.num_index, 0, 0..1);
        }

        {
            state.headless.draw_pass(&mut encoder);
        }

        state.egui_renderer.as_mut().unwrap().begin_frame(window);

        let context = state.egui_renderer.as_ref().unwrap().context();

        self.toasts.show(context);
        // TODO: Make this slightly more elegant!
        // the ui pass
        {
            state.scene.draw_ui(context, None, None);
            state.factorizer.draw_ui(context, None, None);
            state.stereoscope.draw_ui(context, None, None);
            state.camera_history.draw_ui(context, None, None);
            state.save_manager.draw_ui(context, None, None);

            state.egui_renderer.as_mut().unwrap().end_frame_and_draw(
                &state.device,
                &state.queue,
                &mut encoder,
                window,
                &surface_view,
                screen_descriptor,
            );
        }
        {
            //state.compute_pass(&mut encoder);
        }

        state.queue.submit(Some(encoder.finish()));
        surface_texture.present();

        if state.headless.retrieve_image {
            state.headless.print_image(&state.device);
        }
    }
}

impl ApplicationHandler<FileWatcher> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window = event_loop
                .create_window(Window::default_attributes())
                .unwrap();
            pollster::block_on(self.set_window(window));
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: FileWatcher) {
        let _ = event_loop;
        match event {
            FileWatcher::FileChange => {
                if let Some(state) = self.state.as_mut() {
                    state.build_pipeline();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        // let egui render to process the event first
        self.state
            .as_mut()
            .unwrap()
            .egui_renderer
            .as_mut()
            .unwrap()
            .handle_input(self.window.as_ref().unwrap(), &event);

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
                let now = Instant::now();
                let dt = now.duration_since(self.previous_draw);
                if let Some(state) = self.state.as_mut() {
                    state.camera_history.update_camera(dt);
                }
                self.previous_draw = now;

                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resized(new_size.width, new_size.height);
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                self.process_keyboard(&event);
                if let Some(state) = self.state.as_mut() {
                    state
                        .camera_history
                        .process_keyboard(event, self.disable_controls);
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button: MouseButton::Right,
            } => {
                self.mouse_press = state == ElementState::Pressed;
            }
            _ => (),
        }
    }

    #[allow(unused_variables)]
    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.mouse_press && !self.mouse_on_ui {
                if let Some(state) = self.state.as_mut() {
                    state
                        .camera_history
                        .camera_control
                        .process_mouse(delta.0, delta.1);
                }
            }
        }
    }
}
