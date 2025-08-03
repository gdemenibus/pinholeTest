#[macro_use]
use image::DynamicImage;
use light_field_test::app::*;
use light_field_test::FileWatcher;
use light_field_test::{LFMatrices, LFSettings, Lff, StereoMatrix};
use notify::Watcher;
use winit::event_loop::{EventLoop, EventLoopProxy};

use clap::Parser;
use std::{f32::consts::FRAC_PI_2, path::PathBuf, str::FromStr};

pub const RAY_HEIGHT: usize = 500;
pub const RAY_WIDTH: usize = 500;
pub const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Clone, Debug, clap::ValueEnum)]
enum HeadlessType {
    Sep,
    SepOld,
    Stereo,
}
impl std::fmt::Display for HeadlessType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Commands {
    #[arg(short, long)]
    type_head: Option<HeadlessType>,
}

fn main() {
    let args = Commands::parse();

    let mut diagonal = LFMatrices::load("Kernel.ro".to_string());

    let stereo = StereoMatrix::load("Kernel.ro".to_string());
    let settings = LFSettings {
        debug_prints: false,
        ..Default::default()
    };
    if let Some(bench) = args.type_head {
        match bench {
            HeadlessType::Sep => bench_sep(settings, &mut diagonal),
            HeadlessType::SepOld => bench_old(settings, &mut diagonal),
            HeadlessType::Stereo => bench_stereo(settings, &stereo),
        }
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            pollster::block_on(execute());
        }
    }
}
fn bench_sep(settings: LFSettings, diagonal: &mut LFMatrices) {
    diagonal.c_t = DynamicImage::new_rgb8(diagonal.target_size.0, diagonal.target_size.1);
    diagonal.factorize(&settings);
}
fn bench_old(settings: LFSettings, diagonal: &mut LFMatrices) {
    diagonal.c_t = DynamicImage::new_rgb8(diagonal.target_size.0, diagonal.target_size.1);

    let stacked_matrices = diagonal.stack();

    diagonal.old_factorize(&settings, &stacked_matrices);
}
fn bench_stereo(settings: LFSettings, stereo: &StereoMatrix) {
    stereo.factorize(&settings);
}

async fn execute() {
    let mut builder = EventLoop::<FileWatcher>::with_user_event();

    let event_loop = builder.build().unwrap();

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let proxy = event_loop.create_proxy();
    start_file_watcher(proxy);

    let mut app = App::new(false);
    let _ = event_loop.run_app(&mut app);
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
