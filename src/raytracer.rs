use crate::camera::Camera;
use cgmath::*;

#[derive(crevice::std140::AsStd140)]
pub struct RayTraceInfo {
    ray_origin: Vector3<f32>,
    q_x: Vector3<f32>,
    q_y: Vector3<f32>,
    p_1_m: Vector3<f32>,
}

impl RayTraceInfo {
    pub fn test(camera: &Camera, image_height: u32, image_width: u32) -> Self {
        let camera_dir = camera.direction_vec();

        let d = 0.1;
        let t_n = camera_dir;
        let b_n = t_n.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
        let v_n = t_n.cross(b_n).normalize();
        let g_x = (camera.fov.0 / 2.0).tan() * d;
        // IMPORTANT: Might have width and height confused?

        // straight from the wikipedia article on raytracing
        let g_y = g_x * ((image_height as f32 - 1.0) / (image_width as f32 - 1.0));
        let q_x = (2.0 * g_x) / (image_width as f32 - 1.0) * b_n;
        let q_y = (2.0 * g_y) / (image_height as f32 - 1.0) * v_n;
        let p_1_m = t_n * d - g_x * b_n - g_y * v_n;
        RayTraceInfo {
            ray_origin: camera.position.to_vec(),
            q_x,
            q_y,
            p_1_m,
        }
    }
}
