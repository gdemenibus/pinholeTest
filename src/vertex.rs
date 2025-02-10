use cgmath::{BaseNum, Matrix4, Rad, Vector3};
use glium::{Program, Texture2d, VertexBuffer};

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
    pub texture: Option<Texture2d>,
    pub program: Option<Program>,
}
impl Shape {
    pub fn draw(self){

    }
}

pub fn floor() -> Shape {
    let shape = vec![
        Vertex { position: [-0.5, 0.0, -0.5], tex_coords: [0.0, 0.0] },
        Vertex { position: [ 0.5, 0.0, -0.5], tex_coords: [1.0, 0.0] },
        Vertex { position: [ 0.5, 0.0,  0.5], tex_coords: [1.0, 1.0] },

        Vertex { position: [ 0.5,  0.0, 0.5], tex_coords: [1.0, 1.0] },
        Vertex { position: [-0.5,  0.0, 0.5], tex_coords: [0.0, 1.0] },
        Vertex { position: [-0.5,  0.0,-0.5], tex_coords: [0.0, 0.0] },
    ];
    
    let translate: Matrix4<f32>= Matrix4::from_translation(Vector3::new(0.0, -1.0, 0.0));
    let matrix: Matrix4<f32>  = Matrix4::from_scale(10.0);
    let movement = translate * matrix;
    Shape{vertex_buffer: shape, model_matrix: movement, texture: None, program: None}
}

