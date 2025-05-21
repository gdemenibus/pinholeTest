use std::num::NonZero;

use cgmath::Vector2;
use egui::{ahash::HashSet, mutex::RwLock};
use faer::{
    stats::prelude::{thread_rng, Rng},
    unzip, zip, Mat,
};
use image::{DynamicImage, GenericImageView, ImageBuffer};
use wgpu::{util::DeviceExt, Buffer};

use crate::{scene::DrawUI, utils};

/// Objective is to implement the write up
pub fn image_to_matrix(image: &DynamicImage) -> Mat<f32> {
    let rows = image.height() as usize;
    let column = image.width() as usize;
    //let image = image.grayscale();

    Mat::from_fn(rows, column, |x, y| {
        let pixel = image.get_pixel(y as u32, x as u32).0;

        pixel[0] as f32 * 0.299 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32
    })
}

pub fn matrix_to_image(mat: &Mat<f32, usize, usize>) -> DynamicImage {
    let (height, width) = mat.shape();
    let image_buffer = ImageBuffer::from_par_fn(width as u32, height as u32, |x, y| {
        let value = mat[(y as usize, x as usize)];

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

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    iter_count: usize,
    show_steps: bool,
    starting_values: (f32, f32),
    rng: bool,
    sample_next_redraw_flag: bool,
    progress: Option<RwLock<f32>>,
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
            rng: false,
            iter_count: 50,
            show_steps: false,
            starting_values: (0.5, 0.5),
            sample_next_redraw_flag: false,
            progress: None,
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
        seen
    }
    pub fn will_sample(&self) -> bool {
        self.sample_next_redraw_flag
    }

    pub fn build_m_a(
        &self,
        device: &wgpu::Device,
        target_size: (u32, u32),
        panel_size: (u32, u32),
    ) -> (Mat<f32>, Mat<f32>) {
        let tripltets_m_a_x = Self::buffer_to_triplet(&self.m_a_x_buffer, device);
        // Height t times height a
        let matrix_m_a_x = Mat::from_fn(target_size.1 as usize, panel_size.1 as usize, |x, y| {
            if tripltets_m_a_x.contains(&(x as u32, y as u32)) {
                1.0f32
            } else {
                0.0f32
            }
        });
        let tripltes_m_a_y = Self::buffer_to_triplet(&self.m_a_y_buffer, device);

        let matrix_m_a_y = Mat::from_fn(target_size.0 as usize, panel_size.0 as usize, |x, y| {
            if tripltes_m_a_y.contains(&(x as u32, y as u32)) {
                1.0f32
            } else {
                0.0f32
            }
        });

        (matrix_m_a_x, matrix_m_a_y)
    }

    pub fn build_m_b(
        &self,
        device: &wgpu::Device,
        target_size: (u32, u32),
        panel_size: (u32, u32),
    ) -> (Mat<f32>, Mat<f32>) {
        let tripltets_m_b_x = Self::buffer_to_triplet(&self.m_b_x_buffer, device);
        // Height t times height a
        let matrix_m_b_x = Mat::from_fn(target_size.1 as usize, panel_size.1 as usize, |x, y| {
            if tripltets_m_b_x.contains(&(x as u32, y as u32)) {
                1.0f32
            } else {
                0.0f32
            }
        });
        let tripltes_m_b_y = Self::buffer_to_triplet(&self.m_b_y_buffer, device);

        let matrix_m_b_y = Mat::from_fn(target_size.0 as usize, panel_size.0 as usize, |x, y| {
            if tripltes_m_b_y.contains(&(x as u32, y as u32)) {
                1.0f32
            } else {
                0.0f32
            }
        });

        (matrix_m_b_x, matrix_m_b_y)
    }
    pub fn sample_light_field(
        &mut self,
        ct_image: &DynamicImage,
        device: &wgpu::Device,
        pixel_count_a: Vector2<u32>,
        pixel_count_b: Vector2<u32>,
        target_size: (u32, u32),
    ) {
        // Flag has not been raised
        if !self.sample_next_redraw_flag {
            return;
        }
        let panel_a_size = (pixel_count_a.x, pixel_count_a.y);
        let panel_b_size = (pixel_count_b.x, pixel_count_b.y);
        let (ma_x, ma_y) = self.build_m_a(device, target_size, panel_a_size);
        let (mb_x, mb_y) = self.build_m_b(device, target_size, panel_b_size);

        let c_t = image_to_matrix(ct_image);

        let (a, b) = self.factorize(c_t, ma_y, ma_x, mb_y, mb_x);

        let image_a = matrix_to_image(&a);
        image_a
            .save_with_format(
                "./resources/panel_compute/panel_1.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let image_b = matrix_to_image(&b);

        image_b
            .save_with_format(
                "./resources/panel_compute/panel_2.png",
                image::ImageFormat::Png,
            )
            .unwrap();
        // We have already sampled, no need to sample again
        self.sample_next_redraw_flag = false;
    }

    pub fn factorize(
        &self,
        c_t: Mat<f32>,
        m_a_y: Mat<f32>,
        m_a_x: Mat<f32>,
        m_b_y: Mat<f32>,
        m_b_x: Mat<f32>,
    ) -> (Mat<f32>, Mat<f32>) {
        // Give 10 threads
        matrix_to_image(&c_t)
            .save_with_format(
                "./resources/panel_compute/intermediate/C_T.png",
                image::ImageFormat::Png,
            )
            .unwrap();
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));
        let h_a = m_a_y.shape().1;
        let h_b = m_b_y.shape().1;

        let w_a = m_a_x.shape().1;
        let w_b = m_b_x.shape().1;

        println!("w_a: {}", w_a);
        println!("h_a: {}", h_a);
        println!("h_b: {}", h_b);
        println!("w_b: {}", w_b);
        println!("w_t: {}", c_t.shape().1);
        println!("h_t: {}", c_t.shape().0);

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

        let mut upper = Mat::<f32>::zeros(c_t.shape().0, c_t.shape().1);

        let mut lower = Mat::<f32>::zeros(c_t.shape().0, c_t.shape().1);

        for x in 0..self.iter_count {
            if self.show_steps {
                let path_1 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 1
                );
                let path_2 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 2
                );
                matrix_to_image(&c_a).save_with_format(path_1, image::ImageFormat::Png);
                matrix_to_image(&c_b).save_with_format(path_2, image::ImageFormat::Png);
            }
            // CA update
            //
            {
                let c_b_m_product = &m_b_y * &c_b * m_b_x.transpose();
                let c_a_m_product = &m_a_y * &c_a * m_a_x.transpose();

                zip!(&mut upper, &c_b_m_product, &c_t).for_each(|unzip!(upper, c_b, c_t)| {
                    *upper = *c_b * *c_t;
                });

                zip!(&mut lower, &c_b_m_product, &c_a_m_product).for_each(
                    |unzip!(lower, c_b, c_a)| {
                        *lower = *c_a * *c_b * *c_b;
                    },
                );
                let numerator = m_a_y.transpose() * &upper * &m_a_x;
                let denominator = m_a_y.transpose() * &lower * &m_a_x;
                zip!(&mut c_a, &numerator, &denominator)
                    .for_each(|unzip!(c_a, n, d)| *c_a *= *n / (*d + 0.0000001f32));
            }
            {
                let c_b_m_product = &m_b_y * &c_b * m_b_x.transpose();
                let c_a_m_product = &m_a_y * &c_a * m_a_x.transpose();

                zip!(&mut upper, &c_a_m_product, &c_t).for_each(|unzip!(upper, c_a, c_t)| {
                    *upper = *c_a * *c_t;
                });

                zip!(&mut lower, &c_b_m_product, &c_a_m_product).for_each(
                    |unzip!(lower, c_b, c_a)| {
                        *lower = *c_b * *c_a * *c_a;
                    },
                );

                let numerator = m_b_y.transpose() * &upper * &m_b_x;
                let denominator = m_b_y.transpose() * &lower * &m_b_x;
                zip!(&mut c_b, &numerator, &denominator)
                    .for_each(|unzip!(c_b, n, d)| *c_b *= *n / (*d + 0.000000001f32));
            }
        }
        (c_a, c_b)
    }
}
impl DrawUI for LFBuffers {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>) {
        let title = title.unwrap_or("Light Field Sampler".to_string());

        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 150.0])
            .default_open(false)
            .show(ctx, |ui| {
                ui.label("Iteration count");
                ui.add(egui::Slider::new(&mut self.iter_count, 1..=1000));
                ui.checkbox(&mut self.show_steps, "Print steps");
                if ui.button("Solve").clicked() {
                    self.sample_next_redraw_flag = true;
                }
                if self.progress.is_some() {
                    let progress = *self.progress.as_ref().unwrap().read();
                    ui.add(egui::ProgressBar::new(progress));
                } else {
                    ui.label("Not solving");
                }
                if ui.button("Reset").clicked() {
                    todo!("Reset functionality not implemented yet");
                }
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
        let image = image::open("./resources/textures/4 by 4 T filled.png").unwrap();
        let matrix = image_to_matrix(&image);
        let new_image = matrix_to_image(&matrix);
        let new_matrix = image_to_matrix(&new_image);

        assert_eq!(image, new_image);
        assert_eq!(new_matrix, matrix);
    }
}
