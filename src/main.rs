#[macro_use]
extern crate glium;
mod vertex;
mod shader;
mod matrix;
mod texture;
mod camera;
mod app;
use app::App;
use egui_glium::egui_winit::egui;

fn main() {
    // Event loop handles windows and device events
    // Make a window builder 
    // Call build method of the simple window builder to get the window and display
    let event_loop = glium::winit::event_loop::EventLoop::builder().build().expect("event loop building");
    event_loop.set_control_flow(glium::winit::event_loop::ControlFlow::Poll);

    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new().build(&event_loop);

    // egui
    let context = egui::Context::default();
    let viewport = context.viewport_id();
    //let mut egui_state = egui_winit::State::new(context, viewport, &window, None, None, None);

    //let input = egui_state.take_egui_input(&window);
    let egui_render = egui_glium::EguiGlium::new(viewport, &display, &window, &event_loop);





    let mut app = App::new(window, display, egui_render);
    app.define_ui();
    let _ = event_loop.run_app(&mut app);
    

}
