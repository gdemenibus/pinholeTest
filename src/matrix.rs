use cgmath::BaseNum;
use cgmath::Matrix4;
use cgmath::Vector3;
use cgmath::Vector4;
use crevice::std140::AsStd140;
use crevice::std140::Std140;
use crevice::std140::Writer;
use egui::ahash::HashMap;
use faer::linalg::matmul::matmul;
use faer::sparse::SparseColMat;
use faer::sparse::Triplet;
use faer::stats::prelude::thread_rng;
use faer::stats::prelude::Rng;
use faer::unzip;
use faer::zip;
use faer::Mat;
use faer::Shape;
use image::ImageBuffer;
use image::ImageError;
use indicatif::ProgressBar;
use wgpu::util::DeviceExt;
use wgpu::ComputePipelineDescriptor;
use wgpu::Device;

use crate::scene::DrawUI;

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
// Possible imporvements:
//https://www.bealto.com/gpu-gemv_v1.html
// Possible idea of how to break this down:
//         W^T V
// H  = H --------
//         (W^T W) H
// Split across multiple entries:
// W^T V ->          Numerator   --\
// W^TW -> Temp --\                --> H
//                 -> Denominator --/
// H            --/
// Minimize the amount of copying by reusing buffers

// Functionality for doing matrix multiplication
pub async fn nmf_pipeline(device: &Device, queue: &wgpu::Queue) {
    let matrix_mul = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Test Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/matrix_mul.wgsl").into()),
    });

    let matrix_mul_trans = device.create_shader_module(wgpu::ShaderModuleDescriptor {
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

    // C = A * B
    // TODO: Change this to take in arbiratry matrix
    let m = 8;
    let k = 8;
    let n = 8;

    let mat_a: Mat<f32> = Mat::from_fn(m, k, |x, y| (x + y * m) as f32);
    let mat_b: Mat<f32> = Mat::from_fn(k, n, |x, y| (x + y * n) as f32);
    let mat_d: Mat<f32> = Mat::from_fn(m, n, |_, _| 2f32);

    let a_size = mat_a.shape();
    let b_size = mat_b.shape();

    assert_eq!(
        b_size.0, a_size.1,
        "Cannot multipluy, a of shape {:?} and b of shape {:?} ",
        a_size, b_size
    );

    //let mat_a = Mat::from_fn(8, 8, |x, y| (x + y * 8) as f32);
    //let mat_b = Mat::from_fn(8, 8, |x, y| (x - y) as f32);
    let mat_c = Mat::from_fn(m, n, |_x, _y| 0 as f32);

    let buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test buffer"),
        contents: &matrix_to_buffer(&mat_a).unwrap(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test buffer"),
        contents: &matrix_to_buffer(&mat_b).unwrap(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let buffer_d = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test buffer"),
        contents: &matrix_to_buffer(&mat_d).unwrap(),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });
    let buffer_nominator = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test buffer"),
        contents: &matrix_to_buffer(&mat_d).unwrap(),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });

    let test_rw_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test rw buffer"),
        contents: &matrix_to_buffer(&mat_c).unwrap(),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::MAP_WRITE
            | wgpu::BufferUsages::MAP_READ
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });

    let test_unifrom = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test uniform"),
        contents: cgmath::vec3(m as u32, n as u32, k as u32)
            .as_std140()
            .as_bytes(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Scene Bind"),
        layout: &matrix_bind_group,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_a.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer_b.as_entire_binding(),
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
    let binding_element_wise_update = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Scene Bind"),
        layout: &matrix_bind_group,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_nominator.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer_d.as_entire_binding(),
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
        module: &matrix_mul,
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
        compute_pass.dispatch_workgroups(2, 2, 1);
    }

    // encoder.copy_buffer_to_buffer(&test_rw_buffer, 0, &buffer_nominator, 0, (m * n * 4) as u64);
    // {
    //     let compute_pass_desc = wgpu::ComputePassDescriptor {
    //         label: Some("Update innards pass"),
    //         timestamp_writes: None,
    //     };
    //     let mut compute_pass = encoder.begin_compute_pass(&compute_pass_desc);

    //     compute_pass.set_pipeline(&element_pass);
    //     compute_pass.set_bind_group(0, Some(&binding_element_wise_update), &[]);
    //     compute_pass.dispatch_workgroups(64, 1, 1);
    // }

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
        println!("Size of buffer back: {}", data_filtered.len());

        let mat = Mat::from_fn(m, n, |x, y| data_filtered[x + y * m]);
        println!("Buffer: {:?}", mat);

        let mut other_mat = Mat::<f32>::zeros(mat_c.shape().0, mat_c.shape().1);
        matmul(
            &mut other_mat,
            faer::Accum::Replace,
            &mat_a,
            &mat_b,
            1.0,
            faer::Par::Seq,
        );

        println!("Faer multiplicaiton: {:?}", other_mat);
        zip!(&mut other_mat, &mat_d).for_each(|unzip!(o, b)| *o = *o / (*b + f32::EPSILON));
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

    let mut writer = Writer::new(&mut buffer[..]);

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
/// Build an image from the provided vector
pub fn vector_to_image(
    mat: &Mat<f32, usize, usize>,
    image_height: usize,
    image_width: usize,
    image_path: String,
) -> Result<(), ImageError> {
    let (_height, width) = mat.shape();
    assert!(width == 1);

    let image_buffer = ImageBuffer::from_fn(image_width as u32, image_height as u32, |x, y| {
        // the color we record is the color, and the opacity is 1 minus the color
        let sample = mat[((x as usize + y as usize * image_width), 0)];
        // Values close to 1 will
        let _opacity = 1.0 - sample;

        image::Rgba::<u8>([
            (sample * 255.0) as u8,
            (sample * 255.0) as u8,
            (sample * 255.0) as u8,
            (255.0) as u8,
        ])
    });
    image_buffer.save_with_format(image_path, image::ImageFormat::Png)
}

