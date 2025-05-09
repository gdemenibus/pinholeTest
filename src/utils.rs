use egui::ahash::HashSet;
use wgpu::Buffer;

pub fn sample_buffer(sample_buffer: &Buffer, device: &wgpu::Device) {
    let buffer_slice = sample_buffer.slice(..);
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    pollster::block_on(rx.receive()).unwrap().unwrap();
    // Scope to drop buffer view, ensuring we can unmap it
    {
        let data = buffer_slice.get_mapped_range();

        let data_filtered: Vec<f32> = data
            .chunks(4)
            .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        let mut triplets = data_filtered
            .chunks(3)
            .filter_map(|chunk| {
                let x_coord = chunk[0] as u32;
                let y_coord = chunk[1] as u32;
                let sample = chunk[2];
                if sample > 0.0 {
                    Some((x_coord, y_coord, sample))
                } else {
                    None
                }
            })
            .collect::<Vec<(u32, u32, f32)>>();
        let mut seen = HashSet::default();
        triplets.retain(|(x, y, _s)| seen.insert((*x, *y)));
        let max = triplets
            .iter()
            .fold(0.0f32, |acc, next| if acc > next.2 { acc } else { next.2 });
        println!("Max Value is: {}", max);
        println!("Triplet count: {}", triplets.len());
        //let size = self.state.as_ref().unwrap().scene.panels_pixel_count();
        //self.nmf_solver.add_sample(triplets, size);
    }
    sample_buffer.unmap();
}
