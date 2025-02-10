#[macro_use]
extern crate glium;
mod vertex;
mod shader;
mod matrix;
mod texture;
mod camera;
mod app;
use app::App;
use camera::CameraState;
use glium::Surface;

fn main() {
    // Event loop handles windows and device events
    // Make a window builder 
    // Call build method of the simple window builder to get the window and display
    let event_loop = glium::winit::event_loop::EventLoop::builder().build().expect("event loop building");
    event_loop.set_control_flow(glium::winit::event_loop::ControlFlow::Poll);

    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new().build(&event_loop);




    let mut app = App::new(Some(window), display);
    let _ = event_loop.run_app(&mut app);
    

}
