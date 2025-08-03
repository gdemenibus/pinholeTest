use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use light_field_test::app::AppState;
use std::{path::PathBuf, time::Duration};
fn bench_transport(c: &mut Criterion) {
    let app = light_field_test::app::App::new(true);
    let sizes = [256, 500, 1000, 2000];
    let panel_sizes = [250, 500, 1000, 2000];
    //let sizes = [500];
    // Grab a target image from curated
    let mut state = app.state.unwrap();
    let samples = 100;
    for panel_size in panel_sizes {
        state.scene.change_panel_res(panel_size);

        benchmark_transfer_1vp(c, &mut state, &sizes, samples, panel_size);
        benchmark_transfer_1kernel(c, &mut state, &sizes, samples, panel_size);
        benchmark_transfer_2kernel(c, &mut state, &sizes, samples, panel_size);

        benchmark_solving_1vp(c, &mut state, &sizes, samples, panel_size);
        benchmark_solving_1kernel(c, &mut state, &sizes, samples, panel_size);
        benchmark_solving_2kernel(c, &mut state, &sizes, samples, panel_size);
    }
}

pub fn benchmark_transfer_1kernel(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    panel_size: usize,
) {
    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();
        let group_name = format!("Kernel Transfer, Panel: {panel_size}");

        let mut group = c.benchmark_group(group_name);
        group.warm_up_time(Duration::from_secs(30));

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
}

pub fn benchmark_transfer_2kernel(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],

    sample_size: usize,
    panel_size: usize,
) {
    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();
        state.camera_history.benchmove();
        state.camera_history.save_point();

        let group_name = format!(" 2 Kernel Transfer, Panel: {panel_size}");

        let mut group = c.benchmark_group(group_name);

        group.warm_up_time(Duration::from_secs(30));
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
}

pub fn benchmark_transfer_1vp(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    panel_size: usize,
) {
    state.camera_history.reset();
    {
        state.camera_history.save_point();

        let group_name = format!("1VP transfer, Panel: {panel_size}");

        let mut group = c.benchmark_group(group_name);

        group.warm_up_time(Duration::from_secs(30));
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
}

pub fn benchmark_solving_1kernel(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],

    sample_size: usize,
    panel_size: usize,
) {
    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();

        let group_name = format!("1 Kernel Factorize, Panel: {panel_size}");

        let mut group = c.benchmark_group(group_name);
        group.sample_size(sample_size);

        group.warm_up_time(Duration::from_secs(30));
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
}

pub fn benchmark_solving_2kernel(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    panel_size: usize,
) {
    state.camera_history.reset();
    state.camera_history.kernel = true;
    state.camera_history.save_point();
    state.camera_history.benchmove();
    state.camera_history.save_point();

    let group_name = format!("2 Kernel Factorize, Panel: {panel_size}");

    let mut group = c.benchmark_group(group_name);

    group.sample_size(sample_size);
    group.warm_up_time(Duration::from_secs(30));
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
pub fn benchmark_solving_1vp(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],

    sample_size: usize,
    panel_size: usize,
) {
    state.camera_history.reset();
    {
        state.camera_history.save_point();

        let group_name = format!("1VP Factorize, Panel: {panel_size}");

        let mut group = c.benchmark_group(group_name);

        group.warm_up_time(Duration::from_secs(30));
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
}

criterion_group!(benches, bench_transport);
criterion_main!(benches);
