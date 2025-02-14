use cgmath::{BaseNum, InnerSpace, Matrix4, Point2, Point3, Vector3, Vector4};
use egui_glium::egui_winit::egui::ahash::HashSet;
use egui_glium::egui_winit::egui::{self, Align2, Context, Ui};
use glium::{glutin::surface::WindowSurface, winit::window::Window, Display, Texture2d};
use crate::texture;
use crate::matrix::{ToArr, FromArr};

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

// Go back to array
impl<T: BaseNum> ToArr for Vector3<T> {
    type Output = [T; 3];
    fn to_arr(&self) -> Self::Output {
        (*self).into()
    }
}
// Create from array
impl<T:BaseNum> FromArr for Vector3<T> {
    type Input = [T; 3];
    fn from_arr(array: Self::Input) -> Vector3<T>{
        Vector3::new(array[0], array[1], array[2])

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


pub fn debug_triangle()-> Vec<Vertex> {
    let shape = vec![
        Vertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 0.0] },
        Vertex { position: [ 0.5, -0.5, 0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 1.0] },

        Vertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [-0.5,  0.5, 0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 0.0] },
    ];
    shape
}
pub struct Shape {
    pub vertex_buffer: Vec<Vertex>,
    pub model_matrix: Matrix4<f32>,
    pub placement_matrix: Matrix4<f32>,
    pub texture: glium::Texture2d,
    pub ui_state: ShapeUI
}
impl Shape {
    pub fn new(vertex_buffer: Vec<Vertex>, model_matrix: Matrix4<f32>, texture: Texture2d, title: String) -> Shape{

        Shape{vertex_buffer, model_matrix, placement_matrix: model_matrix, texture, ui_state: ShapeUI::default(title, Align2::LEFT_TOP)}
    }

