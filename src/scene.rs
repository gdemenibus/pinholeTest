use std::path::PathBuf;
use wgpu::{util::DeviceExt, BindGroupLayout, Queue, RenderPass};

use crate::{
    camera::Camera,
    file_picker::FilePicker,
    raytracer::RayTraceInfo,
    shape::{Quad, Shape, VWPanel},
    texture,
};
use cgmath::{Matrix4, Rad, SquareMatrix, Vector3, Vector4};
use crevice::std140::{AsStd140, Std140, Writer};
use egui::{Color32, RichText};
use egui_winit::egui::{self, Context};
use wgpu::{BindGroup, Buffer, Device};
pub trait DrawUI {
    /*
    Draw UI for this element
    */
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let _ = title;
        let _ = ctx;
    }
}

/*
TODO: SCENE ONLY USES QUAD, MIGHT WANT MORE?
Scene struct. Encapsulates UI and handles access to the raw quads
*/
pub struct Scene {
    world: Vec<Target>,
    pub panels: Vec<ScenePanel>,
    ray_tracer: RayTraceInfo,
    pub target_binds: TargetBinds,
    pub panel_binds: PanelBinds,
    pub texture_binds: TextureBinds,
}

pub struct ScenePanel {
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    roll: Rad<f32>,
    placement: Matrix4<f32>,
    scale: Matrix4<f32>,
    pub panel: VWPanel,
    pub texture: FilePicker,
    lock_pixel: bool,
}

pub struct TargetBinds {
    pub bind_group: BindGroup,
    pub bind_layout: BindGroupLayout,
    pub ray_tracer_buffer: Buffer,
    pub scene_buffer: Buffer,
}
pub struct TextureBinds {
    pub bind_group: BindGroup,
    pub bind_layout: BindGroupLayout,
    pub target_texture: texture::Texture,
}

pub struct PanelBinds {
    pub bind_group: BindGroup,
    pub bind_layout: BindGroupLayout,
    pub panel_buffer: Buffer,
    pub panel_texture: texture::Texture,
    pub panel_bool_buffer: Buffer,
}

pub struct Target {
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    roll: Rad<f32>,
    placement: Matrix4<f32>,
    scale: Matrix4<f32>,
    quad: Quad,
}

impl TextureBinds {
    pub fn new(device: &wgpu::Device, queue: &Queue) -> Self {
        let texture_bytes = include_bytes!("../resources/textures/Aircraft_code.png");

        let texture = texture::Texture::from_bytes(device, queue, texture_bytes, "Damn");

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filter﻿able field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let text_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Size of texture being passed!"),
            contents: texture.dimensions.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: text_size_buffer.as_entire_binding(),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        TextureBinds {
            bind_group: diffuse_bind_group,
            bind_layout: texture_bind_group_layout,
            target_texture: texture,
        }
    }
}

impl TargetBinds {
    pub fn new(device: &wgpu::Device, buffer: &[u8], rt: &RayTraceInfo) -> Self {
        let rt_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ray trace buffer, contains info for shooting rays"),
            contents: rt.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer for Scene, contains all objects"),
            contents: buffer,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let scene_bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
            ],
            label: Some("Binding group for Scene"),
        });

        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Bind"),
            layout: &scene_bind_group,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scene_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rt_buffer.as_entire_binding(),
                },
            ],
        });
        TargetBinds {
            bind_group: scene_bind,
            ray_tracer_buffer: rt_buffer,
            scene_buffer,
            bind_layout: scene_bind_group,
        }
    }
}
impl PanelBinds {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &[u8]) -> Self {
        let panel_1 = image::DynamicImage::new_rgb8(1000, 1000);
        let panel_2 = image::DynamicImage::new_rgb8(1000, 1000);
        let texture_vec = vec![panel_1, panel_2];

        let panel_textures =
            texture::Texture::from_images(device, queue, &texture_vec, Some("2D Panel Array"));
        let panel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer for Panels"),
            contents: buffer,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let panel_bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filter﻿able field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Binding group for Scene"),
        });

        let panel_textures_size_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Size of texture being passed!"),
                contents: panel_textures.dimensions.as_std140().as_bytes(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let panel_bool_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boolean for panel textures"),
            contents: 0u32.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let panel_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binding For Panel group"),
            layout: &panel_bind_group,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: panel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: panel_bool_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&panel_textures.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&panel_textures.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: panel_textures_size_buffer.as_entire_binding(),
                },
            ],
        });
        PanelBinds {
            bind_layout: panel_bind_group,
            bind_group: panel_bind,
            panel_buffer,
            panel_bool_buffer,
            panel_texture: panel_textures,
        }
    }
}

impl Target {
    fn new(place_vec: Vector4<f32>) -> Self {
        let yaw = Rad(0.0);

        let pitch = Rad(0.0);
        let roll = Rad(0.0);
        let mut placement = Matrix4::identity();
        placement.w = place_vec;
        let scale = Matrix4::identity();
        let quad = Quad::new(
            Vector3::new(-0.5, 0.5, 0.0),
            Vector3::new(0.5, 0.5, 0.0),
            Vector3::new(-0.5, -0.5, 0.0),
            Vector3::new(0.5, -0.5, 0.0),
        );

        Target {
            quad,
            yaw,
            pitch,
            roll,
            placement,
            scale,
        }
    }

