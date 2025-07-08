use std::num::NonZero;

use cgmath::Vector2;
use egui::{panel, Ui};
use faer::sparse::{SparseColMat, Triplet};
use faer::stats::prelude::{thread_rng, Rng};
use faer::{unzip, zip, Mat};
use image::DynamicImage;
use wgpu::{util::DeviceExt, Buffer};

use crate::scene::DrawUI;
use crate::utils;
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
    blend: bool,
    blend_sigma: f32,
    early_stop: bool,
    matrix_rep: Option<StereoMatrix>,
    filter: bool,
    save_error: bool,
}
const BUFFER_SIZE: usize = 2560 * 1600 * 4 * 3;

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
            blend: false,
            blend_sigma: 0.1f32,
            matrix_rep: None,
            early_stop: false,
            filter: false,
            save_error: true,
        }
    }

    pub fn build_l(&self, device: &wgpu::Device, rays_cast: (u32, u32)) -> Mat<f32> {
        let raw_bytes = utils::sample_buffer(&self.l_buffer, device);
        let entries: Vec<f32> = raw_bytes
            .chunks(4)
            .map(|x| f32::from_ne_bytes(x[0..4].try_into().unwrap()))
            .collect();
        let ray_count = (rays_cast.0 * rays_cast.1) as usize;
        Mat::from_fn(ray_count, 1, |x, _y| entries[x])
        // TODO:
        // Debug print to check for sanity (is it being sampled correctly)
    }
    pub fn build_m_a(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        panel_size: (u32, u32),
    ) -> SparseColMat<u32, f32> {
        // Build triplets
        // Build Matrix from Triplets
        let rows = rays_cast.0 * rays_cast.1;
        let columns = panel_size.0 * panel_size.1;
        let triplets = utils::buffer_to_triplet(&self.a_buffer, device);
        let mut triplet_list = triplets
            .iter()
            .map(|(x, y)| Triplet::new(*x, *y, 1.0f32))
            .collect();
        utils::check_triplets(rows, columns, &mut triplet_list);

        SparseColMat::try_new_from_triplets(rows as usize, columns as usize, &triplet_list).unwrap()
    }

    pub fn build_m_b(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        panel_size: (u32, u32),
    ) -> SparseColMat<u32, f32> {
        // Build triplets
        // Build Matrix from Triplets
        let rows = rays_cast.0 * rays_cast.1;
        let columns = panel_size.0 * panel_size.1;
        let triplets = utils::buffer_to_triplet(&self.b_buffer, device);
        let mut triplet_list = triplets
            .iter()
            .map(|(x, y)| Triplet::new(*x, *y, 1.0f32))
            .collect();
        utils::check_triplets(rows, columns, &mut triplet_list);

        SparseColMat::try_new_from_triplets(rows as usize, columns as usize, &triplet_list).unwrap()
    }
    pub fn sample_light_field(
        &mut self,
        device: &wgpu::Device,
        pixel_count_a: Vector2<u32>,
        pixel_count_b: Vector2<u32>,
        target_size: (u32, u32),
        number_of_view_points: u32,
    ) {
        let rays_cast = (
            target_size.0 * number_of_view_points,
            target_size.1 * number_of_view_points,
        );
        let panel_a_size = (pixel_count_a.x, pixel_count_a.y);
        let panel_b_size = (pixel_count_b.x, pixel_count_b.y);
        println!("Building L");
        let l_vec = self.build_l(device, rays_cast);
        // Should look like content stacked in some way
        let image = utils::vector_to_image(&l_vec, rays_cast.0, rays_cast.1);
        image
            .save_with_format(
                "./resources/panel_compute/intermediate/L.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        // Warning in case scene exceds buffers.
        // Worst case analysis: Every Ray Creates a triplet
        // Triplet is 4 bytes * 3, 12 bytes
        //
        let ray_total_memory = 12 * rays_cast.0 * rays_cast.1;
        if ray_total_memory as usize > BUFFER_SIZE {
            panic!("Cannot store the results of all rays in allocated buffers");
        }

        println!("Building Stereo A");
        let a_matrix = self.build_m_a(device, rays_cast, panel_a_size);
        println!("Building Stereo B");
        let b_matrix = self.build_m_b(device, rays_cast, panel_b_size);
        let stereo = StereoMatrix {
            l_vec,
            a_matrix,
            b_matrix,
        };
        self.matrix_rep = Some(stereo);
    }
    pub fn factorize_stereo(
        &mut self,
        panel_a_size: (u32, u32),
        panel_b_size: (u32, u32),
    ) -> Option<(DynamicImage, DynamicImage, Option<Vec<f32>>)> {
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));
        println!(
            "Global Parallelism is: {:?}",
            faer::get_global_parallelism()
        );

        let matrices = self.matrix_rep.as_ref()?;
        let rows_a = panel_a_size.0 * panel_a_size.1;
        let rows_b = panel_b_size.0 * panel_b_size.1;
        let mut vec_a = Mat::from_fn(rows_a as usize, 1, |_x, _y| {
            if self.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                self.starting_values.0
            }
        });

        let mut vec_b = Mat::from_fn(rows_b as usize, 1, |_x, _y| {
            if self.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                self.starting_values.1
            }
        });
        // Precompute the transpose
        let m_a_trans = matrices.a_matrix.transpose();
        let m_b_trans = matrices.b_matrix.transpose();

        println!("Computing Stereo Approach");
        let progress_bar = indicatif::ProgressBar::new(self.iter_count as u64);
        for _x in 0..self.iter_count {
            progress_bar.inc(1);
            // Step for A
            {
                // Upper area
                let t2_rays = &matrices.b_matrix * &vec_b;
                let t1_rays = &matrices.a_matrix * &vec_a;

                let upper = zip!(&t2_rays, &matrices.l_vec).map(|unzip!(u, l)| *u * *l);
                let numerator = m_a_trans * upper;

                let lower = zip!(&t2_rays, &t1_rays).map(|unzip!(t2, t1)| *t2 * *t2 * *t1);
                let denominator = m_a_trans * lower;
                zip!(&mut vec_a, &numerator, &denominator)
                    .for_each(|unzip!(a, n, d)| *a = 1.0_f32.min(*a * *n / (*d + 0.0000001f32)));

                // Denominator
            }
            {
                let t2_rays = &matrices.b_matrix * &vec_b;
                let t1_rays = &matrices.a_matrix * &vec_a;

                let upper = zip!(&t1_rays, &matrices.l_vec).map(|unzip!(u, l)| *u * *l);
                let numerator = m_b_trans * upper;

                let lower = zip!(&t2_rays, &t1_rays).map(|unzip!(t2, t1)| *t2 * *t1 * *t1);
                let denominator = m_b_trans * lower;

                zip!(&mut vec_b, &numerator, &denominator)
                    .for_each(|unzip!(b, n, d)| *b = 1.0_f32.min(*b * *n / (*d + 0.0000001f32)));
            }
            {
                // Compute error
            }
        }
        utils::verify_matrix(&vec_a);
        utils::verify_matrix(&vec_b);
        let a = utils::vector_to_image(&vec_a, panel_a_size.0, panel_a_size.1);
        let b = utils::vector_to_image(&vec_b, panel_b_size.0, panel_b_size.1);
        Some((a, b, None))

        // Build a && B
        // M_a^T (L Hm M_B b)
        // Divided by
        // M_a^T(M_a a Hm M_b b HM M_b b)
    }
}

impl DrawUI for StereoscopeBuffer {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut Ui>) {
        let title = title.unwrap_or("Stereo Settings".to_string());
        let _ = ui;

        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 150.0])
            .default_open(false)
            .show(ctx, |ui| {
                ui.label("Iteration count");
                ui.add(egui::Slider::new(&mut self.iter_count, 1..=1000));
                ui.checkbox(&mut self.show_steps, "Print steps");
                ui.checkbox(&mut self.early_stop, "Early stop?");
                ui.checkbox(&mut self.filter, "Filter Columns");
                ui.checkbox(&mut self.save_error, "Save Error");

                if ui.button("Reset").clicked() {
                    self.matrix_rep = None;
                }
                ui.checkbox(&mut self.blend, "Blend Out Image");
                ui.label("Sigma");
                ui.add(egui::Slider::new(&mut self.blend_sigma, 0.0..=1.0));
                ui.checkbox(&mut self.rng, "Random starting values");
                ui.label("Initial guesses");
                ui.add(egui::Slider::new(
                    &mut self.starting_values.0,
                    0.0f32..=1.0f32,
                ));

                ui.add(egui::Slider::new(
                    &mut self.starting_values.1,
                    0.0f32..=1.0f32,
                ));
            });
    }
}
