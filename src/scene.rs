use crate::utils::DrawUI;
use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wgpu::{util::DeviceExt, BindGroupLayout, ComputePass, Queue, RenderPass};
use winit::event_loop::EventLoopProxy;

use crate::{
    camera::Camera,
    file_picker::FilePicker,
    raytracer::RayTraceInfo,
    shape::{Quad, Shape, Sphere, VWPanel},
    texture::{self, Texture},
};
use cgmath::{vec4, Matrix4, Rad, SquareMatrix, Vector2, Vector3, Vector4};
use crevice::std140::{self, AsStd140, Std140, Writer};
use egui::{Color32, RichText, Ui};
use egui_winit::egui::{self, Context};
use wgpu::{BindGroup, Buffer, Device};

const BACK: &[u8] = include_bytes!("../resources/skybox/back.jpg");
const BOTTOM: &[u8] = include_bytes!("../resources/skybox/bottom.jpg");
const FRONT: &[u8] = include_bytes!("../resources/skybox/front.jpg");
const LEFT: &[u8] = include_bytes!("../resources/skybox/left.jpg");
const RIGHT: &[u8] = include_bytes!("../resources/skybox/right.jpg");
const TOP: &[u8] = include_bytes!("../resources/skybox/top.jpg");

/*
TODO: SCENE ONLY USES QUAD, MIGHT WANT MORE?
Scene struct. Encapsulates UI and handles access to the raw quads
*/
pub struct Scene {
    pub world: Target,
    pub sphere: SphereHolder,
    pub panels: Vec<ScenePanel>,
    ray_tracer: RayTraceInfo,
    pub target_binds: TargetBinds,
    pub panel_binds: PanelBinds,
    pub texture_binds: TextureBinds,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScenePanel {
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    roll: Rad<f32>,
    placement: Matrix4<f32>,
    scale: Matrix4<f32>,
    pub panel: VWPanel,
    #[serde(skip)]
    pub texture: FilePicker,
    lock_pixel: bool,
}

pub struct TargetBinds {
    pub bind_group: BindGroup,
    pub bind_layout: BindGroupLayout,
    pub ray_tracer_buffer: Buffer,
    pub scene_buffer: Buffer,
    pub background_buffer: Buffer,
    pub sphere_buffer: Buffer,
    pub transparent_content_buffer: Buffer,
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
    pub distort_rays_buffer: Buffer,
}

pub struct SphereHolder {
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    roll: Rad<f32>,
    placement: Matrix4<f32>,
    scale: Matrix4<f32>,
    sphere: Sphere,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Target {
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    roll: Rad<f32>,
    placement: Matrix4<f32>,
    scale: Matrix4<f32>,
    quad: Quad,
    pub pixel_count: Vector2<u32>,
    // TODO: Change this to UOM
    pub size: Vector2<f32>,
    #[serde(skip)]
    pub texture: FilePicker,
    pub world_color: Vector4<f32>,
    pub target_transparent: bool,
}

impl SphereHolder {
    pub fn new(sphere: Sphere) -> Self {
        let yaw = Rad(0.0);
        let pitch = Rad(0.0);
        let roll = Rad(0.0);
        let mut placement = Matrix4::identity();
        placement.w = Vector4::new(sphere.position.x, sphere.position.y, sphere.position.x, 1.0);
        let scale = Matrix4::identity();
        SphereHolder {
            yaw,
            pitch,
            roll,
            placement,
            scale,
            sphere,
        }
    }
    pub fn place_sphere(&self) -> Sphere {
        let yaw_matrix = Matrix4::from_angle_x(self.yaw);
        let pitch_matrix = Matrix4::from_angle_y(self.pitch);
        let roll_matrix = Matrix4::from_angle_z(self.roll);
        // will need to double check this ordering
        let placement_matrix =
            self.placement * self.scale * yaw_matrix * pitch_matrix * roll_matrix;
        self.sphere.place(&placement_matrix)
    }
}

impl TextureBinds {
    fn new(device: &wgpu::Device, queue: &Queue, cube_map: CubeMap) -> Self {
        //let texture_bytes = include_bytes!("../resources/textures/Aircraft_code.png");

        //let texture = texture::Texture::from_bytes(device, queue, texture_bytes, "Damn");
        let img = image::DynamicImage::new_rgb8(6000, 6000);
        let label = Some("Target Texture");
        let texture = texture::Texture::from_image(device, queue, &img, label);

        let copy = texture.texture.as_image_copy();
        let default_img = image::open("./resources/textures/256.png").unwrap();
        let img_dimensions = default_img.dimensions();

        queue.write_texture(
            copy,
            &default_img.to_rgba8(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * img_dimensions.0),
                rows_per_image: Some(img_dimensions.1),
            },
            wgpu::Extent3d {
                width: img_dimensions.0,
                height: img_dimensions.1,
                depth_or_array_layers: 1,
            },
        );

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::all(),
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::all(),
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::all(),
                        // This should match the filter﻿able field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::all(),
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::all(),
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&cube_map.cube_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&cube_map.cube_texture.sampler),
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

