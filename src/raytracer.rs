use std::sync::Arc;

use crate::matrix::FromArr;
use crate::matrix::ToArr;
use crate::vertex::Line;
use crate::vertex::LineVertex;
use crate::vertex::ShapeWorld;
use crate::RAY_HEIGHT;
use crate::RAY_WIDTH;
use crate::SAFE_FRAC_PI_2;
use cgmath::EuclideanSpace;
use cgmath::InnerSpace;
use cgmath::Point3;
use cgmath::Vector3;
use cgmath::Vector4;
use egui_winit::egui;
use egui_winit::egui::ahash::HashMap;
use egui_winit::egui::mutex::Mutex;
use egui_winit::egui::mutex::RwLock;
use egui_winit::egui::Align2;
use egui_winit::egui::ColorImage;
use egui_winit::egui::Context;
use egui_winit::egui::TextureHandle;
use egui_winit::egui::TextureOptions;
use egui_winit::egui::Vec2;
use faer::linalg::solvers::SolveLstsqCore;
use faer::reborrow::ReborrowMut;
use faer::sparse::SparseColMat;
use faer::sparse::Triplet;
use faer::Mat;
use image::ImageBuffer;

pub struct Raytracer {
    ray_trace_display: bool,
    ray_trace_save: bool,
    ui_debug_rays: bool,
    ui_debug_prime_rays: bool,
    ui_solve: bool,
    ray_tace_file_name: String,
    raytrace_handler: TextureHandle,
    raytrace_height: usize,
    raytrace_width: usize,
    pub debug_rays: RwLock<Vec<Line>>,
    pub debug_b_rays: RwLock<Vec<Line>>,
}

type TargetPixel = (u32, u32);
type PanelPixel = (u32, u32);

impl Raytracer {
    pub fn new(
        ray_trace_display: bool,
        ray_trace_save: bool,
        ray_tace_file_name: String,
        raytrace_handler: TextureHandle,
    ) -> Self {
        Self {
            ray_trace_display,
            ray_trace_save,
            ui_debug_rays: false,
            ui_debug_prime_rays: false,
            ui_solve: false,
            ray_tace_file_name,
            raytrace_handler,
            raytrace_height: RAY_HEIGHT,
            raytrace_width: RAY_WIDTH,
            debug_rays: RwLock::new(Vec::new()),
            debug_b_rays: RwLock::new(Vec::new()),
        }
    }
    pub fn debug_line_vertex(&self) -> Vec<LineVertex> {
        if self.debug_rays.read().len() < 200 {
            self.debug_rays
                .read()
                .iter()
                .flat_map(|x| x.vertices())
                .collect()
        } else {
            self.debug_rays
                .read()
                .iter()
                .enumerate()
                .filter(|&(index, _)| (index + 1) % 1000 == 0)
                .map(|(_, val)| val)
                .flat_map(|x| x.vertices())
                .collect()
        }
    }

    pub fn debug_b_line_vertex(&self) -> Vec<LineVertex> {
        self.debug_b_rays
            .read()
            .iter()
            .flat_map(|x| x.vertices())
            .collect()
    }