    pub fn intersect(&self, vect_pos: Point3<f32>, vec_dir: Vector3<f32>) -> bool {

        let trig_1 : [Point3<f32>; 3]= self.vertex_buffer[0..3].iter().map(|x|  x.place_vertex(&self.placement_matrix)   ).collect::<Vec<_>>().try_into().unwrap();
        
        let trig_2 : [Point3<f32>; 3]= self.vertex_buffer[3..6].iter().map(|x|  x.place_vertex(&self.placement_matrix) ).collect::<Vec<_>>().try_into().unwrap();
        
        let inter_1 = Self::moller_trumbore_intersection(vect_pos, vec_dir, trig_1);
        let inter_2 = Self::moller_trumbore_intersection(vect_pos, vec_dir, trig_2);
        if inter_1.is_some() {
            let _intersect_point = inter_1.unwrap().0;
            let _bary_point = inter_1.unwrap().1;
        }
        inter_1.is_some() || inter_2.is_some()
        

    }
    fn moller_trumbore_intersection(origin: Point3<f32>, direction: Vector3<f32>, triangle: [Point3<f32>; 3] ) -> Option<(Point3<f32>, Point3<f32>)> {
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

	if t > f32::EPSILON { // ray intersection
		let intersection_point = origin + direction * t;
		Some((intersection_point, Point3::new(u, v, w)))
	}
	else { // This means that there is a line intersection but not a ray intersection.
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

pub struct ShapeUI {
    pub title: String,
    pub distance: f32,
    pub resolution: (f32, f32),
    pub pixel_size: f32,
    pub size: (f32, f32),
    pub alignment: Align2,
    pub lock: Lock,
    pub changed: LastChanged,
}
impl ShapeUI {
    pub fn define_ui(&mut self, ctx: &Context) {
        egui::Window::new(self.title.clone()).pivot(self.alignment).show(ctx, |ui| {
            egui::Grid::new("my_grid")
                .num_columns(2)
                .spacing([4.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Distance:");
                    ui.add(egui::Slider::new(&mut self.distance, -10.0..=10.0));
                    ui.end_row();
                    ui.label("Resolution:");
                    let locked_res =  Lock::Resolution == self.lock;

                    let check_res_w = ui.add_enabled(!locked_res, egui::DragValue::new(&mut self.resolution.0).speed(1.0)).changed();
                    let check_res_h = ui.add_enabled(!locked_res,egui::DragValue::new(&mut self.resolution.1).speed(1.0)).changed();

                    if check_res_h || check_res_w {
                        self.changed = LastChanged::Resolution;
                    }
                    if let Lock::Resolution = self.lock {
                        let _ = ui.button("🔒".to_string());
                    }

                    else if ui.button("🔓".to_string()).clicked() {
                        self.lock = Lock::Resolution;
                    }

                    ui.end_row();

                    ui.label("Pixel Size");

                    let locked_res =  Lock::Pixel == self.lock;

                    let check_pixel = ui.add_enabled(!locked_res, egui::Slider::new(&mut self.pixel_size, -10.0..=10.0)).changed();
                    if check_pixel {
                        self.changed = LastChanged::Pixel;
                    }

                    if let Lock::Pixel= self.lock {
                        let _ = ui.button("🔒".to_string());
                    }

                    else if ui.button("🔓".to_string()).clicked() {
                        self.lock = Lock::Pixel;
                    }
                    ui.end_row();
                    ui.label("Physical Size:");

                    let locked_res =  Lock::Size == self.lock;

                    let physical_check_h = ui.add_enabled(!locked_res, egui::DragValue::new(&mut self.size.0).speed(1.0)).changed();
                    let physical_check_w = ui.add_enabled(!locked_res, egui::DragValue::new(&mut self.size.1).speed(1.0)).changed();

                    if physical_check_h || physical_check_w {
                        self.changed = LastChanged::Size;
                    }

                    if let Lock::Size= self.lock {
                        let _ = ui.button("🔒".to_string());
                    }
                    else if ui.button("🔓".to_string()).clicked() {
                        self.lock = Lock::Size;
                    }
                    ui.label(format!("{:?}", self.changed));
                    ui.end_row();

                });
            self.resolution_compute();

        });

    }
    pub fn default(title: String, alignment: Align2) -> ShapeUI {
        ShapeUI {
            title,
            distance: 10.0,
            resolution: (1.0, 1.0),
            pixel_size: 1.0,
            size: (1.0, 1.0),
            alignment,
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
                        self.pixel_size = self.size.0 / self.resolution.0;
                        // sanity check
                        let pixel_other = self.size.1 / self.resolution.1;
                        if self.pixel_size != pixel_other {
                            println!("WARNING, pixel's are no longer square? Height is {} and width is {}", self.pixel_size, pixel_other);
                        }

                    }
                    Lock::Pixel => {
                        // Pixel size is locked, size is changed, resolution must change
                        self.resolution.0 = self.size.0 / self.pixel_size;
                        self.resolution.1 = self.size.1 / self.pixel_size;

                    }
                    _ => {}
                }

            }
            LastChanged::Pixel => {
                match self.lock  {
                    Lock::Resolution => {
                        self.size.0 = self.resolution.0 * self.pixel_size;
                        self.size.1 = self.resolution.1 * self.pixel_size;

                    }

                    Lock::Size => {

                        self.resolution.0 = self.size.0 / self.pixel_size;
                        self.resolution.1 = self.size.1 / self.pixel_size;

                    }
                    _ => {}
                }

            }
            LastChanged::Resolution => {
                match self.lock {
                    Lock::Size => {

                        // Change pixel size
                        self.pixel_size = self.size.0 / self.resolution.0;
                        // sanity check
                        let pixel_other = self.size.1 / self.resolution.1;
                        if self.pixel_size != pixel_other {
                            println!("WARNING, pixel's are no longer square? Height is {} and width is {}", self.pixel_size, pixel_other);
                        }

                    }
                    Lock::Pixel => {

                        self.size.0 = self.resolution.0 * self.pixel_size;
                        self.size.1 = self.resolution.1 * self.pixel_size;

                    }
                    _ => {}

                }

            }
        }


    }
}

