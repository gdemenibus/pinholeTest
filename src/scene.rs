use std::cmp::Ordering;

use crate::{
    camera::Camera,
    shape::{Quad, Shape, VWPanel},
};
use cgmath::{AbsDiffEq, Matrix4, Rad, SquareMatrix, Vector3};
use crevice::std140::Writer;
use egui_winit::egui::{self, Context, Pos2};
pub trait DrawUI {
    /*
    Draw UI for this element
    */
    fn draw_ui(&mut self, ctx: &Context) {}
}

/*
TODO: SCENE ONLY USES QUAD, MIGHT WANT MORE?
Scene struct. Encapsulates UI and handles access to the raw quads
*/
pub struct Scene {
    world: Vec<(Matrix4<f32>, Quad)>,
}

/// Wrapper around panel that controls access to the UI, as well as their placement
///
pub struct ScenePanel {
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    roll: Rad<f32>,
    placement: Matrix4<f32>,
    panel: VWPanel,
}
impl ScenePanel {
    pub fn place_panel(&self) -> VWPanel {
        let yaw_matrix = Matrix4::from_angle_x(self.yaw);
        let pitch_matrix = Matrix4::from_angle_y(self.pitch);
        let roll_matrix = Matrix4::from_angle_z(self.roll);
        // will need to double check this ordering
        let placement_matrix = yaw_matrix * pitch_matrix * roll_matrix * self.placement;
        self.panel.place(&placement_matrix)
    }
    pub fn test() -> ScenePanel {
        let yaw = Rad(0.0);

        let pitch = Rad(0.0);
        let roll = Rad(0.0);
        let mut placement = Matrix4::identity();
        placement.w.x = 0.5;
        placement.w.y = 1.5;
        placement.w.z = 3.0;
        let panel = VWPanel::demo_panel();
        ScenePanel {
            pitch,
            yaw,
            roll,
            placement,
            panel,
        }
    }
}

impl DrawUI for ScenePanel {
    fn draw_ui(&mut self, ctx: &Context) {
        egui_winit::egui::Window::new("VWPanel")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            .show(ctx, |ui| {
                ui.label("Move x");
                ui.add(egui::Slider::new(&mut self.placement.w.x, -10.0..=10.0));
                ui.label("Move y");
                ui.add(egui::Slider::new(&mut self.placement.w.y, -10.0..=10.0));
                ui.label("Move z");
                ui.add(egui::Slider::new(&mut self.placement.w.z, -10.0..=10.0));
                ui.label("Yaw");
                ui.add(egui::Slider::new(&mut self.yaw.0, -1.0..=1.0));
                ui.label("Pitch");
                ui.add(egui::Slider::new(&mut self.pitch.0, -1.0..=1.0));
                ui.label("Roll");
                ui.add(egui::Slider::new(&mut self.roll.0, -1.0..=1.0));
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
    fn draw_ui(&mut self, ctx: &Context) {
        let mut count = 1;
        for (matrix, _quad) in self.world.iter_mut() {
            egui_winit::egui::Window::new(format!("Test quad {}", count))
                .resizable(true)
                .vscroll(true)
                .default_open(false)
                .show(ctx, |ui| {
                    ui.label("Move x");
                    ui.add(egui::Slider::new(&mut matrix.w.x, -10.0..=10.0));
                    ui.label("Move y");
                    ui.add(egui::Slider::new(&mut matrix.w.y, -10.0..=10.0));
                    ui.label("Move z");
                    ui.add(egui::Slider::new(&mut matrix.w.z, -10.0..=10.0));
                });
            count += 1;
        }
    }
}

impl Scene {
    /// Make a quad with coordinates, but in scene space, not clip space
    pub fn test() -> Self {
        Scene {
            world: vec![
                (
                    Matrix4::identity(),
                    Quad::new(
                        Vector3::new(1.0, 0.0, 1.0),
                        Vector3::new(0.0, 0.0, 0.0),
                        Vector3::new(1.0, 1.0, 1.0),
                        Vector3::new(0.0, 1.0, 0.0),
                    ),
                ),
                (
                    Matrix4::identity(),
                    Quad::new(
                        Vector3::new(2.0, 1.0, 1.0),
                        Vector3::new(1.0, 1.0, 1.0),
                        Vector3::new(2.0, 2.0, 1.0),
                        Vector3::new(1.0, 2.0, 1.0),
                    ),
                ),
            ],
        }
    }
    pub fn as_bytes(&self, camera: &Camera) -> [u8; 256] {
        let mut buffer = [0u8; 256];
        let mut writer = Writer::new(&mut buffer[..]);
        let mut shapes: Vec<Quad> = self
            .world
            .iter()
            .map(|(matrix, shape)| shape.place(matrix))
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
}
