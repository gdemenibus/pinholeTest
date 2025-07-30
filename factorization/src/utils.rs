use std::{collections::HashSet, iter::zip};

use faer::{
    Mat, MatRef, RowRef,
    sparse::{SparseColMat, Triplet},
};
use image::{DynamicImage, GenericImageView, ImageBuffer};

use crate::{CompleteMapping, MappingMatrix};

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

pub fn build_tripltes(
    buffer: Vec<u32>,
    rays_per_view_point: usize,
) -> Vec<Vec<Triplet<u32, u32, f32>>> {
    let triplets = buffer
        .chunks(rays_per_view_point)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    //let real_entry = ()
                    Triplet::new(index as u32, *entry, 1.0)
                })
                .collect()
        })
        .collect();
    //check_triplets(rows, columns, &mut triplets);
    triplets
}

pub fn filter_zeroes(mat: &mut Mat<f32, usize, usize>, mapping_mat: &CompleteMapping) {
    let x_filter = filter_combine(&mapping_mat.x);
    let y_filter = filter_combine(&mapping_mat.y);
    let filters: Vec<(HashSet<usize>, HashSet<usize>)> = zip(x_filter, y_filter).collect();

    for (row, x) in mat.row_iter_mut().enumerate() {
        for (column, y) in x.iter_mut().enumerate() {
            if !filters
                .iter()
                .any(|set| set.1.contains(&row) && set.0.contains(&column))
            {
                *y = 1.0;
            }
        }
    }
}
fn filter_combine(mapping: &MappingMatrix) -> Vec<HashSet<usize>> {
    mapping.matrix.iter().map(active_columns).collect()
}
fn active_columns(mat: &SparseColMat<u32, f32>) -> HashSet<usize> {
    let pntr = mat.col_ptr();
    let mut set = HashSet::new();
    for column in 0..mat.ncols() {
        if pntr[column + 1] != pntr[column] {
            set.insert(column);
        }
    }
    set
}

fn filter_helper(
    mat: &mut Mat<f32, usize, usize>,
    mat_x: &SparseColMat<u32, f32>,
    mat_y: &SparseColMat<u32, f32>,
) {
    let x_ncols = mat_x.col_ptr();
    let y_ncols = mat_y.col_ptr();
    let rows = mat.nrows();
    let columns = mat.ncols();

    // Column Check
    for column in 0..columns {
        if x_ncols[column + 1] == x_ncols[column] {
            let mut test = mat.as_mut().col_mut(column);
            test.fill(1.0);
        }
    }

    for row in 0..rows {
        if y_ncols[row + 1] == y_ncols[row] {
            let mut test = mat.as_mut().row_mut(row);

            test.fill(1.0);
        }
    }
}
fn filter_brute(
    mat: &mut Mat<f32, usize, usize>,
    mat_x: &SparseColMat<u32, f32>,
    mat_y: &SparseColMat<u32, f32>,
) {
    let x_ncols = mat_x.col_ptr();
    let y_ncols = mat_y.col_ptr();
    for (row, x) in mat.row_iter_mut().enumerate() {
        for (column, y) in x.iter_mut().enumerate() {
            if x_ncols[column + 1] == x_ncols[column] || y_ncols[row + 1] == y_ncols[row] {
                *y = 1.0;
            }
        }
    }
}

#[cfg(test)]
mod test {

    use faer::*;

    use super::*;

    #[test]
    fn image_around_the_world() {
        let mut image = image::open("./resources/textures/Gibbon.jpg").unwrap();
        image = image.grayscale();
        let matrix = image_to_matrix(&image);
        let new_image = matrix_to_image(&matrix);
        let new_matrix = image_to_matrix(&new_image);
        // Write both into
        image.save("./resources/test/OG.png").unwrap();
        new_image.save("./resources/test/NEW.png").unwrap();
        for (og, new) in std::iter::zip(image.pixels(), new_image.pixels()) {
            assert_eq!(og, new);
        }

        //assert_eq!(image, new_image);
        assert_eq!(new_matrix, matrix);
    }
}