    pub fn raytrace(
        &mut self,
        camera_dir: Vector3<f32>,
        camera_position: Point3<f32>,
        shapes: Vec<Arc<RwLock<ShapeWorld>>>,
    ) {
        // Reset the debug print rays
        self.debug_rays = RwLock::new(Vec::new());
        self.debug_b_rays = RwLock::new(Vec::new());

        // Rebiuld the world?
        // Eye Position
        // Target position
        // FOV (90 degrees)
        // number of quare pixels on the viewport on each direction
        // Vector which indicates where up is (given by camera)
        //
        // Target image, by pixel
        // Distance between camera and plane?
        //
        //println!("Starting Ray trace");
        let image_height: u32 = self.raytrace_height.try_into().unwrap();
        let image_width: u32 = self.raytrace_width.try_into().unwrap();

        // CAMERA MATH!

        let d = 1.0;
        let t_n = camera_dir;
        let b_n = t_n.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
        let v_n = t_n.cross(b_n).normalize();
        let g_x = (SAFE_FRAC_PI_2 / 2.0).tan() * d;
        // IMPORTANT: Might have width and height confused?

        // straight from the wikipedia article on raytracing
        let g_y = g_x * ((image_height as f32 - 1.0) / (image_width as f32 - 1.0));
        let q_x = (2.0 * g_x) / (image_height as f32 - 1.0) * b_n;
        let q_y = (2.0 * g_y) / (image_width as f32 - 1.0) * v_n;
        let p_1_m = t_n * d - g_x * b_n - g_y * v_n;

        let ray_origin = camera_position;
        let second_image = Mutex::new(HashMap::default());

        //We need to build our matrix that maps perceived imaged pixels (x, y) to pixels in screen
        //a (x,y)
        let matrix_mapping_a = Mutex::new(HashMap::default());

        let buf = ImageBuffer::from_par_fn(image_width, image_height, |x, y| {
            let f_y = y as f32;
            let f_x = x as f32;

            let mut color_sum = Vector4::<u8>::from_arr([0_u8, 0_u8, 0_u8, 0_u8]);

            let ray_dir = (p_1_m + q_x * (f_x - 1.0) + q_y * (f_y - 1.0)).normalize();
            let mut intersections = Vec::new();

            // TODO: Right now the order of the shapes matters! Need to change that
            for shape in shapes.clone() {
                let shape = shape.read();

                let intersect = shape.intersect(ray_origin, ray_dir);

                if let Some((color, point, pixel_center, pixel_space)) = intersect {
                    if shape.is_transparent {
                        let pixel_center = Point3::from_vec(pixel_center);
                        intersections.push(pixel_center);
                        //let line = Line::new(ray_origin, pixel_center);
                        //self.debug_b_rays.write().push(line);
                        // edit the color*A monkey
                        //update the intersection info in an intelligent way
                        //
                        //
                        // TODO: THIS IS SUPER SKETCH, I JUST WANT TO TRY SOMETHING
                        matrix_mapping_a.lock().entry((x, y)).or_insert(pixel_space);
                    } else {
                        let color_vec = Vector4::<u8>::from_arr(color);
                        let line = Line::new(ray_origin, point);
                        self.debug_rays.write().push(line);
                        // edit the color?
                        color_sum = color_vec;
                    }
                }
            }

            // We have the intersection points, reason with them
            if intersections.len() == 2 {
                let a_pixel = intersections[0];
                let b_pixel = intersections[1];

                let ray_prime_origin = a_pixel;
                let ray_prime_direction = b_pixel - a_pixel;
                let shape = shapes[2].read();

                if let Some((color, point, _center, _pixel_coords)) =
                    shape.intersect(ray_prime_origin, ray_prime_direction)
                {
                    //let line = Line::new(ray_prime_origin, point);
                    //self.debug_b_rays.write().push(line);
                    second_image.lock().insert((x, y), color);
                    let line = Line::new(ray_prime_origin, point);
                    self.debug_b_rays.write().push(line);
                } else {
                    second_image.lock().insert((x, y), [0_u8, 0_u8, 0_u8, 0_u8]);
                }
            }

            image::Rgba(color_sum.to_arr())
        });

        let second_buffer = ImageBuffer::from_fn(image_width, image_height, |x, y| {
            let image = second_image.lock();
            if let Some(color) = image.get(&(x, y)) {
                image::Rgba(*color)
            } else {
                image::Rgba([0_u8, 0_u8, 0_u8, 0_u8])
            }
        });

        if self.ray_trace_save {
            let res =
                buf.save_with_format(self.ray_tace_file_name.clone(), image::ImageFormat::Png);
            let second_name = format!("SECOND{}", self.ray_tace_file_name.clone());
            let _ = second_buffer.save_with_format(second_name, image::ImageFormat::Png);

            if res.is_err() {
                println!("Could not write to file? {:?}", res);
            }
        }

        let raw = ColorImage::from_rgba_unmultiplied(
            [self.raytrace_width, self.raytrace_height],
            &buf.clone().into_raw(),
        );
        self.raytrace_handler.set(raw, TextureOptions::default());
        self.ray_trace_display = true;
        println!("Rays traced!");
        println!("Number of red rays: {}", self.debug_b_rays.read().len());
        println!("Number of white rays: {}", self.debug_rays.read().len());
        {
            //
            let a = shapes[0].read();
            let size_s = image_width * image_height;
            let size_w = a.ui_state.resolution.0 * a.ui_state.resolution.1;
            let target_image_width = image_width;
            let panel_resolution_width = a.ui_state.resolution.0;
            if self.ui_solve {
                let mat = Self::build_and_solve_a(
                    matrix_mapping_a,
                    &buf.into_raw(),
                    size_s as usize,
                    size_w as usize,
                    target_image_width,
                    panel_resolution_width,
                );

                let third = ImageBuffer::from_par_fn(
                    a.ui_state.resolution.0,
                    a.ui_state.resolution.1,
                    |x, y| {
                        let offset = y * panel_resolution_width + x;
                        let mut sample = mat.get(offset as usize, 0);
                        if sample.is_nan() {
                            sample = &0.0
                        }
                        let bright = sample * 255.0;
                        image::Rgba::from([bright as u8, bright as u8, bright as u8, 1])
                    },
                );
                let attempt = third.save_with_format("solutionOut.png", image::ImageFormat::Png);
                if attempt.is_err() {
                    println!("Couldn't save: {:?}", attempt)
                }
            }
        }
    }

