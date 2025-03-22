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

const eps = 0.00001;


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
    if intersection(ray_dir, scene.a, scene.b, scene.c) {
        return vec4f(1.0, 0.0, 1.0, 1.0);

    } else {

        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }

}


fn intersection(ray_direction: vec3f, a: vec3f, b: vec3f, c: vec3f) -> bool {
    var e1 = b - a;
    var e2 = c - a;
    var rey_cross_e2 = cross(e2, ray_direction);
    var det = dot(rey_cross_e2, e1);

    if (det > -eps && det < eps) {
        return false;
    }
    var inv_det = 1.0 / det;
    var s = rt.ray_origin - a;
    var u = inv_det * dot(rey_cross_e2, s);

    if ((u < 0.0 && abs(u) > eps) || (u > 1.0 && abs(u - 1.0) > eps)) {
        return false;
    }
    var s_cross_e1 = cross(e1, s);
    var v = inv_det * dot(s_cross_e1, ray_direction);
    var w = 1.0 - v - u;

    if (v < 0.0 || u + v > 1.0) {
        return false;
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    var t = inv_det * dot(s_cross_e1, e2);
    if (t > eps) {
        return true;
    }
    return false;

}
