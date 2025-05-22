use std::cmp::Ordering;

use crate::vertex::Vertex;
use cgmath::{EuclideanSpace, Matrix4, MetricSpace, Point3, Vector2, Vector3, Vector4};
use egui_winit::egui::{self, Context, Pos2};
use image::Rgba;
use uom::si::f32::Length;
use uom::si::length::{meter, millimeter};

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

#[derive(crevice::std140::AsStd140, Clone)]
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
// Struct Representing video window Panel
#[derive(crevice::std140::AsStd140)]
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
    pub fn from_quad(quad: Quad, pixel_count: Vector2<u32>, size: Vector2<f32>) -> Self {
        VWPanel {
            quad,
            pixel_count,
            size,
        }
    }
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
        let pixel_count = Vector2::new(30, 30);
        let size = Vector2::new(1.0, 1.0);
        VWPanel {
            quad,
            pixel_count,
            size,
        }
    }
    pub fn new(
        a: Vector3<f32>,
        b: Vector3<f32>,
        c: Vector3<f32>,
        d: Vector3<f32>,
        pixel_count: Vector2<u32>,
        size: Vector2<f32>,
    ) -> Self {
        let quad = Quad::new(a, b, c, d);

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
