use std::{
    collections::VecDeque,
    num::NonZero,
    sync::mpsc::channel,
    thread,
    time::{Duration, Instant},
};

use cgmath::Vector2;
use egui::{epaint::text::InsertFontFamily, Ui};
use faer::{
    sparse::{SparseColMat, Triplet},
    stats::prelude::{thread_rng, Rng},
    unzip, zip, Mat, MatMut,
};
use image::DynamicImage;
use wgpu::{util::DeviceExt, Buffer};

use crate::{
    scene::DrawUI,
    utils::{self, buffer_to_sparse_triplet},
};

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
struct LFSettings {
    iter_count: usize,
    show_steps: bool,
    starting_values: (f32, f32),
    rng: bool,
    sample_next_redraw_flag: bool,
    solve_next_redraw_flag: bool,
    blend: bool,
    blend_sigma: f32,
    early_stop: bool,
    filter: bool,
    save_error: bool,
}

#[derive(Clone)]
pub struct MappingMatrix {
    matrix: SparseColMat<u32, f32>,
    mapping_col: Vec<Option<usize>>,
    mapping_row: Vec<Option<usize>>,
}

#[derive(Clone)]
pub struct CompleteMapping {
    x: MappingMatrix,
    y: MappingMatrix,
}

/// Struct to hold the matrices that we will build.
/// Observations will be
#[derive(Clone)]
pub struct LFMatrices {
    a: CompleteMapping,
    b: CompleteMapping,
    t: CompleteMapping,
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
        let settings = LFSettings {
            rng: false,
            iter_count: 50,
            show_steps: false,
            starting_values: (0.5, 0.5),
            sample_next_redraw_flag: false,
            solve_next_redraw_flag: false,
            blend: false,
            blend_sigma: 0.1f32,
            early_stop: false,
            filter: false,
            save_error: true,
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

    pub fn has_sampled(&mut self) {
        self.settings.sample_next_redraw_flag = false;
    }
    pub fn has_solved(&mut self) {
        self.settings.solve_next_redraw_flag = false;
    }
    pub fn will_sample(&self) -> bool {
        self.settings.sample_next_redraw_flag
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
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        target_size: (u32, u32),
    ) -> CompleteMapping {
        println!("Building M_t_y");

        let m_t_y = {
            let vec_t_y = buffer_to_sparse_triplet(&self.m_t_y_buffer, device, rays_cast.0);
            let rows = rays_cast.0;

            let columns = target_size.0;

            let triplets = utils::build_tripltes(vec_t_y, rows, columns);
            let mapping_col = utils::selection_col_vec_from_matrix(&triplets, rows as usize);
            let mapping_row = utils::selection_row_vec_from_matrix(&triplets, rows as usize);

            let matrix = Self::build_sparse_matrix(triplets, rows, columns);
            MappingMatrix {
                matrix,
                mapping_col,
                mapping_row,
            }
        };

        println!("Building M_t_x");
        let m_t_x = {
            let vec_t_x = buffer_to_sparse_triplet(&self.m_t_x_buffer, device, rays_cast.1);
            let rows = rays_cast.1;
            let columns = target_size.1;

            let triplets = utils::build_tripltes(vec_t_x, rows, columns);
            let mapping_col = utils::selection_col_vec_from_matrix(&triplets, columns as usize);
            let mapping_row = utils::selection_row_vec_from_matrix(&triplets, columns as usize);

            let matrix = Self::build_sparse_matrix(triplets, rows, columns);
            MappingMatrix {
                matrix,
                mapping_col,
                mapping_row,
            }
        };

        CompleteMapping { x: m_t_x, y: m_t_y }
    }

    pub fn build_m_a(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        panel_size: (u32, u32),
    ) -> CompleteMapping {
        println!("Building M_A_Y");

        let m_a_y = {
            let vec_a_y = buffer_to_sparse_triplet(&self.m_a_y_buffer, device, rays_cast.0);
            let rows = rays_cast.0;

            let columns = panel_size.0;
            let triplets = utils::build_tripltes(vec_a_y, rows, columns);
            let mapping_col = utils::selection_col_vec_from_matrix(&triplets, columns as usize);
            let mapping_row = utils::selection_row_vec_from_matrix(&triplets, columns as usize);

            let matrix = Self::build_sparse_matrix(triplets, rows, columns);
            MappingMatrix {
                matrix,
                mapping_col,
                mapping_row,
            }
        };

        println!("Building M_A_X");
        let m_a_x = {
            let vec_a_x = buffer_to_sparse_triplet(&self.m_a_x_buffer, device, rays_cast.1);
            let rows = rays_cast.1;
            let columns = panel_size.1;

            let triplets = utils::build_tripltes(vec_a_x, rows, columns);
            let mapping_col = utils::selection_col_vec_from_matrix(&triplets, columns as usize);
            let mapping_row = utils::selection_row_vec_from_matrix(&triplets, columns as usize);

            let matrix = Self::build_sparse_matrix(triplets, rows, columns);
            MappingMatrix {
                matrix,
                mapping_col,
                mapping_row,
            }
        };
        CompleteMapping { x: m_a_x, y: m_a_y }
    }

    pub fn build_m_b(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        panel_size: (u32, u32),
    ) -> CompleteMapping {
        println!("Building M_B_Y");
        let m_b_y = {
            let vec_b_y = buffer_to_sparse_triplet(&self.m_b_y_buffer, device, rays_cast.0);

            let rows = rays_cast.0;

            let columns = panel_size.0;
            let triplets = utils::build_tripltes(vec_b_y, rows, columns);
            let mapping_col = utils::selection_col_vec_from_matrix(&triplets, columns as usize);
            let mapping_row = utils::selection_row_vec_from_matrix(&triplets, columns as usize);

            let matrix = Self::build_sparse_matrix(triplets, rows, columns);
            MappingMatrix {
                matrix,
                mapping_col,
                mapping_row,
            }
        };
        println!("Building M_B_X");
        let m_b_x = {
            let vec_b_x = buffer_to_sparse_triplet(&self.m_b_x_buffer, device, rays_cast.1);
            let rows = rays_cast.1;
            let columns = panel_size.1;

            let triplets = utils::build_tripltes(vec_b_x, rows, columns);
            let mapping_col = utils::selection_col_vec_from_matrix(&triplets, columns as usize);
            let mapping_row = utils::selection_row_vec_from_matrix(&triplets, columns as usize);

            let matrix = Self::build_sparse_matrix(triplets, rows, columns);
            MappingMatrix {
                matrix,
                mapping_col,
                mapping_row,
            }
        };
        CompleteMapping { x: m_b_x, y: m_b_y }
    }
    pub fn sample_light_field(
        &mut self,
        device: &wgpu::Device,
        pixel_count_a: Vector2<u32>,
        pixel_count_b: Vector2<u32>,
        target_size: (u32, u32),
        number_of_view_points: u32,
    ) {
        let number_of_rays = (
            target_size.1 * number_of_view_points,
            target_size.0 * number_of_view_points,
        );
        let panel_a_size = (pixel_count_a.x, pixel_count_a.y);
        let panel_b_size = (pixel_count_b.x, pixel_count_b.y);

        let a = self.build_m_a(device, number_of_rays, panel_a_size);
        let b = self.build_m_b(device, number_of_rays, panel_b_size);
        // TO BE CHANGED SOON
        let t = self.build_m_t(device, number_of_rays, target_size);
        let matrices = LFMatrices { a, b, t };

        self.matrix_rep = Some(matrices);
    }

    pub fn factorize(
        &mut self,
        c_t: &DynamicImage,
        target_size: (u32, u32),
        number_of_view_points: u32,
    ) -> Option<(DynamicImage, DynamicImage, Option<Vec<f32>>)> {
        // Give 10 threads
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));
        println!(
            "Global Parallelism is: {:?}",
            faer::get_global_parallelism()
        );
        let settings = &self.settings;

