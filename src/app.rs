use crate::camera::{Camera, CameraController};
use crate::egui_tools::EguiRenderer;
use crate::raytracer::RaytraceTest;
use crate::scene::{DrawUI, Scene};
use crate::shape::{Quad, VWPanel};
use crate::{texture, vertex};
use crevice::std140::{AsStd140, Std140};
use egui::ahash::{HashMap, HashMapExt};
use egui_wgpu::wgpu::SurfaceError;
use egui_wgpu::{wgpu, ScreenDescriptor};
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, RenderPipeline};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
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

        let features = wgpu::Features::empty();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: features,
                    required_limits: Default::default(),
                    memory_hints: Default::default(),
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
        let texture_bytes = include_bytes!("../resources/textures/Mandril.jpg");

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
                        // This should match the filterable field of the
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
        let rt_test = RaytraceTest::test(camera, imag_height, image_width);

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

        let panel_bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::all(),
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            }],
            label: Some("Binding group for Scene"),
        });

        let panel_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Bind"),
            layout: &panel_bind_group,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: panel_buffer.as_entire_binding(),
            }],
        });

        let mut bind_map = HashMap::new();
        bind_map.insert(0, scene_bind);
        bind_map.insert(1, diffuse_bind_group);
        bind_map.insert(2, panel_bind);

        // TODO: Might replace this with enums?
        let mut buffer_map = HashMap::new();
        buffer_map.insert(0, scene_buffer);
        buffer_map.insert(1, rt_buffer);
        buffer_map.insert(2, panel_buffer);

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
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

// Handles the drawing and the app logic
pub struct App {
    instance: wgpu::Instance,
    state: Option<AppState>,
    window: Option<Arc<Window>>,
    camera: Camera,
    camera_control: CameraController,
    previous_draw: Instant,
    mouse_press: bool,
    mouse_on_ui: bool,
}

impl App {
    pub fn new() -> Self {
        let instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        Self {
            instance,
            mouse_press: false,
            mouse_on_ui: false,
            state: None,
            window: None,
            camera: Camera::new((0.0, 2.0, 4.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0)),
            camera_control: CameraController::new(4.0, 1.0),
            previous_draw: Instant::now(),
        }
    }

    async fn set_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 1360;
        let initial_height = 768;

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
            let rt_test = RaytraceTest::test(
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

            state.egui_renderer.end_frame_and_draw(
                &state.device,
                &state.queue,
                &mut encoder,
                window,
                &surface_view,
                screen_descriptor,
            );
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
                self.camera_control.process_keyboard(event);
            }
            WindowEvent::MouseInput {
                device_id,
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
