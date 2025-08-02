use egui::Id;
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

use crate::utils::DrawUI;
use crate::{
    camera::Camera,
    scene::{Scene, ScenePanel, Target},
};

type OutCache = Option<(DynamicImage, DynamicImage, Option<Vec<f32>>)>;
/// Cache the current textures if they need to be saved
pub struct ImageCache {
    pub target_image: DynamicImage,
    pub panels: Vec<DynamicImage>,
    pub stereo_out: OutCache,
    pub separable_out: OutCache,
}
impl ImageCache {
    pub fn plot_error(
        &self,
        location: PathBuf,
        stereo: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let out = {
            if stereo {
                &self.stereo_out
            } else {
                &self.separable_out
            }
        };
        if let Some((_x, _y, Some(error))) = out {
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
    pub fn save_out_cache(
        &self,
        root_path: PathBuf,
    ) -> (
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
    ) {
        todo!("Doesn't save output yet");
    }

    pub fn cache_output(&mut self, stereo: bool, out: OutCache) {
        if stereo {
            self.stereo_out = out;
            let _ = self.load_output(stereo);
        } else {
            self.separable_out = out;
            let _ = self.load_output(stereo);
        }
    }
    pub fn load_output(&mut self, stereo: bool) -> Result<(), ()> {
        if stereo {
            if let Some((image_1, image_2, _)) = self.stereo_out.as_ref() {
                let images = vec![image_1.clone(), image_2.clone()];
                self.panels = images;
                Ok(())
            } else {
                Err(())
            }
        } else if let Some((image_1, image_2, _)) = self.separable_out.as_ref() {
            let images = vec![image_1.clone(), image_2.clone()];
            self.panels = images;

            Ok(())
        } else {
            Err(())
        }
    }

    pub fn cache_panel(&mut self, entry: usize, image: DynamicImage) {
        self.panels[entry] = image;
    }
    pub fn load_world(&mut self, img: DynamicImage) {
        // let (width, height) = img.dimensions();
        // let size = width.max(height); // target size for the square

        // // Create a new white RGBA image of the target size
        // let mut square = RgbaImage::from_pixel(size, size, Rgba([255, 255, 255, 255]));

        // // Calculate top-left coordinates to place the original image centered
        // let x_offset = (size - width) / 2;
        // let y_offset = (size - height) / 2;

        // // Copy the original image onto the new square image
        // let rgba_img = img.to_rgba8();
        // image::imageops::overlay(&mut square, &rgba_img, x_offset.into(), y_offset.into());

        self.target_image = img;
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        let img_1 = DynamicImage::default();
        let img_2 = DynamicImage::default();
        Self {
            target_image: Default::default(),
            panels: vec![img_1, img_2],
            stereo_out: None,
            separable_out: None,
        }
    }
}

/// Save the current state of things to a directory
#[derive(Serialize, Deserialize)]
pub struct Save {
    pub cameras: VecDeque<Camera>,
    target_path: PathBuf,
    pub target: Target,
    panel_1: ScenePanel,
    panel_2: ScenePanel,
    panel_1_texture_sep: Option<PathBuf>,
    panel_2_texture_sep: Option<PathBuf>,
    panel_1_texture_stereo: Option<PathBuf>,
    panel_2_texture_stereo: Option<PathBuf>,
    pub name: String,
}

impl Save {
    pub fn from_cache(
        cameras: &VecDeque<Camera>,
        name: &String,
        cache: &ImageCache,
        scene: &Scene,
    ) -> Self {
        let path_core = PathBuf::from(format!("./saves/scene_capture/{name}/"));

        if !path_core.exists() {
            std::fs::create_dir(&path_core).unwrap();
        }

        let mut plot_core = path_core.clone();
        plot_core.push("Errors.png");
        //let _ = cache.plot_error(plot_core);

        let mut target_image_path = path_core.clone();
        target_image_path.push("target.png");
        cache.target_image.save(&target_image_path).ok();

        let save = Save {
            target: scene.world.clone(),
            cameras: cameras.clone(),
            name: name.clone(),
            target_path: target_image_path,
            panel_1_texture_sep: None,
            panel_2_texture_sep: None,

            panel_1_texture_stereo: None,
            panel_2_texture_stereo: None,
            panel_1: scene.panels[0].clone(),
            panel_2: scene.panels[1].clone(),
        };
        save.save_settings();
        save
    }
    pub fn save_settings(&self) {
        let content = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default()).unwrap();

        let path_core = PathBuf::from(format!("./saves/scene_capture/{}/save.ro", self.name));
        fs::write(path_core, content).unwrap();
    }

    pub fn to_cache(&self) -> ImageCache {
        let target = ImageReader::open(&self.target_path)
            .unwrap()
            .decode()
            .unwrap();

        ImageCache {
            target_image: target,
            stereo_out: None,
            separable_out: None,
            ..Default::default()
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
    pub current_save_name: String,
    pub save_open: bool,
    pub first_draw: bool,
    pub name_inserted: bool,
}
impl SaveManager {
    pub fn boot() -> SaveManager {
        let path = PathBuf::from("./saves/scene_capture/");
        let mut saves: VecDeque<Save> = VecDeque::new();
        for entry in WalkDir::new(path) {
            let mut save_path = entry.unwrap().into_path();
            save_path.push("save.ro");
            let s = std::fs::read_to_string(save_path);

            if let Ok(string) = s {
                let save = ron::from_str::<Save>(string.as_str());
                match save {
                    Ok(mut save_struct) => {
                        save_struct.target.texture.texture_file = save_struct.target_path.clone();

                        saves.push_back(save_struct);
                    }
                    Err(err) => {
                        println!("Error with save: {err}")
                    }
                }
            }
        }
        println!("Saves found: {}", saves.len());
        let current_save_name = "".to_string();
        SaveManager {
            current_save_name,
            saves,
            save_open: false,
            first_draw: false,
            name_inserted: false,
        }
    }

    pub fn add_save(&mut self, save: Save) {
        save.save_settings();
        self.saves.push_back(save);
        self.save_open = false;
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
        let _title = title.unwrap_or("Save manager".to_string());
        let _ = ctx;
        let _ = ui;
        // Early exit, was not requested
        if !self.save_open {
            self.first_draw = true;
            self.name_inserted = false;
            return;
        }

        egui_winit::egui::Window::new("SAVE:")
            .resizable(false)
            .vscroll(false)
            .show(ctx, |ui| {
                ui.label("Save file as:");

                // Create the text field with auto focus
                let text_edit = egui::TextEdit::singleline(&mut self.current_save_name)
                    .hint_text("Dir Name")
                    .id(Id::new(20))
                    .desired_width(f32::INFINITY);
                if !self.first_draw {
                    ui.add(text_edit);
                } else {
                    ui.add(text_edit).request_focus();
                }

                // Only request focus the first frame we show the prompt

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        println!("Saving to file: {}", self.current_save_name);

                        self.first_draw = false;
                        self.name_inserted = true;
                    }

                    if ui.button("Cancel").clicked() {
                        self.save_open = false;
                        self.name_inserted = false;
                        self.current_save_name.clear(); // optional
                    }
                });
                self.first_draw = false;
            });
    }
}
