use std::{sync::Arc,   time::Instant};

use cgmath::{InnerSpace, Matrix4, Vector3, Vector4};
use egui_glium::egui_winit::egui::{self, Align2, Color32, ColorImage, TextureHandle, TextureOptions, Vec2};
use egui_glium::egui_winit::egui::ImageData;
use glium::{glutin::surface::WindowSurface, index::NoIndices, winit::{event::{DeviceEvent, ElementState, MouseButton}, keyboard::KeyCode, window::Window}, Display, DrawParameters, Program};
use glium::winit::application::ApplicationHandler;
use ::image::ImageBuffer;
use crate::{matrix::ToArr, shader, vertex:: Shape};
use glium::Surface;
use glium::winit::event::WindowEvent;


use crate::camera::{Camera, CameraController, Projection};

use std::f32::consts::FRAC_PI_2;
const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

const RAY_HEIGHT: usize = 500;
const RAY_WIDTH: usize = 500;

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
    program: Program,
    draw_params: DrawParameters<'a>,
    ui: egui_glium::EguiGlium,
    mouse_press: bool,
    mouse_on_ui: bool,
    ray_trace_display: bool,
    ray_tace_file_name: String,
    raytrace_handler: TextureHandle,


}


impl App<'_> {
    pub fn new<'a>(window: Window, display: Display<WindowSurface>, ui: egui_glium::EguiGlium) -> App<'a> {


        let shapes = vec![Shape::b(&display), Shape::a(&display), Shape::f(&display), Shape::floor(&display),  ];

        let vertex_shader = shader::load_shader("./shaders/vertex.glsl");
        let fragment_shader = shader::load_shader("./shaders/fragment.glsl");


        let camera = Camera::new((0.0, 8.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection = Projection::new(window.inner_size().width, window.inner_size().height, cgmath::Deg(45.0), 0.1, 100.0);
        let controller = CameraController::new(4.0, 0.4);

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let program = glium::Program::from_source(&display, &vertex_shader, &fragment_shader, None).unwrap();

        let raytrace_handler = ui.egui_ctx().load_texture("Rayrender", 
            ImageData::Color(Arc::new(ColorImage::new([RAY_WIDTH, RAY_HEIGHT], Color32::TRANSPARENT))),
            TextureOptions::default());

        let draw_params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            //backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            .. Default::default()
        };
        let last_step = Instant::now();

        App{window, display, last_step,  camera, projection, controller,  shapes, indices, program, draw_params, ui,  mouse_press: false, mouse_on_ui: false, ray_trace_display: false, ray_tace_file_name: "test.png".to_string(), raytrace_handler}
    }
    pub fn draw_debug(&mut self) {

        let mut frame = self.display.draw();
        frame.clear_color_and_depth((0.0, 0.0,1.0 , 1.0), 1.0);

        for shape in self.shapes.iter_mut() {

            let scale_matrix: Matrix4<f32> = Matrix4::from_cols(
                Vector4::new(shape.ui_state.size.0, 0.0, 0.0, 0.0), 
                Vector4::new(0.0, shape.ui_state.size.1, 0.0, 0.0),
                Vector4::new(0.0, 0.0, 1.0, 0.0),
                Vector4::new(0.0, 0.0, 0.0, 1.0),
            );
            let matrix = Matrix4::<f32>::from_translation(Vector3::new(0.0,0.0, shape.ui_state.distance))* shape.model_matrix * scale_matrix;
            shape.placement_matrix = matrix;


            let view_proj = self.projection.calc_matrix() * self.camera.calc_matrix();


            let uniforms = uniform!{
                model: matrix.to_arr(),
                tex: &shape.texture,
                view_proj: view_proj.to_arr(),
            };
            let buffer = glium::VertexBuffer::new(&self.display, &shape.vertex_buffer).unwrap();

            frame.draw(&buffer, self.indices, &self.program, &uniforms,&self.draw_params).unwrap();


        }


        // Testing out the floor before doing anything wacky with it


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
                shape.ui_state.define_ui(ctx);
            }


                egui::Window::new("RAY TRACER").anchor(Align2::RIGHT_TOP, Vec2::new(1.0,1.0)).default_open(false).show(ctx, |ui| {
                    ui.add(
                        egui::Image::new(&self.raytrace_handler)
                    );
                    ui.checkbox(&mut self.ray_trace_display, "Save as file");
                    ui.text_edit_singleline(&mut self.ray_tace_file_name);
                });

        } );
    }
    pub fn raytrace(&mut self) {
        // Rebiuld the world?
        // Eye Position
        // Target position
        // FOV (90 degrees)
        // number of quare pixels on the viewport on each direction
        // Vector which indicates where up is (given by camera)
        //
        // Target image, by pixel
        // Distance between camera and plane?
        //
        //println!("Starting Ray trace");
        let image_height: u32 = RAY_HEIGHT.try_into().unwrap();
        let image_width: u32 = RAY_WIDTH.try_into().unwrap();

        // CAMERA MATH!

        let d = 1.0;
        let t_n = self.camera.direction_vec();
        let b_n = t_n.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
        let v_n = t_n.cross(b_n).normalize();
        let g_x = (SAFE_FRAC_PI_2 / 2.0).tan() * d;
        // IMPORTANT: Might have width and height confused?

        let g_y = g_x * ((image_height as f32 - 1.0) / (image_width as f32 - 1.0) );
        let q_x = (2.0 * g_x) / (image_height as f32 - 1.0) * b_n;
        let q_y = (2.0 * g_y) / (image_width as f32 - 1.0) * v_n;
        let p_1_m = t_n * d - g_x * b_n - g_y * v_n;

        let buf = ImageBuffer::from_fn(image_width, image_height, |x, y| {

            let f_y = y as f32;
            let f_x = x as f32;
            let ray_dir = (p_1_m + q_x*(f_x - 1.0) + q_y *(f_y - 1.0)).normalize();
            let ray_origin = self.camera.position;
            let  color = [0_u8, 0_u8, 0_u8, 0_u8];
            // TODO: Right now the order of the shapes matters! Need to change that
            for shape in self.shapes.iter() {
                let intersect = shape.intersect(ray_origin, ray_dir);
                if let Some((color, _point)) = intersect {
                    return image::Rgba(color);

                }

            }

            image::Rgba(color)
        });
        if self.ray_trace_display {
            let res = buf.save_with_format(self.ray_tace_file_name.clone(), image::ImageFormat::Png);
            
            if res.is_err() {
                println!("Could not write to file? {:?}", res);
            }
        }

        let raw = ColorImage::from_rgba_unmultiplied([RAY_WIDTH, RAY_HEIGHT], &buf.into_raw());
        self.raytrace_handler.set(raw, TextureOptions::default());
        self.ray_trace_display = true;
        println!("Rays traced!");
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
            },
            WindowEvent::Resized(window_size) => {
                self.projection.resize(window_size.width, window_size.height);
                self.display.resize(window_size.into());


            }

            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.


                self.define_ui();
                // Draw.
                //
                self.draw_debug();



                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.request_redraw();
            }
            WindowEvent::KeyboardInput { device_id, event, is_synthetic } => {
                if event.state == ElementState::Pressed && !self.mouse_on_ui {
                    if let glium::winit::keyboard::PhysicalKey::Code(KeyCode::KeyR) = event.physical_key{
                        self.raytrace();
                    }
                }
                
                if !self.mouse_on_ui{
                    self.controller.process_keyboard(event);
                }

            }
            WindowEvent::MouseInput { device_id, state, button: MouseButton::Right } => {
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
            if self.mouse_press && !self.mouse_on_ui{
                self.controller.process_mouse(delta.0, delta.1);
            }
        }

    }
}