pub struct NmfSolver {
    target_matrix: Option<Mat<f32>>,
    weight_matrix: Option<SparseColMat<usize, f32>>,
    iter_count: usize,
    show_steps: bool,
    pub size: [u32; 4],
    pub progress: Option<f32>,
    rng: bool,
    starting_values: (f32, f32),
}

impl NmfSolver {
    pub fn new() -> Self {
        NmfSolver {
            target_matrix: None,
            iter_count: 100,
            show_steps: false,
            size: [30u32; 4],
            progress: None,
            starting_values: (0.5, 0.5),
            rng: false,
            weight_matrix: None,
        }
    }
    pub fn reset(&mut self) {
        self.target_matrix = None;
        self.weight_matrix = None;
    }
    pub fn modified_nmf_cput(&mut self, iter_count: usize) -> (Mat<f32>, Mat<f32>) {
        let v = self.target_matrix.as_ref().unwrap();
        let (rows, columns) = v.shape();

        let mut f = faer::Mat::from_fn(rows, 1, |_i, _j| {
            if self.rng {
                thread_rng().gen_range(0.0..1.0)
            } else {
                self.starting_values.0
            }
        });

        let mut g = faer::Mat::from_fn(1, columns, |_i, _j| {
            if self.rng {
                thread_rng().gen_range(0.0..1.0)
            } else {
                self.starting_values.1
            }
        });

        println!("Starting NMF Modified!");
        let pg = ProgressBar::new(iter_count as u64);
        for x in 0..iter_count {
            let progress = (x + 1) as f32 / iter_count as f32;
            pg.inc(1);
            self.progress = Some(progress);

            if self.show_steps {
                let path_1 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 1
                );
                let path_2 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 2
                );
                let _ = vector_to_image(&f, self.size[0] as usize, self.size[1] as usize, path_1);
                let _ = vector_to_image(
                    &g.transpose().to_owned(),
                    self.size[2] as usize,
                    self.size[3] as usize,
                    path_2,
                );
            }
            // Update F
            //
        }
        self.progress = None;

        todo!("Not done")
    }

    pub fn nmf_cpu(&mut self, iter_count: usize) -> (Mat<f32>, Mat<f32>) {
        let v_sparse = self.target_matrix.as_ref().unwrap();
        let (rows, columns) = v_sparse.shape();

        let v = v_sparse;

        let mut w = faer::Mat::from_fn(rows, 1, |_i, _j| {
            if self.rng {
                thread_rng().gen_range(0.0..1.0)
            } else {
                self.starting_values.0
            }
        });

        let mut h = faer::Mat::from_fn(1, columns, |_i, _j| {
            if self.rng {
                thread_rng().gen_range(0.0..1.0)
            } else {
                self.starting_values.1
            }
        });

        println!("Starting NMF!");
        let pg = ProgressBar::new(iter_count as u64);
        for x in 0..iter_count {
            let progress = (x + 1) as f32 / iter_count as f32;
            pg.inc(1);
            self.progress = Some(progress);

            if self.show_steps {
                let path_1 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 1
                );
                let path_2 = format!(
                    "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
                    x, 2
                );
                let _ = vector_to_image(&w, self.size[0] as usize, self.size[1] as usize, path_1);
                let _ = vector_to_image(
                    &h.transpose().to_owned(),
                    self.size[2] as usize,
                    self.size[3] as usize,
                    path_2,
                );
            }

            let num_h = w.transpose() * v;

            let denom_h = (w.transpose() * &w) * &h;

            zip!(&mut h, &num_h, &denom_h)
                .for_each(|unzip!(h, a, b)| *h *= *a / (*b + f32::EPSILON));

            let num_w = v * h.transpose();

            let denom_w = &w * (&h * h.transpose());

            zip!(&mut w, &num_w, &denom_w)
                .for_each(|unzip!(w, a, b)| *w *= *a / (*b + f32::EPSILON));

            // TODO: This is a stopgap, and might not work.
            zip!(&mut w).for_each(|unzip!(w)| *w = f32::min(*w, 1.0));
            zip!(&mut h).for_each(|unzip!(h)| *h = f32::min(*h, 1.0));
        }
        self.progress = None;
        // DROP THE OBSERVATION MATRIX
        //self.reset();

        // Get both to have same shape, one big column
        (w, h.transpose().to_owned())
    }

    pub fn add_sample(&mut self, triplets: Vec<(u32, u32, f32)>, size: [u32; 4]) {
        if triplets.is_empty() {
            return;
        }
        println!("Size is: {:?}", size);
        self.size = size;
        let mut entries = HashMap::default();

        for (row, column, entry) in triplets.iter() {
            entries.insert((*row as usize, *column as usize), entry);
        }
        println!("Entries are: {:?}", entries);

        let row = size[0] * size[1];
        let column = size[2] * size[3];
        if self.target_matrix.is_none()
            || self.target_matrix.as_ref().unwrap().shape() != (row as usize, column as usize)
        {
            self.target_matrix = Some(Mat::from_fn(row as usize, column as usize, |x, y| {
                if let Some(_entry) = entries.get(&(x, y)) {
                    0.0
                } else {
                    1.0
                }
            }));
        } else {
            for ((x, y), _sample) in entries {
                self.target_matrix.as_mut().unwrap()[(x, y)] = 0.0;
            }
        }
        if self.weight_matrix.is_none() {
            let weights: Vec<Triplet<usize, usize, f32>> = triplets
                .iter()
                .map(|(x, y, _entry)| Triplet::new(*x as usize, *y as usize, 1.0))
                .collect();
            self.weight_matrix = Some(
                SparseColMat::try_new_from_triplets(row as usize, column as usize, &weights)
                    .unwrap(),
            )
        }

        //println!("Matrix is: {:?}", self.target_matrix);

        //println!("New matrix:{:?} ", self.target_matrix);
    }
}

