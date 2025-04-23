use std::collections::HashMap;

use cgmath::BaseNum;
use cgmath::Matrix4;
use cgmath::Vector3;
use cgmath::Vector4;
use wgpu::core::device;
use wgpu::core::pipeline::ProgrammableStageDescriptor;
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
pub async fn nmf_pipeline(device: Device) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Test Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/matrix_mul.wgsl").into()),
    });
    let compute_pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });
    let compute_pipe = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("Compute Pass"),
        layout: Some(&compute_pipe_layout),
        cache: None,
        module: &shader,
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
        compute_pass.dispatch_workgroups(8, 8, 0);
    }

    // Create resources
    // Two shader passes that need to be done repeatedly
}
