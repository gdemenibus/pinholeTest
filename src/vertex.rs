use crate::matrix::ToArr;
use crate::texture;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, Vector3, Vector4};
use egui_glium::egui_winit::egui::mutex::RwLock;
use egui_glium::egui_winit::egui::{self, Context, Pos2};
use glium::uniforms::UniformBuffer;
use glium::{glutin::surface::WindowSurface, Display, Texture2d};
use image::{DynamicImage, Rgba, RgbaImage};
use std::sync::Arc;
use uom::si::f32::Length;
use uom::si::length::{meter, millimeter};

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct LineVertex {
    position: [f32; 3],
}

implement_vertex!(LineVertex, position);

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Line {
    vertices: [LineVertex; 2],
}

impl Line {
    pub fn new(start: Point3<f32>, end: Point3<f32>) -> Line {
        Line {
            vertices: LineVertex::line(start, end),
        }
    }
    pub fn vertices(self) -> [LineVertex; 2] {
        self.vertices
    }
}

impl LineVertex {
    pub fn to_cg_math(self) -> Point3<f32> {
        Point3::new(self.position[0], self.position[1], self.position[2])
    }

    pub fn place_vertex(self, model: &Matrix4<f32>) -> Point3<f32> {
        let point = self.to_cg_math();
        let vector = Vector4::new(point.x, point.y, point.z, 1.0);
        let placement = model * vector;
        Point3::new(placement.x, placement.y, placement.z)
    }

    pub fn line(start: Point3<f32>, end: Point3<f32>) -> [LineVertex; 2] {
        [
            LineVertex {
                position: start.to_vec().to_arr(),
            },
            LineVertex {
                position: end.to_vec().to_arr(),
            },
        ]
    }
}

impl Vertex {
    pub fn to_cg_math(self) -> Point3<f32> {
        Point3::new(self.position[0], self.position[1], self.position[2])
    }
    pub fn place_vertex(self, model: &Matrix4<f32>) -> Point3<f32> {
        let point = self.to_cg_math();
        let vector = Vector4::new(point.x, point.y, point.z, 1.0);
        let placement = model * vector;
        Point3::new(placement.x, placement.y, placement.z)
    }
}
pub fn gross_method() -> Vec<Vertex> {
    vec![
        Vertex {
            position: [-1.0, -1.0, 0.1],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [1.0, -1.0, 0.1],
            tex_coords: [1.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0, 0.1],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 0.1],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [-1.0, 1.0, 0.1],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [-1.0, -1.0, 0.1],
            tex_coords: [0.0, 0.0],
        },
    ]
}

pub struct Shape {
    pub world: Arc<RwLock<ShapeWorld>>,
    pub texture: glium::Texture2d,
}
impl Shape {
    pub fn new(
        vertex_buffer: Vec<Vertex>,
        model_matrix: Matrix4<f32>,
        texture: Texture2d,
        texture_path: String,
        ui_state: ShapeUI,
        opacity: f32,
        is_transparent: bool,
    ) -> Shape {
        let default = DynamicImage::ImageRgba8(RgbaImage::new(10, 10));
        let tex = image::open(texture_path).unwrap_or(default);
        let world = ShapeWorld {
            vertex_buffer,
            model_matrix,
            placement_matrix: model_matrix,
            texture_image: tex.to_rgba8(),
            ui_state,
            opacity,
            is_transparent,
        };

        Shape {
            world: Arc::new(RwLock::new(world)),
            texture,
        }
    }

