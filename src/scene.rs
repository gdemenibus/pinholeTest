use std::path::PathBuf;

use crate::{
    camera::Camera,
    file_picker::FilePicker,
    shape::{Quad, Shape, VWPanel},
};
use cgmath::{Matrix4, Rad, SquareMatrix, Vector2, Vector3, Vector4};
use crevice::std140::Writer;
use egui::{Color32, RichText};
use egui_winit::egui::{self, Context};
pub trait DrawUI {
    /*
    Draw UI for this element
    */
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let _ = title;
        let _ = ctx;
    }
}

/*
TODO: SCENE ONLY USES QUAD, MIGHT WANT MORE?
Scene struct. Encapsulates UI and handles access to the raw quads
*/
pub struct Scene {
    world: Vec<Target>,
    pub panels: Vec<ScenePanel>,
}

pub struct Target {
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    roll: Rad<f32>,
    placement: Matrix4<f32>,
    scale: Matrix4<f32>,
    quad: Quad,
}

impl Target {
    fn new(place_vec: Vector4<f32>) -> Self {
        let yaw = Rad(0.0);

        let pitch = Rad(0.0);
        let roll = Rad(0.0);
        let mut placement = Matrix4::identity();
        placement.w = place_vec;
        let scale = Matrix4::identity();
        let quad = Quad::new(
            Vector3::new(-0.5, 0.5, 0.0),
            Vector3::new(0.5, 0.5, 0.0),
            Vector3::new(-0.5, -0.5, 0.0),
            Vector3::new(0.5, -0.5, 0.0),
        );

        Target {
            quad,
            yaw,
            pitch,
            roll,
            placement,
            scale,
        }
    }

    fn place_target(&self) -> Quad {
        let yaw_matrix = Matrix4::from_angle_x(self.yaw);
        let pitch_matrix = Matrix4::from_angle_y(self.pitch);
        let roll_matrix = Matrix4::from_angle_z(self.roll);
        // will need to double check this ordering
        let placement_matrix =
            self.placement * self.scale * yaw_matrix * pitch_matrix * roll_matrix;
        self.quad.place(&placement_matrix)
    }
}

impl DrawUI for Target {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let title = title.unwrap_or("VW Panel".to_string());
        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 175.0])
            .default_open(false)
            .show(ctx, |ui| {
                ui.label(RichText::new("Move x").color(Color32::RED));
                ui.add(egui::Slider::new(&mut self.placement.w.x, -10.0..=10.0));
                ui.label(RichText::new("Move y").color(Color32::GREEN));
                ui.add(egui::Slider::new(&mut self.placement.w.y, -10.0..=10.0));
                ui.label(RichText::new("Move z").color(Color32::LIGHT_BLUE));
                ui.add(egui::Slider::new(&mut self.placement.w.z, -10.0..=10.0));
                ui.label(RichText::new("Yaw").color(Color32::RED));
                ui.add(egui::Slider::new(&mut self.yaw.0, -1.0..=1.0));
                ui.label(RichText::new("Pitch").color(Color32::GREEN));
                ui.add(egui::Slider::new(&mut self.pitch.0, -1.0..=1.0));
                ui.label(RichText::new("Roll").color(Color32::LIGHT_BLUE));
                ui.add(egui::Slider::new(&mut self.roll.0, -1.0..=1.0));

                ui.label("Scale x");
                ui.add(egui::DragValue::new(&mut self.scale.x.x).speed(1.0));

                ui.label("Scale y");
                ui.add(egui::DragValue::new(&mut self.scale.y.y).speed(1.0));
            });
    }
}

/// Wrapper around panel that controls access to the UI, as well as their placement
///
pub struct ScenePanel {
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    roll: Rad<f32>,
    placement: Matrix4<f32>,
    scale: Matrix4<f32>,
    pub panel: VWPanel,
    pub texture: FilePicker,
    lock_pixel: bool,
}
impl ScenePanel {
    fn place_panel(&self) -> VWPanel {
        let yaw_matrix = Matrix4::from_angle_x(self.yaw);
        let pitch_matrix = Matrix4::from_angle_y(self.pitch);
        let roll_matrix = Matrix4::from_angle_z(self.roll);
        // will need to double check this ordering
        let placement_matrix =
            self.placement * self.scale * yaw_matrix * pitch_matrix * roll_matrix;
        self.panel.place(&placement_matrix)
    }

    fn new(place_vec: Vector4<f32>, position: usize) -> ScenePanel {
        let yaw = Rad(0.0);

        let pitch = Rad(0.0);
        let roll = Rad(0.0);
        let mut placement = Matrix4::identity();
        placement.w = place_vec;
        let scale = Matrix4::identity();
        let panel = VWPanel::demo_panel();
        let default_path =
            PathBuf::from(format!("./resources/panel_compute/panel_{}.png", position));
        ScenePanel {
            pitch,
            yaw,
            roll,
            placement,
            panel,
            lock_pixel: false,
            texture: FilePicker::new("./resources/panel_compute/".to_string(), default_path),
            scale,
        }
    }
}