    fn place_target(&self) -> Quad {
        let yaw_matrix = Matrix4::from_angle_x(self.yaw);
        let pitch_matrix = Matrix4::from_angle_y(self.pitch);
        let roll_matrix = Matrix4::from_angle_z(self.roll);
        // will need to double check this ordering
        let placement_matrix =
            self.placement * self.scale * yaw_matrix * pitch_matrix * roll_matrix;
        self.quad.place(&placement_matrix)
    }
}

impl DrawUI for Target {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let title = title.unwrap_or("VW Panel".to_string());
        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 175.0])
            .default_open(false)
            .show(ctx, |ui| {
                ui.label(RichText::new("Move x").color(Color32::RED));
                ui.add(egui::Slider::new(&mut self.placement.w.x, -10.0..=10.0));
                ui.label(RichText::new("Move y").color(Color32::GREEN));
                ui.add(egui::Slider::new(&mut self.placement.w.y, -10.0..=10.0));
                ui.label(RichText::new("Move z").color(Color32::LIGHT_BLUE));
                ui.add(egui::Slider::new(&mut self.placement.w.z, -10.0..=10.0));
                ui.label(RichText::new("Yaw").color(Color32::RED));
                ui.add(egui::Slider::new(&mut self.yaw.0, -1.0..=1.0));
                ui.label(RichText::new("Pitch").color(Color32::GREEN));
                ui.add(egui::Slider::new(&mut self.pitch.0, -1.0..=1.0));
                ui.label(RichText::new("Roll").color(Color32::LIGHT_BLUE));
                ui.add(egui::Slider::new(&mut self.roll.0, -1.0..=1.0));

                ui.label("Scale x");
                ui.add(egui::DragValue::new(&mut self.scale.x.x).speed(1.0));

                ui.label("Scale y");
                ui.add(egui::DragValue::new(&mut self.scale.y.y).speed(1.0));
            });
    }
}

/// Wrapper around panel that controls access to the UI, as well as their placement
///
impl ScenePanel {
    fn place_panel(&self) -> VWPanel {
        let yaw_matrix = Matrix4::from_angle_x(self.yaw);
        let pitch_matrix = Matrix4::from_angle_y(self.pitch);
        let roll_matrix = Matrix4::from_angle_z(self.roll);
        // will need to double check this ordering
        let placement_matrix =
            self.placement * self.scale * yaw_matrix * pitch_matrix * roll_matrix;
        self.panel.place(&placement_matrix)
    }

    fn new(place_vec: Vector4<f32>, position: usize) -> ScenePanel {
        let yaw = Rad(0.0);

        let pitch = Rad(0.0);
        let roll = Rad(0.0);
        let mut placement = Matrix4::identity();
        placement.w = place_vec;
        let scale = Matrix4::identity();
        let panel = VWPanel::demo_panel();
        let default_path =
            PathBuf::from(format!("./resources/panel_compute/panel_{}.png", position));
        ScenePanel {
            pitch,
            yaw,
            roll,
            placement,
            panel,
            lock_pixel: false,
            texture: FilePicker::new("./resources/panel_compute/".to_string(), default_path),
            scale,
        }
    }
}

impl Scene {
    /// Make a quad with coordinates, but in scene space, not clip space
    pub fn new(ray_tracer: RayTraceInfo, camera: &Camera, device: &Device, queue: &Queue) -> Self {
        let place_1 = Vector4::new(0.5, 1.5, 3.0, 1.0);

        let place_2 = Vector4::new(0.5, 1.5, 2.0, 1.0);
        let target_1 = Vector4::new(0.5, 1.5, 0.0, 1.0);
        let target_2 = Vector4::new(7.5, 1.5, 0.0, 1.0);
        let world = vec![Target::new(target_1), Target::new(target_2)];
        let panels = vec![ScenePanel::new(place_1, 1), ScenePanel::new(place_2, 2)];

        let world_buffer = Self::world_as_bytes(&world, camera);
        let panels_buffer = Self::panels_as_bytes(&panels, camera);
        let target_binds = TargetBinds::new(device, &world_buffer, &ray_tracer);
        let panel_binds = PanelBinds::new(device, queue, &panels_buffer);
        let texture_binds = TextureBinds::new(device, queue);

        Scene {
            world,
            panels,
            ray_tracer,
            target_binds,
            panel_binds,
            texture_binds,
        }
    }

    pub fn update_rt_info(&mut self, camera: &Camera, height: u32, width: u32) {
        self.ray_tracer = RayTraceInfo::test(camera, height, width);
    }

