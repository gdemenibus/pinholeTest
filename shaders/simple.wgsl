// Vertex shader
//
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

struct RayTraceInfo {
    ray_origin: vec3f,
    q_x: vec3f,
    q_y: vec3f,
    p_1_m: vec3f,
}
struct Ray {
    origin: vec3f,
    direction: vec3f,
}

const eps = 0.00001;
const scene_size: u32 = 2;
const miss_color: vec4f = vec4(0.0, 0.0, 0.0, 0.0);


// Scene group
@group(0) @binding(0)
var<uniform> scene: array<Quad, scene_size>;
@group(0) @binding(1)
var<uniform> rt: RayTraceInfo;
// Panel Group

// Texture group
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(1) @binding(2)
var<uniform> tex_size: vec2<u32>;

// Panel group!
struct Panel {
    quad: Quad,
    pixel_count: vec2u,
    size: vec2f,
}
@group(2) @binding(0)
var<uniform> panels: array<Panel, 1>;

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
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Clip position tells us the fragment location
    var position: vec2<f32> = in.clip_position.xy;
    var damn = scene[0].a;
    var miss_color = miss_color;
    var size = vec2<f32>(1000, 2000);
    var f_x = position.x;
    var f_y = position.y;
    var ray_dir = rt.p_1_m + rt.q_x * (f_x - 1.0) + rt.q_y * (f_y - 1.0);

    var ray = Ray(rt.ray_origin, ray_dir);

    ray_dir = normalize(ray_dir);
    light_field_distortion(&ray);
    // Check if you
    //
    //
    //var panel = intersection(ray_dir, scene[index].a, scene[index].b, scene[index].c, true)
    // Loop through all the geometry in scene! (for now, very simple)
    // Rely on CPU to give us the correct order?
    for (var index = 0u; index < scene_size; index++) {
        var color = intersection(ray, scene[index].a, scene[index].b, scene[index].c, true);
        if (color.w != 0.0) {
            return color;
        }
        color = intersection(ray, scene[index].b, scene[index].c, scene[index].d, false);

        if (color.w != 0.0) {
            return color;
        }

    }

    return vec4f(0.0, 0.0, 0.3, 1.0);

}

// Distortion of Ray caused by limits of the panel
fn light_field_distortion(ray: ptr<function, Ray>) {
// Intersection Panel 1
// Intersection Panel 2
// Build ray
// edit ray
}
;
//
fn pixel_center_real_space(
    bary_coords: vec3f,
    triangle: array<vec3f, 3>,
) {

}
;


fn intersection(ray: Ray, a: vec3f, b: vec3f, c: vec3f, abc: bool) -> vec4f {
    var e1 = b - a;
    var e2 = c - a;
    var rey_cross_e2 = cross(e2, ray.direction);
    var det = dot(rey_cross_e2, e1);

    if (det > -eps && det < eps) {
        return miss_color;
    }
    var inv_det = 1.0 / det;
    var s = ray.origin - a;
    var u = inv_det * dot(rey_cross_e2, s);

    if ((u < 0.0 && abs(u) > eps) || (u > 1.0 && abs(u - 1.0) > eps)) {
        return miss_color;
    }
    var s_cross_e1 = cross(e1, s);
    var v = inv_det * dot(s_cross_e1, ray.direction);
    var w = 1.0 - v - u;

    if (v < 0.0 || u + v > 1.0) {
        return miss_color;
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    var t = inv_det * dot(s_cross_e1, e2);

    if (t > eps) {
        let bary_coords = vec3f(u, v, w);

        // Tex coordinates
        // a -> 0.0, 1.0
        // b -> 1.0, 1.0
        // c = 0.0, 0.0
        // d = 1.0, 0.0
        // a ==== b
        // |      |
        // |      |
        // c ==== d

        // A, B, C
        if abc {

            //
            //       v
            //     / |
            //    /  |
            //   /   |
            //  /    |
            // u === w
            return sample_texture(bary_coords, vec2f(0.0, 0.0), vec2f(1.0, 1.0), vec2f(1.0, 0.0), tex_size);
        } else {
            // B, C, D

            //v === u
            //|   /
            //|  /
            //| /
            //w
            return sample_texture(bary_coords, vec2f(1.0, 1.0), vec2f(0.0, 1.0), vec2f(0.0, 0.0), tex_size);
        }

    //return vec4f(0.0, 0.5, 0.5, 1.0);
    }

    return miss_color;

}

// Texture is upside down for Shader? why?
fn sample_texture(bary_coords: vec3f, tex_coord_0: vec2f, tex_coord1: vec2f, tex_coord2: vec2f, texture_size: vec2u) -> vec4f {
    // sample expects between [0,0] and [1,1]
    let x_coord = (bary_coords.x * tex_coord_0.x + bary_coords.y * tex_coord1.x + bary_coords.z * tex_coord2.x);
    // 0,0 is bottom left, not top left. Flip the Y axis to get the right image
    let y_coord = 1.0 - (bary_coords.x * tex_coord_0.y + bary_coords.y * tex_coord1.y + bary_coords.z * tex_coord2.y);
    let coordinates = vec2f(x_coord, y_coord);

    return textureSample(t_diffuse, s_diffuse, coordinates);

}

fn collapse(in: vec4<bool>) -> bool {
    return in.x || in.y || in.z && in.w;
}
