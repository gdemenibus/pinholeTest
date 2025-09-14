struct Matrix {
    data: array<f32>,
}

@group(0) @binding(0) var<storage, read> Numerator: Matrix; //  All of same size
@group(0) @binding(1) var<storage, read> Denominator: Matrix; //
@group(0) @binding(2) var<storage, read_write> Output: Matrix; //

@group(0) @binding(3) var<uniform> dim: vec3<u32>; // m, n, k?

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let total_matrix_size = dim.x * dim.y;
    let work_per_thread = u32(ceil(f32(total_matrix_size) / 64.0));
    let idx = global_id.x;
    let eps = 1e-8;
    for (var i = u32(idx * work_per_thread); i < (idx + 1) * work_per_thread && i < total_matrix_size; i++) {
        Output.data[i] = Output.data[i] * Numerator.data[i] / (Denominator.data[i] + eps);
    }
}