impl DrawUI for NmfSolver {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>) {
        let title = title.unwrap_or("Solver".to_string());

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
                    let (panel_1, panel_2) = self.nmf_cpu(self.iter_count);

                    vector_to_image(
                        &panel_1,
                        self.size[0] as usize,
                        self.size[1] as usize,
                        "./resources/panel_compute/panel_1.png".to_string(),
                    )
                    .unwrap();
                    vector_to_image(
                        &panel_2,
                        self.size[2] as usize,
                        self.size[3] as usize,
                        "./resources/panel_compute/panel_2.png".to_string(),
                    )
                    .unwrap();
                }
                if self.progress.is_some() {
                    ui.add(egui::ProgressBar::new(self.progress.unwrap()));
                } else {
                    ui.label("Not solving");
                }
                if ui.button("Reset").clicked() {
                    self.reset();
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
    use faer::mat;

    use super::*;
    #[test]
    fn small_test() {
        let target_matrix = mat![[0.0, 0.5, 0.0, 1.0],];
        let mut solver = NmfSolver {
            target_matrix: Some(target_matrix),
            weight_matrix: None,
            iter_count: 100,
            show_steps: true,
            size: [1, 1, 2, 2],
            progress: None,
            rng: true,
            starting_values: (0.5, 0.5),
        };
        let (w, h) = solver.nmf_cpu(100);
        println!("W: {:?}", w);
        println!("H: {:?}", h);
        panic!("End");
    }

    #[test]
    fn medium_test() {
        // 2 rows, 4 columns
        let target_matrix = mat![[0.0, 0.5, 0.0, 1.0], [0.0, 0.5, 0.0, 1.0]];
        println!("Size is:{:?}", target_matrix.shape());
        let mut solver = NmfSolver {
            target_matrix: Some(target_matrix),

            weight_matrix: None,
            iter_count: 100,
            show_steps: true,
            size: [1, 2, 2, 2],
            progress: None,
            rng: true,
            starting_values: (0.5, 0.5),
        };
        let (w, h) = solver.nmf_cpu(100);
        println!("W: {:?}", w);
        println!("H: {:?}", h);
        panic!("End");
    }

    #[test]
    fn medium_test_inversion() {
        // 2 rows, 4 columns
        let target_matrix = mat![[1.0, 0.5, 1.0, 0.0], [1.0, 0.5, 1.0, 0.0]];
        println!("Size is:{:?}", target_matrix.shape());
        let mut solver = NmfSolver {
            target_matrix: Some(target_matrix),

            weight_matrix: None,
            iter_count: 100,
            show_steps: true,
            size: [1, 2, 2, 2],
            progress: None,
            rng: true,
            starting_values: (0.5, 0.5),
        };
        let (w, h) = solver.nmf_cpu(100);
        println!("W: {:?}", w);
        println!("H: {:?}", h);
        pub fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>) {
            let title = title.unwrap_or("Solver".to_string());

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
                        self
                    }
                    if self.progress.is_some() {
                        ui.add(egui::ProgressBar::new(self.progress.unwrap()));
                    } else {
                        ui.label("Not solving");
                    }
                    if ui.button("Reset").clicked() {
                        todo!("Haven't implemented yet");
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
        pub fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>) {
            let title = title.unwrap_or("Solver".to_string());

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
                        self
                    }
                    if self.progress.is_some() {
                        ui.add(egui::ProgressBar::new(self.progress.unwrap()));
                    } else {
                        ui.label("Not solving");
                    }
                    if ui.button("Reset").clicked() {
                        todo!("Haven't implemented yet");
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
        panic!("End");
    }
}
