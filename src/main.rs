#[macro_use]
extern crate glium;
mod app;
mod camera;
mod matrix;
mod raytracer;
mod shader;
mod texture;
mod vertex;
use app::App;
use egui_glium::egui_winit::egui;

use std::f32::consts::FRAC_PI_2;
pub const RAY_HEIGHT: usize = 1500;
pub const RAY_WIDTH: usize = 1500;
pub const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

fn main() {
    // Event loop handles windows and device events
    // Make a window builder
    // Call build method of the simple window builder to get the window and display
    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build()
        .expect("event loop building");
    event_loop.set_control_flow(glium::winit::event_loop::ControlFlow::Poll);

    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_inner_size(1920, 1080)
        .build(&event_loop);

    // egui
    let context = egui::Context::default();
    let viewport = context.viewport_id();
    //let mut egui_state = egui_winit::State::new(context, viewport, &window, None, None, None);

    //let input = egui_state.take_egui_input(&window);
    let egui_render = egui_glium::EguiGlium::new(viewport, &display, &window, &event_loop);

    egui_extras::install_image_loaders(&context);

    let mut app = App::new(window, display, egui_render);
    app.define_ui();
    let _ = event_loop.run_app(&mut app);
}