    pub fn ui_draw(&mut self, ctx: &Context) {
        egui::Window::new("RAY TRACER")
            .anchor(Align2::RIGHT_TOP, Vec2::new(1.0, 1.0))
            .default_open(self.ray_trace_display)
            .show(ctx, |ui| {
                ui.add(egui::Image::new(&self.raytrace_handler).max_size(Vec2::new(500.0, 500.0)));
                ui.checkbox(&mut self.ray_trace_save, "Save as file");
                ui.text_edit_singleline(&mut self.ray_tace_file_name);
                ui.checkbox(&mut self.ui_debug_rays, "Show rays");
                ui.checkbox(&mut self.ui_debug_prime_rays, "Show prime rays");
                ui.checkbox(&mut self.ui_solve, "Solve matrix System");
                ui.add(egui::DragValue::new(&mut self.raytrace_height).speed(1.0));
                ui.add(egui::DragValue::new(&mut self.raytrace_width).speed(1.0));
            });
    }

    fn build_and_solve_a(
        mapping: Mutex<HashMap<TargetPixel, PanelPixel>>,
        target_image_raw: &[u8],
        size_s: usize,
        size_w: usize,
        target_image_width: u32,
        panel_resolution_width: u32,
    ) -> Mat<f32> {
        let mapping = mapping.lock();
        // WARNING, does image know what it looks like?
        // Build the vector
        //
        let mut mat = Mat::from_fn(size_s, 1, |i, _j| (target_image_raw[i] as f32) / 255.0);
        println!("Size s: {size_s}");
        println!("Size w: {size_w}");
        println!("Size of mapping: {}", mapping.len());
        println!("Mapping insides: {:?}", mapping.iter().collect::<Vec<_>>());

        let mut target_vec = mat.as_dyn_rows_mut();
        let trip_vec: Vec<Triplet<usize, usize, f32>> = mapping
            .iter()
            .map(|x| {
                let target_location = x.0 .1 * target_image_width + x.0 .0;
                let panel_location = x.1 .1 * panel_resolution_width + x.0 .0;

                Triplet::new(
                    target_location as usize,
                    panel_location as usize - 1,
                    1.0f32,
                )
            })
            .filter(|triplet| {
                if triplet.row > size_s || triplet.col > size_w {
                    println!("Filtering out triplet: {:?}", triplet);
                    false
                } else {
                    true
                }
            })
            .collect();

        println!("Triplet vec: {:?}", trip_vec);
        println!("Image as matrix: {:?}", target_vec);

        let target_sparse =
            SparseColMat::<usize, f32>::try_new_from_triplets(size_s, size_w, &trip_vec).unwrap();
        let qr = target_sparse.sp_qr().unwrap();

        qr.solve_lstsq_in_place_with_conj(faer::Conj::No, target_vec.rb_mut());
        println!("After QR: {:?}", target_vec);
        //mat.row_iter().map(|x| x[0]).filter(|x|)
        mat

        // Build the matrix
    }
}
