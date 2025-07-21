pub mod utils;
use std::num::NonZero;

// Library File that exposes and will be used to import as well
//
use faer::sparse::{SparseColMat, SparseColMatRef, Triplet};
use utils::DrawUI;

use std::{
    collections::VecDeque,
    sync::mpsc::channel,
    thread,
    time::{Duration, Instant},
};

use faer::{
    stats::prelude::{thread_rng, Rng},
    unzip, zip, Mat,
};
use image::DynamicImage;
use serde::{ser::SerializeSeq, Deserialize, Serialize};

#[derive(Deserialize)]
enum SparsePass {
    List(Vec<SparseAsList>),
}

#[derive(Deserialize, Serialize)]
pub struct SparseAsList {
    shape: (usize, usize),
    triplet_list: Vec<(usize, usize, f32)>,
}
impl Into<SparseColMat<u32, f32>> for &SparseAsList {
    fn into(self) -> SparseColMat<u32, f32> {
        let entries: Vec<Triplet<u32, u32, f32>> = self
            .triplet_list
            .iter()
            .map(|(row, col, val)| Triplet::new(*row as u32, *col as u32, *val))
            .collect();
        SparseColMat::<u32, f32>::try_new_from_triplets(self.shape.0, self.shape.1, &entries)
            .unwrap()
    }
}
impl From<SparseColMatRef<'_, u32, f32>> for SparseAsList {
    fn from(value: SparseColMatRef<u32, f32>) -> Self {
        let shape = value.shape();
        let triplet_list = value
            .triplet_iter()
            .map(|triplet| (triplet.row, triplet.col, *triplet.val))
            .collect();
        SparseAsList {
            triplet_list,
            shape,
        }
    }
}

#[derive(Clone)]
pub struct MappingMatrix {
    pub matrix: Vec<SparseColMat<u32, f32>>,
}
impl Serialize for MappingMatrix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.matrix.len()))?;

        for matrix in self.matrix.iter() {
            let list = SparseAsList::from(matrix.as_ref());
            sequence.serialize_element(&list)?;
        }
        sequence.end()
    }
}
impl<'de> Deserialize<'de> for MappingMatrix {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match SparsePass::deserialize(deserializer)? {
            SparsePass::List(x) => Ok(MappingMatrix::new(x.iter().map(|x| x.into()).collect())),
        }
    }
}
impl MappingMatrix {
    pub fn new(matrix: Vec<SparseColMat<u32, f32>>) -> Self {
        MappingMatrix { matrix }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CompleteMapping {
    pub x: MappingMatrix,
    pub size: (u32, u32),
    pub y: MappingMatrix,
}
impl CompleteMapping {
    pub fn new(x: MappingMatrix, y: MappingMatrix, size: (u32, u32)) -> Self {
        CompleteMapping { x, y, size }
    }
}

/// Struct to hold the matrices that we will build.
/// Observations will be
#[derive(Clone, Serialize, Deserialize)]
pub struct LFMatrices {
    pub a: CompleteMapping,
    pub b: CompleteMapping,
    pub t: CompleteMapping,
}

impl LFMatrices {
    pub fn new(a: CompleteMapping, b: CompleteMapping, t: CompleteMapping) -> Self {
        LFMatrices { a, b, t }
    }
}

#[derive(Clone)]
pub struct StereoSparseWrapper {
    pub matrix: SparseColMat<u32, f32>,
}

impl From<SparseColMat<u32, f32>> for StereoSparseWrapper {
    fn from(value: SparseColMat<u32, f32>) -> Self {
        StereoSparseWrapper { matrix: value }
    }
}

impl Serialize for StereoSparseWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SparseAsList::from(self.matrix.as_ref()).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for StereoSparseWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let matrix = &SparseAsList::deserialize(deserializer)?;
        Ok(StereoSparseWrapper {
            matrix: matrix.into(),
        })
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct StereoMatrix {
    pub l_vec: Mat<f32>,
    pub a_matrix: StereoSparseWrapper,
    pub b_matrix: StereoSparseWrapper,

    pub panel_a_size: (u32, u32),
    pub panel_b_size: (u32, u32),
}

pub struct LFSettings {
    pub iter_count: usize,
    pub show_steps: bool,
    pub starting_values: (f32, f32),
    pub rng: bool,
    pub sample_next_redraw_flag: bool,
    pub solve_next_redraw_flag: bool,
    pub blend: bool,
    pub blend_sigma: f32,
    pub early_stop: bool,
    pub filter: bool,
    pub save_error: bool,
    pub debug_prints: bool,
}
impl Default for LFSettings {
    fn default() -> Self {
        LFSettings {
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
            debug_prints: true,
        }
    }
}
impl DrawUI for LFSettings {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut egui::Ui>) {
        let _ = title;
        let _ = ctx;
        if let Some(ui) = ui {
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
        }
    }
}

type L2Norm = Vec<f32>;
pub trait Lff {
    fn factorize(
        &self,
        c_t: &DynamicImage,
        target_size: (u32, u32),
        number_of_view_points: u32,
        settings: &LFSettings,
    ) -> Option<(DynamicImage, DynamicImage, Option<L2Norm>)> {
        let _ = c_t;
        let _ = target_size;
        let _ = number_of_view_points;
        let _ = settings;
        let _ = self;
        None
    }
}

impl Lff for LFMatrices {
    fn factorize(
        &self,
        c_t: &DynamicImage,
        target_size: (u32, u32),
        number_of_view_points: u32,
        settings: &LFSettings,
    ) -> Option<(DynamicImage, DynamicImage, Option<L2Norm>)> {
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));
        if settings.debug_prints {
            println!(
                "Global Parallelism is: {:?}",
                faer::get_global_parallelism()
            );
        }

