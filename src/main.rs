#[macro_use]
mod app;
mod camera;
mod egui_tools;
mod matrix;
mod raytracer;
mod scene;
mod shader;
mod shape;
mod texture;
mod vertex;
use app::App;
use winit::event_loop::EventLoop;

use std::f32::consts::FRAC_PI_2;
pub const RAY_HEIGHT: usize = 500;
pub const RAY_WIDTH: usize = 500;
pub const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(execute());
    }
}

async fn execute() {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::new();
    let _ = event_loop.run_app(&mut app);
}
