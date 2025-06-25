use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, path::PathBuf};

use crate::{camera::Camera, scene::DrawUI};

/// Cache the current textures if they need to be saved
pub struct ImageCache {
    pub target_image: DynamicImage,
    pub panels: Vec<DynamicImage>,
}
impl Default for ImageCache {
    fn default() -> Self {
        let img_1 = DynamicImage::default();
        let img_2 = DynamicImage::default();
        Self {
            target_image: Default::default(),
            panels: vec![img_1, img_2],
        }
    }
}

/// Save the current state of things to a directory
#[derive(Serialize, Deserialize)]
pub struct Save {
    cameras: VecDeque<Camera>,
    target: PathBuf,
    panel_1: PathBuf,
    panel_2: PathBuf,
    name: String,
}

impl Save {
    pub fn from_cache(cameras: &VecDeque<Camera>, name: String, cache: &ImageCache) -> Self {
        let path_core = PathBuf::from(format!("./saves/{}/", name));
        if !path_core.exists() {
            std::fs::create_dir(&path_core).unwrap();
        }

        let mut target_image_path = path_core.clone();
        target_image_path.push("target.png");
        //cache.target_image.save(&target_image_path).unwrap();

        let mut panel_1_image_path = path_core.clone();
        panel_1_image_path.push("panel_1.png");
        //cache.panels[0].save(&panel_1_image_path).unwrap();

        let mut panel_2_image_path = path_core.clone();
        panel_2_image_path.push("panel_2.png");
        //cache.panels[1].save(&panel_2_image_path).unwrap();
        Save {
            cameras: cameras.clone(),
            name,
            target: target_image_path,
            panel_1: panel_1_image_path,
            panel_2: panel_2_image_path,
        }
    }
}

/// Structure to load and manage saves
struct SaveManager {
    saves: VecDeque<Save>,
}
impl DrawUI for SaveManager {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut egui::Ui>) {
        let title = title.unwrap_or("Save manaer".to_string());
        let _ = ctx;
        let _ = ui;
    }
}
