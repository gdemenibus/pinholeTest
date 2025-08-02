use std::time::Instant;

use cgmath::Vector2;
use egui::Ui;
use faer::sparse::{SparseColMat, Triplet};
use faer::Mat;
use image::DynamicImage;
use wgpu::{util::DeviceExt, Buffer};

use crate::utils::DrawUI;
use crate::*;

pub struct StereoscopeBuffer {
    l_buffer: Buffer,
    a_buffer: Buffer,
    b_buffer: Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    matrix_rep: Option<StereoMatrix>,
    settings: crate::LFSettings,
}
const BUFFER_SIZE: usize = 6000 * 6000 * 4 * 10;

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
            contents: &[0u8; BUFFER_SIZE],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let b_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("M_B buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; BUFFER_SIZE],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let l_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("T buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; BUFFER_SIZE],
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
        let settings = LFSettings::default();
        Self {
            a_buffer,
            b_buffer,
            l_buffer,
            bind_group_layout,
            bind_group,
            matrix_rep: None,
            settings,
        }
    }

    pub fn build_l(&self, device: &wgpu::Device, rays_cast: u32) -> Mat<f32> {
        let raw_bytes = utils::sample_buffer(&self.l_buffer, device);
        let entries: Vec<f32> = raw_bytes[0..(rays_cast * 4) as usize]
            .chunks(4)
            .map(|x| f32::from_ne_bytes(x[0..4].try_into().unwrap()))
            .collect();
        println!("L is of size: {}", entries.len());
        Mat::from_fn(rays_cast as usize, 1, |x, _y| entries[x])
        // TODO:
        // Debug print to check for sanity (is it being sampled correctly)
    }

    pub fn build_m_a(
        &self,
        device: &wgpu::Device,
        rays_cast: u32,
        panel_size: (u32, u32),
    ) -> SparseColMat<u32, f32> {
        // Build triplets
        // Build Matrix from Triplets
        let rows = rays_cast;
        let columns = panel_size.0 * panel_size.1;

        let vec_a = utils::buffer_to_sparse_triplet(&self.a_buffer, device, rays_cast);
        let mut triplets = vec_a
            .into_iter()
            .enumerate()
            .map(|(index, entry)| Triplet::new(index as u32, entry, 1.0f32))
            .collect();
        utils::check_triplets(rows, columns, &mut triplets);

        SparseColMat::try_new_from_triplets(rows as usize, columns as usize, &triplets).unwrap()
    }

    pub fn build_m_b(
        &self,
        device: &wgpu::Device,
        rays_cast: u32,
        panel_size: (u32, u32),
    ) -> SparseColMat<u32, f32> {
        // Build triplets
        // Build Matrix from Triplets
        let rows = rays_cast;
        let columns = panel_size.0 * panel_size.1;

        let vec_b = utils::buffer_to_sparse_triplet(&self.b_buffer, device, rays_cast);
        let mut triplets = vec_b
            .into_iter()
            .enumerate()
            .map(|(index, entry)| Triplet::new(index as u32, entry, 1.0f32))
            .collect();
        utils::check_triplets(rows, columns, &mut triplets);

        SparseColMat::try_new_from_triplets(rows as usize, columns as usize, &triplets).unwrap()
    }
    pub fn sample_light_field(
        &mut self,
        device: &wgpu::Device,
        pixel_count_a: Vector2<u32>,
        pixel_count_b: Vector2<u32>,
        target_size: (u32, u32),
        number_of_view_points: u32,
    ) {
        let rays_cast = target_size.0 * target_size.1 * number_of_view_points;
        let panel_a_size = (pixel_count_a.x, pixel_count_a.y);
        let panel_b_size = (pixel_count_b.x, pixel_count_b.y);
        let l_vec = self.build_l(device, rays_cast);
        let ray_total_memory = 4 * rays_cast;
        if ray_total_memory as usize > BUFFER_SIZE {
            panic!("Cannot store the results of all rays in allocated buffers");
        }

        let a_matrix = self.build_m_a(device, rays_cast, panel_a_size).into();
        let b_matrix = self.build_m_b(device, rays_cast, panel_b_size).into();
        let stereo = StereoMatrix {
            panel_a_size: (pixel_count_a.x, pixel_count_a.y),
            panel_b_size: (pixel_count_b.x, pixel_count_b.y),
            l_vec,
            a_matrix,
            b_matrix,
            target_size,
            number_of_view_points,
        };
        self.matrix_rep = Some(stereo);
    }

    pub fn has_solved(&mut self) {
        self.settings.solve_next_redraw_flag = false;
    }
    pub fn will_solve(&self) -> bool {
        self.settings.solve_next_redraw_flag
    }
    pub fn factorize_stereo(&self) -> Option<(DynamicImage, DynamicImage, Option<Vec<f32>>)> {
        self.matrix_rep.as_ref()?.factorize(&self.settings)
    }
}

impl DrawUI for StereoscopeBuffer {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut Ui>) {
        let title = title.unwrap_or("Stereo Settings".to_string());
        let _ = ui;

        egui_winit::egui::Window::new(&title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 150.0])
            .default_open(false)
            .show(ctx, |ui| {
                if ui.button("Reset").clicked() {
                    self.matrix_rep = None;
                }

                if ui.button("Save").clicked() {
                    if let Some(rep) = &self.matrix_rep {
                        rep.save(self.settings.save_to.clone());
                    }
                }
                self.settings.draw_ui(ctx, Some(title), Some(ui));
            });
    }
}
