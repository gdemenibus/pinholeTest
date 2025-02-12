use cgmath::{BaseNum, Matrix4, Vector3};
use egui_glium::egui_winit::egui::{self, Align2, Context, Ui};
use glium::{glutin::surface::WindowSurface, winit::window::Window, Display, Texture2d};
use crate::texture;
use crate::matrix::{ToArr, FromArr};

#[derive(Copy, Clone)]
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
    pub texture: glium::Texture2d,
    pub ui_state: ShapeUI
}
impl Shape {
    pub fn new(vertex_buffer: Vec<Vertex>, model_matrix: Matrix4<f32>, texture: Texture2d, title: String) -> Shape{

        Shape{vertex_buffer, model_matrix, texture, ui_state: ShapeUI::default(title, Align2::LEFT_TOP)}
    }

    fn distance(position: Matrix4<f32>, distance: f32) -> Matrix4<f32> {
        todo!()

    }
    pub fn position<F>(&mut self, transform: F) where F: FnOnce(&Matrix4<f32>, f32) -> Matrix4<f32>  {
        todo!()

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
    Shape{vertex_buffer: shape, model_matrix: movement, texture, ui_state: ShapeUI::default("Floor".to_string(), alignment)}
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
    Shape{vertex_buffer: shape, model_matrix: movement, texture, ui_state: ShapeUI::default("Far Plane".to_string(), alignment)}
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
    Shape{vertex_buffer: shape, model_matrix: movement, texture, ui_state: ShapeUI::default("A Plane".to_string(), alignment)}
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
    Shape{vertex_buffer: shape, model_matrix: movement, texture, ui_state: ShapeUI::default("B Plane".to_string(), alignment)}
}
