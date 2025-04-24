struct Matrix {
    data: array<f32>,
}

// C = A^T B
@group(0) @binding(0) var<storage, read> A: Matrix;
@group(0) @binding(1) var<storage, read> B: Matrix;
@group(0) @binding(2) var<storage, read_write> C: Matrix;

@group(0) @binding(3) var<uniform> dim: vec3<u32>; // m, n, k

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let row = global_id.x;
    let col = global_id.y;
    let m = dim.x;
    let n = dim.y;
    let k = dim.z;

    if (row >= m || col >= n) {
    } else {

        var sum: f32 = 0.0;
        for (var i = 0u; i < k; i++) {
            // A is transposed
            let aVal = A.data[k * i + row];
            let bVal = B.data[i * n + col];
            sum += aVal * bVal;
        }

        C.data[row * n + col] = sum;
    }

}
