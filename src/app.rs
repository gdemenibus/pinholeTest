use std::{ str::FromStr, time::Instant};

use cgmath::{Matrix4, SquareMatrix, Vector3};
use egui_glium::egui_winit::egui::{self, Align2};
use glium::{glutin::surface::WindowSurface, index::NoIndices, winit::{application::ApplicationHandler, event::{DeviceEvent, ElementState, KeyEvent, MouseButton}, window::Window}, Display, DrawParameters, Program, Texture2d, VertexBuffer};
use crate::{matrix::ToArr, shader, vertex::{self, f, floor, Shape, Vertex}};
use glium::Surface;
use glium::winit::event::WindowEvent;


use crate::{camera::{Camera, CameraController, Projection}, texture};

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

}


impl App<'_> {
    pub fn new<'a>(window: Window, display: Display<WindowSurface>, ui: egui_glium::EguiGlium) -> App<'a> {


        let shapes = vec![floor(&display), f(&display)];

        let vertex_shader = shader::load_shader("./shaders/vertex.glsl");
        let fragment_shader = shader::load_shader("./shaders/fragment.glsl");


        let camera = Camera::new((0.0, 8.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection = Projection::new(window.inner_size().width, window.inner_size().height, cgmath::Deg(45.0), 0.1, 100.0);
        let controller = CameraController::new(4.0, 0.4);

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let program = glium::Program::from_source(&display, &vertex_shader, &fragment_shader, None).unwrap();

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

        App{window, display, last_step,  camera, projection, controller,  shapes, indices, program, draw_params, ui,  mouse_press: false, mouse_on_ui: false}
    }
    pub fn draw_debug(&mut self) {

        let mut frame = self.display.draw();
        frame.clear_color_and_depth((0.0, 0.0,1.0 , 1.0), 1.0);

        for shape in self.shapes.iter() {
            let matrix = Matrix4::<f32>::from_translation(Vector3::new(0.0,0.0, shape.ui_state.distance)) * shape.model_matrix;

            
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
            for shape in self.shapes.iter_mut() {
                
                shape.ui_state.define_ui(ctx);
            }


            } );
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
                self.controller.process_keyboard(event);
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
