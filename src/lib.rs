pub mod utils;
use std::{num::NonZero, path::PathBuf};

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
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
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
impl MappingMatrix {
    pub fn stack(&self) -> SparseColMat<u32, f32> {
        let n_views = self.matrix.len();
        let (rays_per_view, columns) = self.matrix[0].shape();
        let mut triplet_list: Vec<Triplet<u32, u32, f32>> = Vec::new();
        for (index, matrix) in self.matrix.iter().enumerate() {
            let list: SparseAsList = matrix.as_ref().into();
            let triplet = list.triplet_list.iter().map(|(old_row, col, val)| {
                let row = old_row + index * rays_per_view;

                Triplet::new(row as u32, *col as u32, *val)
            });
            triplet_list.extend(triplet);
        }
        SparseColMat::try_new_from_triplets(rays_per_view * n_views, columns, &triplet_list)
            .unwrap()
    }
}

impl Serialize for MappingMatrix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SparsePass::List(self.matrix.iter().map(|x| x.as_ref().into()).collect())
            .serialize(serializer)
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
    #[serde(skip)]
    pub c_t: DynamicImage,
    pub target_size: (u32, u32),
    pub number_of_view_points: u32,
}

pub struct OldLFMatrices {
    m_a_x: SparseColMat<u32, f32>,
    m_a_y: SparseColMat<u32, f32>,
    m_b_x: SparseColMat<u32, f32>,
    m_b_y: SparseColMat<u32, f32>,
    m_t_x: SparseColMat<u32, f32>,
    m_t_y: SparseColMat<u32, f32>,
}

