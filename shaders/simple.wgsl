// Vertex shader
//
struct VertexInput {
    @location(0) position: vec3<f32>,
}
;
struct Scene {
    a: vec3<f32>,
    b: vec3<f32>,
    c: vec3<f32>,
    d: vec3<f32>,
}
;

struct RayTraceInfo {
    ray_origin: vec3f,
    q_x: vec3f,
    q_y: vec3f,
    p_1_m: vec3f,
}


@group(0) @binding(0)
var<uniform> scene: Scene;
@group(0) @binding(1)
var<uniform> rt: RayTraceInfo;

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
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Clip position tells us the framgent?
    var position: vec2<f32> = in.clip_position.xy;
    var damn = scene.a;
    var size = vec2<f32>(1000, 2000);
    var f_x = position.x;
    var f_y = position.y;
    var ray_dir = rt.p_1_m + rt.q_x * (f_x - 1.0) + rt.q_y * (f_y - 1.0);

    ray_dir = normalize(ray_dir);
    if intersection(ray_dir) {
        return vec4f(1.0, 1.0, 1.0, 1.0);

    } else {

        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

}

fn intersection(ray_direction: vec3f) -> bool {
    // Build the plane
    // Make a the origin
    var b_a = scene.b - scene.a;
    var d_a = scene.d - scene.a;
    var c_a = scene.c - scene.a;
    var ray_origin_a = rt.ray_origin - scene.a;
    // intersect plane:
    var nor = cross(b_a, d_a);
    var t = -dot(ray_origin_a, nor) / dot(ray_direction, nor);
    if (t < 0.0) {
        return false;
    } else {
        return true;
    }

}
