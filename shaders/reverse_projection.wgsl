@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;


@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let screen_pos: vec2<i32> = vec2<i32>(i32(GlobalInvocationID.x), i32(GlobalInvocationID.y));
    let color = vec4f(f32(screen_pos.x) / 256.0, 0.0, f32(screen_pos.y) / 256.0, 1.0);
    textureStore(color_buffer, screen_pos, color);
}
