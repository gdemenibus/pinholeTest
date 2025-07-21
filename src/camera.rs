use cgmath::*;
use crevice::std140::AsStd140;
use crevice::std140::Std140;
use crevice::std140::Writer;
use egui::Color32;
use egui::RichText;
use egui::Slider;
use egui::Ui;
use serde::Deserialize;
use serde::Serialize;
use std::collections::VecDeque;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use std::time::Instant;
use wgpu::util::DeviceExt;
use wgpu::BindGroup;
use wgpu::BindGroupLayout;
use wgpu::Buffer;
use wgpu::Device;
use wgpu::Queue;
use winit::event::ElementState;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

use light_field_test::utils::DrawUI;
const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

fn lerp_vec3(a: Point3<f32>, b: Point3<f32>, t: f32) -> Point3<f32> {
    a + (b - a) * t
}
fn lerp_angle(a: Rad<f32>, b: Rad<f32>, t: f32) -> Rad<f32> {
    let mut delta = b.0 - a.0;
    if delta > std::f32::consts::PI {
        delta -= 2.0 * std::f32::consts::PI;
    } else if delta < -std::f32::consts::PI {
        delta += 2.0 * std::f32::consts::PI;
    }
    Rad(a.0 + delta * t)
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    pub fov: Rad<f32>,
}
impl Camera {
    pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>, FOV: Into<Rad<f32>>>(
        position: V,
        yaw: Y,
        pitch: P,
        fov: FOV,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            fov: fov.into(),
        }
    }
    pub fn direction_vec(&self) -> Vector3<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize()
    }
    fn interpolate_camera(a: &Camera, b: &Camera, t: f32) -> Camera {
        Camera {
            position: lerp_vec3(a.position, b.position, t),
            yaw: lerp_angle(a.yaw, b.yaw, t),
            pitch: lerp_angle(a.pitch, b.pitch, t),
            fov: Rad(a.fov.0 + (b.fov.0 - a.fov.0) * t), // or use same angle lerp
        }
    }
}
impl DrawUI for Camera {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut Ui>) {
        let title = title.unwrap_or("Camera Settings".to_string());
        if let Some(ui) = ui {
            ui.label(
                RichText::new(format!(
                    "Camera Position: ({:.2}, {:.2}, {:.2})",
                    self.position.x, self.position.y, self.position.z
                ))
                .color(Color32::RED),
            );
        } else {
            egui_winit::egui::Window::new(title)
                .resizable(true)
                .vscroll(true)
                .default_open(false)
                .default_size([150.0, 125.0])
                .show(ctx, |ui| {
                    ui.label("FOV");
                    // Present in Degrees
                    ui.add(
                        Slider::new(&mut self.fov.0, 0.1..=std::f32::consts::PI)
                            .custom_formatter(|n, _| {
                                let print = n * 180.0 / std::f64::consts::PI;
                                format!("{print}")
                            })
                            .custom_parser(|s| {
                                s.parse::<f64>()
                                    .map(|r| r * std::f64::consts::PI / 180.0)
                                    .ok()
                            }),
                    );
                });
        }
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, event: KeyEvent, disabled: bool) -> bool {
        let amount = if event.state == ElementState::Pressed && !disabled {
            1.0
        } else {
            0.0
        };
        match event.physical_key {
            PhysicalKey::Code(KeyCode::KeyW) => {
                self.amount_forward = amount;
                true
            }
            PhysicalKey::Code(KeyCode::KeyS) => {
                self.amount_backward = amount;
                true
            }
            PhysicalKey::Code(KeyCode::KeyA) => {
                self.amount_left = amount;
                true
            }
            PhysicalKey::Code(KeyCode::KeyD) => {
                self.amount_right = amount;
                true
            }
            PhysicalKey::Code(KeyCode::Space) => {
                self.amount_up = amount;
                true
            }
            PhysicalKey::Code(KeyCode::CapsLock) => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
        let scrollward =
            Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate
        camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non-cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            camera.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
            camera.pitch = Rad(SAFE_FRAC_PI_2);
        }
    }
}