impl LFMatrices {
    pub fn new(
        a: CompleteMapping,
        b: CompleteMapping,
        t: CompleteMapping,
        c_t: DynamicImage,
        target_size: (u32, u32),
        number_of_view_points: u32,
    ) -> Self {
        LFMatrices {
            a,
            b,
            t,
            c_t,
            target_size,
            number_of_view_points,
        }
    }
    pub fn save(&self, path: String) {
        let path = {
            if !path.ends_with(".ro") {
                format!("{path}.ro")
            } else {
                path
            }
        };
        let path_core = PathBuf::from(format!("./saves/matrix_capture/sep/{path}"));
        let mut file = std::fs::File::create(&path_core).unwrap();
        let config = bincode::config::standard();
        bincode::serde::encode_into_std_write(self, &mut file, config).unwrap();
    }
    pub fn load(path: String) -> Self {
        let path = {
            if !path.ends_with(".ro") {
                format!("{path}.ro")
            } else {
                path
            }
        };
        let path_core = PathBuf::from(format!("./saves/matrix_capture/sep/{path}"));
        let mut file = std::fs::File::open(path_core).unwrap();
        let config = bincode::config::standard();
        bincode::serde::decode_from_std_read(&mut file, config).unwrap()
    }
    pub fn stack(&self) -> OldLFMatrices {
        let m_a_x = self.a.x.stack();
        let m_a_y = self.a.y.stack();
        let m_b_x = self.b.x.stack();
        let m_b_y = self.b.y.stack();
        let m_t_x = self.t.x.stack();
        let m_t_y = self.t.y.stack();
        OldLFMatrices {
            m_a_x,
            m_a_y,
            m_b_x,
            m_b_y,
            m_t_x,
            m_t_y,
        }
    }
    pub fn old_factorize(
        &self,
        settings: &LFSettings,
        matrices: &OldLFMatrices,
    ) -> Option<(DynamicImage, DynamicImage, Option<L2Norm>)> {
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));
        let target_size = self.target_size;
        let number_of_view_points = self.number_of_view_points;
        let c_t = &self.c_t;
        if settings.debug_prints {
            println!(
                "Global Parallelism is: {:?}",
                faer::get_global_parallelism()
            );
        }

        let m_a_x = matrices.m_a_x.as_ref();
        let m_a_y = matrices.m_a_y.as_ref();
        let m_b_x = matrices.m_b_x.as_ref();
        let m_b_y = matrices.m_b_y.as_ref();
        let m_t_x = matrices.m_t_x.as_ref();
        let m_t_y = matrices.m_t_y.as_ref();

        let rays_cast = (
            target_size.1 * number_of_view_points,
            target_size.0 * number_of_view_points,
        );
        if settings.debug_prints {
            println!("Rays Cast is: {rays_cast:?}");
        }

        let c_t = utils::image_to_matrix(c_t);
        if settings.debug_prints {
            println!("A_y shape: {:?}", m_a_y.shape());
            println!("A_x shape: {:?}", m_a_x.shape());
            println!("b_y shape: {:?}", m_b_y.shape());
            println!("b_x shape: {:?}", m_b_x.shape());
            println!("t_y shape: {:?}", m_t_y.shape());
            println!("t_x shape: {:?}", m_t_x.shape());
            println!("C_T shape: {:?}", c_t.shape());
        }

        utils::verify_matrix(&c_t);
        utils::matrix_to_image(&c_t)
            .save_with_format(
                "./resources/panel_compute/intermediate/C_T.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let h_a = m_a_y.shape().1;
        let w_a = m_a_x.shape().1;
        let mut c_a = Mat::from_fn(h_a, w_a, |_x, _y| {
            if settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                settings.starting_values.0
            }
        });

        let h_b = m_b_y.shape().1;
        let w_b = m_b_x.shape().1;
        let mut c_b = Mat::from_fn(h_b, w_b, |_x, _y| {
            if settings.rng {
                thread_rng().gen_range(0f32..1.0f32)
            } else {
                settings.starting_values.1
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

        let mut progress_bar = {
            if settings.debug_prints {
                Some(indicatif::ProgressBar::new(settings.iter_count as u64))
            } else {
                None
            }
        };
        let mut error = VecDeque::with_capacity(settings.iter_count);

        let mut time_taken_total: Vec<Duration> = Vec::with_capacity(settings.iter_count);

        let c_t_m_product = (m_t_y * c_t) * m_t_x.transpose();
        if settings.debug_prints {
            println!("C_T_M Shape: {:?}", c_t_m_product.shape());
        }
        for _x in 0..settings.iter_count {
            let start = Instant::now();
            progress_bar.as_mut().inspect(|x| x.inc(1));

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
                let c_b_m_product = m_b_y.as_ref() * &c_b * m_b_x.transpose();
                let c_a_m_product = &m_a_y * &c_a * m_a_x.transpose();
                if settings.debug_prints {
                    println!("C_A_M shape {:?}", c_a_m_product.shape());
                    println!("C_B_M shape {:?}", c_b_m_product.shape());
                }
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
                let numerator = m_a_y.transpose() * &upper * &m_a_x;
                let denominator = m_a_y.transpose() * &lower * &m_a_x;
                zip!(&mut c_a, &numerator, &denominator).for_each(|unzip!(c_a, n, d)| {
                    *c_a = 1.0_f32.min(*c_a * *n / (*d + 0.0000001f32))
                });
            }

            // C_B Update

            {
                let c_b_m_product = m_b_y.as_ref() * &c_b * m_b_x.transpose();
                let c_a_m_product = m_a_y.as_ref() * &c_a * m_a_x.transpose();
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

                let numerator = m_b_y.transpose() * &upper * &m_b_x;
                let denominator = m_b_y.transpose() * &lower * &m_b_x;
                zip!(&mut c_b, &numerator, &denominator).for_each(|unzip!(c_b, n, d)| {
                    *c_b = 1.0_f32.min(*c_b * *n / (*d + 0.000000001f32));
                });
            }

            let end = Instant::now();
            let time_taken = end.duration_since(start);
            time_taken_total.push(time_taken);
        }

        let total_time: Duration = time_taken_total.iter().sum();
        let average_time = total_time / settings.iter_count as u32;
        if settings.debug_prints {
            println!("Average time per iteration: {average_time:?}");
        }

        if settings.filter {
            utils::filter_zeroes(&mut c_a, &self.a);
            utils::filter_zeroes(&mut c_b, &self.b);
        }
        utils::verify_matrix(&c_a);
        utils::verify_matrix(&c_b);

        let image_a = utils::matrix_to_image(&c_a);
        image_a
            .save_with_format(
                "./resources/panel_compute/panel_1.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let image_b = utils::matrix_to_image(&c_b);

        image_b
            .save_with_format(
                "./resources/panel_compute/panel_2.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        if settings.debug_prints {
            println!("Errors is: {error:?}");
        }
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

    pub target_size: (u32, u32),
    pub number_of_view_points: u32,
}
impl StereoMatrix {
    pub fn save(&self, path: String) {
        let path = {
            if !path.ends_with(".ro") {
                format!("{path}.ro")
            } else {
                path
            }
        };

        let path_core = PathBuf::from(format!("./saves/matrix_capture/stereo/{path}"));
        let mut file = std::fs::File::create(&path_core).unwrap();
        let config = bincode::config::standard();
        bincode::serde::encode_into_std_write(self, &mut file, config).unwrap();
    }
    pub fn load(path: String) -> Self {
        let path = {
            if !path.ends_with(".ro") {
                format!("{path}.ro")
            } else {
                path
            }
        };

        let path_core = PathBuf::from(format!("./saves/matrix_capture/stereo/{path}"));
        let mut file = std::fs::File::open(path_core).unwrap();
        let config = bincode::config::standard();
        bincode::serde::decode_from_std_read(&mut file, config).unwrap()
    }
}

pub struct LFSettings {
    pub iter_count: usize,
    pub show_steps: bool,
    pub starting_values: (f32, f32),
    pub rng: bool,
    pub sample_next_redraw_flag: bool,
    pub solve_next_redraw_flag: bool,
    pub early_stop: bool,
    pub filter: bool,
    pub save_error: bool,
    pub debug_prints: bool,
    pub save_to: String,
}
impl Default for LFSettings {
    fn default() -> Self {
        LFSettings {
            rng: false,
            iter_count: 10,
            show_steps: false,
            starting_values: (0.5, 0.5),
            sample_next_redraw_flag: false,
            solve_next_redraw_flag: false,
            early_stop: false,
            filter: false,
            save_error: true,
            debug_prints: true,
            save_to: "Default".to_string(),
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
            ui.text_edit_singleline(&mut self.save_to);
        }
    }
}

type L2Norm = Vec<f32>;
pub trait Lff {
    fn factorize(
        &self,
        settings: &LFSettings,
    ) -> Option<(DynamicImage, DynamicImage, Option<L2Norm>)> {
        let _ = settings;
        let _ = self;
        None
    }
}

impl Lff for LFMatrices {
    fn factorize(
        &self,
        settings: &LFSettings,
    ) -> Option<(DynamicImage, DynamicImage, Option<L2Norm>)> {
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));
        let target_size = self.target_size;
        let number_of_view_points = self.number_of_view_points;
        let c_t = &self.c_t;
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
        if settings.debug_prints {
            println!("Rays Cast is: {rays_cast:?}");
        }

        let matrices = self;

        let c_t = utils::image_to_matrix(c_t);
        if settings.debug_prints {
            println!("C_T shape: {:?}", c_t.shape());
        }

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

        let mut progress_bar = {
            if settings.debug_prints {
                Some(indicatif::ProgressBar::new(settings.iter_count as u64))
            } else {
                None
            }
        };
        let mut error = VecDeque::with_capacity(settings.iter_count);

        let mut time_taken_total: Vec<Duration> = Vec::with_capacity(settings.iter_count);

        let mut numerator_a = Mat::zeros(c_a.nrows(), c_a.ncols());
        let mut denominator_a = Mat::zeros(c_a.nrows(), c_a.ncols());

        let mut numerator_b = Mat::zeros(c_b.nrows(), c_b.ncols());
        let mut denominator_b = Mat::zeros(c_b.nrows(), c_b.ncols());
        for _x in 0..settings.iter_count {
            let start = Instant::now();
            progress_bar.as_mut().inspect(|x| x.inc(1));

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
        if settings.debug_prints {
            println!("Average time per iteration: {average_time:?}");
        }

        if settings.filter {
            utils::filter_zeroes(&mut c_a, &matrices.a);
            utils::filter_zeroes(&mut c_b, &matrices.b);
        }
        utils::verify_matrix(&c_a);
        utils::verify_matrix(&c_b);

        let image_a = utils::matrix_to_image(&c_a);
        image_a
            .save_with_format(
                "./resources/panel_compute/panel_1.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        let image_b = utils::matrix_to_image(&c_b);

        image_b
            .save_with_format(
                "./resources/panel_compute/panel_2.png",
                image::ImageFormat::Png,
            )
            .unwrap();

        if settings.debug_prints {
            println!("Errors is: {error:?}");
        }
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
        settings: &LFSettings,
    ) -> Option<(DynamicImage, DynamicImage, Option<L2Norm>)> {
        faer::set_global_parallelism(faer::Par::Rayon(NonZero::new(10).unwrap()));

        let matrices = self;
        if settings.debug_prints {
            println!(
                "Size of A Stereo Matrix is: {:?}",
                matrices.a_matrix.matrix.shape()
            );
            println!(
                "Size of B Stereo Matrix is: {:?}",
                matrices.b_matrix.matrix.shape()
            );
        }
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
        let ray_space_size = self.l_vec.shape();
        let number_rays = ray_space_size.0 * ray_space_size.1;
        // Precompute the transpose
        let m_a_trans = matrices.a_matrix.matrix.transpose();
        let m_b_trans = matrices.b_matrix.matrix.transpose();

        let mut time_taken_total: Vec<Duration> = Vec::with_capacity(settings.iter_count);
        let mut m_a_vec: Mat<f32> =
            Mat::from_fn(ray_space_size.0, 1, |_, _| 1.0 / number_rays as f32);
        let mut m_b_vec: Mat<f32> =
            Mat::from_fn(ray_space_size.0, 1, |_, _| 1.0 / number_rays as f32);

        let mut error = VecDeque::with_capacity(settings.iter_count);
        if settings.debug_prints {
            println!("Computing Stereo Approach");
        }
        let mut progress_bar = {
            if settings.debug_prints {
                Some(indicatif::ProgressBar::new(settings.iter_count as u64))
            } else {
                None
            }
        };
        for _x in 0..settings.iter_count {
            progress_bar.as_mut().inspect(|x| x.inc(1));

            let start = Instant::now();
            {
                let upper = zip!(&m_b_vec, &matrices.l_vec).map(|unzip!(u, l)| *u * *l);

                let lower = zip!(&m_b_vec, &m_a_vec).map(|unzip!(b, a)| *b * *b * *a);

                zip!(&mut m_a_vec, &upper, &lower)
                    .for_each(|unzip!(a, n, d)| *a = 1.0_f32.min(*a * *n / (*d + 0.0000001f32)));

                vec_a = m_a_trans * m_a_vec;
                zip!(&mut vec_a).for_each(|unzip!(a)| *a = 1.0_f32.min(*a));
                m_a_vec = matrices.a_matrix.matrix.as_ref() * vec_a.as_ref();
            }

            {
                let upper = zip!(&m_a_vec, &matrices.l_vec).map(|unzip!(u, l)| *u * *l);

                let lower = zip!(&m_b_vec, &m_a_vec).map(|unzip!(b, a)| *b * *a * *a);

                zip!(&mut m_b_vec, &upper, &lower)
                    .for_each(|unzip!(b, n, d)| *b = 1.0_f32.min(*b * *n / (*d + 0.0000001f32)));

                vec_b = m_b_trans * m_b_vec;
                zip!(&mut vec_b).for_each(|unzip!(b)| *b = 1.0_f32.min(*b));
                m_b_vec = matrices.b_matrix.matrix.as_ref() * vec_b.as_ref();
            }

            let end = Instant::now();
            let time_taken = end.duration_since(start);
            time_taken_total.push(time_taken);
        }
        println!("Vec_a shape is: {:?}", vec_a.shape());
        println!("Vec_b shape is: {:?}", vec_b.shape());

        utils::verify_matrix(&vec_a);
        utils::verify_matrix(&vec_b);
        let a = utils::vector_to_image(&vec_a, self.panel_a_size.0, self.panel_a_size.1);
        let b = utils::vector_to_image(&vec_b, self.panel_b_size.0, self.panel_b_size.1);
        let total_time: Duration = time_taken_total.iter().sum();
        let average_time = total_time / settings.iter_count as u32;
        if settings.debug_prints {
            println!("Average time per iteration: {average_time:?}");

            println!("Errors is: {error:?}");
        }
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
