use crate::camera::{Camera, CameraController};
use crate::egui_tools::EguiRenderer;
use crate::file_picker::FilePicker;
use crate::matrix::{vector_to_image, NmfSolver};
use crate::raytracer::RayTraceInfo;
use crate::scene::{DrawUI, Scene};
use crate::shape::Quad;
use crate::texture::Texture;
use crate::{matrix, texture, vertex};
use crevice::std140::{AsStd140, Std140};
use egui::ahash::{HashMap, HashMapExt};
use egui_wgpu::wgpu::SurfaceError;
use egui_wgpu::{wgpu, ScreenDescriptor};
use image::GenericImageView;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, RenderPipeline};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

type BindNumber = usize;
// Struct to deal with all the drawing. This is where all the setting live
pub struct AppState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub scale_factor: f32,
    pub egui_renderer: EguiRenderer,
    pub render_pipe: RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub bind_map: HashMap<BindNumber, BindGroup>,
    pub num_index: u32,
    pub buffer_map: HashMap<BindNumber, Buffer>,
    pub scene: Scene,
    pub texture: Texture,
    pub panel_textures: Texture,
}

impl AppState {
    async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: &Window,
        width: u32,
        height: u32,
        camera: &Camera,
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

        let features =
            wgpu::Features::VERTEX_WRITABLE_STORAGE | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: features,
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Failed to create device");

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

        //
        let texture_bytes = include_bytes!("../resources/textures/High res text.png");

        let texture = texture::Texture::from_bytes(&device, &queue, texture_bytes, "Damn");

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filter﻿able field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let text_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Size of texture being passed!"),
            contents: texture.dimensions.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: text_size_buffer.as_entire_binding(),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        // Panel group

        // Buffers to pass info?
        let scene = Scene::test();

        let imag_height = surface_config.height;
        let image_width = surface_config.width;
        let rt_test = RayTraceInfo::test(camera, imag_height, image_width);

