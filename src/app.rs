use cgmath::{Matrix4, SquareMatrix};
use egui_glium::egui_winit::egui;
use glium::{glutin::surface::WindowSurface, index::NoIndices, winit::{application::ApplicationHandler, window::Window}, Display, DrawParameters, Program, Texture2d, VertexBuffer};
use crate::{matrix::ToArr, shader, vertex::{self, floor, Vertex}};
use glium::Surface;
use glium::winit::event::WindowEvent;


use crate::{camera::CameraState, texture};

// Deal with application State
// RN, only does 
pub struct App<'a> {
    window: Option<Window>,
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

}
impl App<'_> {
    pub fn new<'a>(window: Option<Window>, display: Display<WindowSurface>, ui: egui_glium::EguiGlium) -> App<'a> {


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

        App{window, display, _t: 0.0, texture, camera,  vertex_buffer, indices, program, draw_params, ui}
    }
    pub fn draw_debug(&mut self) {

        let test: Matrix4<f32> = cgmath::Matrix4::identity();
        let matrix = test.to_arr();

        let mut frame = self.display.draw();
        frame.clear_color_and_depth((0.0, 0.0,1.0 , 1.0), 1.0);

        self.ui.paint(&self.display, &mut frame);

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

        let matrix = shape.model_matrix;
        let uniforms = uniform!{

            matrix: matrix.to_arr(),
            tex: &self.texture,
            perspective: self.camera.get_perspective(),
            view: self.camera.get_view(),
        };
        let buffer = glium::VertexBuffer::new(&self.display, &shape.vertex_buffer).unwrap();

        frame.draw(&buffer, self.indices, &self.program, &uniforms,&self.draw_params).unwrap();
        frame.finish().unwrap();
        self.camera.update();

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
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            },
            WindowEvent::Resized(window_size) => {
                self.display.resize(window_size.into());
            }

            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

            
                let window = self.window.as_ref();
                self.ui.run(window.unwrap(), |ctx| {
                    egui::Window::new("Hello World").show(ctx, |ui|{
                        ui.label("Hello Wold!");
                    } );
                });
                // Draw.
                //
                self.draw_debug();

        

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::MouseInput { device_id, state, button } => {()}

            _ => self.camera.process_input(&event),
        }
    }
}