    pub fn floor(display: &Display<WindowSurface>) -> Shape {
        let shape = vec![
            Vertex {
                position: [-0.5, 0.0, -0.5],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.0, -0.5],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.0, 0.5],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.0, 0.5],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [-0.5, 0.0, 0.5],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [-0.5, 0.0, -0.5],
                tex_coords: [0.0, 0.0],
            },
        ];

        let translate: Matrix4<f32> = Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0));
        let matrix: Matrix4<f32> = Matrix4::from_scale(10.0);
        let movement = translate * matrix;
        let texture = texture::load_texture("./resources/textures/Gibbon.jpg".to_string(), display);
        let ui_state = ShapeUI::default("Floor".to_string(), Pos2::new(0.0, 500.0));
        Shape::new(
            shape,
            movement,
            texture,
            "./resources/textures/Gibbon.jpg".to_string(),
            ui_state,
            1.0,
            false,
        )
    }

    pub fn f(display: &Display<WindowSurface>) -> Shape {
        let shape = vec![
            Vertex {
                position: [-0.5, -0.5, 0.5],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5],
                tex_coords: [0.0, 0.0],
            },
        ];

        let translate: Matrix4<f32> = Matrix4::from_translation(Vector3::new(0.0, 5.0, -10.0));
        let matrix: Matrix4<f32> = Matrix4::from_scale(10.0);
        let movement = translate * matrix;
        // For real tests!
        //let texture = texture::load_texture("./resources/textures/Planes_airport.jpeg".to_string(), display);
        let texture_path = "./resources/textures/Golden monkey.jpg".to_string();

        let texture = texture::load_texture(texture_path.clone(), display);
        let ui_state = ShapeUI::default("Far plane".to_string(), Pos2::new(0.0, 200.0));

        Shape::new(shape, movement, texture, texture_path, ui_state, 1.0, false)
    }
    pub fn a(display: &Display<WindowSurface>) -> Shape {
        let shape = vec![
            Vertex {
                position: [-0.5, -0.5, 0.5],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5],
                tex_coords: [0.0, 0.0],
            },
        ];

        let translate: Matrix4<f32> = Matrix4::from_translation(Vector3::new(0.0, 5.0, -5.0));
        let matrix: Matrix4<f32> = Matrix4::from_scale(3.0);
        let movement = translate * matrix;
        let texture_path = "./resources/textures/Mandril.jpg".to_string();

        let texture = texture::load_texture(texture_path.clone(), display);
        let ui_state = ShapeUI::default("A".to_string(), Pos2::new(0.0, 150.0));

        Shape::new(shape, movement, texture, texture_path, ui_state, 0.3, true)
    }

    pub fn b(display: &Display<WindowSurface>) -> Shape {
        let shape = vec![
            Vertex {
                position: [-0.5, -0.5, 0.5],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5],
                tex_coords: [0.0, 0.0],
            },
        ];

        let translate: Matrix4<f32> = Matrix4::from_translation(Vector3::new(0.0, 5.0, -0.0));
        let matrix: Matrix4<f32> = Matrix4::from_scale(3.0);
        let movement = translate * matrix;
        let texture_path = "./resources/textures/new-debrazza.jpg".to_string();

        let texture = texture::load_texture(texture_path.clone(), display);
        let ui_state = ShapeUI::default("B".to_string(), Pos2::new(0.0, 0.0));
        Shape::new(shape, movement, texture, texture_path, ui_state, 0.3, true)
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

impl ShapeWorld {
    /*
    6 vertices, each with 3 points
    */
    pub fn to_uniform_buffer(&self) -> [f32; 9] {
        let vertices: [f32; 9] = self.vertex_buffer[0..3]
            .iter()
            .map(|x| x.place_vertex(&self.placement_matrix))
            .flat_map(|x| vec![x.x, x.y, x.z])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        vertices
    }
    pub fn first_triangle(&self) -> [Point3<f32>; 3] {
        let vertices: [Point3<f32>; 3] = self.vertex_buffer[0..3]
            .iter()
            .map(|x| x.place_vertex(&self.placement_matrix))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        vertices
    }

