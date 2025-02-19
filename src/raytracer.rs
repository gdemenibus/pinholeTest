use std::sync::Arc;

use cgmath::InnerSpace;
use cgmath::Point3;
use cgmath::Vector3;
use egui_glium::egui_winit::egui;
use egui_glium::egui_winit::egui::mutex::RwLock;
use egui_glium::egui_winit::egui::Align2;
use egui_glium::egui_winit::egui::ColorImage;
use egui_glium::egui_winit::egui::Context;
use egui_glium::egui_winit::egui::TextureHandle;
use egui_glium::egui_winit::egui::TextureOptions;
use egui_glium::egui_winit::egui::Vec2;
use image::ImageBuffer;
use crate::vertex::ShapeWorld;
use crate::SAFE_FRAC_PI_2;
use crate::RAY_HEIGHT;
use crate::RAY_WIDTH;



pub struct Raytracer{
    ray_trace_display: bool,
    ray_trace_save: bool,
    ray_tace_file_name: String,
    raytrace_handler: TextureHandle,
}
impl Raytracer {

    pub fn new(ray_trace_display: bool, ray_trace_save: bool, ray_tace_file_name: String, raytrace_handler: TextureHandle) -> Self {
        Self { ray_trace_display, ray_trace_save, ray_tace_file_name, raytrace_handler }
    }

    pub fn raytrace(&mut self, camera_dir: Vector3<f32>, camera_position: Point3<f32>, shapes: Vec<Arc<RwLock<ShapeWorld>>>,  ) {

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

        let d =1.0;
        let t_n = camera_dir;
        let b_n = t_n.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
        let v_n = t_n.cross(b_n).normalize();
        let g_x = (SAFE_FRAC_PI_2 / 2.0).tan() * d;
        // IMPORTANT: Might have width and height confused?

        let g_y = g_x * ((image_height as f32 - 1.0) / (image_width as f32 - 1.0) );
        let q_x = (2.0 * g_x) / (image_height as f32 - 1.0) * b_n;
        let q_y = (2.0 * g_y) / (image_width as f32 - 1.0) * v_n;
        let p_1_m = t_n * d - g_x * b_n - g_y * v_n;

        let ray_origin = camera_position;



        let buf = ImageBuffer::from_par_fn(image_width, image_height, |x, y| {

            let f_y = y as f32;
            let f_x = x as f32;
            let  color = [0_u8, 0_u8, 0_u8, 0_u8];
            let ray_dir = (p_1_m + q_x*(f_x - 1.0) + q_y *(f_y - 1.0)).normalize();

            // TODO: Right now the order of the shapes matters! Need to change that
            for shape in shapes.clone() {
                let shape = shape.read();


                let intersect = shape.intersect(ray_origin, ray_dir);
                if let Some((color, _point, _pixel_center)) = intersect {

                    return image::Rgba(color);

                }

            }

            image::Rgba(color)
        });
        if self.ray_trace_save {
            let res = buf.save_with_format(self.ray_tace_file_name.clone(), image::ImageFormat::Png);

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

        egui::Window::new("RAY TRACER").anchor(Align2::RIGHT_TOP, Vec2::new(1.0,1.0)).default_open(self.ray_trace_display).show(ctx, |ui| {
            ui.add(
                egui::Image::new(&self.raytrace_handler).max_size(Vec2::new(500.0, 500.0))
            );
            ui.checkbox(&mut self.ray_trace_save, "Save as file");
            ui.text_edit_singleline(&mut self.ray_tace_file_name);
        });

    }

}

