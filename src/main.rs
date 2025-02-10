#[macro_use]
extern crate glium;
mod vertex;
mod shader;
mod matrix;
mod texture;
mod camera;
use glium::Surface;
use matrix::view_matrix;

#[allow(deprecated)]
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

                let view = view_matrix(&[2.0, -1.0, 1.0], &[-2.0, 1.0, 1.0], &[0.0, 1.0, 0.0]);                let mut frame = display.draw();
                frame.clear_color_and_depth((0.0, 0.0,1.0 , 1.0), 1.0);

                let perspective = {
                    let (width, height) = frame.get_dimensions();
                    let aspect_ratio = height as f32 / width as f32;

                    let fov: f32 = 3.141592 / 3.0;
                    let zfar = 1024.0;
                    let znear = 0.1;

                    let f = 1.0 / (fov / 2.0).tan();

                    [
                        [f *   aspect_ratio   ,    0.0,              0.0              ,   0.0],
                        [         0.0         ,     f ,              0.0              ,   0.0],
                        [         0.0         ,    0.0,  (zfar+znear)/(zfar-znear)    ,   1.0],
                        [         0.0         ,    0.0, -(2.0*zfar*znear)/(zfar-znear),   0.0],
                    ]
                };

                let uniforms = uniform! {
                    matrix: matrix,
                    tex: &texture,
                    perspective: perspective,
                    view: view,

                };

                frame.draw(&vertex_buffer, indices, &program, &glium::uniforms::EmptyUniforms,
                            &Default::default()).unwrap();

                frame.finish().unwrap();
                // We update `t`
                t += 0.02;
                    // We use the sine of t as an offset, this way we get a nice smooth animation
                let x_off = t.sin() * 0.5;

                let mut target = display.draw();
                target.clear_color(0.0, 0.0, 1.0, 1.0);
                target.draw(&vertex_buffer, &indices, &program, &uniforms,&params).unwrap();
                target.finish().unwrap();
            },
                _ => (),
            },
            glium::winit::event::Event::AboutToWait => {
                window.request_redraw();
            },
            _ => (),
        };
    });

}
