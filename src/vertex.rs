use crevice::std140::AsStd140;

#[derive(Copy, Clone, PartialEq, Debug, crevice::std140::AsStd140)]
pub struct Vertex {
    position: cgmath::Vector3<f32>,
}
impl Vertex {
    // Tell WGPU the order we want our vertex to come in
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::std140_size_static() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
