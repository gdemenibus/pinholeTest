const TILE_SIZE: u32 = 4;

// C = A * B
@group(0) @binding(0) var<storage, read> A: array<f32>;
@group(0) @binding(1) var<storage, read> B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;
@group(0) @binding(3) var<uniform> dims: vec3<u32>; // M, N, K

var<workgroup> tileA: array<array<f32, TILE_SIZE>, TILE_SIZE>;
var<workgroup> tileB: array<array<f32, TILE_SIZE>, TILE_SIZE>;

// Possible way to extend this:
// Add padding to get to arbitray sizes
//https://cnugteren.github.io/tutorial/pages/page12.html
@compute @workgroup_size(TILE_SIZE, TILE_SIZE)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>, @builtin(workgroup_id) group_id: vec3<u32>) {

    let M = dims.x;
    let N = dims.y;
    let K = dims.z;

    let row = local_id.x;
    let col = local_id.y;

    let globalRow = TILE_SIZE * group_id.x + row;
    let globalCol = TILE_SIZE * group_id.y + col;

    var sum: f32 = 0.0;

    let numTiles = K / TILE_SIZE;

    for (var i = 0u; i < numTiles; i++) {
        // Load Tiles into local memory
        let tiledRow = TILE_SIZE * i + row;
        let tiledCol = TILE_SIZE * i + col;

        tileA[col][row] = A[tiledCol * M + globalRow];
        tileB[col][row] = B[globalCol * K + tiledRow];

        workgroupBarrier();
        for (var bar = 0u; bar < TILE_SIZE; bar++) {
            sum += tileA[bar][row] * tileB[col][bar];
        }
        workgroupBarrier();
    }
    C[globalCol * M + globalRow] = sum;

}