// Struct to store camera positions, especially when sampling!
pub struct CameraHistory {
    pub history: VecDeque<Camera>,
    pub kernel_size: f32,
    pub kernel: bool,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub history_buffer: Buffer,
    pub size_buffer: Buffer,
    pub animate: bool,
    pub animation_duration: f32,
    pub animation_start: Option<Instant>,
}
impl CameraHistory {
    pub fn new(device: &Device) -> Self {
        let history_layout = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };
        let size_layout = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::all(),
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera History Bind Group Layout"),
            entries: &[history_layout, size_layout],
        });
        let history_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            // Each camera position is 3 f32s. 12 bytes. 21 postions should be fine
            size: 1024,
            label: Some("Camera History Buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        let size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera History size buffer"),
            contents: 0u32.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binding For Camera History"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: history_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: size_buffer.as_entire_binding(),
                },
            ],
        });
        history_buffer.unmap();

        CameraHistory {
            animation_start: None,
            bind_group_layout,
            kernel: false,
            animate: false,
            animation_duration: 2.0,
            size_buffer,
            history_buffer,
            bind_group,
            kernel_size: 0.01,
            history: VecDeque::new(),
        }
    }
    pub fn update_history(&mut self, new_vec: VecDeque<Camera>) {
        self.history = new_vec.clone();
    }
    pub fn save_point(&mut self, camera: &Camera) {
        if !self.history.contains(camera) {
            // Tiny kernel
            let center_camera = camera.clone();

            let mut top_camera = center_camera.clone();
            top_camera.position += Vector3::new(0.00, self.kernel_size, 0.0);

            let mut bottom_camera = center_camera.clone();
            bottom_camera.position -= Vector3::new(0.00, self.kernel_size, 0.0);

            let mut left_camera = center_camera.clone();
            left_camera.position -= Vector3::new(self.kernel_size, 0.0, 0.0);
            let mut right_camera = center_camera.clone();
            right_camera.position += Vector3::new(self.kernel_size, 0.0, 0.0);

            self.history.push_back(center_camera);
            if self.kernel {
                self.history.push_back(top_camera);
                self.history.push_back(bottom_camera);
                self.history.push_back(right_camera);
                self.history.push_back(left_camera);
            }
        }
    }
    pub fn next_save(&mut self) -> Option<&Camera> {
        if self.history.is_empty() {
            return None;
        }
        self.history.rotate_left(1);
        let next = self.history.back();
        next
    }
    pub fn previous_save(&mut self) -> Option<&Camera> {
        if self.history.is_empty() {
            return None;
        }
        self.history.rotate_right(1);
        let next = self.history.back();
        next
    }
    pub fn len(&self) -> usize {
        self.history.len()
    }
    pub fn history_to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut writer = Writer::new(&mut buffer);
        for x in self.history.iter() {
            writer.write(&x.position).unwrap();
        }
        buffer
    }
    pub fn size_to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut writer = Writer::new(&mut buffer);
        writer.write(&(self.history.len() as u32)).unwrap();

        buffer
    }
    pub fn update_buffer(&self, queue: &Queue) {
        queue.write_buffer(&self.history_buffer, 0, &self.history_to_bytes());
        queue.write_buffer(&self.size_buffer, 0, &self.size_to_bytes());
    }
    pub fn reset(&mut self) {
        self.history = VecDeque::new();
    }

    pub fn animate_camera(&self) -> Option<Camera> {
        let time = self.animation_start?.elapsed().as_secs_f32();
        let duration = self.animation_duration;
        let keyframes = &self.history;
        let num_segments = keyframes.len() - 1;
        let total_time = duration * num_segments as f32;
        let t = (time % total_time) / duration;
        let i = ((time % total_time) / duration).floor() as usize;

        let current = &keyframes[i];
        let next = &keyframes[i + 1];
        Some(Camera::interpolate_camera(current, next, t.fract()))
    }
}
impl DrawUI for CameraController {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut Ui>) {
        let _ = title;
        let _ = ctx;
        let _ = ui;
        if let Some(ui) = ui {
            ui.label("Camera Speed:");
            ui.add(egui::DragValue::new(&mut self.speed).speed(0.5));
        }
    }
}

impl DrawUI for CameraHistory {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut Ui>) {
        let title = title.unwrap_or("Camera Settings".to_string());
        let _ = ui;
        if let Some(ui) = ui {
            ui.label(format!(
                "Current camera positions saved:{}",
                self.history.len()
            ));

            self.animate = ui.button("Play Animation").clicked();
            if self.animate {
                self.animation_start = Some(Instant::now());
            } else if let Some(start) = self.animation_start {
                if start.elapsed().as_secs_f32()
                    > (self.animation_duration * (self.history.len() - 1) as f32)
                {
                    self.animation_start = None;
                }
            }

            ui.label("Animation Position duration:");
            ui.add(egui::DragValue::new(&mut self.animation_duration).speed(0.1));

            if ui.button("Reset").clicked() {
                self.history = VecDeque::new();
            };
            ui.label("Kernel size:");

            ui.add(egui::DragValue::new(&mut self.kernel_size).speed(0.01));
            ui.checkbox(&mut self.kernel, "Use Kernl");
        } else {
            egui_winit::egui::Window::new(title)
                .resizable(true)
                .vscroll(true)
                .default_open(false)
                .default_size([150.0, 125.0])
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Current camera positions saved:{}",
                        self.history.len()
                    ));

                    if ui.button("Reset").clicked() {
                        self.history = VecDeque::new();
                    }
                });
        }
    }
}