pub fn floor(display: &Display<WindowSurface>) -> Shape {
    let shape = vec![
        Vertex { position: [-0.5, 0.0, -0.5], tex_coords: [0.0, 0.0] },
        Vertex { position: [ 0.5, 0.0, -0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [ 0.5, 0.0,  0.5], tex_coords: [1.0, 1.0] },

        Vertex { position: [ 0.5,  0.0, 0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [-0.5,  0.0, 0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [-0.5,  0.0,-0.5], tex_coords: [0.0, 0.0] },
    ];
    
    let translate: Matrix4<f32>= Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0));
    let matrix: Matrix4<f32>  = Matrix4::from_scale(10.0);
    let movement = translate * matrix;
    let texture = texture::load_texture("./resources/textures/Gibbon.jpg".to_string(), display);
    let alignment = Align2::LEFT_TOP;
    Shape{vertex_buffer: shape, model_matrix: movement, placement_matrix: movement,  texture, ui_state: ShapeUI::default("Floor".to_string(), alignment)}
}

pub fn f(display: &Display<WindowSurface>) -> Shape {
    let shape = vec![
        Vertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 0.0] },
        Vertex { position: [ 0.5, -0.5, 0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 1.0] },

        Vertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [-0.5,  0.5, 0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 0.0] },
    ];

    let translate: Matrix4<f32>= Matrix4::from_translation(Vector3::new(0.0, 5.0, -10.0));
    let matrix: Matrix4<f32>  = Matrix4::from_scale(10.0);
    let movement = translate * matrix;
    // For real tests!
    //let texture = texture::load_texture("./resources/textures/Planes_airport.jpeg".to_string(), display);
    let texture = texture::load_texture("./resources/textures/Gibbon.jpg".to_string(), display);

    let alignment = Align2::LEFT_CENTER;
    Shape{vertex_buffer: shape, model_matrix: movement,  placement_matrix: movement,texture, ui_state: ShapeUI::default("Far Plane".to_string(), alignment)}
}
pub fn a(display: &Display<WindowSurface>) -> Shape {
    let shape = vec![
        Vertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 0.0] },
        Vertex { position: [ 0.5, -0.5, 0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 1.0] },

        Vertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [-0.5,  0.5, 0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 0.0] },
    ];

    let translate: Matrix4<f32>= Matrix4::from_translation(Vector3::new(0.0, 5.0, -5.0));
    let matrix: Matrix4<f32>  = Matrix4::from_scale(3.0);
    let movement = translate * matrix;
    let texture = texture::load_texture("./resources/textures/Gibbon.jpg".to_string(), display);

    let alignment = Align2::LEFT_BOTTOM;
    Shape{vertex_buffer: shape, model_matrix: movement,  placement_matrix: movement,texture, ui_state: ShapeUI::default("A Plane".to_string(), alignment)}
}


pub fn b(display: &Display<WindowSurface>) -> Shape {
    let shape = vec![
        Vertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 0.0] },
        Vertex { position: [ 0.5, -0.5, 0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 1.0] },

        Vertex { position: [ 0.5,  0.5, 0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [-0.5,  0.5, 0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [-0.5, -0.5, 0.5], tex_coords: [0.0, 0.0] },
    ];

    let translate: Matrix4<f32>= Matrix4::from_translation(Vector3::new(0.0, 5.0, -0.0));
    let matrix: Matrix4<f32>  = Matrix4::from_scale(3.0);
    let movement = translate * matrix;
    let texture = texture::load_texture("./resources/textures/Gibbon.jpg".to_string(), display);

    let alignment = Align2::LEFT_BOTTOM;
    Shape{vertex_buffer: shape, model_matrix: movement,  placement_matrix: movement,texture, ui_state: ShapeUI::default("B Plane".to_string(), alignment)}
}
