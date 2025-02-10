#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);


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




