const TILE_SIZE: u32 = 16;

@group(0) @binding(0) var<storage, read> A: array<f32>;
@group(0) @binding(1) var<storage, read> B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;
@group(0) @binding(3) var<uniform> dims: vec3<u32>; // M, N, K

var<workgroup> tileA: array<array<f32, TILE_SIZE>, TILE_SIZE>;
var<workgroup> tileB: array<array<f32, TILE_SIZE>, TILE_SIZE>;

@compute @workgroup_size(TILE_SIZE, TILE_SIZE)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>, @builtin(workgroup_id) group_id: vec3<u32>) {

    let M = dims.x;
    let N = dims.y;
    let K = dims.z;

    let row = global_id.y;
    let col = global_id.x;

    var sum: f32 = 0.0;

    // Step through tiles of K
    let numTiles = (K + TILE_SIZE - 1u) / TILE_SIZE;
    for (var t = 0u; t < numTiles; t++) {
        // Load tile of A
        let tiledRow = row;
        let tiledCol = t * TILE_SIZE + local_id.x;
        if (tiledRow < M && tiledCol < K) {
            tileA[local_id.y][local_id.x] = A[tiledRow + tiledCol * K];
        } else {
            tileA[local_id.y][local_id.x] = 0.0;
        }

        // Load tile of B
        let tiledRowB = t * TILE_SIZE + local_id.y;
        let tiledColB = col;
        if (tiledRowB < K && tiledColB < N) {
            tileB[local_id.y][local_id.x] = B[tiledRowB * N + tiledColB];
        } else {
            tileB[local_id.y][local_id.x] = 0.0;
        }

        workgroupBarrier();

        // Multiply tile
        for (var k = 0u; k < TILE_SIZE; k++) {
            sum += tileA[local_id.y][k] * tileB[k][local_id.x];
        }

        workgroupBarrier();
    }

    // Write result
    if (row < M && col < N) {
        C[row * N + col] = sum;
    }
}
