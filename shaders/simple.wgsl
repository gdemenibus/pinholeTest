// Vertex shader
//
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec2<f32>,
};

struct VertexOutput {
    // Like gl_position
    // Gives us the pixel that we are drawing for
    // y = 0 is the top of the screen
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}
;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // f32() is a cast
    // var is mutable but needs to have type clarified
    // let is immutable but has inferred type
    out.color = vec3<f32>(model.color,1.0);
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
