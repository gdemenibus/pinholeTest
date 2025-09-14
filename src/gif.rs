use std::time::Instant;

use crate::{save::ImageCache, utils::DrawUI};
use egui::Ui;
use image::DynamicImage;

pub struct GifPlayer {
    gif: Vec<(DynamicImage, DynamicImage)>,
    pub animation_duration: f32,

    pub animate: bool,
    pub animation_start: Option<Instant>,
}

impl GifPlayer {
    pub fn create(frames: Vec<(DynamicImage, DynamicImage)>) -> Self {
        GifPlayer {
            gif: frames,
            animation_duration: 0.5,
            animation_start: None,
            animate: false,
        }
    }
    pub fn start_animation(&mut self) {
        self.animation_start = Some(Instant::now());
    }

    pub fn animate_gif(&mut self, cache: &mut ImageCache) -> Option<()> {
        let time = self.animation_start?.elapsed().as_secs_f32();
        let duration = self.animation_duration;
        let keyframes = &self.gif;
        let num_segments = keyframes.len() - 1;

        let total_time = duration * num_segments as f32;
        let i = ((time % total_time) / duration).floor() as usize;

        let next = &keyframes[i + 1];
        let out = Some((next.0.clone(), next.1.clone(), None));
        cache.cache_output(false, out);
        Some(())
    }
}

impl DrawUI for GifPlayer {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut Ui>) {
        let _ = title;
        let _ = ui;
        egui_winit::egui::Window::new("Gif Player")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            .default_size([150.0, 125.0])
            .show(ctx, |ui| {
                self.animate = ui.button("Play Gif").clicked();
                if self.animate {
                    self.animation_start = Some(Instant::now());
                } else if let Some(start) = self.animation_start {
                    if start.elapsed().as_secs_f32()
                        > (self.animation_duration * (self.gif.len() - 1) as f32)
                    {
                        self.animation_start = None;
                    }
                }
            });
    }
}
