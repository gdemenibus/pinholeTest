use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use light_field_test::app::AppState;
use std::path::PathBuf;
fn bench_transport(c: &mut Criterion) {
    let app = light_field_test::app::App::new(true);
    let sizes = [4000];
    //let sizes = [500];
    // Grab a target image from curated
    let mut state = app.state.unwrap();
    let samples = 10;
    benchmark_transfer(c, &mut state, &sizes, samples);
    benchmark_solving(c, &mut state, &sizes, samples);
    let sizes = [500, 1000, 2000];
    let samples = 200;
    benchmark_transfer(c, &mut state, &sizes, samples);
    benchmark_solving(c, &mut state, &sizes, samples);
}
pub fn benchmark_transfer(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
) {
    state.camera_history.reset();
    {
        state.camera_history.save_point();

        let mut group = c.benchmark_group("1 View point Curated");
        group.sample_size(sample_size);
        for size in sizes.iter() {
            group.throughput(Throughput::Elements(*size ^ 2));
            // Load image
            let path = PathBuf::from(format!("./resources/textures/Curated/{size}.png"));
            let new_image = image::open(path).unwrap();
            state.update_target(new_image);
            state.compute_pass();

            // DO a compute pass
            group.bench_with_input(BenchmarkId::new("Sep", size), size, |b, &_size| {
                b.iter(|| state.sample_sep());
            });

            group.bench_with_input(BenchmarkId::new("Stereo", size), size, |b, &_size| {
                b.iter(|| state.sample_stereo());
            });
        }

        group.finish();
    }
    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();

        let mut group = c.benchmark_group("Kernel Curated");

        group.sample_size(sample_size);
        for size in sizes.iter() {
            group.throughput(Throughput::Elements(*size ^ 2));
            // Load image
            let path = PathBuf::from(format!("./resources/textures/Curated/{size}.png"));
            let new_image = image::open(path).unwrap();
            state.update_target(new_image);
            state.compute_pass();

            // DO a compute pass
            group.bench_with_input(BenchmarkId::new("KernelSep", size), size, |b, &_size| {
                b.iter(|| state.sample_sep());
            });

            group.bench_with_input(BenchmarkId::new("KernelStereo", size), size, |b, &_size| {
                b.iter(|| state.sample_stereo());
            });
        }

        group.finish();
    }

    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();
        state.camera_history.benchmove();
        state.camera_history.save_point();

        let mut group = c.benchmark_group("2 Kernels Curated");

        group.sample_size(sample_size);
        for size in sizes.iter() {
            group.throughput(Throughput::Elements(*size ^ 2));
            // Load image
            let path = PathBuf::from(format!("./resources/textures/Curated/{size}.png"));
            let new_image = image::open(path).unwrap();
            state.update_target(new_image);
            state.compute_pass();

            // DO a compute pass
            group.bench_with_input(BenchmarkId::new("KernelSep", size), size, |b, &_size| {
                b.iter(|| state.sample_sep());
            });

            group.bench_with_input(BenchmarkId::new("KernelStereo", size), size, |b, &_size| {
                b.iter(|| state.sample_stereo());
            });
        }

        group.finish();
    }
}

pub fn benchmark_solving(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
) {
    state.camera_history.reset();
    {
        state.camera_history.save_point();

        let mut group = c.benchmark_group("1 View point Factorize");

        group.sample_size(sample_size);
        for size in sizes.iter() {
            group.throughput(Throughput::Elements(*size ^ 2));
            // Load image
            let path = PathBuf::from(format!("./resources/textures/Curated/{size}.png"));
            let new_image = image::open(path).unwrap();
            state.update_target(new_image);
            state.compute_pass();
            state.sample_sep();
            state.sample_stereo();

            // DO a compute pass
            group.bench_with_input(BenchmarkId::new("Sep", size), size, |b, &_size| {
                b.iter(|| state.factorizer.alternative_factorization());
            });

            group.bench_with_input(BenchmarkId::new("Stereo", size), size, |b, &_size| {
                b.iter(|| state.stereoscope.factorize_stereo());
            });
        }

        group.finish();
    }

    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();

        let mut group = c.benchmark_group("1 Kernel Factorize");
        group.sample_size(sample_size);

        for size in sizes.iter() {
            group.throughput(Throughput::Elements(*size ^ 2));
            // Load image
            let path = PathBuf::from(format!("./resources/textures/Curated/{size}.png"));
            let new_image = image::open(path).unwrap();
            state.update_target(new_image);
            state.compute_pass();
            state.sample_sep();
            state.sample_stereo();

            // DO a compute pass
            group.bench_with_input(BenchmarkId::new("Sep", size), size, |b, &_size| {
                b.iter(|| state.factorizer.alternative_factorization());
            });

            group.bench_with_input(BenchmarkId::new("Stereo", size), size, |b, &_size| {
                b.iter(|| state.stereoscope.factorize_stereo());
            });
        }

        group.finish();
    }

    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();
        state.camera_history.benchmove();
        state.camera_history.save_point();

        let mut group = c.benchmark_group("2 Kernel Factorize");

        group.sample_size(sample_size);
        for size in sizes.iter() {
            group.throughput(Throughput::Elements(*size ^ 2));
            // Load image
            let path = PathBuf::from(format!("./resources/textures/Curated/{size}.png"));
            let new_image = image::open(path).unwrap();
            state.update_target(new_image);
            state.compute_pass();
            state.sample_sep();
            state.sample_stereo();

            // DO a compute pass
            group.bench_with_input(BenchmarkId::new("Sep", size), size, |b, _| {
                b.iter(|| state.factorizer.alternative_factorization());
            });

            group.bench_with_input(BenchmarkId::new("Stereo", size), size, |b, _| {
                b.iter(|| state.stereoscope.factorize_stereo());
            });
        }

        group.finish();
    }
}

criterion_group!(benches, bench_transport);
criterion_main!(benches);
