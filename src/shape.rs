use crate::vertex::Vertex;
use cgmath::{Matrix4, Point3, Vector3, Vector4};
use crevice::std140::{self, AsStd140};
use egui_winit::egui::{self, Context, Pos2};
use image::Rgba;
use uom::si::f32::Length;
use uom::si::length::{meter, millimeter};

pub trait Shape: crevice::std140::AsStd140 {
    // Change the position of the shape by the matrix.
    fn place(&mut self, model_matrix: Matrix4<f32>);
}
/*
* A ==== B
* |      |
* |      |
* |      |
* C ==== D

*/
#[derive(crevice::std140::AsStd140)]
pub struct Quad {
    a: Vector3<f32>,
    b: Vector3<f32>,
    c: Vector3<f32>,
    d: Vector3<f32>,
}
impl Shape for Quad {
    // We place the quad by multiplying every point
    fn place(&mut self, model_matrix: Matrix4<f32>) {
        self.a = (model_matrix * Vector4::new(self.a.x, self.a.y, self.a.z, 1.0)).xyz();
        self.b = (model_matrix * Vector4::new(self.b.x, self.b.y, self.b.z, 1.0)).xyz();
        self.c = (model_matrix * Vector4::new(self.c.x, self.c.y, self.c.z, 1.0)).xyz();
        self.d = (model_matrix * Vector4::new(self.d.x, self.d.y, self.d.z, 1.0)).xyz();
    }
}
impl Quad {
    pub fn new(a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>, d: Vector3<f32>) -> Self {
        Quad { a, b, c, d }
    }
    pub fn screen_quad() -> Self {
        let a = Vector3::new(-1.0, -1.0, 0.1);
        let b = Vector3::new(1.0, -1.0, 0.1);
        let c = Vector3::new(1.0, 1.0, 0.1);
        let d = Vector3::new(-1.0, 1.0, 0.1);
        Quad { a, b, c, d }
    }
    // Convert Quad to vector of vertices, assumes clockwise rotation
    // Does not create an index list
    pub fn create_vertex_buffer(&self) -> Vec<u8> {
        let mut triangle_one = vec![
            Vertex::new(self.a),
            Vertex::new(self.b),
            Vertex::new(self.c),
        ];
        let mut triangle_two = vec![
            Vertex::new(self.b),
            Vertex::new(self.d),
            Vertex::new(self.c),
        ];
        triangle_two.append(&mut triangle_one);
        //triangle_two.iter_mut().map(|x| x.as_std140()).collect()
        let length = triangle_two.len() as u32;
        let mut buffer = [0u8; 128];
        let mut writer = std140::Writer::new(&mut buffer[..]);
        let first_write = writer.write(&length).unwrap();
        let second_write = writer.write(triangle_two.as_slice()).unwrap();
        buffer[..first_write + second_write].to_vec()
    }
}

#[derive(Debug)]
pub struct ShapeWorld {
    pub vertex_buffer: Vec<Vertex>,
    pub model_matrix: Matrix4<f32>,
    pub placement_matrix: Matrix4<f32>,
    pub texture_image: image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub ui_state: ShapeUI,
    pub opacity: f32,
    pub is_transparent: bool,
}

type ReportedColour = [u8; 4];
type IntersectionPoint = Point3<f32>;
type PixelCenter = Vector3<f32>;
type PixelRelativeCoordinates = (u32, u32);

#[derive(Debug, PartialEq)]
pub enum LastChanged {
    Resolution,
    Pixel,
    Size,
}

#[derive(Debug, PartialEq)]
pub enum Lock {
    Resolution,
    Pixel,
    Size,
}
type Resolution = (u32, u32);

#[derive(Debug)]
pub struct ShapeUI {
    pub title: String,
    pub distance: Length,
    pub resolution: Resolution,
    pub pixel_size: Length,
    pub size: (Length, Length),
    pub position: Pos2,
    pub lock: Lock,
    pub changed: LastChanged,
}

