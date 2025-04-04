use std::path::{Path, PathBuf};

use egui::Context;
use egui_file::FileDialog;

use crate::scene::DrawUI;

pub struct FilePicker {
    pub texture_file: PathBuf,
    file_dialog: FileDialog,
    pub change_file: bool,
}

impl FilePicker {
    pub fn new() -> Self {
        let path = Path::new("./resources/textures");

        let texture_file = PathBuf::from(path);
        let file_dialog = FileDialog::open_file(Some(texture_file.clone()));
        FilePicker {
            texture_file,
            file_dialog,
            change_file: false,
        }
    }
}
impl DrawUI for FilePicker {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let title = title.unwrap_or("Texture Selection".to_string());

        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([50.0, 50.0])
            .default_open(false)
            .show(ctx, |ui| {
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
