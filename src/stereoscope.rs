use faer::sparse::SparseColMat;
use faer::Mat;
use wgpu::{util::DeviceExt, Buffer};
pub struct StereoscopeBuffer {
    l_buffer: Buffer,
    a_buffer: Buffer,
    b_buffer: Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    iter_count: usize,
    show_steps: bool,
    starting_values: (f32, f32),
    rng: bool,
    sample_next_redraw_flag: bool,
    solve_next_redraw_flag: bool,
    blend: bool,
    blend_sigma: f32,
    early_stop: bool,
    matrix_rep: Option<StereoMatrix>,
    filter: bool,
    save_error: bool,
}

#[derive(Clone)]
pub struct StereoMatrix {
    l_vec: Mat<f32>,
    // Might get away with a hashmap, as this is a matrix free operation.
    a_matrix: SparseColMat<u32, f32>,
    b_matrix: SparseColMat<u32, f32>,
    //
}
impl StereoscopeBuffer {
    pub fn set_up(device: &wgpu::Device) -> Self {
        let layout_entry_0 = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };

        // Bind group implements copy, so we are actually copying it around
        let mut layout_entry_1 = layout_entry_0;
        let mut layout_entry_2 = layout_entry_0;
        layout_entry_1.binding = 1;
        layout_entry_2.binding = 2;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LFFactorizer Bind group layout"),
            entries: &[layout_entry_0, layout_entry_1, layout_entry_2],
        });

        let a_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("M_A buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let b_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("M_B buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let l_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("T buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind group for Sampler"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: l_buffer.as_entire_binding(),
                },
            ],
        });
        Self {
            a_buffer,
            b_buffer,
            l_buffer,
            bind_group_layout,
            bind_group,
            rng: false,
            iter_count: 50,
            show_steps: false,
            starting_values: (0.5, 0.5),
            sample_next_redraw_flag: false,
            solve_next_redraw_flag: false,
            blend: false,
            blend_sigma: 0.1f32,
            matrix_rep: None,
            early_stop: false,
            filter: false,
            save_error: true,
        }
    }
}