        let rt_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ray trace buffer, contains all objects"),
            contents: rt_test.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer for Scene, contains all objects"),
            contents: &scene.world_as_bytes(camera),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let scene_bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
            ],
            label: Some("Binding group for Scene"),
        });

        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Bind"),
            layout: &scene_bind_group,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scene_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rt_buffer.as_entire_binding(),
                },
            ],
        });

        let panel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer for Panels"),
            contents: &scene.panels_as_bytes(camera),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let panel_textures = Self::panel_textures_set_up(&device, &queue);

        let panel_bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filter﻿able field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Binding group for Scene"),
        });

        let panel_textures_size_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Size of texture being passed!"),
                contents: panel_textures.dimensions.as_std140().as_bytes(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let panel_bool_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boolean for panel textures"),
            contents: 0u32.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let panel_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binding For Panel group"),
            layout: &panel_bind_group,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: panel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: panel_bool_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&panel_textures.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&panel_textures.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: panel_textures_size_buffer.as_entire_binding(),
                },
            ],
        });

        // Bind group for reading from
        //
        let sampler_bind_layout = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };

        let sampler_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind group for Sampler"),
                entries: &[sampler_bind_layout],
            });
        let sampler_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer for sample"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let sampler_double_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Second buffer for sampler. Only exists to be copied into"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST,
        });

        let sampler_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind group for Sampler"),
            layout: &sampler_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sampler_buffer.as_entire_binding(),
            }],
        });

        let mut bind_map = HashMap::new();
        bind_map.insert(0, scene_bind);
        bind_map.insert(1, diffuse_bind_group);
        bind_map.insert(2, panel_bind);
        bind_map.insert(3, sampler_bind);

        // TODO: Might replace this with enums?
        // Make a bind group struct, go for greater generality
        let mut buffer_map = HashMap::new();
        buffer_map.insert(0, scene_buffer);
        buffer_map.insert(1, rt_buffer);
        buffer_map.insert(2, panel_buffer);
        buffer_map.insert(3, sampler_buffer);
        buffer_map.insert(4, sampler_double_buffer);
        buffer_map.insert(5, panel_bool_buffer);

        let egui_renderer = EguiRenderer::new(&device, surface_config.format, None, 1, window);

        let scale_factor = 1.0;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/simple.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &scene_bind_group,
                    &texture_bind_group_layout,
                    &panel_bind_group,
                    &sampler_group_layout,
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
                targets: &[Some(wgpu::ColorTargetState {
                    // 4. // What color outputs it should set up!
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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

        Self {
            device,
            queue,
            surface,
            surface_config,
            egui_renderer,
            scale_factor,
            render_pipe,
            vertex_buffer,
            index_buffer,
            bind_map,
            num_index: index_list.len() as u32,
            buffer_map,
            scene,
            texture,
            panel_textures,
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn panel_textures_set_up(device: &wgpu::Device, queue: &wgpu::Queue) -> Texture {
        let panel_1_texture_bytes = include_bytes!("../resources/panel_compute/panel_1.png");
        let panel_2_texture_bytes = include_bytes!("../resources/panel_compute/panel_2.png");

        let panel_1 = image::load_from_memory(panel_1_texture_bytes).unwrap();
        let panel_2 = image::load_from_memory(panel_2_texture_bytes).unwrap();
        let texture_vec = vec![panel_1, panel_2];

        texture::Texture::from_images(device, queue, &texture_vec, Some("2D Panel Array"))
    }
}

// Handles the drawing and the app logic
pub struct App {
    instance: wgpu::Instance,
    state: Option<AppState>,
    window: Option<Arc<Window>>,
    camera: Camera,
    camera_control: CameraController,
    file_picker: FilePicker,
    previous_draw: Instant,
    nmf_solver: NmfSolver,
    mouse_press: bool,
    mouse_on_ui: bool,
    disable_controls: bool,
    sampling_light_field: bool,
    displaying_panel_textures: bool,
}

impl App {
    pub fn new() -> Self {
        let instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let file_picker = FilePicker::new();
        Self {
            instance,
            nmf_solver: NmfSolver::new(),
            sampling_light_field: false,
            mouse_press: false,
            mouse_on_ui: false,
            disable_controls: false,
            displaying_panel_textures: false,
            state: None,
            window: None,
            camera: Camera::new(
                (0.0, 2.0, 4.0),
                cgmath::Deg(-90.0),
                cgmath::Deg(-20.0),
                cgmath::Deg(45.0),
            ),
            camera_control: CameraController::new(4.0, 1.0),
            previous_draw: Instant::now(),
            file_picker,
        }
    }

    pub fn process_keyboard(&mut self, event: &KeyEvent) {
        if !(event.state == ElementState::Pressed) || self.mouse_on_ui {
            return;
        }
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Minus) => {
                self.disable_controls = !self.disable_controls;

                if self.disable_controls {
                    println!("Controls Disabled!");
                } else {
                    println!("Controls Enabled");
                }
            }
            PhysicalKey::Code(KeyCode::Slash) => {
                println!("DEBUG KEY PRESSED");

                let device_ref = &self.state.as_ref().unwrap().device;
                let queue = &self.state.as_ref().unwrap().queue;
                pollster::block_on(matrix::nmf_pipeline(device_ref, queue));
            }
            PhysicalKey::Code(KeyCode::Comma) => {
                self.get_sample_light_field();
            }
            PhysicalKey::Code(KeyCode::KeyM) => {
                println!("DEBUGGING PANEL ON");
                self.update_panel_texture(true);
                self.displaying_panel_textures = !self.displaying_panel_textures;
            }
            PhysicalKey::Code(KeyCode::KeyB) => {
                self.update_panel_texture(false);
                self.displaying_panel_textures = !self.displaying_panel_textures;
            }
            _ => (),
        }
    }
    pub fn get_sample_light_field(&mut self) -> Result<(), String> {
        self.sampling_light_field = true;
        let state = self.state.as_ref().unwrap();
        let sample_buffer = state.buffer_map.get(&4).unwrap();
        //sample_buffer.unmap();
        //let

        let buffer_slice = sample_buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        state.device.poll(wgpu::Maintain::Wait);
        pollster::block_on(rx.receive()).unwrap().unwrap();
        // Scope to drop buffer view, ensuring we can unmap it
        {
            let data = buffer_slice.get_mapped_range();

            let data_filtered: Vec<f32> = data
                .chunks(4)
                .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            let triplets = data_filtered
                .chunks(3)
                .filter_map(|chunk| {
                    let x_coord = chunk[0];
                    let y_coord = chunk[1];
                    let sample = chunk[2];
                    if sample > 0.0 {
                        Some((x_coord, y_coord, sample))
                    } else {
                        None
                    }
                })
                .collect::<Vec<(f32, f32, f32)>>();
            let max = triplets
                .iter()
                .fold(0.0f32, |acc, next| if acc > next.2 { acc } else { next.2 });
            println!("Max Value is: {}", max);
            println!("Triplet count: {}", triplets.len());
            let size = self.state.as_ref().unwrap().scene.panel_size();
            self.nmf_solver.add_sample(triplets, size);
        }
        sample_buffer.unmap();

        self.sampling_light_field = false;

        Ok(())
    }

    // Take the new texture and queue an update
    pub fn update_texture(&mut self) {
        let state = self.state.as_mut().unwrap();
        let path = self.file_picker.texture_file.clone();
        let file = File::open(&path).unwrap();
        if file.metadata().unwrap().is_file() {
            //let reader = std::io::BufReader::new(file);
            let img = image::ImageReader::open(path).unwrap().decode().unwrap();

            self.nmf_solver.reset();

            // Ensure you are of the same size??
            let img = img.resize_to_fill(
                state.texture.dimensions.x - 1,
                state.texture.dimensions.y - 1,
                image::imageops::FilterType::Nearest,
            );

            let copy = state.texture.texture.as_image_copy();
            let dimensions = img.dimensions();
            state.queue.write_texture(
                copy,
                &img.to_rgba8(),
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
    }

    pub fn update_panel_texture(&mut self, debug: bool) {
        let state = self.state.as_mut().unwrap();
        for x in 1..3 {
            let path_string = format!(
                "./resources/panel_compute/panel_{}{}.png",
                x,
                if debug { "_debug" } else { "" }
            );
            let path = Path::new(path_string.as_str());

            let file = File::open(path).unwrap();
            if file.metadata().unwrap().is_file() {
                let img = image::ImageReader::open(path).unwrap().decode().unwrap();

                // Ensure you are of the same size??
                let img = img.resize_to_fill(
                    state.panel_textures.dimensions.x,
                    state.panel_textures.dimensions.y,
                    image::imageops::FilterType::Nearest,
                );

                let dimensions = img.dimensions();

                state.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        aspect: wgpu::TextureAspect::All,
                        texture: &state.panel_textures.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: x - 1,
                        },
                    },
                    &img.to_rgba8(),
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

        let state = AppState::new(
            &self.instance,
            surface,
            &window,
            initial_width,
            initial_width,
            &self.camera,
        )
        .await;

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
        if self.file_picker.change_file {
            self.update_texture();
            self.file_picker.change_file = false;
        }

        let state = self.state.as_mut().unwrap();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [state.surface_config.width, state.surface_config.height],
            pixels_per_point: self.window.as_ref().unwrap().scale_factor() as f32
                * state.scale_factor,
        };

        let surface_texture = state.surface.get_current_texture();

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
            let rt_test = RayTraceInfo::test(
                &self.camera,
                state.surface_config.height,
                state.surface_config.width,
            );
            state.queue.write_buffer(
                state.buffer_map.get(&1).unwrap(),
                0,
                rt_test.as_std140().as_bytes(),
            );
            state.queue.write_buffer(
                state.buffer_map.get(&0).unwrap(),
                0,
                &state.scene.world_as_bytes(&self.camera),
            );
            state.queue.write_buffer(
                state.buffer_map.get(&2).unwrap(),
                0,
                &state.scene.panels_as_bytes(&self.camera),
            );
            if self.displaying_panel_textures {
                state.queue.write_buffer(
                    state.buffer_map.get(&5).unwrap(),
                    0,
                    1.as_std140().as_bytes(),
                );
            } else {
                state.queue.write_buffer(
                    state.buffer_map.get(&5).unwrap(),
                    0,
                    0.as_std140().as_bytes(),
                );
            }
            // Need to get: The texture being passed to wgpu, and the new data.
            // Should not be called every time, that is ineffective. Instead, as response to
            // Changes
            // Change texture
            //
            //state.queue.write_texture(state.texture.texture.as_image_copy(), data, data_layout, size);

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
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
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&state.render_pipe);
            // Pass uniform!
            render_pass.set_bind_group(0, state.bind_map.get(&0), &[]);
            render_pass.set_bind_group(1, state.bind_map.get(&1), &[]);

            render_pass.set_bind_group(2, state.bind_map.get(&2), &[]);
            render_pass.set_bind_group(3, state.bind_map.get(&3), &[]);
            // Takes 2 params, as you might pass multiple vertex buffers
            render_pass.set_vertex_buffer(0, state.vertex_buffer.slice(..));
            render_pass.set_index_buffer(state.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..state.num_index, 0, 0..1);
        }

        // TODO: Make this slightly more elegant!
        // the ui pass
        {
            state.egui_renderer.begin_frame(window);
            let context = state.egui_renderer.context();

            state.scene.draw_ui(context, None);
            self.camera.draw_ui(context, None);
            self.file_picker.draw_ui(context, None);
            self.nmf_solver.draw_ui(context, None);

            state.egui_renderer.end_frame_and_draw(
                &state.device,
                &state.queue,
                &mut encoder,
                window,
                &surface_view,
                screen_descriptor,
            );
        }
        if !self.sampling_light_field {
            // Can copy
            let source = state.buffer_map.get(&3).unwrap();
            let destination = state.buffer_map.get(&4).unwrap();
            encoder.copy_buffer_to_buffer(source, 0, destination, 0, 2560 * 1600 * 4 * 3);
        }

        state.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        pollster::block_on(self.set_window(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        // let egui render to process the event first
        self.state
            .as_mut()
            .unwrap()
            .egui_renderer
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
                self.camera_control.update_camera(&mut self.camera, dt);
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
                self.camera_control
                    .process_keyboard(event, self.disable_controls);
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
                self.camera_control.process_mouse(delta.0, delta.1);
            }
        }
    }
}