        let rays_cast = (
            target_size.1 * number_of_view_points,
            target_size.0 * number_of_view_points,
        );

        let matrices = self.matrix_rep.as_ref()?;

        let m_a_y = &matrices.a.y.matrix;
        let m_a_x = &matrices.a.x.matrix;
        let m_b_y = &matrices.b.y.matrix;
        let m_b_x = &matrices.b.x.matrix;
        let m_t_y = &matrices.t.y.matrix;

        let m_t_x = &matrices.t.x.matrix.to_dense();

        let h_a = m_a_y.shape().1;
        let h_b = m_b_y.shape().1;

        let w_a = m_a_x.shape().1;
        let w_b = m_b_x.shape().1;

        println!("A_y shape: {:?}", m_a_y.shape());
        println!("A_x shape: {:?}", m_a_x.shape());
        println!("b_y shape: {:?}", m_b_y.shape());
        println!("b_x shape: {:?}", m_b_x.shape());
        println!("t_y shape: {:?}", m_t_y.shape());
        println!("t_x shape: {:?}", m_t_x.shape());
        println!("Rays cast: {rays_cast:?}");

        let c_t = utils::image_to_matrix(c_t);

        println!("C_T shape: {:?}", c_t.shape());
        utils::verify_matrix(&c_t);
        utils::matrix_to_image(&c_t)
            .save_with_format(
                "./resources/panel_compute/intermediate/C_T.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let mut c_a = Mat::from_fn(h_a, w_a, |_x, _y| {
            if self.settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                self.settings.starting_values.0
            }
        });
        let mut c_b = Mat::from_fn(h_b, w_b, |_x, _y| {
            if self.settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                self.settings.starting_values.1
            }
        });

        let mut upper = Mat::<f32>::zeros(rays_cast.0 as usize, rays_cast.1 as usize);

        let mut lower = Mat::<f32>::zeros(rays_cast.0 as usize, rays_cast.1 as usize);

        // Move IO out of loop and into dedicated thread
        let (sender, receiver) = channel::<(String, DynamicImage)>();
        thread::spawn(move || {
            for (path, image) in receiver {
                image.save_with_format(path, image::ImageFormat::Png).ok();
            }
        });

        // Doesn't change
        let c_t_m_product = (m_t_y * c_t) * m_t_x.transpose();
        let progress_bar = indicatif::ProgressBar::new(settings.iter_count as u64);
        let mut error = VecDeque::with_capacity(settings.iter_count);

        let mut time_taken_total: Vec<Duration> = Vec::with_capacity(settings.iter_count);
        for x in 0..settings.iter_count {
            let start = Instant::now();
            progress_bar.inc(1);

            if settings.show_steps {
                let path_1 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 1
                );
                let path_2 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 2
                );
                let image_a = utils::matrix_to_image(&c_a);
                let image_b = utils::matrix_to_image(&c_b);
                sender.send((path_1, image_a)).unwrap();
                sender.send((path_2, image_b)).unwrap();

                // Dispatch a thread to do
            }
            // CA update
            //
            {
                let c_b_m_product = m_b_y * &c_b * m_b_x.transpose();
                let c_a_m_product = m_a_y * &c_a * m_a_x.transpose();

                zip!(&mut upper, &c_b_m_product, &c_t_m_product).for_each(
                    |unzip!(upper, c_b, c_t)| {
                        *upper = *c_b * *c_t;
                    },
                );

                zip!(&mut lower, &c_b_m_product, &c_a_m_product).for_each(
                    |unzip!(lower, c_b, c_a)| {
                        *lower = *c_a * *c_b * *c_b;
                    },
                );
                let numerator = m_a_y.transpose() * &upper * m_a_x;
                let denominator = m_a_y.transpose() * &lower * m_a_x;
                zip!(&mut c_a, &numerator, &denominator).for_each(|unzip!(c_a, n, d)| {
                    *c_a = 1.0_f32.min(*c_a * *n / (*d + 0.0000001f32))
                });
            }
            // C_B Update
            {
                let c_b_m_product = m_b_y * &c_b * m_b_x.transpose();
                let c_a_m_product = m_a_y * &c_a * m_a_x.transpose();

                zip!(&mut upper, &c_a_m_product, &c_t_m_product).for_each(
                    |unzip!(upper, c_a, c_t)| {
                        *upper = *c_a * *c_t;
                    },
                );

                zip!(&mut lower, &c_b_m_product, &c_a_m_product).for_each(
                    |unzip!(lower, c_b, c_a)| {
                        *lower = *c_b * *c_a * *c_a;
                    },
                );

                let numerator = m_b_y.transpose() * &upper * m_b_x;
                let denominator = m_b_y.transpose() * &lower * m_b_x;
                zip!(&mut c_b, &numerator, &denominator).for_each(|unzip!(c_b, n, d)| {
                    *c_b = 1.0_f32.min(*c_b * *n / (*d + 0.000000001f32));
                });
            }

            if settings.save_error {
                let c_b_m_product = m_b_y * &c_b * m_b_x.transpose();
                let c_a_m_product = m_a_y * &c_a * m_a_x.transpose();
                zip!(&mut upper, &c_b_m_product, &c_a_m_product).for_each(
                    |unzip!(upper, c_b, c_a)| {
                        *upper = *c_b * *c_a;
                    },
                );
                let iter_error = &c_t_m_product - &upper;
                let error_norm = iter_error.norm_l2();

                if let Some(previous) = error.back() {
                    let diff: f32 = error_norm - previous;
                    if settings.early_stop && diff.abs() < 0.0000001f32 {
                        break;
                    }
                }
                error.push_back(error_norm);
            }
            let end = Instant::now();
            let time_taken = end.duration_since(start);
            time_taken_total.push(time_taken);
        }

        let total_time: Duration = time_taken_total.iter().sum();
        let average_time = total_time / settings.iter_count as u32;
        println!("Average time per iteration: {average_time:?}");
        if settings.filter {
            Self::filter_zeroes(c_a.as_mut(), &matrices.a);
            Self::filter_zeroes(c_b.as_mut(), &matrices.b);
        }
        utils::verify_matrix(&c_a);
        utils::verify_matrix(&c_b);

        let image_a = {
            let mut output = utils::matrix_to_image(&c_a);
            if settings.blend {
                output = output.fast_blur(settings.blend_sigma);
            }

            output
        };
        image_a
            .save_with_format(
                "./resources/panel_compute/panel_1.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let image_b = {
            let mut output = utils::matrix_to_image(&c_b);
            if settings.blend {
                output = output.fast_blur(settings.blend_sigma);
            }
            output
        };

        image_b
            .save_with_format(
                "./resources/panel_compute/panel_2.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        println!("Errors is: {error:?}");
        let error = {
            if settings.save_error {
                Some(error.into())
            } else {
                None
            }
        };

        Some((image_a, image_b, error))
    }

    pub fn alternative_factorization(
        &mut self,
        c_t: &DynamicImage,
        target_size: (u32, u32),
        number_of_view_points: u32,
    ) -> Option<(DynamicImage, DynamicImage, Option<Vec<f32>>)> {
        // Give 10 threads
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));
        println!(
            "Global Parallelism is: {:?}",
            faer::get_global_parallelism()
        );
        let settings = &self.settings;

        let rays_cast = (
            target_size.1 * number_of_view_points,
            target_size.0 * number_of_view_points,
        );

        let matrices = self.matrix_rep.as_ref()?;

        let m_a_y = &matrices.a.y.matrix.to_dense();
        let m_a_x = &matrices.a.x.matrix.to_dense();
        let m_b_y = &matrices.b.y.matrix.to_dense();
        let m_b_x = &matrices.b.x.matrix.to_dense();
        let m_t_y = &matrices.t.y.matrix.to_dense();

        let m_t_x = &matrices.t.x.matrix.to_dense();

        let h_a = m_a_y.shape().1;
        let h_b = m_b_y.shape().1;

        let w_a = m_a_x.shape().1;
        let w_b = m_b_x.shape().1;

        println!("A_y shape: {:?}", m_a_y.shape());
        println!("A_x shape: {:?}", m_a_x.shape());
        println!("b_y shape: {:?}", m_b_y.shape());
        println!("b_x shape: {:?}", m_b_x.shape());
        println!("t_y shape: {:?}", m_t_y.shape());
        println!("t_x shape: {:?}", m_t_x.shape());
        println!("Rays cast: {rays_cast:?}");

        let c_t = utils::image_to_matrix(c_t);

        println!("C_T shape: {:?}", c_t.shape());
        utils::verify_matrix(&c_t);
        utils::matrix_to_image(&c_t)
            .save_with_format(
                "./resources/panel_compute/intermediate/C_T.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let mut c_a = Mat::from_fn(h_a, w_a, |_x, _y| {
            if self.settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                self.settings.starting_values.0
            }
        });
        let mut c_b = Mat::from_fn(h_b, w_b, |_x, _y| {
            if self.settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                self.settings.starting_values.1
            }
        });
        let single_pass_size = (
            rays_cast.0 / number_of_view_points,
            rays_cast.1 / number_of_view_points,
        );

        let mut upper = Mat::<f32>::zeros(single_pass_size.0 as usize, single_pass_size.1 as usize);

        let mut lower = Mat::<f32>::zeros(single_pass_size.0 as usize, single_pass_size.1 as usize);

        // Move IO out of loop and into dedicated thread
        let (sender, receiver) = channel::<(String, DynamicImage)>();
        thread::spawn(move || {
            for (path, image) in receiver {
                image.save_with_format(path, image::ImageFormat::Png).ok();
            }
        });

        // Doesn't change

        let progress_bar = indicatif::ProgressBar::new(settings.iter_count as u64);
        let mut error = VecDeque::with_capacity(settings.iter_count);

        let mut time_taken_total: Vec<Duration> = Vec::with_capacity(settings.iter_count);

        let mut numerator_a = Mat::zeros(c_a.nrows(), c_a.ncols());
        let mut denominator_a = Mat::zeros(c_a.nrows(), c_a.ncols());

        let mut numerator_b = Mat::zeros(c_b.nrows(), c_b.ncols());
        let mut denominator_b = Mat::zeros(c_b.nrows(), c_b.ncols());
        for x in 0..settings.iter_count {
            let start = Instant::now();
            progress_bar.inc(1);

            if settings.show_steps {
                let path_1 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 1
                );
                let path_2 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 2
                );
                let image_a = utils::matrix_to_image(&c_a);
                let image_b = utils::matrix_to_image(&c_b);
                sender.send((path_1, image_a)).unwrap();
                sender.send((path_2, image_b)).unwrap();

                // Dispatch a thread to do
            }
            // CA update
            //
            {
                for view_point in 0..number_of_view_points {
                    let start_row = view_point * single_pass_size.0;
                    let end_row = (view_point + 1) * single_pass_size.0;

                    let m_a_x = m_a_x.get(start_row as usize..end_row as usize, ..);
                    let m_a_y = m_a_y.get(start_row as usize..end_row as usize, ..);

                    let m_b_x = m_b_x.get(start_row as usize..end_row as usize, ..);
                    let m_b_y = m_b_y.get(start_row as usize..end_row as usize, ..);

                    let m_t_x = m_t_x.get(start_row as usize..end_row as usize, ..);
                    let m_t_y = m_t_y.get(start_row as usize..end_row as usize, ..);

                    let c_t_m_product = (m_t_y * &c_t) * m_t_x.transpose();
                    let c_b_m_product = m_b_y * &c_b * m_b_x.transpose();
                    let c_a_m_product = m_a_y * &c_a * m_a_x.transpose();

                    zip!(&mut upper, &c_b_m_product, &c_t_m_product).for_each(
                        |unzip!(upper, c_b, c_t)| {
                            *upper = *c_b * *c_t;
                        },
                    );

                    zip!(&mut lower, &c_b_m_product, &c_a_m_product).for_each(
                        |unzip!(lower, c_b, c_a)| {
                            *lower = *c_a * *c_b * *c_b;
                        },
                    );

                    numerator_a += m_a_y.transpose() * &upper * m_a_x;
                    denominator_a += m_a_y.transpose() * &lower * m_a_x;
                }

                zip!(&mut c_a, &mut numerator_a, &mut denominator_a).for_each(
                    |unzip!(c_a, n, d)| {
                        *c_a = 1.0_f32.min(*c_a * *n / (*d + 0.0000001f32));
                        *n = 0.0;
                        *d = 0.0;
                    },
                );
            }

            {
                for view_point in 0..number_of_view_points {
                    let start_row = view_point * single_pass_size.0;
                    let end_row = (view_point + 1) * single_pass_size.0;

                    let m_a_x = m_a_x.get(start_row as usize..end_row as usize, ..);
                    let m_a_y = m_a_y.get(start_row as usize..end_row as usize, ..);

                    let m_b_x = m_b_x.get(start_row as usize..end_row as usize, ..);
                    let m_b_y = m_b_y.get(start_row as usize..end_row as usize, ..);

                    let m_t_x = m_t_x.get(start_row as usize..end_row as usize, ..);
                    let m_t_y = m_t_y.get(start_row as usize..end_row as usize, ..);

                    let c_t_m_product = (m_t_y * &c_t) * m_t_x.transpose();
                    let c_b_m_product = m_b_y * &c_b * m_b_x.transpose();
                    let c_a_m_product = m_a_y * &c_a * m_a_x.transpose();

                    zip!(&mut upper, &c_a_m_product, &c_t_m_product).for_each(
                        |unzip!(upper, c_a, c_t)| {
                            *upper = *c_a * *c_t;
                        },
                    );

                    zip!(&mut lower, &c_b_m_product, &c_a_m_product).for_each(
                        |unzip!(lower, c_b, c_a)| {
                            *lower = *c_b * *c_a * *c_a;
                        },
                    );

                    numerator_b += m_b_y.transpose() * &upper * m_b_x;
                    denominator_b += m_b_y.transpose() * &lower * m_b_x;
                }
                zip!(&mut c_b, &mut numerator_b, &mut denominator_b).for_each(
                    |unzip!(c_b, n, d)| {
                        *c_b = 1.0_f32.min(*c_b * *n / (*d + 0.000000001f32));
                        *n = 0.0;
                        *d = 0.0;
                    },
                );
            }

            let end = Instant::now();
            let time_taken = end.duration_since(start);
            time_taken_total.push(time_taken);
        }

        let total_time: Duration = time_taken_total.iter().sum();
        let average_time = total_time / settings.iter_count as u32;
        println!("Average time per iteration: {average_time:?}");
        if settings.filter {
            Self::filter_zeroes(c_a.as_mut(), &matrices.a);
            Self::filter_zeroes(c_b.as_mut(), &matrices.b);
        }
        utils::verify_matrix(&c_a);
        utils::verify_matrix(&c_b);

        let image_a = {
            let mut output = utils::matrix_to_image(&c_a);
            if settings.blend {
                output = output.fast_blur(settings.blend_sigma);
            }

            output
        };
        image_a
            .save_with_format(
                "./resources/panel_compute/panel_1.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let image_b = {
            let mut output = utils::matrix_to_image(&c_b);
            if settings.blend {
                output = output.fast_blur(settings.blend_sigma);
            }
            output
        };

        image_b
            .save_with_format(
                "./resources/panel_compute/panel_2.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        println!("Errors is: {error:?}");
        let error = {
            if settings.save_error {
                Some(error.into())
            } else {
                None
            }
        };

        Some((image_a, image_b, error))
    }

    fn filter_zeroes(mat: MatMut<f32, usize, usize>, mapping_mat: &CompleteMapping) {
        let mat_x = &mapping_mat.x.matrix;
        let mat_y = &mapping_mat.y.matrix;
        let x_ncols = mat_x.col_ptr();
        let y_ncols = mat_y.col_ptr();
        for (column, x) in mat.col_iter_mut().enumerate() {
            for (row, y) in x.iter_mut().enumerate() {
                if *y != 0.0 {
                    //break;
                }
                // This means there are no entries for this column
                if x_ncols[column + 1] == x_ncols[column] || y_ncols[row + 1] == y_ncols[row] {
                    *y = 1.0;
                }
            }
        }
    }
}

