use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use light_field_test::app::AppState;
use std::{collections::HashSet, path::PathBuf, time::Duration};

#[derive(Eq, Hash, PartialEq)]
pub enum Bench {
    Sep,
    Stereo,
    SepOld,
}
type BenchSelection = HashSet<Bench>;

fn bench_transport(c: &mut Criterion) {
    let app = light_field_test::app::App::new(true);
    let sizes = [256, 500, 1000, 2000];
    let panel_sizes = [250, 500, 1000, 2000];
    //let sizes = [500];
    // Grab a target image from curated
    let mut state = app.state.unwrap();
    let samples = 100;
    let mut selection = BenchSelection::new();
    selection.insert(Bench::SepOld);
    for panel_size in panel_sizes {
        state.scene.change_panel_res(panel_size);

        benchmark_transfer_1vp(c, &mut state, &sizes, samples, panel_size, &selection);
        benchmark_transfer_1kernel(c, &mut state, &sizes, samples, panel_size, &selection);
        benchmark_transfer_2kernel(c, &mut state, &sizes, samples, panel_size, &selection);

        benchmark_solving_1vp(c, &mut state, &sizes, samples, panel_size, &selection);
        benchmark_solving_1kernel(c, &mut state, &sizes, samples, panel_size, &selection);
        benchmark_solving_2kernel(c, &mut state, &sizes, samples, panel_size, &selection);
    }
}

pub fn benchmark_transfer_1kernel(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    panel_size: usize,
    selection: &BenchSelection,
) {
    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();
        let group_name = format!("Kernel Transfer, Panel: {panel_size}");

        run_transfer_bench(group_name, c, state, sizes, sample_size, selection);
    }
}

pub fn benchmark_transfer_2kernel(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],

    sample_size: usize,
    panel_size: usize,

    selection: &BenchSelection,
) {
    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();
        state.camera_history.benchmove();
        state.camera_history.save_point();

        let group_name = format!(" 2 Kernel Transfer, Panel: {panel_size}");
        run_transfer_bench(group_name, c, state, sizes, sample_size, selection);
    }
}

pub fn benchmark_transfer_1vp(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    panel_size: usize,

    selection: &BenchSelection,
) {
    state.camera_history.reset();
    {
        state.camera_history.save_point();

        let group_name = format!("1VP transfer, Panel: {panel_size}");
        run_transfer_bench(group_name, c, state, sizes, sample_size, selection);
    }
}

pub fn benchmark_solving_1kernel(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],

    sample_size: usize,
    panel_size: usize,
    selection: &BenchSelection,
) {
    {
        state.camera_history.reset();
        state.camera_history.kernel = true;
        state.camera_history.save_point();

        let group_name = format!("1 Kernel Factorize, Panel: {panel_size}");
        run_fact_bench(group_name, c, state, sizes, sample_size, selection);
    }
}

pub fn benchmark_solving_2kernel(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    panel_size: usize,

    selection: &BenchSelection,
) {
    state.camera_history.reset();
    state.camera_history.kernel = true;
    state.camera_history.save_point();
    state.camera_history.benchmove();
    state.camera_history.save_point();

    let group_name = format!("2 Kernel Factorize, Panel: {panel_size}");

    run_fact_bench(group_name, c, state, sizes, sample_size, selection);
}
pub fn benchmark_solving_1vp(
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    panel_size: usize,
    selection: &BenchSelection,
) {
    state.camera_history.reset();
    {
        state.camera_history.save_point();

        let group_name = format!("1VP Factorize, Panel: {panel_size}");
        run_fact_bench(group_name, c, state, sizes, sample_size, selection);
    }
}

fn run_fact_bench(
    group_name: String,
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    selection: &BenchSelection,
) {
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

        if selection.contains(&Bench::Sep) {
            group.bench_with_input(BenchmarkId::new("Sep", size), size, |b, &_size| {
                b.iter(|| state.factorizer.alternative_factorization());
            });
        }
        if selection.contains(&Bench::Stereo) {
            group.bench_with_input(BenchmarkId::new("Stereo", size), size, |b, &_size| {
                b.iter(|| state.stereoscope.factorize_stereo());
            });
        }
        if selection.contains(&Bench::SepOld) {
            group.bench_with_input(BenchmarkId::new("MatrixSep", size), size, |b, &_size| {
                b.iter(|| state.factorizer.old_factorization());
            });
        }
        // DO a compute pass
    }

    group.finish();
}
fn run_transfer_bench(
    group_name: String,
    c: &mut Criterion,
    state: &mut AppState,
    sizes: &[u64],
    sample_size: usize,
    selection: &BenchSelection,
) {
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
        if selection.contains(&Bench::Sep) {
            group.bench_with_input(BenchmarkId::new("Sep", size), size, |b, &_size| {
                b.iter(|| state.sample_sep());
            });
        }
        if selection.contains(&Bench::Stereo) {
            group.bench_with_input(BenchmarkId::new("Stereo", size), size, |b, &_size| {
                b.iter(|| state.sample_stereo());
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_transport);
criterion_main!(benches);