    pub fn intersect(
        &self,
        vect_pos: Point3<f32>,
        vec_dir: Vector3<f32>,
    ) -> Option<(
        ReportedColour,
        IntersectionPoint,
        PixelCenter,
        PixelRelativeCoordinates,
    )> {
        let pixel_count_height = self.ui_state.resolution.1 as f32;
        let pixel_count_width = self.ui_state.resolution.0 as f32;

        let pixel_size = self.ui_state.pixel_size;

        let trig_1: [Point3<f32>; 3] = self.vertex_buffer[0..3]
            .iter()
            .map(|x| x.place_vertex(&self.placement_matrix))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let trig_2: [Point3<f32>; 3] = self.vertex_buffer[3..6]
            .iter()
            .map(|x| x.place_vertex(&self.placement_matrix))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let inter_1 = Self::moller_trumbore_intersection(vect_pos, vec_dir, trig_1);
        let inter_2 = Self::moller_trumbore_intersection(vect_pos, vec_dir, trig_2);

        if let Some((inter_point, bary_point)) = inter_1 {
            let contact_pixel = Self::pixel_center_real_space(
                bary_point,
                [
                    self.vertex_buffer[0],
                    self.vertex_buffer[1],
                    self.vertex_buffer[2],
                ],
                trig_1,
                pixel_count_height,
                pixel_count_width,
                pixel_size.get::<meter>(),
            );
            let texture = self.sample_texture(
                bary_point,
                [
                    self.vertex_buffer[0],
                    self.vertex_buffer[1],
                    self.vertex_buffer[2],
                ],
            );
            Some((texture, inter_point, contact_pixel.0, contact_pixel.1))
        } else if let Some((inter_point, bary_point)) = inter_2 {
            let contact_pixel = Self::pixel_center_real_space(
                bary_point,
                [
                    self.vertex_buffer[0],
                    self.vertex_buffer[1],
                    self.vertex_buffer[2],
                ],
                trig_2,
                pixel_count_height,
                pixel_count_width,
                pixel_size.get::<meter>(),
            );
            let texture = self.sample_texture(
                bary_point,
                [
                    self.vertex_buffer[3],
                    self.vertex_buffer[4],
                    self.vertex_buffer[5],
                ],
            );
            Some((texture, inter_point, contact_pixel.0, contact_pixel.1))
        } else {
            None
        }
    }

    fn sample_texture(&self, bary_coords: Point3<f32>, triangle: [Vertex; 3]) -> ReportedColour {
        let x_coord = (1.0
            - (bary_coords.x * triangle[0].tex_coords[0]
                + bary_coords.y * triangle[1].tex_coords[0]
                + bary_coords.z * triangle[2].tex_coords[0]))
            * self.texture_image.width() as f32
            - 1.0;
        let y_coord = (1.0
            - (bary_coords.x * triangle[0].tex_coords[1]
                + bary_coords.y * triangle[1].tex_coords[1]
                + bary_coords.z * triangle[2].tex_coords[1]))
            * self.texture_image.height() as f32
            - 1.0;

        // TODO: Interplate the texture
        let pixel = self.texture_image.get_pixel(x_coord as u32, y_coord as u32);
        [
            pixel.0[0],
            pixel.0[1],
            pixel.0[2],
            (self.opacity * 255.0) as u8,
        ]
    }

