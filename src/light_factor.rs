use std::{collections::VecDeque, num::NonZero, sync::mpsc::channel, thread};

use cgmath::Vector2;
use egui::{ahash::HashSet, Ui};
use faer::{
    sparse::{SparseColMat, Triplet},
    stats::prelude::{thread_rng, Rng},
    unzip, zip, Mat, MatMut,
};
use image::{DynamicImage, GenericImageView, ImageBuffer};
use wgpu::{util::DeviceExt, Buffer};

use crate::{scene::DrawUI, utils};

/// Objective is to implement the write up
pub fn image_to_matrix(image: &DynamicImage) -> Mat<f32> {
    let rows = image.height() as usize;
    let column = image.width() as usize;
    let image = image.grayscale();

    Mat::from_fn(rows, column, |x, y| {
        // Pixel is in RGBA
        let pixel = image.get_pixel(y as u32, x as u32).0;
        // Transform to floating point
        let pixel = pixel.map(|pixel| pixel as f32 / 255.0);
        for x in pixel.iter() {
            assert!(*x <= 1.0, "Pixel value is {}", x);
        }

        pixel[0] * 0.299 + 0.587 * pixel[1] + 0.114 * pixel[2]
    })
}

pub fn matrix_to_image(mat: &Mat<f32, usize, usize>) -> DynamicImage {
    let (height, width) = mat.shape();
    let image_buffer = ImageBuffer::from_par_fn(width as u32, height as u32, |x, y| {
        let value = mat[(y as usize, x as usize)];

        assert!(value <= 1.0, "Pixel value is {}", x);

        image::Rgba::<u8>([
            (value * 255.0) as u8,
            (value * 255.0) as u8,
            (value * 255.0) as u8,
            (255.0) as u8,
        ])
    });
    DynamicImage::ImageRgba8(image_buffer)
}
pub struct LFBuffers {
    m_a_y_buffer: Buffer,
    m_a_x_buffer: Buffer,
    m_b_y_buffer: Buffer,
    m_b_x_buffer: Buffer,
    m_t_x_buffer: Buffer,
    m_t_y_buffer: Buffer,

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
    matrix_rep: Option<LFMatrices>,
    filter: bool,
    save_error: bool,
}

