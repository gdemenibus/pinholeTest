use image::{DynamicImage, GenericImageView};
use wgpu::Buffer;

pub fn sample_buffer(sample_buffer: &Buffer, device: &wgpu::Device) -> Vec<u8> {
    let buffer_slice = sample_buffer.slice(..);
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    pollster::block_on(rx.receive()).unwrap().unwrap();
    // Scope to drop buffer view, ensuring we can unmap it
    let data_filtered = {
        let data = buffer_slice.get_mapped_range();

        data.to_vec()
    };
    sample_buffer.unmap();
    data_filtered
}
