use cgmath::Vector2;
use wgpu::{BindGroup, Buffer, CommandEncoder, ComputePipeline, ComputePipelineDescriptor};

use crate::{camera::CameraHistory, light_factor::LFBuffers, scene, texture::Texture};

pub struct ReverseProj {
    compute_pipeline: ComputePipeline,
    debug_texture_buffer: Buffer,
    debug_texture_texture: Texture,
    debug_bind_group: BindGroup,
}

impl ReverseProj {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &scene::Scene,
        factorizer: &LFBuffers,
        camera_history: &CameraHistory,
    ) -> Self {
        let rv_proj = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Reverse Projection Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/reverse_projection.wgsl").into(),
            ),
        });

        let test_texture = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
            count: None,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: wgpu::TextureFormat::Rgba8Unorm,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
        };

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene Bind"),
            entries: &[test_texture],
        });

        let texture_size = 256;
        let image = image::DynamicImage::new_rgba8(texture_size, texture_size);
        let texture = Texture::from_image(device, queue, &image, Some("Target"));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reverse Projection"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            }],
        });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Reverse Projection Pipeline"),
                bind_group_layouts: &[
                    &scene.target_binds.bind_layout,
                    &scene.texture_binds.bind_layout,
                    &scene.panel_binds.bind_layout,
                    &factorizer.bind_group_layout,
                    &bind_group_layout,
                    &camera_history.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Reverse Projection Ray Tracing"),
            layout: Some(&compute_pipeline_layout),
            cache: None,
            module: &rv_proj,
            entry_point: Some("main"),
            compilation_options: Default::default(),
        });

        let u32_size = std::mem::size_of::<u32>() as u32;

        let output_buffer_size = (u32_size * texture_size * texture_size) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);
        ReverseProj {
            compute_pipeline,
            debug_texture_buffer: output_buffer,
            debug_texture_texture: texture,
            debug_bind_group: bind_group,
        }
    }

    pub fn work_group_size(target_dimensions: Vector2<u32>) -> (u32, u32) {
        (target_dimensions.x / 8 + 1, (target_dimensions.y / 8 + 1))
    }
    pub fn compute_pass(
        &self,
        encoder: &mut CommandEncoder,
        scene: &scene::Scene,
        factorizer: &LFBuffers,
        camera_history: &CameraHistory,
    ) {
        let work_group_size = Self::work_group_size(scene.world[0].pixel_count);

        let texture_size = 256;

        let u32_size = std::mem::size_of::<u32>() as u32;

        {
            let compute_pass_desc = wgpu::ComputePassDescriptor {
                label: Some("Compute pass"),
                timestamp_writes: None,
            };
            let mut compute_pass = encoder.begin_compute_pass(&compute_pass_desc);
            compute_pass.set_pipeline(&self.compute_pipeline);
            scene.compute_pass(&mut compute_pass);

            compute_pass.set_bind_group(3, &factorizer.bind_group, &[]);
            compute_pass.set_bind_group(4, Some(&self.debug_bind_group), &[]);
            compute_pass.set_bind_group(5, &camera_history.bind_group, &[]);

            compute_pass.dispatch_workgroups(work_group_size.0, work_group_size.1, 1);
        }

        {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    aspect: wgpu::TextureAspect::All,
                    texture: &self.debug_texture_texture.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &self.debug_texture_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(u32_size * texture_size),
                        rows_per_image: Some(texture_size),
                    },
                },
                self.debug_texture_texture.texture.size(),
            );
        }
    }
    pub fn print_image(&self, device: &wgpu::Device) {
        let texture_size = 256;

        {
            let buffer_slice = self.debug_texture_buffer.slice(..);

            // NOTE: We have to create the mapping THEN device.poll() before await
            // the future. Otherwise the application will freeze.
            let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            device.poll(wgpu::Maintain::Wait);
            pollster::block_on(rx.receive()).unwrap().unwrap();

            let data = buffer_slice.get_mapped_range();

            use image::{ImageBuffer, Rgba};
            let buffer =
                ImageBuffer::<Rgba<u8>, _>::from_raw(texture_size, texture_size, data).unwrap();
            buffer.save("image.png").unwrap();
        }
        self.debug_texture_buffer.unmap();
    }
}

// We need to scope the mapping variables so that we can
// unmap the buffer
