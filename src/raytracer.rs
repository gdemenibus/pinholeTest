use std::collections::HashMap;
use std::sync::Arc;

use crate::matrix::FromArr;
use crate::matrix::ToArr;
use crate::vertex::Line;
use crate::vertex::LineVertex;
use crate::vertex::ShapeWorld;
use crate::RAY_HEIGHT;
use crate::RAY_WIDTH;
use crate::SAFE_FRAC_PI_2;
use cgmath::InnerSpace;
use cgmath::Matrix4;
use cgmath::Point3;
use cgmath::Vector3;
use cgmath::Vector4;
use egui_glium::egui_winit::egui;
use egui_glium::egui_winit::egui::mutex::Mutex;
use egui_glium::egui_winit::egui::mutex::RwLock;
use egui_glium::egui_winit::egui::Align2;
use egui_glium::egui_winit::egui::ColorImage;
use egui_glium::egui_winit::egui::Context;
use egui_glium::egui_winit::egui::TextureHandle;
use egui_glium::egui_winit::egui::TextureOptions;
use egui_glium::egui_winit::egui::Vec2;
use glium::glutin::surface::WindowSurface;
use glium::Display;
use glium::DrawParameters;
use glium::Frame;
use glium::Program;
use glium::Surface;
use image::ImageBuffer;

pub struct Raytracer {
    ray_trace_display: bool,
    ray_trace_save: bool,
    ui_debug_rays: bool,
    ui_debug_prime_rays: bool,
    ray_tace_file_name: String,
    raytrace_handler: TextureHandle,
    pub debug_rays: RwLock<Vec<Line>>,
    pub debug_b_rays: RwLock<Vec<Line>>,
}

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
            ray_tace_file_name,
            raytrace_handler,
            debug_rays: RwLock::new(Vec::new()),
            debug_b_rays: RwLock::new(Vec::new()),
        }
    }
    pub fn debug_line_vertex(&self) -> Vec<LineVertex> {
        self.debug_rays
            .read()
            .iter()
            .enumerate()
            .filter(|&(index, _)| (index + 1) % 1000 == 0)
            .map(|(_, val)| val)
            .flat_map(|x| x.vertices())
            .collect()
    }
    pub fn rasterize_debug(
        &self,
        frame: &mut Frame,
        draw_params: &DrawParameters,
        program: &Program,
        view_proj: Matrix4<f32>,
        display: &Display<WindowSurface>,
    ) {
        if self.ui_debug_rays {
            let indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);
            let uniforms = uniform! {
                view_proj: view_proj.to_arr(),
                color_in: [1.0f32, 1.0f32, 1.0f32, 1.0f32],
            };

            let lines = self.debug_line_vertex();

            let buffer = glium::VertexBuffer::new(display, &lines).unwrap();

            frame
                .draw(&buffer, indices, program, &uniforms, draw_params)
                .unwrap();
        }
        if self.ui_debug_prime_rays {
            let indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);
            let uniforms = uniform! {
                view_proj: view_proj.to_arr(),
                color_in: [1.0f32, 0.0f32, 0.0f32, 0.0f32],
            };

            let lines = self.debug_b_line_vertex();

            let buffer = glium::VertexBuffer::new(display, &lines).unwrap();

            frame
                .draw(&buffer, indices, program, &uniforms, draw_params)
                .unwrap();
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
        let image_height: u32 = RAY_HEIGHT.try_into().unwrap();
        let image_width: u32 = RAY_WIDTH.try_into().unwrap();

        // CAMERA MATH!

        let d = 0.1;
        let t_n = camera_dir;
        let b_n = t_n.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
        let v_n = t_n.cross(b_n).normalize();
        let g_x = (SAFE_FRAC_PI_2 / 2.0).tan() * d;
        // IMPORTANT: Might have width and height confused?

        let g_y = g_x * ((image_height as f32 - 1.0) / (image_width as f32 - 1.0));
        let q_x = (2.0 * g_x) / (image_height as f32 - 1.0) * b_n;
        let q_y = (2.0 * g_y) / (image_width as f32 - 1.0) * v_n;
        let p_1_m = t_n * d - g_x * b_n - g_y * v_n;

        let ray_origin = camera_position;
        let second_image = Mutex::new(HashMap::new());

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

                if let Some((color, point, pixel_center)) = intersect {
                    let line = Line::new(ray_origin, point);
                    self.debug_rays.write().push(line);

                    if shape.is_transparent {
                        intersections.push(pixel_center);
                        // edit the color

                        continue;
                        //update the intersection info in an intelligent way
                    }
                    let color_vec = Vector4::<u8>::from_arr(color);
                    // edit the color?
                    color_sum = color_vec;
                }
            }

            // We have the intersection points, reason with them
            if intersections.len() == 2 {
                let a_pixel = intersections[0];
                let b_pixel = intersections[1];
                let ray_prime_origin = a_pixel;
                let ray_prime_direction = b_pixel - a_pixel;
                let shape = shapes[2].read();
                let ray_prime_origin =
                    Point3::new(ray_prime_origin.x, ray_prime_origin.y, ray_prime_origin.z);

                if let Some((color, point, _center)) =
                    shape.intersect(ray_prime_origin, ray_prime_direction)
                {
                    let line = Line::new(ray_prime_origin, point);
                    self.debug_b_rays.write().push(line);

                    second_image.lock().insert((x, y), color);
                }
            }

            image::Rgba(color_sum.to_arr())
        });
        println!(
            "Intersection between pixels is: {:}",
            second_image.lock().len()
        );

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

        let raw = ColorImage::from_rgba_unmultiplied([RAY_WIDTH, RAY_HEIGHT], &buf.into_raw());
        self.raytrace_handler.set(raw, TextureOptions::default());
        self.ray_trace_display = true;
        println!("Rays traced!");
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
                ui.checkbox(&mut self.ui_debug_prime_rays, "Show prime rays")
            });
    }
}