    pub fn update_target_texture(&self, img: &DynamicImage, queue: &Queue) -> Result<(), ()> {
        let our_dimensions = self.target_texture.dimensions;
        let img_dimensions = img.dimensions();
        if our_dimensions.x < img_dimensions.0 || our_dimensions.y < img_dimensions.1 {
            println!(
                "Selected texture {:?} is larger than allocated buffer {:?}",
                img_dimensions,
                (our_dimensions.x, our_dimensions.y)
            );
            return Err(());
        }
        let copy = self.target_texture.texture.as_image_copy();

        queue.write_texture(
            copy,
            &img.to_rgba8(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * img_dimensions.0),
                rows_per_image: Some(img_dimensions.1),
            },
            wgpu::Extent3d {
                width: img_dimensions.0,
                height: img_dimensions.1,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }
}

impl TargetBinds {
    pub fn new(device: &wgpu::Device, buffer: &[u8], rt: &RayTraceInfo, sphere: &Sphere) -> Self {
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
        let bg_color: Vector4<f32> = vec4(0.5, 0.5, 0.5, 1.0);

        let background_color = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Background Color Buffer"),
            contents: bg_color.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sphere_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Buffer"),
            contents: sphere.as_std140().as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let transparent_content_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Setting Background Buffer"),
                contents: 1u32.as_std140().as_bytes(),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: background_color.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sphere_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: transparent_content_buffer.as_entire_binding(),
                },
            ],
        });
        TargetBinds {
            transparent_content_buffer,
            sphere_buffer,
            background_buffer: background_color,
            bind_group: scene_bind,
            ray_tracer_buffer: rt_buffer,
            scene_buffer,

            bind_layout: scene_bind_group,
        }
    }
}
impl PanelBinds {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &[u8]) -> Self {
        let panel_1 = image::DynamicImage::new_rgb8(6000, 6000);
        let panel_2 = image::DynamicImage::new_rgb8(6000, 6000);
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
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::all(),
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::all(),
                    // This should match the filter﻿able field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
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

        let distort_rays_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boolean for Ray Distortion"),
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
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: distort_rays_buffer.as_entire_binding(),
                },
            ],
        });
        PanelBinds {
            distort_rays_buffer,
            bind_layout: panel_bind_group,
            bind_group: panel_bind,
            panel_buffer,
            panel_bool_buffer,
            panel_texture: panel_textures,
        }
    }
}

impl Target {
    fn new(place_vec: Vector4<f32>, pixel_count: Vector2<u32>, size: Vector2<f32>) -> Self {
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

        let default_path = PathBuf::from("./resources/textures/256.png".to_string());

        let world_color = vec4(0.5, 0.5, 0.5, 1.0);
        Target {
            target_transparent: true,
            pixel_count,
            world_color,
            size,
            quad,
            yaw,
            pitch,
            roll,
            placement,
            scale,

            texture: FilePicker::new("./resources/textures/".to_string(), default_path),
        }
    }
    pub fn update_pixel_count(&mut self, pixel_count: (u32, u32)) {
        let pixel_count = Vector2::new(pixel_count.0, pixel_count.1);
        self.pixel_count = pixel_count;
    }