/// Struct to hold the matrices that we will build.
/// Observations will be
#[derive(Clone)]
pub struct LFMatrices {
    m_a_y_matrix: SparseColMat<u32, f32>,
    m_a_x_matrix: SparseColMat<u32, f32>,
    m_b_y_matrix: SparseColMat<u32, f32>,
    m_b_x_matrix: SparseColMat<u32, f32>,
    m_t_y_matrix: SparseColMat<u32, f32>,
    m_t_x_matrix: SparseColMat<u32, f32>,
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
        Self {
            m_a_y_buffer,
            m_a_x_buffer,
            m_b_y_buffer,
            m_b_x_buffer,
            m_t_y_buffer,
            m_t_x_buffer,
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

    pub fn buffer_to_triplet(buffer: &Buffer, device: &wgpu::Device) -> HashSet<(u32, u32)> {
        let raw_bytes = utils::sample_buffer(buffer, device);
        let entries: Vec<u32> = raw_bytes
            .chunks(4)
            .map(|x| u32::from_ne_bytes(x[0..4].try_into().unwrap()))
            .collect();
        let mut seen: HashSet<(u32, u32)> = HashSet::default();
        let mut triplet_list: Vec<(u32, u32, u32)> = entries
            .chunks(3)
            .filter_map(|x| {
                // no recording done
                if x[2] == 0 {
                    None
                } else {
                    Some((x[0], x[1], x[2]))
                }
            })
            .collect();
        // Remove duplicates!
        triplet_list.retain(|(x, y, _entry)| seen.insert((*x, *y)));
        let max_index = triplet_list.iter().max();
        println!("Max seen is: {:?}", max_index);
        seen
    }
    pub fn has_sampled(&mut self) {
        self.sample_next_redraw_flag = false;
    }
    pub fn has_solved(&mut self) {
        self.solve_next_redraw_flag = false;
    }
    pub fn will_sample(&self) -> bool {
        self.sample_next_redraw_flag
    }
    pub fn will_solve(&self) -> bool {
        self.solve_next_redraw_flag
    }
    fn check_triplets(rows: u32, columns: u32, triplets: &mut Vec<Triplet<u32, u32, f32>>) {
        let pre_filter = triplets.len();
        triplets.retain(|x| x.row < rows && x.col < columns);
        let post_filer = triplets.len();
        let diff = pre_filter - post_filer;
        println!("Filtered {}, entries", diff);
        println!("Triplet size is: {}", triplets.len());
        if triplets.len() < 5 {
            println!("Triplets are: {:#?}", triplets);
        }
    }
    pub fn build_m_t(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        target_size: (u32, u32),
    ) -> (SparseColMat<u32, f32>, SparseColMat<u32, f32>) {
        println!("Building M_t");
        let tripltets_m_t_x = Self::buffer_to_triplet(&self.m_t_x_buffer, device);
        let mut triplet_list: Vec<Triplet<u32, u32, f32>> = tripltets_m_t_x
            .iter()
            .map(|(x, y)| Triplet::new(*x, *y, 1.0f32))
            .collect();
        Self::check_triplets(rays_cast.1, target_size.1, &mut triplet_list);

        // Height t times height a
        let matrix_m_t_x = SparseColMat::try_new_from_triplets(
            rays_cast.1 as usize,
            target_size.1 as usize,
            &triplet_list,
        )
        .unwrap();

        let tripltets_m_t_y = Self::buffer_to_triplet(&self.m_t_y_buffer, device);
        let mut triplet_list: Vec<Triplet<u32, u32, f32>> = tripltets_m_t_y
            .iter()
            .map(|(x, y)| Triplet::new(*x, *y, 1.0f32))
            .collect();

        Self::check_triplets(rays_cast.0, target_size.0, &mut triplet_list);
        // Height t times height a
        let matrix_m_t_y = SparseColMat::try_new_from_triplets(
            rays_cast.0 as usize,
            target_size.0 as usize,
            &triplet_list,
        )
        .unwrap();

        (matrix_m_t_x, matrix_m_t_y)
    }

    pub fn build_m_a(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        panel_size: (u32, u32),
    ) -> (SparseColMat<u32, f32>, SparseColMat<u32, f32>) {
        println!("Building M_A");

        let tripltets_m_a_x = Self::buffer_to_triplet(&self.m_a_x_buffer, device);
        let mut triplet_list: Vec<Triplet<u32, u32, f32>> = tripltets_m_a_x
            .iter()
            .map(|(x, y)| Triplet::new(*x, *y, 1.0f32))
            .collect();
        Self::check_triplets(rays_cast.1, panel_size.1, &mut triplet_list);

        // Height t times height a
        let matrix_m_a_x = SparseColMat::try_new_from_triplets(
            rays_cast.1 as usize,
            panel_size.1 as usize,
            &triplet_list,
        )
        .unwrap();

        let tripltets_m_a_y = Self::buffer_to_triplet(&self.m_a_y_buffer, device);
        let mut triplet_list: Vec<Triplet<u32, u32, f32>> = tripltets_m_a_y
            .iter()
            .map(|(x, y)| Triplet::new(*x, *y, 1.0f32))
            .collect();

        Self::check_triplets(rays_cast.0, panel_size.0, &mut triplet_list);
        // Height t times height a
        let matrix_m_a_y = SparseColMat::try_new_from_triplets(
            rays_cast.0 as usize,
            panel_size.0 as usize,
            &triplet_list,
        )
        .unwrap();

        (matrix_m_a_x, matrix_m_a_y)
    }

    pub fn build_m_b(
        &self,
        device: &wgpu::Device,
        rays_cast: (u32, u32),
        panel_size: (u32, u32),
    ) -> (SparseColMat<u32, f32>, SparseColMat<u32, f32>) {
        println!("Building M_B");
        let tripltets_m_b_x = Self::buffer_to_triplet(&self.m_b_x_buffer, device);

        let mut triplet_list: Vec<Triplet<u32, u32, f32>> = tripltets_m_b_x
            .iter()
            .map(|(x, y)| Triplet::new(*x, *y, 1.0f32))
            .collect();

        Self::check_triplets(rays_cast.1, panel_size.1, &mut triplet_list);
        // Height t times height a
        let matrix_m_b_x = SparseColMat::try_new_from_triplets(
            rays_cast.1 as usize,
            panel_size.1 as usize,
            &triplet_list,
        )
        .unwrap();

        let tripltes_m_b_y = Self::buffer_to_triplet(&self.m_b_y_buffer, device);

        let mut triplet_list: Vec<Triplet<u32, u32, f32>> = tripltes_m_b_y
            .iter()
            .map(|(x, y)| Triplet::new(*x, *y, 1.0f32))
            .collect();

        Self::check_triplets(rays_cast.0, panel_size.0, &mut triplet_list);
        let matrix_m_b_y = SparseColMat::try_new_from_triplets(
            rays_cast.0 as usize,
            panel_size.0 as usize,
            &triplet_list,
        )
        .unwrap();

        (matrix_m_b_x, matrix_m_b_y)
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
            target_size.0 * number_of_view_points,
            target_size.1 * number_of_view_points,
        );
        let panel_a_size = (pixel_count_a.x, pixel_count_a.y);
        let panel_b_size = (pixel_count_b.x, pixel_count_b.y);
        let (ma_x, ma_y) = self.build_m_a(device, number_of_rays, panel_a_size);
        let (mb_x, mb_y) = self.build_m_b(device, number_of_rays, panel_b_size);
        // TO BE CHANGED SOON
        let (mt_x, mt_y) = self.build_m_t(device, number_of_rays, target_size);

        let matrices = LFMatrices {
            m_a_y_matrix: ma_y,
            m_a_x_matrix: ma_x,
            m_b_y_matrix: mb_y,
            m_b_x_matrix: mb_x,
            m_t_x_matrix: mt_x,
            m_t_y_matrix: mt_y,
        };
        self.matrix_rep = Some(matrices);
    }

