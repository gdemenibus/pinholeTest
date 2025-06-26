use image::{DynamicImage, ImageReader};
use plotters::{
    chart::ChartBuilder,
    prelude::{BitMapBackend, IntoDrawingArea},
    series::LineSeries,
    style::{IntoFont, RED, WHITE},
};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fs, path::PathBuf};
use walkdir::WalkDir;

use crate::{
    camera::Camera,
    scene::{DrawUI, Scene, ScenePanel, Target},
};

/// Cache the current textures if they need to be saved
pub struct ImageCache {
    pub target_image: DynamicImage,
    pub panels: Vec<DynamicImage>,
    pub error: Option<Vec<f32>>,
}
impl ImageCache {
    pub fn plot_error(&self, location: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(error) = self.error.as_ref() {
            let max = error.clone().into_iter().reduce(f32::max).unwrap();

            let root = BitMapBackend::new(&location, (640, 480)).into_drawing_area();
            root.fill(&WHITE)?;
            let mut chart = ChartBuilder::on(&root)
                .caption("Spectral Norm over time", ("sans-serif", 50).into_font())
                .margin(5)
                .x_label_area_size(30)
                .y_label_area_size(30)
                .build_cartesian_2d(0f32..(error.len() as f32), 0.0f32..max)?;

            chart.configure_mesh().draw()?;
            let series = LineSeries::new((0..error.len()).map(|x| (x as f32, error[x])), &RED);

            chart.draw_series(series)?;
            root.present()?;
            return Ok(());
        }
        Err("No Errors in this cache".into())
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        let img_1 = DynamicImage::default();
        let img_2 = DynamicImage::default();
        Self {
            target_image: Default::default(),
            panels: vec![img_1, img_2],
            error: None,
        }
    }
}

/// Save the current state of things to a directory
#[derive(Serialize, Deserialize)]
pub struct Save {
    pub cameras: VecDeque<Camera>,
    target_path: PathBuf,
    target: Target,
    panel_1: ScenePanel,
    panel_2: ScenePanel,
    panel_1_texture: PathBuf,
    panel_2_texture: PathBuf,
    name: String,
}

impl Save {
    pub fn from_cache(
        cameras: &VecDeque<Camera>,
        name: String,
        cache: &ImageCache,
        scene: &Scene,
    ) -> Self {
        let path_core = PathBuf::from(format!("./saves/{}/", name));

        if !path_core.exists() {
            std::fs::create_dir(&path_core).unwrap();
        }

        let mut plot_core = path_core.clone();
        plot_core.push("Errors.png");
        let _ = cache.plot_error(plot_core);

        let mut target_image_path = path_core.clone();
        target_image_path.push("target.png");
        cache.target_image.save(&target_image_path).unwrap();

        let mut panel_1_image_path = path_core.clone();
        panel_1_image_path.push("panel_1.png");
        cache.panels[0].save(&panel_1_image_path).unwrap();

        let mut panel_2_image_path = path_core.clone();
        panel_2_image_path.push("panel_2.png");
        cache.panels[1].save(&panel_2_image_path).unwrap();
        let save = Save {
            target: scene.world.clone(),
            cameras: cameras.clone(),
            name,
            target_path: target_image_path,
            panel_1_texture: panel_1_image_path,
            panel_2_texture: panel_2_image_path,
            panel_1: scene.panels[0].clone(),
            panel_2: scene.panels[1].clone(),
        };
        save.save_settings();
        save
    }
    pub fn save_settings(&self) {
        let content = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default()).unwrap();

        let path_core = PathBuf::from(format!("./saves/{}/save.ro", self.name));
        fs::write(path_core, content).unwrap();
    }

    pub fn to_cache(&self) -> ImageCache {
        let target = ImageReader::open(&self.target_path)
            .unwrap()
            .decode()
            .unwrap();
        let panel_1 = ImageReader::open(&self.panel_1_texture)
            .unwrap()
            .decode()
            .unwrap();
        let panel_2 = ImageReader::open(&self.panel_2_texture)
            .unwrap()
            .decode()
            .unwrap();
        ImageCache {
            target_image: target,
            panels: vec![panel_1, panel_2],
            error: None,
        }
    }
    pub fn update_scene(&self, scene: &mut Scene) {
        scene.world = self.target.clone();
        scene.panels[0] = self.panel_1.clone();
        scene.panels[1] = self.panel_2.clone();
    }
}

/// Structure to load and manage saves
pub struct SaveManager {
    pub saves: VecDeque<Save>,
}
impl SaveManager {
    pub fn boot() -> SaveManager {
        let path = PathBuf::from("./saves/");
        let mut saves: VecDeque<Save> = VecDeque::new();
        for entry in WalkDir::new(path) {
            let mut save_path = entry.unwrap().into_path();
            save_path.push("save.ro");
            let s = std::fs::read_to_string(save_path);

            if let Ok(string) = s {
                let save = ron::from_str(string.as_str());
                if let Ok(save) = save {
                    saves.push_back(save);
                }
            }
        }
        println!("Saves found: {}", saves.len());
        SaveManager { saves }
    }
    pub fn add_save(&mut self, save: Save) {
        self.saves.push_back(save);
    }
    pub fn next_save(&mut self) -> Option<&Save> {
        if self.saves.is_empty() {
            return None;
        }
        self.saves.rotate_left(1);
        let next = self.saves.back();
        next
    }
    pub fn previous_save(&mut self) -> Option<&Save> {
        if self.saves.is_empty() {
            return None;
        }
        self.saves.rotate_right(1);
        let next = self.saves.back();
        next
    }
}

impl DrawUI for SaveManager {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut egui::Ui>) {
        let _title = title.unwrap_or("Save manaer".to_string());
        let _ = ctx;
        let _ = ui;
    }
}