    fn place_target(&self) -> Target {
        let yaw_matrix = Matrix4::from_angle_x(self.yaw);
        let pitch_matrix = Matrix4::from_angle_y(self.pitch);
        let roll_matrix = Matrix4::from_angle_z(self.roll);
        // will need to double check this ordering
        let placement_matrix =
            self.placement * self.scale * yaw_matrix * pitch_matrix * roll_matrix;
        let quad = self.quad.place(&placement_matrix);
        let mut clone = self.clone();
        clone.quad = quad;
        clone
    }
    fn target_to_bytes(&self) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = std140::Writer::new(&mut output);
        writer.write(&self.quad).unwrap();
        writer.write(&self.pixel_count).unwrap();
        writer.write(&self.size).unwrap();
        output
    }
}

impl DrawUI for Target {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>, ui: Option<&mut Ui>) {
        let _ = ui;
        let title = title.unwrap_or("Target".to_string());
        egui_winit::egui::Window::new(title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 175.0])
            .default_open(false)
            .show(ctx, |ui| {
                self.texture.button(ctx, ui);
                ui.label(
                    RichText::new(format!("Pixels X: {}", self.pixel_count.x))
                        .color(Color32::ORANGE),
                );
                ui.label(
                    RichText::new(format!("Pixels Y: {}", self.pixel_count.y))
                        .color(Color32::ORANGE),
                );
                let mut rgb = [self.world_color.x, self.world_color.y, self.world_color.z];
                let _response = egui::widgets::color_picker::color_edit_button_rgb(ui, &mut rgb);
                self.world_color.x = rgb[0];
                self.world_color.y = rgb[1];
                self.world_color.z = rgb[2];

                ui.checkbox(&mut self.target_transparent, "Content is transparent");
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
                self.size.x = self.scale.x.x;

                ui.label("Scale y");
                ui.add(egui::DragValue::new(&mut self.scale.y.y).speed(1.0));

                self.size.y = self.scale.y.y;
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
        let default_path = PathBuf::from(format!("./resources/panel_compute/panel_{position}.png"));
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
        let pixel_count = Vector2::new(256, 256);
        let size = Vector2::new(1.0, 1.0);
        let world = Target::new(target_1, pixel_count, size);
        let panels = vec![ScenePanel::new(place_1, 1), ScenePanel::new(place_2, 2)];

        let world_buffer = Self::world_as_bytes(&world, camera);
        let panels_buffer = Self::panels_as_bytes(&panels, camera);
        let position = Vector3::new(0.5, 0.5, -3.0);
        let radius = 0.3f32;
        let red = Vector4::new(1.0, 0.0, 0.0, 1.0);
        let white = Vector4::new(1.0, 1.0, 1.0, 1.0);
        let sphere_raw = Sphere::new(position, radius, red, white);

        let target_binds = TargetBinds::new(device, &world_buffer, &ray_tracer, &sphere_raw);

        let sphere = SphereHolder::new(sphere_raw);
        let panel_binds = PanelBinds::new(device, queue, &panels_buffer);
        let cube_map = CubeMap::default(device, queue);
        let texture_binds = TextureBinds::new(device, queue, cube_map);

        Scene {
            sphere,
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

    pub fn update_draw(
        &self,
        queue: &Queue,
        camera: &Camera,
        display_panel_texture: bool,
        distort_rays: bool,
    ) {
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
            &self.target_binds.sphere_buffer,
            0,
            self.sphere.place_sphere().as_std140().as_bytes(),
        );
        if self.world.target_transparent {
            queue.write_buffer(
                &self.target_binds.transparent_content_buffer,
                0,
                1.as_std140().as_bytes(),
            );
        } else {
            queue.write_buffer(
                &self.target_binds.transparent_content_buffer,
                0,
                0.as_std140().as_bytes(),
            );
        }

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
        if distort_rays {
            queue.write_buffer(
                &self.panel_binds.distort_rays_buffer,
                0,
                1.as_std140().as_bytes(),
            );
        } else {
            queue.write_buffer(
                &self.panel_binds.distort_rays_buffer,
                0,
                0.as_std140().as_bytes(),
            );
        }
        queue.write_buffer(
            &self.target_binds.background_buffer,
            0,
            self.world.world_color.as_std140().as_bytes(),
        );
    }
    pub fn render_pass(&self, render_pass: &mut RenderPass) {
        render_pass.set_bind_group(0, Some(&self.target_binds.bind_group), &[]);
        render_pass.set_bind_group(1, Some(&self.texture_binds.bind_group), &[]);

        render_pass.set_bind_group(2, Some(&self.panel_binds.bind_group), &[]);
    }

    pub fn compute_pass(&self, compute_pass: &mut ComputePass) {
        compute_pass.set_bind_group(0, Some(&self.target_binds.bind_group), &[]);
        compute_pass.set_bind_group(1, Some(&self.texture_binds.bind_group), &[]);

        compute_pass.set_bind_group(2, Some(&self.panel_binds.bind_group), &[]);
    }

    /// Will always place the closest quad first
    pub fn world_as_bytes(world: &Target, camera: &Camera) -> [u8; 256] {
        let mut shapes: Vec<Target> = vec![world.place_target()];

        shapes.sort_by(|x, y| {
            let camera_origin = camera.position;
            let x_dist = x.quad.distance_to(camera_origin);
            let y_dist = y.quad.distance_to(camera_origin);
            x_dist.total_cmp(&y_dist)
        });
        let byte_vec: Vec<u8> = shapes
            .iter()
            .flat_map(|shape| shape.target_to_bytes())
            .collect();
        // Pad?
        let mut buffer = [0u8; 256];
        for (i, x) in byte_vec.iter().enumerate() {
            buffer[i] = *x;
        }
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
    pub fn change_panel_res(&mut self, new_res: usize) {
        let new_res = new_res as u32;
        self.panels[0].panel.pixel_count = Vector2::new(new_res, new_res);
        self.panels[1].panel.pixel_count = Vector2::new(new_res, new_res);
    }
}

impl DrawUI for ScenePanel {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>, ui: Option<&mut Ui>) {
        let _ = ui;
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

struct CubeMap {
    cube_texture: Texture,
}
impl CubeMap {
    fn default(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let right = image::load_from_memory(RIGHT).unwrap();
        let left = image::load_from_memory(LEFT).unwrap();

        let top = image::load_from_memory(TOP).unwrap();
        let bottom = image::load_from_memory(BOTTOM).unwrap();

        let front = image::load_from_memory(FRONT).unwrap();
        let back = image::load_from_memory(BACK).unwrap();

        let images = vec![right, left, top, bottom, front, back];
        let texture = Texture::cube_map(device, queue, &images, Some("CubeMap"));

        Self {
            cube_texture: texture,
        }
    }
}

impl DrawUI for Scene {
    /**
    Draw the UI for this element
    We want a system to place quads in space
    Translation, take in three coords
    Rotation: Slider
    */
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>, ui: Option<&mut Ui>) {
        let _ = ui;
        let _title = title.unwrap_or("Scene".to_string());
        let target = &mut self.world;
        let title = Some("Target Quad".to_string());
        target.draw_ui(ctx, title.clone(), None);
        self.sphere.draw_ui(ctx, title, ui);
        let mut count = 1;
        for panel in self.panels.iter_mut() {
            let title = format!("VW Panel# {count} ");
            panel.draw_ui(ctx, Some(title), None);

            count += 1;
        }
    }
}
impl DrawUI for SphereHolder {
    fn draw_ui(&mut self, ctx: &Context, title: Option<String>, ui: Option<&mut Ui>) {
        let _ = title;
        let _ = ctx;
        let _ = ui;
        let title = "Sphere Controls".to_string();

        egui_winit::egui::Window::new(&title)
            .resizable(true)
            .vscroll(true)
            .default_size([150.0, 175.0])
            .default_open(false)
            .show(ctx, |ui| {
                self.sphere.draw_ui(ctx, Some(title), Some(ui));

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
