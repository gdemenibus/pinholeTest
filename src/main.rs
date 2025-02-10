#[macro_use]
extern crate glium;
mod vertex;
mod shader;
mod matrix;
mod texture;
mod camera;
use camera::CameraState;
use glium::Surface;

fn main() {
    // Event loop handles windows and device events
    // Make a window builder 
    // Call build method of the simple window builder to get the window and display
    let event_loop = glium::winit::event_loop::EventLoop::builder().build().expect("event loop building");
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new().build(&event_loop);

    // load shaders
    let vertex_shader = shader::load_shader("./shaders/vertex.glsl");
    let fragment_shader = shader::load_shader("./shaders/fragment.glsl");
    
    //
    // Create triangle
    let shape = vertex::debug_triangle();
    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();

    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
    let program = glium::Program::from_source(&display, &vertex_shader, &fragment_shader, None).unwrap();


    // Create our camera
    let mut camera = CameraState::new();



    
    let params = glium::DrawParameters {
        depth: glium::Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            .. Default::default()
        },
        backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
        .. Default::default()
    };

    // Texture
    let texture = texture::load_texture("./resources/textures/Gibbon.jpg".to_string(), &display);


    
    // Event loop to handle quitting the window
    // There is a more up to date version of doing this with an App struct that implements the
    // Application handler Trait. Will deal this later
    let mut t: f32 = 0.0;
    let _ = event_loop.run(move |event, window_target| {

        let matrix = [
            [ t.cos(), t.sin(), 0.0, 0.0],
            [-t.sin(), t.cos(), 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0f32],
        ];

        match event {
            glium::winit::event::Event::WindowEvent { event, .. } => match event {
                glium::winit::event::WindowEvent::CloseRequested => window_target.exit(),
                glium::winit::event::WindowEvent::Resized(window_size) => {
                    display.resize(window_size.into());
                },
            glium::winit::event::WindowEvent::RedrawRequested => {

                t += 0.02;
                let mut frame = display.draw();
                frame.clear_color_and_depth((0.0, 0.0,1.0 , 1.0), 1.0);


                let uniforms = uniform! {
                    matrix: matrix,
                    tex: &texture,
                    perspective: camera.get_perspective(),
                    view: camera.get_view(),

                };

                frame.draw(&vertex_buffer, indices, &program, &uniforms,&params).unwrap();

                frame.finish().unwrap();

                camera.update();
            },
                _ => camera.process_input(&event),
            },
            glium::winit::event::Event::AboutToWait => {
                window.request_redraw();
            },
            _ => (),
        };
    });

}
