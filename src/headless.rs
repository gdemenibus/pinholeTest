use crate::{texture::Texture, utils::DrawUI};
use egui::Id;
use wgpu::Buffer;
pub struct HeadlessImage {
    pub texture_buffer: Buffer,
    pub texture: Texture,
    pub width: u32,
    pub height: u32,
    pub retrieve_image: bool,
    pub first_draw: bool,
    pub name_inserted: bool,
    pub save_name: String,
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
            first_draw: false,
            save_name: "".to_string(),
            name_inserted: false,
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
        if !self.name_inserted {
            return;
        }
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

            buffer.save(self.save_name.clone()).unwrap();
        }
        self.texture_buffer.unmap();
        self.retrieve_image = false;
    }
}

impl DrawUI for HeadlessImage {
    fn draw_ui(&mut self, ctx: &egui::Context, title: Option<String>, ui: Option<&mut egui::Ui>) {
        let _title = title.unwrap_or("Save manager".to_string());
        let _ = ctx;
        let _ = ui;
        // Early exit, was not requested
        if !self.retrieve_image {
            self.first_draw = true;
            self.name_inserted = false;
            return;
        }

        egui_winit::egui::Window::new("Save Print:")
            .resizable(false)
            .vscroll(false)
            .show(ctx, |ui| {
                ui.label("Save print as:");

                // Create the text field with auto focus
                let text_edit = egui::TextEdit::singleline(&mut self.save_name)
                    .hint_text("Image name")
                    .id(Id::new(20))
                    .desired_width(f32::INFINITY);
                if !self.first_draw {
                    ui.add(text_edit);
                } else {
                    ui.add(text_edit).request_focus();
                }

                // Only request focus the first frame we show the prompt

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        println!("Saving to file: {}", self.save_name);
                        if !self.save_name.ends_with(".png") {
                            self.save_name.push_str(".png");
                        }

                        self.first_draw = false;
                        self.name_inserted = true;
                    }

                    if ui.button("Cancel").clicked() {
                        self.retrieve_image = false;
                        self.name_inserted = false;
                        self.save_name.clear(); // optional
                    }
                });
                self.first_draw = false;
            });
    }
}