    pub fn update_draw(&self, queue: &Queue, camera: &Camera, display_panel_texture: bool) {
        queue.write_buffer(
            &self.target_binds.ray_tracer_buffer,
            0,
            self.ray_tracer.as_std140().as_bytes(),
        );

        queue.write_buffer(
            &self.target_binds.scene_buffer,
            0,
            &Self::world_as_bytes(&self.world, camera),
        );

        queue.write_buffer(
            &self.panel_binds.panel_buffer,
            0,
            &Self::panels_as_bytes(&self.panels, camera),
        );
        if display_panel_texture {
            queue.write_buffer(
                &self.panel_binds.panel_bool_buffer,
                0,
                1.as_std140().as_bytes(),
            );
        } else {
            queue.write_buffer(
                &self.panel_binds.panel_bool_buffer,
                0,
                0.as_std140().as_bytes(),
            );
        }
    }
    pub fn render_pass(&self, render_pass: &mut RenderPass) {
        render_pass.set_bind_group(0, Some(&self.target_binds.bind_group), &[]);
        render_pass.set_bind_group(1, Some(&self.texture_binds.bind_group), &[]);

        render_pass.set_bind_group(2, Some(&self.panel_binds.bind_group), &[]);
    }

    /// Will always place the closest quad first
    pub fn world_as_bytes(world: &[Target], camera: &Camera) -> [u8; 256] {
        let mut buffer = [0u8; 256];
        let mut writer = Writer::new(&mut buffer[..]);
        let mut shapes: Vec<Quad> = world.iter().map(|target| target.place_target()).collect();

        shapes.sort_by(|x, y| {
            let camera_origin = camera.position;
            let x_dist = x.distance_to(camera_origin);
            let y_dist = y.distance_to(camera_origin);
            x_dist.total_cmp(&y_dist)
        });

        let _count = writer.write(shapes.as_slice()).unwrap();
        buffer
    }
    /// Will always place the closest panel first
    pub fn panels_as_bytes(panels: &[ScenePanel], camera: &Camera) -> [u8; 256] {
        let mut buffer = [0u8; 256];
        let mut writer = Writer::new(&mut buffer[..]);
        let mut panels: Vec<VWPanel> = panels
            .iter()
            .map(|x| x.place_panel())
            .map(|x| x.border_correction())
            .collect();
        panels.sort_by(|x, y| x.distance_compar(y, camera.position));
        let _count = writer.write(panels.as_slice()).unwrap();
        buffer
    }
}

impl DrawUI for ScenePanel {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let title = title.unwrap_or("VW Panel".to_string());
        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 175.0])
            .default_open(false)
            .show(ctx, |ui| {
                egui::Grid::new("By")
                    .num_columns(6)
                    .spacing([0.0, 0.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Pixel Density: ");

                        ui.add(egui::DragValue::new(&mut self.panel.pixel_count.x).speed(1.0));

                        if self.lock_pixel {
                            if ui.button("🔒".to_string()).clicked() {
                                self.lock_pixel = false;
                            }

                            self.panel.pixel_count.y = self.panel.pixel_count.x
                        } else if ui.button("🔓".to_string()).clicked() {
                            self.lock_pixel = true;
                        }

                        ui.add_enabled(
                            !self.lock_pixel,
                            egui::DragValue::new(&mut self.panel.pixel_count.y).speed(1.0),
                        );
                    });

                ui.label(RichText::new("Move x").color(Color32::RED));
                ui.add(egui::Slider::new(&mut self.placement.w.x, -10.0..=10.0));
                ui.label(RichText::new("Move y").color(Color32::GREEN));
                ui.add(egui::Slider::new(&mut self.placement.w.y, -10.0..=10.0));
                ui.label(RichText::new("Move z").color(Color32::LIGHT_BLUE));
                ui.add(egui::Slider::new(&mut self.placement.w.z, -10.0..=10.0));
                ui.label(RichText::new("Yaw").color(Color32::RED));
                ui.add(egui::Slider::new(&mut self.yaw.0, -1.0..=1.0));
                ui.label(RichText::new("Pitch").color(Color32::GREEN));
                ui.add(egui::Slider::new(&mut self.pitch.0, -1.0..=1.0));
                ui.label(RichText::new("Roll").color(Color32::LIGHT_BLUE));
                ui.add(egui::Slider::new(&mut self.roll.0, -1.0..=1.0));

                ui.label("Scale x");
                ui.add(egui::DragValue::new(&mut self.scale.x.x).speed(1.0));

                ui.label("Scale y");
                ui.add(egui::DragValue::new(&mut self.scale.y.y).speed(1.0));
                self.texture.button(ctx, ui);
            });
    }
}

impl DrawUI for Scene {
    /**
    Draw the UI for this element
    We want a system to place quads in space
    Translation, take in three coords
    Rotation: Slider
    */
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>) {
        let _title = title.unwrap_or("Scene".to_string());
        let mut count = 1;
        for target in self.world.iter_mut() {
            let title = Some(format!("Target Quad {}", count));
            count += 1;
            target.draw_ui(ctx, title);
        }
        count = 1;
        for panel in self.panels.iter_mut() {
            let title = format!("VW Panel# {} ", count);
            panel.draw_ui(ctx, Some(title));

            count += 1;
        }
    }
}