        let rays_cast = (
            target_size.1 * number_of_view_points,
            target_size.0 * number_of_view_points,
        );
        println!("Rays Cast is: {rays_cast:?}");

        let matrices = self;

        let c_t = utils::image_to_matrix(c_t);

        println!("C_T shape: {:?}", c_t.shape());
        utils::verify_matrix(&c_t);
        utils::matrix_to_image(&c_t)
            .save_with_format(
                "./resources/panel_compute/intermediate/C_T.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let h_a = matrices.a.size.0 as usize;
        let w_a = matrices.a.size.1 as usize;
        let mut c_a = Mat::from_fn(h_a, w_a, |_x, _y| {
            if settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                settings.starting_values.0
            }
        });

        let h_b = matrices.b.size.0 as usize;
        let w_b = matrices.b.size.1 as usize;
        let mut c_b = Mat::from_fn(h_b, w_b, |_x, _y| {
            if settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                settings.starting_values.1
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
        for _x in 0..settings.iter_count {
            let start = Instant::now();
            progress_bar.inc(1);

            // if settings.show_steps {
            //     let path_1 = format!(
            //         "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
            //         x, 1
            //     );
            //     let path_2 = format!(
            //         "./resources/panel_compute/intermediate/intermdiate_{}_panel_{}.png",
            //         x, 2
            //     );
            //     let image_a = utils::matrix_to_image(&c_a);
            //     let image_b = utils::matrix_to_image(&c_b);
            //     sender.send((path_1, image_a)).unwrap();
            //     sender.send((path_2, image_b)).unwrap();

            //     // Dispatch a thread to do
            // }
            // CA update
            //
            {
                for view_point in 0..number_of_view_points as usize {
                    let m_a_x = matrices.a.x.matrix[view_point].as_ref();
                    let m_a_y = matrices.a.y.matrix[view_point].as_ref();

                    let m_b_x = matrices.b.x.matrix[view_point].as_ref();
                    let m_b_y = matrices.b.y.matrix[view_point].as_ref();

                    let m_t_x = matrices.t.x.matrix[view_point].as_ref();
                    let m_t_y = matrices.t.y.matrix[view_point].as_ref();

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
                for view_point in 0..number_of_view_points as usize {
                    let m_a_x = matrices.a.x.matrix[view_point].as_ref();
                    let m_a_y = matrices.a.y.matrix[view_point].as_ref();

                    let m_b_x = matrices.b.x.matrix[view_point].as_ref();
                    let m_b_y = matrices.b.y.matrix[view_point].as_ref();

                    let m_t_x = matrices.t.x.matrix[view_point].as_ref();
                    let m_t_y = matrices.t.y.matrix[view_point].as_ref();
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
            utils::filter_zeroes(&mut c_a, &matrices.a);
            utils::filter_zeroes(&mut c_b, &matrices.b);
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
}

impl Lff for StereoMatrix {
    fn factorize(
        &self,
        c_t: &DynamicImage,
        target_size: (u32, u32),
        number_of_view_points: u32,
        settings: &LFSettings,
    ) -> Option<(DynamicImage, DynamicImage, Option<L2Norm>)> {
        let _ = c_t;
        let _ = target_size;
        let _ = number_of_view_points;
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));

        let matrices = self;
        println!(
            "Size of A Stereo Matrix is: {:?}",
            matrices.a_matrix.matrix.shape()
        );
        println!(
            "Size of B Stereo Matrix is: {:?}",
            matrices.b_matrix.matrix.shape()
        );
        let rows_a = self.panel_a_size.0 * self.panel_a_size.1;
        let rows_b = self.panel_b_size.0 * self.panel_b_size.1;
        let mut vec_a = Mat::from_fn(rows_a as usize, 1, |_x, _y| {
            if settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                settings.starting_values.0
            }
        });

        let mut vec_b = Mat::from_fn(rows_b as usize, 1, |_x, _y| {
            if settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                settings.starting_values.1
            }
        });
        // Precompute the transpose
        let m_a_trans = matrices.a_matrix.matrix.transpose();
        let m_b_trans = matrices.b_matrix.matrix.transpose();
        let mut time_taken_total: Vec<Duration> = Vec::with_capacity(settings.iter_count);

        let mut error = VecDeque::with_capacity(settings.iter_count);
        println!("Computing Stereo Approach");
        let progress_bar = indicatif::ProgressBar::new(settings.iter_count as u64);
        for _x in 0..settings.iter_count {
            progress_bar.inc(1);

            let start = Instant::now();
            {
                let t2_rays = &matrices.b_matrix.matrix * &vec_b;
                let t1_rays = &matrices.a_matrix.matrix * &vec_a;

                let upper = zip!(&t1_rays, &matrices.l_vec).map(|unzip!(u, l)| *u * *l);
                let numerator = m_b_trans * upper;

                let lower = zip!(&t2_rays, &t1_rays).map(|unzip!(t2, t1)| *t2 * *t1 * *t1);
                let denominator = m_b_trans * lower;

                zip!(&mut vec_b, &numerator, &denominator)
                    .for_each(|unzip!(b, n, d)| *b = 1.0_f32.min(*b * *n / (*d + 0.0000001f32)));
            }

            // Step for A
            {
                // Upper area
                let t2_rays = &matrices.b_matrix.matrix * &vec_b;
                let t1_rays = &matrices.a_matrix.matrix * &vec_a;

                let upper = zip!(&t2_rays, &matrices.l_vec).map(|unzip!(u, l)| *u * *l);
                let numerator = m_a_trans * upper;

                let lower = zip!(&t2_rays, &t1_rays).map(|unzip!(t2, t1)| *t2 * *t2 * *t1);
                let denominator = m_a_trans * lower;
                zip!(&mut vec_a, &numerator, &denominator)
                    .for_each(|unzip!(a, n, d)| *a = 1.0_f32.min(*a * *n / (*d + 0.0000001f32)));

                // Denominator
            }
            {
                // Compute error
                if settings.save_error {
                    let t2_rays = &matrices.b_matrix.matrix * &vec_b;
                    let t1_rays = &matrices.a_matrix.matrix * &vec_a;
                    let total = zip!(&t1_rays, &t2_rays, &matrices.l_vec)
                        .map(|unzip!(t1, t2, l)| *l - (*t1 * *t2));
                    let norm = total.norm_l2();

                    if let Some(previous) = error.back() {
                        let diff: f32 = norm - previous;
                        if settings.early_stop && diff.abs() < 0.0000001f32 {
                            break;
                        }
                    }
                    error.push_back(norm);
                }
            }

            let end = Instant::now();
            let time_taken = end.duration_since(start);
            time_taken_total.push(time_taken);
        }
        utils::verify_matrix(&vec_a);
        utils::verify_matrix(&vec_b);
        let a = utils::vector_to_image(&vec_a, self.panel_a_size.0, self.panel_a_size.1);
        let b = utils::vector_to_image(&vec_b, self.panel_b_size.0, self.panel_b_size.1);
        let total_time: Duration = time_taken_total.iter().sum();
        let average_time = total_time / settings.iter_count as u32;
        println!("Average time per iteration: {average_time:?}");

        println!("Errors is: {error:?}");
        let error = {
            if settings.save_error {
                Some(error.into())
            } else {
                None
            }
        };
        Some((a, b, error))
    }
}