impl DrawUI for LFBuffers {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut Ui>) {
        let title = title.unwrap_or("Separable Approach".to_string());
        let _ = ui;

        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 150.0])
            .default_open(false)
            .show(ctx, |ui| {
                ui.label("Iteration count");
                let settings = &mut self.settings;
                ui.add(egui::Slider::new(&mut settings.iter_count, 1..=1000));
                ui.checkbox(&mut settings.show_steps, "Print steps");
                ui.checkbox(&mut settings.early_stop, "Early stop?");
                ui.checkbox(&mut settings.filter, "Filter Columns");
                ui.checkbox(&mut settings.save_error, "Save Error");

                if ui.button("Sample").clicked() {
                    settings.sample_next_redraw_flag = true;
                }
                if ui.button("Solve").clicked() {
                    settings.solve_next_redraw_flag = true;
                }
                if ui.button("Reset").clicked() {
                    self.matrix_rep = None;
                }
                ui.checkbox(&mut settings.blend, "Blend Out Image");
                ui.label("Sigma");
                ui.add(egui::Slider::new(&mut settings.blend_sigma, 0.0..=1.0));
                ui.checkbox(&mut settings.rng, "Random starting values");
                ui.label("Initial guesses");
                ui.add(egui::Slider::new(
                    &mut settings.starting_values.0,
                    0.0f32..=1.0f32,
                ));

                ui.add(egui::Slider::new(
                    &mut settings.starting_values.1,
                    0.0f32..=1.0f32,
                ));
            });
    }
}