impl ShapeUI {
    pub fn define_ui(&mut self, ctx: &Context) {
        egui::Window::new(self.title.clone())
            .default_pos(self.position)
            .show(ctx, |ui| {
                egui::Grid::new("my_grid")
                    .num_columns(2)
                    .spacing([4.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Distance:");
                        ui.add(egui::Slider::new(&mut self.distance.value, -10.0..=10.0));
                        ui.label("Meters");
                        ui.end_row();
                        ui.label("Resolution:");
                        let locked_res = Lock::Resolution == self.lock;

                        let check_res_w = ui
                            .add_enabled(
                                !locked_res,
                                egui::DragValue::new(&mut self.resolution.0).speed(1.0),
                            )
                            .changed();
                        let check_res_h = ui
                            .add_enabled(
                                !locked_res,
                                egui::DragValue::new(&mut self.resolution.1).speed(1.0),
                            )
                            .changed();

                        if check_res_h || check_res_w {
                            self.changed = LastChanged::Resolution;
                        }
                        if let Lock::Resolution = self.lock {
                            let _ = ui.button("🔒".to_string());
                        } else if ui.button("🔓".to_string()).clicked() {
                            self.lock = Lock::Resolution;
                        }

                        ui.end_row();

                        ui.label("Pixel Size");

                        let locked_res = Lock::Pixel == self.lock;

                        let check_pixel = ui
                            .add_enabled(
                                !locked_res,
                                egui::Slider::new(&mut self.pixel_size.value, 0.0..=1.0)
                                    .custom_formatter(|n, _| {
                                        let print = n * 1000.0;
                                        format!("{print}")
                                    })
                                    .custom_parser(|s| s.parse::<f64>().map(|r| r / 1000.0).ok()),
                            )
                            .changed();
                        if check_pixel {
                            self.changed = LastChanged::Pixel;
                        }

                        if let Lock::Pixel = self.lock {
                            let _ = ui.button("🔒".to_string());
                        } else if ui.button("🔓".to_string()).clicked() {
                            self.lock = Lock::Pixel;
                        }

                        ui.label("Millimeters");
                        ui.end_row();
                        ui.label("Physical Size:");

                        let locked_res = Lock::Size == self.lock;

                        let physical_check_h = ui
                            .add_enabled(
                                !locked_res,
                                egui::DragValue::new(&mut self.size.0.value).speed(1.0),
                            )
                            .changed();
                        let physical_check_w = ui
                            .add_enabled(
                                !locked_res,
                                egui::DragValue::new(&mut self.size.1.value).speed(1.0),
                            )
                            .changed();

                        if physical_check_h || physical_check_w {
                            self.changed = LastChanged::Size;
                        }

                        if let Lock::Size = self.lock {
                            let _ = ui.button("🔒".to_string());
                        } else if ui.button("🔓".to_string()).clicked() {
                            self.lock = Lock::Size;
                        }
                        ui.label("Meters");
                        ui.label(format!("{:?}", self.changed));
                        ui.end_row();
                    });
                self.resolution_compute();
            });
    }
    pub fn default(title: String, position: Pos2) -> ShapeUI {
        ShapeUI {
            title,
            distance: Length::new::<meter>(10.0),
            resolution: (1000, 1000),
            pixel_size: Length::new::<millimeter>(1.0),
            size: (Length::new::<meter>(1.0), Length::new::<meter>(1.0)),
            position,
            changed: LastChanged::Resolution,
            lock: Lock::Pixel,
        }
    }
    pub fn resolution_compute(&mut self) {
        match self.changed {
            LastChanged::Size => {
                match self.lock {
                    Lock::Resolution => {
                        // Change pixel size
                        self.pixel_size = self.size.0 / self.resolution.0 as f32;
                        // sanity check
                        let pixel_other = self.size.1 / self.resolution.1 as f32;
                        if self.pixel_size != pixel_other {
                            println!("WARNING, pixel's are no longer square? Height is {:?} and width is {:?}", self.pixel_size.value, pixel_other);
                        }
                    }
                    Lock::Pixel => {
                        // Pixel size is locked, size is changed, resolution must change
                        self.resolution.0 = (self.size.0 / self.pixel_size).value as u32;
                        self.resolution.1 = (self.size.1 / self.pixel_size).value as u32;
                    }
                    _ => {}
                }
            }
            LastChanged::Pixel => match self.lock {
                Lock::Resolution => {
                    self.size.0 = self.resolution.0 as f32 * self.pixel_size;
                    self.size.1 = self.resolution.1 as f32 * self.pixel_size;
                }

                Lock::Size => {
                    self.resolution.0 = (self.size.0 / self.pixel_size).value as u32;
                    self.resolution.1 = (self.size.1 / self.pixel_size).value as u32;
                }
                _ => {}
            },
            LastChanged::Resolution => {
                match self.lock {
                    Lock::Size => {
                        // Change pixel size
                        self.pixel_size = self.size.0 / self.resolution.0 as f32;
                        // sanity check
                        let pixel_other = self.size.1 / self.resolution.1 as f32;
                        if self.pixel_size != pixel_other {
                            println!("WARNING, pixel's are no longer square? Height is {:?} and width is {:?}", self.pixel_size, pixel_other);
                        }
                    }
                    Lock::Pixel => {
                        self.size.0 = self.resolution.0 as f32 * self.pixel_size;
                        self.size.1 = self.resolution.1 as f32 * self.pixel_size;
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
