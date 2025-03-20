use egui_glium::egui_winit::egui::ahash::HashMap;
use egui_glium::egui_winit::egui::mutex::RwLock;
use std::{sync::Arc, time::Instant};

use crate::vertex::{self, ShapeWorld};
use crate::{matrix::ToArr, shader, vertex::Shape};
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, SquareMatrix, Vector3, Vector4};
use egui_glium::egui_winit::egui::ImageData;
use egui_glium::egui_winit::egui::{Color32, ColorImage, TextureOptions};
use glium::winit::application::ApplicationHandler;
use glium::winit::event::WindowEvent;
use glium::Surface;
use glium::{
    glutin::surface::WindowSurface,
    index::NoIndices,
    winit::{
        event::{DeviceEvent, ElementState, MouseButton},
        keyboard::KeyCode,
        window::Window,
    },
    Display, DrawParameters, Program,
};

use crate::camera::{Camera, CameraController, Projection};
use crate::raytracer::Raytracer;

use crate::RAY_WIDTH;
use crate::{RAY_HEIGHT, SAFE_FRAC_PI_2};

// Deal with application State
// RN, only does
pub struct App<'a> {
    window: Window,
    display: Display<WindowSurface>,
    last_step: Instant,
    camera: Camera,
    controller: CameraController,
    projection: Projection,
    shapes: Vec<Shape>,
    // TODO: This might not work for complex shapes!
    indices: NoIndices,
    programs: HashMap<Shaders, Program>,
    draw_params: DrawParameters<'a>,
    ui: egui_glium::EguiGlium,
    mouse_press: bool,
    mouse_on_ui: bool,
    raytracer: Raytracer,
}
#[derive(Eq, Hash, PartialEq)]
pub enum Shaders {
    World,
    Lines,
}

