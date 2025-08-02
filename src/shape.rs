use std::cmp::Ordering;

use crate::utils::DrawUI;
use cgmath::{EuclideanSpace, Matrix4, MetricSpace, Point3, Vector2, Vector3, Vector4};
use serde::{Deserialize, Serialize};

pub trait Shape: crevice::std140::AsStd140 {
    /* Return a new shape at the position that the model matrix determined
     *
     */
    fn place(&self, model_matrix: &Matrix4<f32>) -> Self;
}

/*
* A ==== B
* |      |
* |      |
* |      |
* C ==== D
*/

#[derive(crevice::std140::AsStd140, Clone, Serialize, Deserialize)]
pub struct Quad {
    a: Vector3<f32>,
    b: Vector3<f32>,
    c: Vector3<f32>,
    d: Vector3<f32>,
}
impl Shape for Quad {
    // We place the quad by multiplying every point
    fn place(&self, model_matrix: &Matrix4<f32>) -> Self {
        let a = (model_matrix * Vector4::new(self.a.x, self.a.y, self.a.z, 1.0)).xyz();
        let b = (model_matrix * Vector4::new(self.b.x, self.b.y, self.b.z, 1.0)).xyz();
        let c = (model_matrix * Vector4::new(self.c.x, self.c.y, self.c.z, 1.0)).xyz();
        let d = (model_matrix * Vector4::new(self.d.x, self.d.y, self.d.z, 1.0)).xyz();
        Quad { a, b, c, d }
    }
}

impl Quad {
    pub fn new(a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>, d: Vector3<f32>) -> Self {
        Quad { a, b, c, d }
    }
    pub fn screen_quad() -> Self {
        let a = Vector3::new(-1.0, -1.0, 0.1);
        let b = Vector3::new(2.0, -1.0, 0.1);
        let c = Vector3::new(2.0, 1.0, 0.1);
        let d = Vector3::new(-1.0, 1.0, 0.1);
        Quad { a, b, c, d }
    }
    // Convert Quad to vector of vertices, assumes clockwise rotation
    // Does not create an index list
    pub fn distance_to(&self, point: Point3<f32>) -> f32 {
        self.a.distance2(point.to_vec())
            + self.b.distance2(point.to_vec())
            + self.c.distance2(point.to_vec())
            + self.d.distance2(point.to_vec())
    }
}

#[derive(crevice::std140::AsStd140, Clone, Serialize, Deserialize)]
pub struct Sphere {
    pub position: Vector3<f32>,
    pub radius: f32,
    pub color: Vector4<f32>,
    pub swap_color: Vector4<f32>,
}
impl Shape for Sphere {
    fn place(&self, model_matrix: &Matrix4<f32>) -> Self {
        let position = (model_matrix
            * Vector4::new(self.position.x, self.position.y, self.position.z, 1.0))
        .xyz();
        Sphere {
            position,
            radius: self.radius,
            color: self.color,
            swap_color: self.swap_color,
        }
    }
}
impl DrawUI for Sphere {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut egui::Ui>) {
        let _ = title;
        let _ = ctx;
        if let Some(ui) = ui {
            ui.label("Radius");
            ui.add(egui::DragValue::new(&mut self.radius).speed(0.1));
            if self.radius < 0.0 {
                self.radius = 0.01;
            }

            ui.label("Main Color");
            let mut rgb = [self.color.x, self.color.y, self.color.z];
            let _response = egui::widgets::color_picker::color_edit_button_rgb(ui, &mut rgb);

            self.color.x = rgb[0];
            self.color.y = rgb[1];
            self.color.z = rgb[2];
            ui.label("Off Color");

            let mut rgb = [self.swap_color.x, self.swap_color.y, self.swap_color.z];
            let _response = egui::widgets::color_picker::color_edit_button_rgb(ui, &mut rgb);

            self.swap_color.x = rgb[0];
            self.swap_color.y = rgb[1];
            self.swap_color.z = rgb[2];
        }
    }
}
impl Sphere {
    pub fn new(
        position: Vector3<f32>,
        radius: f32,
        color: Vector4<f32>,
        swap_color: Vector4<f32>,
    ) -> Self {
        Sphere {
            position,
            radius,
            color,
            swap_color,
        }
    }
}

// Struct Representing video window Panel
#[derive(crevice::std140::AsStd140, Serialize, Deserialize, Clone)]
pub struct VWPanel {
    quad: Quad,
    pub pixel_count: Vector2<u32>,
    // TODO: Change this to UOM
    pub size: Vector2<f32>,
}
impl Shape for VWPanel {
    fn place(&self, model_matrix: &Matrix4<f32>) -> Self {
        let new_quad = self.quad.place(model_matrix);
        VWPanel {
            quad: new_quad,
            pixel_count: self.pixel_count,
            size: self.size,
        }
    }
}
impl VWPanel {
    pub fn border_correction(&self) -> Self {
        let pixel_count = self.pixel_count + Vector2::new(0, 0);
        VWPanel {
            quad: self.quad.clone(),
            pixel_count,
            size: self.size,
        }
    }
    pub fn demo_panel() -> Self {
        let quad = Quad::new(
            Vector3::new(-0.5, 0.5, 0.0),
            Vector3::new(0.5, 0.5, 0.0),
            Vector3::new(-0.5, -0.5, 0.0),
            Vector3::new(0.5, -0.5, 0.0),
        );
        let pixel_count = Vector2::new(300, 300);
        let size = Vector2::new(1.0, 1.0);
        VWPanel {
            quad,
            pixel_count,
            size,
        }
    }
    pub fn distance_compar(&self, other: &VWPanel, point: Point3<f32>) -> Ordering {
        self.quad
            .distance_to(point)
            .total_cmp(&other.distance_to(point))
    }
    pub fn distance_to(&self, point: Point3<f32>) -> f32 {
        self.quad.distance_to(point)
    }
}
