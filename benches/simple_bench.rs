use image::DynamicImage;
use light_field_test::*;

use criterion::{criterion_group, criterion_main, Criterion};
use std::{hint::black_box, num::NonZero};
// Initialize a matrix for benchmarking.
// Mockup of your three methods:

fn bench_methods(c: &mut Criterion) {
    let settings = LFSettings {
        debug_prints: false,
        ..Default::default()
    };

    let stereo = StereoMatrix::load("Default.ro".to_string());
    println!("Initialized Stereo Matrices");

    let mut diagonal = LFMatrices::load("Default.ro".to_string());

    diagonal.c_t = DynamicImage::new_rgb8(diagonal.target_size.0, diagonal.target_size.1);

    println!("Initialized Separable Matrices");

    c.bench_function("Separable Approach", |b| {
        b.iter(|| diagonal.factorize(black_box(&settings)))
    });
    c.bench_function("Stereo Approach", |b| {
        b.iter(|| stereo.factorize(black_box(&settings)))
    });
}

criterion_group!(benches, bench_methods);
criterion_main!(benches);