impl App<'_> {
    fn load_shaders(display: &Display<WindowSurface>) -> HashMap<Shaders, Program> {
        let vertex_shader = shader::load_shader("./shaders/vertex.glsl");
        let fragment_shader = shader::load_shader("./shaders/fragment.glsl");

        let program =
            glium::Program::from_source(display, &vertex_shader, &fragment_shader, None).unwrap();
        let mut map: HashMap<Shaders, Program> = HashMap::default();
        map.insert(Shaders::World, program);

        let vertex_shader = shader::load_shader("./shaders/l_vertex.glsl");
        let fragment_shader = shader::load_shader("./shaders/l_fragment.glsl");

        let program =
            glium::Program::from_source(display, &vertex_shader, &fragment_shader, None).unwrap();
        map.insert(Shaders::Lines, program);

        map
    }

    pub fn new<'a>(
        window: Window,
        display: Display<WindowSurface>,
        ui: egui_glium::EguiGlium,
    ) -> App<'a> {
        let shapes = vec![
            Shape::b(&display),
            Shape::a(&display),
            Shape::f(&display),
            Shape::floor(&display),
        ];
        let programs = Self::load_shaders(&display);

        let camera = Camera::new((0.0, 8.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));

        let projection = Projection::new(
            window.inner_size().width,
            window.inner_size().height,
            cgmath::Deg(90.0),
            0.1,
            100.0,
        );

        let controller = CameraController::new(4.0, 0.4);

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        let raytrace_handler = ui.egui_ctx().load_texture(
            "Rayrender",
            ImageData::Color(Arc::new(ColorImage::new(
                [RAY_WIDTH, RAY_HEIGHT],
                Color32::TRANSPARENT,
            ))),
            TextureOptions::default(),
        );

        let draw_params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            //backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        };
        let last_step = Instant::now();
        let raytracer = Raytracer::new(false, false, "test.png".to_string(), raytrace_handler);

        App {
            window,
            display,
            last_step,
            camera,
            projection,
            controller,
            shapes,
            indices,
            programs,
            draw_params,
            ui,
            mouse_press: false,
            mouse_on_ui: false,
            raytracer,
        }
    }

    pub fn draw_debug(&mut self) {
        let mut frame = self.display.draw();
        frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

        let image_height: u32 = RAY_HEIGHT.try_into().unwrap();
        let image_width: u32 = RAY_WIDTH.try_into().unwrap();

        // CAMERA MATH!

        let d = 2.0;
        let t_n = self.camera.direction_vec();
        let b_n = t_n.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
        let v_n = t_n.cross(b_n).normalize();
        let g_x = (SAFE_FRAC_PI_2 / 2.0).tan() * d;
        // IMPORTANT: Might have width and height confused?

        // straight from the wikipedia article on raytracing
        let g_y = g_x * ((image_height as f32 - 1.0) / (image_width as f32 - 1.0));
        let q_x = (2.0 * g_x) / (image_height as f32 - 1.0) * b_n;
        let q_y = (2.0 * g_y) / (image_width as f32 - 1.0) * v_n;
        let p_1_m = t_n * d - g_x * b_n - g_y * v_n;

        let ray_origin = self.camera.position.to_vec();

        //let view_proj = self.projection.calc_matrix() * self.camera.calc_matrix();

        //let ub = &self.shapes[2].world.read().to_uniform_buffer();
        let triangle = &self.shapes[3].world.read().first_triangle();
        let a: [f32; 3] = triangle[0].to_vec().to_arr();
        let b: [f32; 3] = triangle[1].to_vec().to_arr();
        let c: [f32; 3] = triangle[2].to_vec().to_arr();
        let vertex_buffer = vertex::gross_method();
        let buffer = glium::VertexBuffer::new(&self.display, &vertex_buffer).unwrap();
        let matrix = Matrix4::<f32>::identity();
        let shape = &self.shapes[0];

        let uniforms = uniform! {
            model: matrix.to_arr(),
            ray_origin: ray_origin.to_arr(),
            p_1_m: p_1_m.to_arr(),
            q_x: q_x.to_arr(),
            q_y: q_y.to_arr(),
            tex: &shape.texture,
            view_proj: matrix.to_arr(),
            a: a,
            b: b,
            c: c,
        };

        frame
            .draw(
                &buffer,
                self.indices,
                self.programs.get(&Shaders::World).unwrap(),
                &uniforms,
                &self.draw_params,
            )
            .unwrap();

        for shape in self.shapes.iter_mut() {
            // lock shape:
            let mut world = shape.world.write();

            let scale_matrix: Matrix4<f32> = Matrix4::from_cols(
                Vector4::new(world.ui_state.size.0.value, 0.0, 0.0, 0.0),
                Vector4::new(0.0, world.ui_state.size.1.value, 0.0, 0.0),
                Vector4::new(0.0, 0.0, 1.0, 0.0),
                Vector4::new(0.0, 0.0, 0.0, 1.0),
            );
            let matrix = Matrix4::<f32>::from_translation(Vector3::new(
                0.0,
                0.0,
                world.ui_state.distance.value,
            )) * world.model_matrix
                * scale_matrix;
            world.placement_matrix = matrix;

            // Super gross
            //matrix = Matrix4::identity();

            //  let uniforms = uniform! {
            //      model: matrix.to_arr(),
            //      ray_origin: ray_origin.to_arr(),
            //      p_1_m: p_1_m.to_arr(),
            //      q_x: q_x.to_arr(),
            //      q_y: q_y.to_arr(),
            //      tex: &shape.texture,
            //      view_proj: matrix.to_arr(),
            //      world: ub,
            //  };
            //  // take up screen space!
            //  let vertex_buffer = vertex::gross_method();

            //  let buffer = glium::VertexBuffer::new(&self.display, &vertex_buffer).unwrap();

            //  frame
            //      .draw(
            //          &buffer,
            //          self.indices,
            //          self.programs.get(&Shaders::World).unwrap(),
            //          &uniforms,
            //          &self.draw_params,
            //      )
            //      .unwrap();
        }
        // let program = self.programs.get(&Shaders::Lines).unwrap();

        // self.raytracer.rasterize_debug(
        //     &mut frame,
        //     &self.draw_params,
        //     program,
        //     view_proj,
        //     &self.display,
        // );

        // Paint the UI
        self.ui.paint(&self.display, &mut frame);
        frame.finish().unwrap();
        // How long has passed?
        let now = Instant::now();
        let dt = now.duration_since(self.last_step);
        self.last_step = now;
        // Update camera
        self.controller.update_camera(&mut self.camera, dt);
    }

    pub fn define_ui(&mut self) {
        let window = &self.window;
        self.ui.run(window, |ctx| {
            egui_extras::install_image_loaders(ctx);

            for shape in self.shapes.iter_mut() {
                let mut world = shape.world.write();
                world.ui_state.define_ui(ctx);
            }
            self.raytracer.ui_draw(ctx);
        });
    }

    pub fn raytrace(&mut self) {
        let t_n = self.camera.direction_vec();

        let ray_origin = self.camera.position;
        let shapes: Vec<Arc<RwLock<ShapeWorld>>> =
            self.shapes.iter().map(|x| x.world.clone()).collect();
        self.raytracer.raytrace(t_n, ray_origin, shapes);
    }
}

impl ApplicationHandler for App<'_> {
    // Exists for android. also called on start up
    #[allow(unused_variables)]
    fn resumed(&mut self, event_loop: &glium::winit::event_loop::ActiveEventLoop) {
        //self.window = Some(event_loop.create_window(Window::default_attributes()).unwrap());
    }

    #[allow(unused_variables)]
    fn window_event(
        &mut self,
        event_loop: &glium::winit::event_loop::ActiveEventLoop,
        window_id: glium::winit::window::WindowId,
        event: glium::winit::event::WindowEvent,
    ) {
        let window = &self.window;
        let _ = self.ui.on_event(window, &event);
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(window_size) => {
                self.projection
                    .resize(window_size.width, window_size.height);
                self.display.resize(window_size.into());
            }

            WindowEvent::RedrawRequested => {
                self.define_ui();
                // Draw.
                //
                self.draw_debug();

                self.window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                if event.state == ElementState::Pressed && !self.mouse_on_ui {
                    if let glium::winit::keyboard::PhysicalKey::Code(KeyCode::KeyR) =
                        event.physical_key
                    {
                        self.raytrace();
                    }
                }

                if !self.mouse_on_ui {
                    self.controller.process_keyboard(event);
                }
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button: MouseButton::Right,
            } => {
                self.mouse_press = state == ElementState::Pressed;
            }

            _ => {}
        }
    }

    #[allow(unused_variables)]
    fn device_event(
        &mut self,
        event_loop: &glium::winit::event_loop::ActiveEventLoop,
        device_id: glium::winit::event::DeviceId,
        event: glium::winit::event::DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.mouse_press && !self.mouse_on_ui {
                self.controller.process_mouse(delta.0, delta.1);
            }
        }
    }
}
