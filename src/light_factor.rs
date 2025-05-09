use egui::ahash::HashSet;
use faer::Mat;
use image::{DynamicImage, GenericImageView};
use wgpu::{util::DeviceExt, Buffer};

use crate::utils;

/// Objective is to implement the write up
pub fn image_to_matrix(image: DynamicImage) -> Mat<f32> {
    let (rows, columns) = (image.width() as usize, image.height() as usize);

    Mat::from_fn(rows, columns, |x, y| {
        let pixel = image.get_pixel(x as u32, y as u32);
        if pixel.0[3] >= 255 / 2 {
            1.0
        } else {
            0.0
        }
    })
}
pub struct LFFactorizer {
    m_a_y_buffer: Buffer,
    m_a_x_buffer: Buffer,
    m_b_y_buffer: Buffer,
    m_b_x_buffer: Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}
impl LFFactorizer {
    /// Sets up the light field factorizer step on the gpu, as well as creating the struct
    /// To be added to the pipeline layout
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
        let mut layout_entry_3 = layout_entry_0;
        layout_entry_1.binding = 1;
        layout_entry_2.binding = 2;
        layout_entry_3.binding = 3;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LFFactorizer Bind group layout"),
            entries: &[
                layout_entry_0,
                layout_entry_1,
                layout_entry_2,
                layout_entry_3,
            ],
        });

        let m_a_y_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("M_a_y buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let m_a_x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("m_a_x buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let m_b_y_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("m_b_y buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let m_b_x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("m_b_x buffer"),
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
                    resource: m_a_y_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: m_a_x_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: m_b_y_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: m_b_x_buffer.as_entire_binding(),
                },
            ],
        });
        Self {
            m_a_y_buffer,
            m_a_x_buffer,
            m_b_y_buffer,
            m_b_x_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn sample_buffer_a_y(&self, device: &wgpu::Device) {
        utils::sample_buffer(&self.m_a_y_buffer, device);
    }
}
