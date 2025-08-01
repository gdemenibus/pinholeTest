use image::DynamicImage;
use light_field_test::*;

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
// Initialize a matrix for benchmarking.
// Mockup of your three methods:

fn bench_methods(c: &mut Criterion) {
    let settings = LFSettings {
        debug_prints: false,
        ..Default::default()
    };

    let stereo = StereoMatrix::load("Kernel.ro".to_string());
    println!("Initialized Stereo Matrices");

    let mut diagonal = LFMatrices::load("Kernel.ro".to_string());

    diagonal.c_t = DynamicImage::new_rgb8(diagonal.target_size.0, diagonal.target_size.1);
    let stacked_matrices = diagonal.stack();

    println!("Initialized Separable Matrices");

    // c.bench_function("Separable Old Approach", |b| {
    //     b.iter(|| diagonal.old_factorize(black_box(&settings), black_box(&stacked_matrices)))
    // });
    c.bench_function("Separable Approach", |b| {
        b.iter(|| diagonal.factorize(black_box(&settings)))
    });

    c.bench_function("Stereo Approach", |b| {
        b.iter(|| stereo.factorize(black_box(&settings)))
    });
}

criterion_group!(benches, bench_methods);
criterion_main!(benches);
