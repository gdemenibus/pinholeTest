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

    }
    
}
pub struct ShapeUI {
    pub title: String,
    pub distance: f32,
    pub resolution: (f32, f32),
    pub resolution_lock: bool,
    pub pixel_size: f32,
    pub pixel_lock: bool,
    pub size: (f32, f32),
    pub size_lock: bool,
    pub alignment: Align2
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
                    ui.add(egui::DragValue::new(&mut self.resolution.0).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.resolution.1).speed(1.0));
                    if self.resolution_lock {
                        ui.checkbox(&mut self.resolution_lock, "🔒".to_string());
                    }
                    else {

                        ui.checkbox(&mut self.resolution_lock, "🔓".to_string());
                    }
                    ui.end_row();
                    ui.label("Pixel Size");
                    ui.add(egui::Slider::new(&mut self.pixel_size, -10.0..=10.0));

                    if self.pixel_lock {
                        ui.checkbox(&mut self.pixel_lock, "🔒".to_string());
                    }
                    else {

                        ui.checkbox(&mut self.pixel_lock, "🔓".to_string());
                    }
                    ui.end_row();
                    ui.label("Physical Size:");
                    ui.add(egui::DragValue::new(&mut self.size.0).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.size.1).speed(1.0));

                    if self.size_lock {
                        ui.checkbox(&mut self.size_lock, "🔒".to_string());
                    }
                    else {

                        ui.checkbox(&mut self.size_lock, "🔓".to_string());
                    }
                    ui.end_row();

                });

        });

    }
    pub fn default(title: String, alignment: Align2) -> ShapeUI {
        ShapeUI {
            title,
            distance: 10.0,
            resolution: (10.0, 10.0),
            resolution_lock: false,
            pixel_size: 1.0,
            pixel_lock: false,
            size: (10.0, 10.0),
            size_lock: false,
            alignment,
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
    Shape{vertex_buffer: shape, model_matrix: movement, texture, ui_state: ShapeUI::default("Far Plane".to_string(), alignment)}
}

