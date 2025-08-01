use crate::texture::Texture;
use wgpu::Buffer;
pub struct HeadlessImage {
    pub texture_buffer: Buffer,
    pub texture: Texture,
    pub width: u32,
    pub height: u32,
    pub retrieve_image: bool,
}
impl HeadlessImage {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) -> Self {
        let image = image::DynamicImage::new_rgba8(width, height);
        let texture = Texture::headless_texture(device, queue, &image, Some("Target"));

        let u32_size = std::mem::size_of::<u32>() as u32;
        let unalisgned_bytes_per_row = u32_size * width;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let aligned_bytes_per_row = unalisgned_bytes_per_row.div_ceil(align) * align;

        let u32_size = std::mem::size_of::<u32>() as u32;

        let output_buffer_size = (u32_size * aligned_bytes_per_row * height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);

        HeadlessImage {
            width,
            height,
            texture_buffer: output_buffer,
            texture,
            retrieve_image: false,
        }
    }
    pub fn handle_resize(
        &mut self,
        width: u32,
        height: u32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let image = image::DynamicImage::new_rgba8(width, height);
        let texture = Texture::headless_texture(device, queue, &image, Some("Texture For view"));
        self.width = width;
        self.height = height;

        let u32_size = std::mem::size_of::<u32>() as u32;
        let unalisgned_bytes_per_row = u32_size * width;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let aligned_bytes_per_row = unalisgned_bytes_per_row.div_ceil(align) * align;

        let u32_size = std::mem::size_of::<u32>() as u32;

        let output_buffer_size = (u32_size * aligned_bytes_per_row * height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);
        self.texture = texture;
        self.texture_buffer = output_buffer
    }

    pub fn draw_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        // Early exit, no image retrieval requested
        if !self.retrieve_image {
            return;
        }
        let u32_size = std::mem::size_of::<u32>() as u32;
        let unalisgned_bytes_per_row = u32_size * self.width;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let aligned_bytes_per_row = unalisgned_bytes_per_row.div_ceil(align) * align;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.texture_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aligned_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            self.texture.texture.size(),
        );
    }

    pub fn print_image(&mut self, device: &wgpu::Device) {
        println!("Printing Screen!");
        let pixel_size: u32 = 4;
        let actual_bytes_per_row = self.width * pixel_size;
        let aligned_bytes_per_row = actual_bytes_per_row.div_ceil(256) * 256;

        {
            let buffer_slice = self.texture_buffer.slice(..);

            // NOTE: We have to create the mapping THEN device.poll() before await
            // the future. Otherwise the application will freeze.
            let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            device.poll(wgpu::Maintain::Wait);
            pollster::block_on(rx.receive()).unwrap().unwrap();

            let data = buffer_slice.get_mapped_range();
            let mut pixels = vec![0u8; (self.width * self.height * pixel_size) as usize];
            for row in 0..self.height {
                let src_offset = (row * aligned_bytes_per_row) as usize;
                let dst_offset = (row * actual_bytes_per_row) as usize;

                let src_range = src_offset..(src_offset + actual_bytes_per_row as usize);
                let dst_range = dst_offset..(dst_offset + actual_bytes_per_row as usize);
                pixels[dst_range].copy_from_slice(&data[src_range]);
            }

            use image::{ImageBuffer, Rgba};
            let buffer =
                ImageBuffer::<Rgba<u8>, _>::from_raw(self.width, self.height, pixels).unwrap();

            buffer.save("test.png").unwrap();
        }
        self.texture_buffer.unmap();
        self.retrieve_image = false;
    }
}