    // Expect to get size in what unit?
    //0,0 ----- 1,0
    // |         |
    // |         |
    // |         |
    //0,1-------1,1
    pub fn pixel_center_real_space(
        bary_coords: Point3<f32>,
        triangle: [Vertex; 3],
        trig: [Point3<f32>; 3],
        number_of_pixels_height: f32,
        number_of_pixels_width: f32,
        pixel_size: f32,
    ) -> (PixelCenter, PixelRelativeCoordinates) {
        // Get the relative coordinates of x, y
        // Express the point in quad space (reusing the texture coordinates)
        let x_coord = 1.0
            - (bary_coords.x * triangle[0].tex_coords[0]
                + bary_coords.y * triangle[1].tex_coords[0]
                + bary_coords.z * triangle[2].tex_coords[0]);
        // Not sure why, but this works?
        let y_coord = (bary_coords.x * triangle[0].tex_coords[1]
            + bary_coords.y * triangle[1].tex_coords[1]
            + bary_coords.z * triangle[2].tex_coords[1]);

        // Which pixel maps to these absolute coordinates?
        let x_f_pixel = x_coord * number_of_pixels_width;
        let y_f_pixel = y_coord * number_of_pixels_height;

        // Round down
        let x_pixel = x_f_pixel.floor();
        let y_pixel = y_f_pixel.floor();

        // Get the pixel center.
        let center_x_pixel = (x_pixel * pixel_size) + (pixel_size / 2.0);
        let center_y_pixel = (y_pixel * pixel_size) + (pixel_size / 2.0);

        // Get the outer two
        let e1 = trig[0] - trig[1];
        let e2 = trig[2] - trig[1];

        // multiply
        let x_vec = e1 * center_x_pixel;

        let y_vec = e2 * center_y_pixel;

        (
            x_vec + y_vec + trig[1].to_vec(),
            (x_pixel as u32, y_pixel as u32),
        )
        // x and y coords are currently expressed in quad space
        // Need to: Get the pixel they intersect with, get that pixel center in quad space, and
        // then get that back view space (apply transoformation to this point?) Yes, should.
    }

    pub fn moller_trumbore_intersection(
        origin: Point3<f32>,
        direction: Vector3<f32>,
        triangle: [Point3<f32>; 3],
    ) -> Option<(Point3<f32>, Point3<f32>)> {
        let e1 = triangle[1] - triangle[0];
        let e2 = triangle[2] - triangle[0];
        let ray_cross_e2 = direction.cross(e2);
        let det = e1.dot(ray_cross_e2);

        if det > -f32::EPSILON && det < f32::EPSILON {
            return None;
        }
        let inv_det = 1.0 / det;
        let s = origin - triangle[0];
        let u = inv_det * s.dot(ray_cross_e2);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let s_cross_e1 = s.cross(e1);
        let v = inv_det * direction.dot(s_cross_e1);
        let w = 1.0 - v - u;

        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        // At this stage we can compute t to find out where the intersection point is on the line.
        let t = inv_det * e2.dot(s_cross_e1);

        if t > f32::EPSILON {
            // ray intersection
            let intersection_point = origin + direction * t;
            Some((intersection_point, Point3::new(w, u, v)))
        } else {
            // This means that there is a line intersection but not a ray intersection.
            None
        }
    }
}
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
    #[test]
    fn pixel_center_test() {
        let triangle = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let trig = [
            Vertex {
                position: [0.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
        ];
        let origin = Point3::new(0.1, 0.1, 1.0);
        let directoin = Vector3::new(0.1, 0.1, -1.0);
        let (intersection, bary) =
            ShapeWorld::moller_trumbore_intersection(origin, directoin, triangle).unwrap();

        let pixel_h = 1.0;
        let pixel_w = 1.0;
        let pixel_size = 1.0;
        let pixel_center =
            ShapeWorld::pixel_center_real_space(bary, trig, triangle, pixel_h, pixel_w, pixel_size);
        let real_center = Vector3::new(0.5, 0.5, 0.0);
        assert_eq!(real_center, pixel_center.0);
        assert_ne!(intersection.to_vec(), real_center);
    }
    #[test]
    fn pixel_2by2_test() {
        let triangle = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let trig = [
            Vertex {
                position: [0.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
        ];
        let origin = Point3::new(0.1, 0.1, 1.0);
        let directoin = Vector3::new(0.1, 0.1, -1.0);
        let (intersection, bary) =
            ShapeWorld::moller_trumbore_intersection(origin, directoin, triangle).unwrap();

        let pixel_h = 2.0;
        let pixel_w = 2.0;
        let pixel_size = 0.5;
        let pixel_center =
            ShapeWorld::pixel_center_real_space(bary, trig, triangle, pixel_h, pixel_w, pixel_size);
        let real_center = Vector3::new(0.25, 0.25, 0.0);
        assert_eq!(real_center, pixel_center.0);
        assert_ne!(intersection.to_vec(), real_center);
    }
}
