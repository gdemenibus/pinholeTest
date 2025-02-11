use cgmath::{Matrix4, SquareMatrix, Vector3};
use egui_glium::egui_winit::egui;
use glium::{glutin::surface::WindowSurface, index::NoIndices, winit::{application::ApplicationHandler, window::Window}, Display, DrawParameters, Program, Texture2d, VertexBuffer};
use crate::{matrix::ToArr, shader, vertex::{self, floor, Vertex}};
use glium::Surface;
use glium::winit::event::WindowEvent;


use crate::{camera::CameraState, texture};

// Deal with application State
// RN, only does 
pub struct App<'a> {
    window: Window,
    display: Display<WindowSurface>,
    _t: f32,
    texture: Texture2d,
    camera: CameraState,
    vertex_buffer: Vec<VertexBuffer<Vertex>>,
    // TODO: This might not work for complex shapes!
    indices: NoIndices,
    program: Program,
    draw_params: DrawParameters<'a>,
    ui: egui_glium::EguiGlium,
    selected_quad: QuadUISelect,
    transform_z: f32,

}

#[derive(PartialEq)]
enum QuadUISelect { A, B, F }

impl App<'_> {
    pub fn new<'a>(window: Window, display: Display<WindowSurface>, ui: egui_glium::EguiGlium) -> App<'a> {


        let shape = vertex::debug_triangle();
        let vertex_buffer = vec![glium::VertexBuffer::new(&display, &shape).unwrap()];

        let vertex_shader = shader::load_shader("./shaders/vertex.glsl");
        let fragment_shader = shader::load_shader("./shaders/fragment.glsl");

        let texture = texture::load_texture("./resources/textures/Gibbon.jpg".to_string(), &display);
        let camera = CameraState::new();

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
        let selected_quad = QuadUISelect::A;

        App{window, display, _t: 0.0, texture, camera,  vertex_buffer, indices, program, draw_params, ui, selected_quad, transform_z: 0.0}
    }
    pub fn draw_debug(&mut self) {

        let test: Matrix4<f32> = cgmath::Matrix4::identity();
        let matrix = test.to_arr();

        let mut frame = self.display.draw();
        frame.clear_color_and_depth((0.0, 0.0,1.0 , 1.0), 1.0);


        let uniforms = uniform! {
            matrix: matrix,
            tex: &self.texture,
            perspective: self.camera.get_perspective(),
            view: self.camera.get_view(),

        };
        for buffer in self.vertex_buffer.iter() {
            //frame.draw(buffer, self.indices, &self.program, &uniforms,&self.draw_params).unwrap();
        }
        // Testing out the floor before doing anything wacky with it
        let shape = floor();

        let mut matrix = shape.model_matrix;
        matrix = matrix * Matrix4::<f32>::from_translation(Vector3::new(0.0,0.0, self.transform_z));
        

        let uniforms = uniform!{

            matrix: matrix.to_arr(),
            tex: &self.texture,
            perspective: self.camera.get_perspective(),
            view: self.camera.get_view(),
        };
        let buffer = glium::VertexBuffer::new(&self.display, &shape.vertex_buffer).unwrap();

        frame.draw(&buffer, self.indices, &self.program, &uniforms,&self.draw_params).unwrap();

        // Paint the UI 
        self.ui.paint(&self.display, &mut frame);
        frame.finish().unwrap();
        self.camera.update();

    }
    pub fn define_ui(&mut self) {

        let window = &self.window;
        self.ui.run(window, |ctx| {
            egui::Window::new("UI").show(ctx, |ui|{
                ui.label("Select Object");
                ui.radio_value(&mut self.selected_quad, QuadUISelect::A, "A");
                ui.radio_value(&mut self.selected_quad, QuadUISelect::B, "B");
                ui.radio_value(&mut self.selected_quad, QuadUISelect::F, "F");
                ui.add(egui::Slider::new(&mut self.transform_z, -1.0..=1.0).text("Distance from next Layer (z transform)"))

            } );
        });
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
                self.camera.resize(window_size.height, window_size.width);
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
            WindowEvent::MouseInput { device_id, state, button } => {()}

            _ => self.camera.process_input(&event),
        }
    }
}
