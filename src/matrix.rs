use std::collections::HashMap;
use std::error::Error;

use cgmath::BaseNum;
use cgmath::Matrix4;
use cgmath::Vector3;
use cgmath::Vector4;
use crevice::std140::AsStd140;
use crevice::std140::Std140;
use crevice::std140::Writer;
use faer::Mat;
use faer::Shape;
use wgpu::core::device;
use wgpu::core::pipeline::ProgrammableStageDescriptor;
use wgpu::util::DeviceExt;
use wgpu::ComputePipelineDescriptor;
use wgpu::Device;

pub trait ToArr {
    type Output;
    fn to_arr(&self) -> Self::Output;
}

pub trait FromArr {
    type Input;
    fn from_arr(array: Self::Input) -> Self;
}

impl<T: BaseNum> ToArr for Matrix4<T> {
    type Output = [[T; 4]; 4];
    fn to_arr(&self) -> Self::Output {
        (*self).into()
    }
}

// Go back to array
impl<T: BaseNum> ToArr for Vector3<T> {
    type Output = [T; 3];
    fn to_arr(&self) -> Self::Output {
        (*self).into()
    }
}
// Create from array
impl<T: BaseNum> FromArr for Vector3<T> {
    type Input = [T; 3];
    fn from_arr(array: Self::Input) -> Vector3<T> {
        Vector3::new(array[0], array[1], array[2])
    }
}

// Go back to array
impl<T: BaseNum> ToArr for Vector4<T> {
    type Output = [T; 4];
    fn to_arr(&self) -> Self::Output {
        (*self).into()
    }
}
// Create from array
impl<T: BaseNum> FromArr for Vector4<T> {
    type Input = [T; 4];
    fn from_arr(array: Self::Input) -> Vector4<T> {
        Vector4::new(array[0], array[1], array[2], array[3])
    }
}

// Functionality for doing matrix multiplication
//
//
pub async fn nmf_pipeline(device: &Device, queue: &wgpu::Queue) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Test Shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../shaders/matrix_mul_transpose.wgsl").into(),
        ),
    });
    let element_wise_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Test Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/element_wise.wgsl").into()),
    }); // 0: A
        // 1: B
        // 2: C
        //3: dimensions (m, n, k)
    let a_bind_group_entry = wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::all(),
        count: None,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    };

    let b_bind_group_entry = wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::all(),
        count: None,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    };

    let c_bind_group_entry = wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::all(),
        count: None,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    };

    let dims_bind_group_entry = wgpu::BindGroupLayoutEntry {
        binding: 3,
        visibility: wgpu::ShaderStages::all(),
        count: None,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    };

    let matrix_bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind group for Matrix"),
        entries: &[
            a_bind_group_entry,
            b_bind_group_entry,
            c_bind_group_entry,
            dims_bind_group_entry,
        ],
    });

    // C = A ^T B
    let mat_a = Mat::from_fn(8, 8, |x, y| (x + y * 8) as f32);
    let mat_b = Mat::from_fn(8, 8, |x, y| (x - y) as f32);
    let mat_c = Mat::from_fn(8, 8, |_x, _y| 0 as f32);

    let test_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test buffer"),
        contents: &matrix_to_buffer(&mat_a).unwrap(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let test_b_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test buffer"),
        contents: &matrix_to_buffer(&mat_b).unwrap(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });

    let test_rw_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test rw buffer"),
        contents: &matrix_to_buffer(&mat_c).unwrap(),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::MAP_WRITE
            | wgpu::BufferUsages::MAP_READ
            | wgpu::BufferUsages::COPY_DST,
    });

    let test_unifrom = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test uniform"),
        contents: cgmath::vec3(8u32, 8u32, 8u32).as_std140().as_bytes(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Scene Bind"),
        layout: &matrix_bind_group,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: test_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: test_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: test_rw_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: test_unifrom.as_entire_binding(),
            },
        ],
    });

    let compute_pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&matrix_bind_group],
        push_constant_ranges: &[],
    });
    let compute_pipe = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("Matrix Multiply Pass"),
        layout: Some(&compute_pipe_layout),
        cache: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
    });

    let element_pass = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("Matrix Multiply Pass"),
        layout: Some(&compute_pipe_layout),
        cache: None,
        module: &element_wise_shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let compute_pass_desc = wgpu::ComputePassDescriptor {
            label: Some("Compute pass"),
            timestamp_writes: None,
        };
        let mut compute_pass = encoder.begin_compute_pass(&compute_pass_desc);
        compute_pass.set_pipeline(&compute_pipe);
        compute_pass.set_bind_group(0, Some(&binding), &[]);
        compute_pass.dispatch_workgroups(8, 8, 1);

        // compute_pass.set_pipeline(&element_pass);
        // compute_pass.set_bind_group(0, Some(&binding), &[]);
        // compute_pass.dispatch_workgroups(64, 0, 0);
    }

    queue.submit(Some(encoder.finish()));
    {
        let buffer_slice = test_rw_buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.receive().await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let data_filtered: Vec<f32> = data
            .chunks(4)
            .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        let mat = Mat::from_fn(8, 8, |x, y| data_filtered[x + y * 8]);
        println!("Buffer: {:?}", mat);
        let other_mat = &mat_a * &mat_a.transpose();
        println!("Faer says: {:?}", other_mat);
    }
}

// Create resources
// Two shader passes that need to be done repeatedly

pub fn matrix_to_buffer<T: crevice::std140::Std140 + std::fmt::Debug>(
    mat: &Mat<T, impl Shape, impl Shape>,
) -> Result<Box<[u8]>, std::io::Error> {
    // subject to change, matrix will get larger?
    // should be 64 f32s, as f32 needs 4 bytes?
    //
    let max_use = 4 * mat.shape().0.unbound() * mat.shape().1.unbound();
    let mut buffer = vec![0u8; max_use];

    let mut writer = Writer::new(&mut buffer[..max_use]);

    // How should the gpu see the world?
    // Drop this into a vec first
    for x in mat.col_iter() {
        for y in x.iter() {
            let _write = writer.write_std140(y)?;
        }
    }
    //WARNING: Into boxed slice may remove excess capicity. For large matrices, this is
    // suspicious!
    Ok(buffer.into_boxed_slice())
    //
    // Need to take the matrix and produce a vec of numbers
}
