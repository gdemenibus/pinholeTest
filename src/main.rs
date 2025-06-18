#[macro_use]
mod app;
mod camera;
mod compute_pass;
mod egui_tools;
mod file_picker;
mod light_factor;
mod raytracer;
mod scene;
mod shape;
mod texture;
mod utils;
mod vertex;
use app::App;
use notify::Watcher;
use winit::event_loop::{EventLoop, EventLoopProxy};

use std::{f32::consts::FRAC_PI_2, path::PathBuf, str::FromStr};
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
    let mut builder = EventLoop::<FileWatcher>::with_user_event();

    let event_loop = builder.build().unwrap();

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let proxy = event_loop.create_proxy();
    start_file_watcher(proxy);

    let mut app = App::new();
    let _ = event_loop.run_app(&mut app);
}

enum FileWatcher {
    FileChange,
}

fn start_file_watcher(proxy: EventLoopProxy<FileWatcher>) {
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx).unwrap();
        let path = PathBuf::from_str("./shaders").unwrap();
        watcher
            .watch(&path, notify::RecursiveMode::Recursive)
            .unwrap();
        loop {
            if rx.recv().is_ok() {
                proxy.send_event(FileWatcher::FileChange).ok();
            }
        }
    });
}