    pub fn verify_matrix(mat: &Mat<f32>) {
        for col in mat.col_iter() {
            for entry in col.iter() {
                assert!(
                    *entry <= 1.0,
                    "Entry in this matrix is too high, entry: {}",
                    entry
                );
            }
        }
    }
    pub fn factorize(
        &mut self,
        c_t: &DynamicImage,
        rays_cast: (u32, u32),
    ) -> Option<(DynamicImage, DynamicImage, Option<Vec<f32>>)> {
        let c_t = image_to_matrix(c_t);
        Self::verify_matrix(&c_t);

        matrix_to_image(&c_t)
            .save_with_format(
                "./resources/panel_compute/intermediate/C_T.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        // Give 10 threads
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));
        println!(
            "Global Parallelism is: {:?}",
            faer::get_global_parallelism()
        );

        self.matrix_rep.as_ref()?;

        let matrices = self.matrix_rep.as_ref()?;

        let m_a_y = &matrices.m_a_y_matrix.to_dense();
        let m_a_x = &matrices.m_a_x_matrix.to_dense();
        let m_b_y = &matrices.m_b_y_matrix.to_dense();
        let m_b_x = &matrices.m_b_x_matrix.to_dense();
        let m_t_x = &matrices.m_t_x_matrix.to_dense();
        let m_t_y = &matrices.m_t_y_matrix.to_dense();

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
        println!("C_T shape: {:?}", c_t.shape());
        println!("Rays cast: {:?}", rays_cast);

        let mut c_a = Mat::from_fn(h_a, w_a, |_x, _y| {
            if self.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                self.starting_values.0
            }
        });
        let mut c_b = Mat::from_fn(h_b, w_b, |_x, _y| {
            if self.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                self.starting_values.1
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
        let progress_bar = indicatif::ProgressBar::new(self.iter_count as u64);
        let mut error = VecDeque::with_capacity(self.iter_count);
        for x in 0..self.iter_count {
            progress_bar.inc(1);

            if self.show_steps {
                let path_1 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 1
                );
                let path_2 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 2
                );
                let image_a = matrix_to_image(&c_a);
                let image_b = matrix_to_image(&c_b);
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

                if self.save_error {
                    zip!(&mut upper, &c_b_m_product, &c_a_m_product).for_each(
                        |unzip!(upper, c_b, c_a)| {
                            *upper = *c_b * *c_a;
                        },
                    );
                    let iter_error = &c_t_m_product - &upper;
                    let cross = iter_error.transpose() * iter_error.clone();
                    let eigen_norm = cross.self_adjoint_eigenvalues(faer::Side::Upper).unwrap();
                    let eigen_max = eigen_norm[eigen_norm.len() - 1];

                    let ret = eigen_max.sqrt();
                    if let Some(previous) = error.back() {
                        let diff: f32 = ret - previous;
                        if self.early_stop && diff.abs() < 0.0000001f32 {
                            break;
                        }
                    }
                    error.push_back(ret);
                }
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
        }

        if self.filter {
            Self::filter_zeroes(c_a.as_mut(), &matrices.m_a_y_matrix, &matrices.m_a_x_matrix);
            Self::filter_zeroes(c_b.as_mut(), &matrices.m_b_y_matrix, &matrices.m_b_x_matrix);
        }
        Self::verify_matrix(&c_a);
        Self::verify_matrix(&c_b);

        let image_a = {
            let mut output = matrix_to_image(&c_a);
            if self.blend {
                output = output.fast_blur(self.blend_sigma);
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
            let mut output = matrix_to_image(&c_b);
            if self.blend {
                output = output.fast_blur(self.blend_sigma);
            }
            output
        };

        image_b
            .save_with_format(
                "./resources/panel_compute/panel_2.png",
                image::ImageFormat::Png,
            )
            .unwrap();
        self.solve_next_redraw_flag = false;

        println!("Errors is: {:?}", error);
        let error = {
            if self.save_error {
                Some(error.into())
            } else {
                None
            }
        };

        Some((image_a, image_b, error))
    }
    fn filter_zeroes(
        mat: MatMut<f32, usize, usize>,
        mat_y: &SparseColMat<u32, f32>,
        mat_x: &SparseColMat<u32, f32>,
    ) {
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
        let title = title.unwrap_or("Light Field Sampler".to_string());
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

                if ui.button("Sample").clicked() {
                    self.sample_next_redraw_flag = true;
                }
                if ui.button("Solve").clicked() {
                    self.solve_next_redraw_flag = true;
                }
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

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn image_around_the_world() {
        let mut image = image::open("./resources/textures/Gibbon.jpg").unwrap();
        image = image.grayscale();
        let matrix = image_to_matrix(&image);
        let new_image = matrix_to_image(&matrix);
        let new_matrix = image_to_matrix(&new_image);
        // Write both into
        image.save("./resources/test/OG.png").unwrap();
        new_image.save("./resources/test/NEW.png").unwrap();
        for (og, new) in std::iter::zip(image.pixels(), new_image.pixels()) {
            assert_eq!(og, new);
        }

        //assert_eq!(image, new_image);
        assert_eq!(new_matrix, matrix);
    }
}