impl DrawUI for ScenePanel {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let title = title.unwrap_or("VW Panel".to_string());
        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 175.0])
            .default_open(false)
            .show(ctx, |ui| {
                egui::Grid::new("By")
                    .num_columns(6)
                    .spacing([0.0, 0.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Pixel Density: ");

                        ui.add(egui::DragValue::new(&mut self.panel.pixel_count.x).speed(1.0));

                        if self.lock_pixel {
                            if ui.button("🔒".to_string()).clicked() {
                                self.lock_pixel = false;
                            }

                            self.panel.pixel_count.y = self.panel.pixel_count.x
                        } else if ui.button("🔓".to_string()).clicked() {
                            self.lock_pixel = true;
                        }

                        ui.add_enabled(
                            !self.lock_pixel,
                            egui::DragValue::new(&mut self.panel.pixel_count.y).speed(1.0),
                        );
                    });

                ui.label(RichText::new("Move x").color(Color32::RED));
                ui.add(egui::Slider::new(&mut self.placement.w.x, -10.0..=10.0));
                ui.label(RichText::new("Move y").color(Color32::GREEN));
                ui.add(egui::Slider::new(&mut self.placement.w.y, -10.0..=10.0));
                ui.label(RichText::new("Move z").color(Color32::LIGHT_BLUE));
                ui.add(egui::Slider::new(&mut self.placement.w.z, -10.0..=10.0));
                ui.label(RichText::new("Yaw").color(Color32::RED));
                ui.add(egui::Slider::new(&mut self.yaw.0, -1.0..=1.0));
                ui.label(RichText::new("Pitch").color(Color32::GREEN));
                ui.add(egui::Slider::new(&mut self.pitch.0, -1.0..=1.0));
                ui.label(RichText::new("Roll").color(Color32::LIGHT_BLUE));
                ui.add(egui::Slider::new(&mut self.roll.0, -1.0..=1.0));

                ui.label("Scale x");
                ui.add(egui::DragValue::new(&mut self.scale.x.x).speed(1.0));

                ui.label("Scale y");
                ui.add(egui::DragValue::new(&mut self.scale.y.y).speed(1.0));
                self.texture.button(ctx, ui);
            });
    }
}

impl DrawUI for Scene {
    /**
    Draw the UI for this element
    We want a system to place quads in space
    Translation, take in three coords
    Rotation: Slider
    */
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let _title = title.unwrap_or("Scene".to_string());
        let mut count = 1;
        for target in self.world.iter_mut() {
            let title = Some(format!("Target Quad {}", count));
            count += 1;
            target.draw_ui(ctx, title);
        }
        count = 1;
        for panel in self.panels.iter_mut() {
            let title = format!("VW Panel# {} ", count);
            panel.draw_ui(ctx, Some(title));

            count += 1;
        }
    }
}

impl Scene {
    /// Make a quad with coordinates, but in scene space, not clip space
    pub fn test() -> Self {
        let place_1 = Vector4::new(0.5, 1.5, 3.0, 1.0);

        let place_2 = Vector4::new(0.5, 1.5, 2.0, 1.0);
        let target_1 = Vector4::new(0.5, 1.5, 0.0, 1.0);
        let target_2 = Vector4::new(7.5, 1.5, 0.0, 1.0);
        Scene {
            world: vec![Target::new(target_1), Target::new(target_2)],

            panels: vec![ScenePanel::new(place_1, 1), ScenePanel::new(place_2, 2)],
        }
    }
    /// Will always place the closest quad first
    pub fn world_as_bytes(&self, camera: &Camera) -> [u8; 256] {
        let mut buffer = [0u8; 256];
        let mut writer = Writer::new(&mut buffer[..]);
        let mut shapes: Vec<Quad> = self
            .world
            .iter()
            .map(|target| target.place_target())
            .collect();

        shapes.sort_by(|x, y| {
            let camera_origin = camera.position;
            let x_dist = x.distance_to(camera_origin);
            let y_dist = y.distance_to(camera_origin);
            x_dist.total_cmp(&y_dist)
        });

        let _count = writer.write(shapes.as_slice()).unwrap();
        buffer
    }
    /// Will always place the closest panel first
    pub fn panels_as_bytes(&self, camera: &Camera) -> [u8; 256] {
        let mut buffer = [0u8; 256];
        let mut writer = Writer::new(&mut buffer[..]);
        let mut panels: Vec<VWPanel> = self
            .panels
            .iter()
            .map(|x| x.place_panel())
            .map(|x| x.border_correction())
            .collect();
        panels.sort_by(|x, y| x.distance_compar(y, camera.position));
        let _count = writer.write(panels.as_slice()).unwrap();
        buffer
    }
    pub fn panels_pixel_count(&self) -> [u32; 4] {
        let mut output = [0; 4];
        for x in 0..2 {
            let panel = self.panels[x].panel.pixel_count;
            output[x * 2] = panel.x;
            output[x * 2 + 1] = panel.y;
        }
        output
    }
}
