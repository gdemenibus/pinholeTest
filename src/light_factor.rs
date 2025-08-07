use cgmath::Vector2;
use egui::Ui;
use faer::sparse::{SparseColMat, Triplet};
use image::DynamicImage;
use wgpu::{util::DeviceExt, Buffer};
use winit::event_loop::EventLoopProxy;

use crate::utils::buffer_to_sparse_triplet;
use crate::utils::DrawUI;
use crate::*;

/// Objective is to implement the write up
pub struct LFBuffers {
    m_a_y_buffer: Buffer,
    m_a_x_buffer: Buffer,
    m_b_y_buffer: Buffer,
    m_b_x_buffer: Buffer,
    m_t_x_buffer: Buffer,
    m_t_y_buffer: Buffer,

    matrix_rep: Option<LFMatrices>,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    settings: LFSettings,
}

impl LFBuffers {
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
        let mut layout_entry_4 = layout_entry_0;
        let mut layout_entry_5 = layout_entry_0;
        layout_entry_1.binding = 1;
        layout_entry_2.binding = 2;
        layout_entry_3.binding = 3;
        layout_entry_4.binding = 4;
        layout_entry_5.binding = 5;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LFFactorizer Bind group layout"),
            entries: &[
                layout_entry_0,
                layout_entry_1,
                layout_entry_2,
                layout_entry_3,
                layout_entry_4,
                layout_entry_5,
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

        let m_t_y_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("m_t_y buffer"),
            // Resolution times 4, as it's a floating 32 per entry, and 3 entries
            contents: &[0u8; 2560 * 1600 * 4 * 3],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let m_t_x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("m_t_x buffer"),
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
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: m_t_y_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: m_t_x_buffer.as_entire_binding(),
                },
            ],
        });
        let settings = crate::LFSettings {
            debug_prints: true,
            ..Default::default()
        };
        Self {
            matrix_rep: None,
            m_a_y_buffer,
            m_a_x_buffer,
            m_b_y_buffer,
            m_b_x_buffer,
            m_t_y_buffer,
            m_t_x_buffer,
            bind_group_layout,
            bind_group,
            settings,
        }
    }

    pub fn has_solved(&mut self) {
        self.settings.solve_next_redraw_flag = false;
    }
    pub fn will_solve(&self) -> bool {
        self.settings.solve_next_redraw_flag
    }

    pub fn build_sparse_matrix(
        triplets: Vec<Triplet<u32, u32, f32>>,
        rows: u32,
        columns: u32,
    ) -> SparseColMat<u32, f32> {
        SparseColMat::try_new_from_triplets(rows as usize, columns as usize, &triplets).unwrap()
    }
    pub fn build_m_t(
        &self,
        number_of_view_points: u32,
        device: &wgpu::Device,
        rays_cast: (u32, u32),

        rays_cast_per_viewpoint: (u32, u32),
        target_size: (u32, u32),
    ) -> CompleteMapping {
        let m_t_x = {
            let vec_t_x = buffer_to_sparse_triplet(&self.m_t_x_buffer, device, rays_cast.0);
            let columns = target_size.0;

            let triplets = utils::build_tripltes(vec_t_x, rays_cast_per_viewpoint.0 as usize);

            let matrix = triplets
                .iter()
                .map(|triplet_list| {
                    SparseColMat::try_new_from_triplets(
                        rays_cast_per_viewpoint.0 as usize,
                        columns as usize,
                        triplet_list,
                    )
                    .unwrap()
                })
                .collect();

            MappingMatrix { matrix }
        };

        let m_t_y = {
            let vec_t_y = buffer_to_sparse_triplet(&self.m_t_y_buffer, device, rays_cast.1);
            let columns = target_size.1;

            let triplets = utils::build_tripltes(vec_t_y, rays_cast_per_viewpoint.1 as usize);

            let matrix = triplets
                .iter()
                .map(|triplet_list| {
                    SparseColMat::try_new_from_triplets(
                        rays_cast_per_viewpoint.1 as usize,
                        columns as usize,
                        triplet_list,
                    )
                    .unwrap()
                })
                .collect();
            MappingMatrix { matrix }
        };

        CompleteMapping {
            x: m_t_x,
            y: m_t_y,
            size: target_size,
        }
    }

    pub fn build_m_a(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        rays_cast_per_viewpoint: (u32, u32),
        panel_size: (u32, u32),
    ) -> CompleteMapping {
        let m_a_y = {
            let vec_a_y = buffer_to_sparse_triplet(&self.m_a_y_buffer, device, rays_cast.1);

            let columns = panel_size.0;

            let triplets = utils::build_tripltes(vec_a_y, rays_cast_per_viewpoint.1 as usize);

            let matrix = triplets
                .iter()
                .map(|triplet_list| {
                    SparseColMat::try_new_from_triplets(
                        rays_cast_per_viewpoint.1 as usize,
                        columns as usize,
                        triplet_list,
                    )
                    .unwrap()
                })
                .collect();
            MappingMatrix { matrix }
        };

        let m_a_x = {
            let vec_a_x = buffer_to_sparse_triplet(&self.m_a_x_buffer, device, rays_cast.0);
            let columns = panel_size.1;

            let triplets = utils::build_tripltes(vec_a_x, rays_cast_per_viewpoint.0 as usize);

            let matrix = triplets
                .iter()
                .map(|triplet_list| {
                    SparseColMat::try_new_from_triplets(
                        rays_cast_per_viewpoint.0 as usize,
                        columns as usize,
                        triplet_list,
                    )
                    .unwrap()
                })
                .collect();
            MappingMatrix { matrix }
        };

        CompleteMapping {
            x: m_a_x,
            y: m_a_y,
            size: panel_size,
        }
    }

    pub fn build_m_b(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        rays_cast_per_viewpoint: (u32, u32),
        panel_size: (u32, u32),
    ) -> CompleteMapping {
        let m_b_y = {
            let vec_b_y = buffer_to_sparse_triplet(&self.m_b_y_buffer, device, rays_cast.1);

            let columns = panel_size.0;

            let triplets = utils::build_tripltes(vec_b_y, rays_cast_per_viewpoint.1 as usize);

            let matrix = triplets
                .iter()
                .map(|triplet_list| {
                    SparseColMat::try_new_from_triplets(
                        rays_cast_per_viewpoint.1 as usize,
                        columns as usize,
                        triplet_list,
                    )
                    .unwrap()
                })
                .collect();
            MappingMatrix { matrix }
        };

        let m_b_x = {
            let vec_b_x = buffer_to_sparse_triplet(&self.m_b_x_buffer, device, rays_cast.0);
            let columns = panel_size.1;

            let triplets = utils::build_tripltes(vec_b_x, rays_cast_per_viewpoint.0 as usize);

            let matrix = triplets
                .iter()
                .map(|triplet_list| {
                    SparseColMat::try_new_from_triplets(
                        rays_cast_per_viewpoint.0 as usize,
                        columns as usize,
                        triplet_list,
                    )
                    .unwrap()
                })
                .collect();
            MappingMatrix { matrix }
        };
        CompleteMapping {
            x: m_b_x,
            y: m_b_y,
            size: panel_size,
        }
    }

    pub fn sample_light_field(
        &mut self,
        device: &wgpu::Device,
        pixel_count_a: Vector2<u32>,
        pixel_count_b: Vector2<u32>,

        c_t: &DynamicImage,
        target_size: (u32, u32),
        number_of_view_points: u32,
    ) {
        let number_of_rays = (
            target_size.1 * number_of_view_points,
            target_size.0 * number_of_view_points,
        );

        let rays_per_view_point = (target_size.1, target_size.0);

        let panel_a_size = (pixel_count_a.x, pixel_count_a.y);
        let panel_b_size = (pixel_count_b.x, pixel_count_b.y);

        let a = self.build_m_a(device, number_of_rays, rays_per_view_point, panel_a_size);
        let b = self.build_m_b(device, number_of_rays, rays_per_view_point, panel_b_size);
        // TO BE CHANGED SOON
        let t = self.build_m_t(
            number_of_view_points,
            device,
            number_of_rays,
            rays_per_view_point,
            rays_per_view_point,
        );

        let matrices = LFMatrices {
            a,
            b,
            t,
            c_t: c_t.clone(),
            target_size,
            number_of_view_points,
        };

        self.matrix_rep = Some(matrices);
    }

    pub fn alternative_factorization(
        &self,
    ) -> Option<(DynamicImage, DynamicImage, Option<Vec<f32>>)> {
        if let Some(rep) = &self.matrix_rep {
            rep.factorize(&self.settings)
        } else {
            None
        }
    }
}

impl DrawUI for LFBuffers {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut Ui>) {
        let title = title.unwrap_or("Separable Approach".to_string());
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
                    if let Some(rep) = self.matrix_rep.as_ref() {
                        rep.save(self.settings.save_to.clone());
                    }
                }
                self.settings.draw_ui(ctx, Some(title), Some(ui));
            });
    }
}
