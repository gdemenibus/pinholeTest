struct VertexInput {
    @location(0) position: vec3<f32>,
}
;

struct Quad {
    a: vec3<f32>,
    b: vec3<f32>,
    c: vec3<f32>,
    d: vec3<f32>,
}
;

@group(0) @binding(0)
var<uniform> projection_quad: Quad;

@group(1) @binding(0)
var<uniform> camera_pos: vec3f;

@group(1) @binding(1)
var<uniform> d: f32;

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
    //  ALIGN IT ALL HERE
    // Write this all to one texture!

    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { }
