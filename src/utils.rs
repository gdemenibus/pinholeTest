use std::u32;

use egui::ahash::{HashMap, HashSet};
use faer::{sparse::Triplet, Mat};
use image::{DynamicImage, GenericImageView, ImageBuffer};
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

pub fn check_triplets(rows: u32, columns: u32, triplets: &mut Vec<Triplet<u32, u32, f32>>) {
    let pre_filter = triplets.len();
    triplets.retain(|x| x.row < rows && x.col < columns);
    let post_filer = triplets.len();
    let diff = pre_filter - post_filer;
    println!("Filtered {diff} entries");
    println!("Triplet size is: {}", triplets.len());
    if triplets.len() < 5 {
        println!("Triplets are: {triplets:#?}");
    }
}

pub fn buffer_to_triplet(buffer: &Buffer, device: &wgpu::Device) -> HashSet<(u32, u32)> {
    let raw_bytes = sample_buffer(buffer, device);
    let entries: Vec<u32> = raw_bytes
        .chunks(4)
        .map(|x| u32::from_ne_bytes(x[0..4].try_into().unwrap()))
        .collect();
    let mut seen: HashSet<(u32, u32)> = HashSet::default();
    let mut triplet_list: Vec<(u32, u32, u32)> = entries
        .chunks(3)
        .filter_map(|x| {
            // no recording done
            if x[2] == 0 {
                None
            } else {
                Some((x[0], x[1], x[2]))
            }
        })
        .collect();
    // Remove duplicates!
    triplet_list.retain(|(x, y, _entry)| seen.insert((*x, *y)));
    let max_index = triplet_list.iter().max();
    println!("Max seen is: {max_index:?}");
    seen
}

pub fn buffer_increasing_check(buffer: &Buffer, device: &wgpu::Device) {
    let raw_bytes = sample_buffer(buffer, device);
    let entries: Vec<u32> = raw_bytes
        .chunks(4)
        .map(|x| u32::from_ne_bytes(x[0..4].try_into().unwrap()))
        .collect();
    println!("Found: {} entries", entries.len());

    let mut previous = u32::MIN;

    for (index, x) in entries.iter().enumerate() {
        if index != *x as usize {
            println!("index is {index}, x is {x}");
        }
        // First index should be zero
        if index == 0 {
            if *x != 0u32 {
                println!("First index was not zero, instead got: {x}");
            }
            previous = *x;
        } else if previous > *x {
            println!("Previous entry larger than current. Index is: {index}");
            return;
        } else if *x == 0u32 {
            println!("Current entry is zero? Index is: {index}");
            return;
        } else {
            previous = *x;
        }
    }
}

pub fn buffer_to_sparse_triplet(
    buffer: &Buffer,
    device: &wgpu::Device,
    rays_cast: u32,
) -> Vec<u32> {
    let raw_bytes = sample_buffer(buffer, device);
    let entries: Vec<u32> = raw_bytes[0..(rays_cast * 4) as usize]
        .chunks(4)
        .map(|x| u32::from_ne_bytes(x[0..4].try_into().unwrap()))
        .collect();
    entries
}

pub fn image_to_matrix(image: &DynamicImage) -> Mat<f32> {
    let rows = image.height() as usize;
    let column = image.width() as usize;
    let image = image.grayscale();

    Mat::from_fn(rows, column, |x, y| {
        // Pixel is in RGBA
        let pixel = image.get_pixel(y as u32, x as u32).0;
        // Transform to floating point
        let pixel = pixel.map(|pixel| pixel as f32 / 255.0);
        for x in pixel.iter() {
            assert!(*x <= 1.0, "Pixel value is {x}");
        }

        pixel[0] * 0.299 + 0.587 * pixel[1] + 0.114 * pixel[2]
    })
}

pub fn matrix_to_image(mat: &Mat<f32, usize, usize>) -> DynamicImage {
    let (height, width) = mat.shape();
    let image_buffer = ImageBuffer::from_par_fn(width as u32, height as u32, |x, y| {
        let value = mat[(y as usize, x as usize)];

        assert!(value <= 1.0, "Pixel value is {x}");

        image::Rgba::<u8>([
            (value * 255.0) as u8,
            (value * 255.0) as u8,
            (value * 255.0) as u8,
            (255.0) as u8,
        ])
    });
    DynamicImage::ImageRgba8(image_buffer)
}

pub fn vector_to_image(mat: &Mat<f32, usize, usize>, height: u32, width: u32) -> DynamicImage {
    assert!(mat.shape().1 <= 1, "This vector has more than 1 Column?");
    let image_buffer = ImageBuffer::from_par_fn(width, height, |x, y| {
        // Assuming the image is
        let coordinate = x + (y * height);

        let value = mat[(coordinate as usize, 0)];

        image::Rgba::<u8>([
            (value * 255.0) as u8,
            (value * 255.0) as u8,
            (value * 255.0) as u8,
            (255.0) as u8,
        ])
    });

    DynamicImage::ImageRgba8(image_buffer)
}

pub fn verify_matrix(mat: &Mat<f32>) {
    for col in mat.col_iter() {
        for entry in col.iter() {
            assert!(
                *entry <= 1.0,
                "Entry in this matrix is too high, entry: {entry}"
            );
        }
    }
}
