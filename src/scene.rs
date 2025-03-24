use std::cmp::Ordering;

use crate::{
    camera::Camera,
    shape::{Quad, Shape},
};
use cgmath::{Matrix4, SquareMatrix, Vector3};
use crevice::std140::Writer;
use egui_winit::egui::{self, Context, Pos2};

/*
TODO: SCENE ONLY USES QUAD, MIGHT WANT MORE?
Scene struct. Encapsulates UI and handles access to the raw quads
*/
pub struct Scene {
    world: Vec<(Matrix4<f32>, Quad)>,
}

pub trait DrawUI {
    /*
    Draw UI for this element
    */
    fn draw_ui(&mut self, ctx: &Context) {}
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
