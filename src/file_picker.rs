use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::utils::DrawUI;
use egui::{Context, Ui};
use egui_file::FileDialog;
use image::DynamicImage;

pub struct FilePicker {
    pub texture_file: PathBuf,
    pub file_dialog: FileDialog,
    pub change_file: bool,
    pub default_texture: PathBuf,
}
impl Clone for FilePicker {
    fn clone(&self) -> Self {
        Self {
            texture_file: self.texture_file.clone(),
            file_dialog: FileDialog::open_file(Some(self.texture_file.clone())),
            change_file: self.change_file,
            default_texture: self.default_texture.clone(),
        }
    }
}

impl FilePicker {
    pub fn new(path: String, default_texture: PathBuf) -> Self {
        let path = Path::new(&path);

        let texture_file = PathBuf::from(path);
        let file_dialog = FileDialog::open_file(Some(texture_file.clone()));
        FilePicker {
            texture_file,
            file_dialog,
            change_file: false,
            default_texture,
        }
    }
    pub fn button(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        if ui.button("Change Texture").clicked() {
            self.file_dialog.open();
        }
        if self.file_dialog.show(ctx).selected() {
            if let Some(file) = self.file_dialog.path() {
                self.texture_file = file.to_path_buf();
                self.change_file = true;
            }
        }
    }
    pub fn default_texture(&self) -> &PathBuf {
        &self.default_texture
    }
    pub fn load_texture(&self) -> DynamicImage {
        let path = {
            let path = &self.texture_file;
            println!("{path:?}");

            let file = File::open(path).unwrap();
            if file.metadata().unwrap().is_file() {
                path
            } else {
                println!("Default Texture ");
                self.default_texture()
            }
        };

        image::ImageReader::open(path).unwrap().decode().unwrap()
    }
}
impl Default for FilePicker {
    fn default() -> Self {
        let path = Path::new("./resources/");

        let texture_file = PathBuf::from(path);
        let file_dialog = FileDialog::open_file(Some(texture_file.clone()));
        Self {
            texture_file,
            file_dialog,
            change_file: false,
            default_texture: Default::default(),
        }
    }
}

impl DrawUI for FilePicker {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>, ui: Option<&mut Ui>) {
        let title = title.unwrap_or("Texture Selection".to_string());

        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([50.0, 50.0])
            .default_open(false)
            .show(ctx, |_ui| {
                self.file_dialog.open();
                if self.file_dialog.show(ctx).selected() {
                    if let Some(file) = self.file_dialog.path() {
                        self.texture_file = file.to_path_buf();
                        self.change_file = true;
                    }
                }
            });
    }
}
