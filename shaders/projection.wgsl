struct Quad {
    a: vec3<f32>,
    b: vec3<f32>,
    c: vec3<f32>,
    d: vec3<f32>,
}

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

struct Panel {
    // Coordinates assume
    quad: Quad,
    pixel_count: vec2u,
    size: vec2f,
}

struct Target {
    quad: Quad,
    pixel_count: vec2u,
    size: vec2f,
}

const scene_size: u32 = 1;

const background_color: vec4f = vec4(0.0, 0.0, 0.3, 1.0);
const eps = 0.00001;
// Scene group
@group(0) @binding(0)
var<uniform> scene: Target;
@group(0) @binding(1)
var<uniform> rt: RayTraceInfo;
// Panel Group

// Texture group
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

// Panel group!
@group(2) @binding(0)
var<uniform> panels: array<Panel, 2>;

@group(2) @binding(1)
// SUPER HACKY, can't pass bool. Treat 0 as false, rest as true??
var<uniform> panels_use_texture: u32;
// first entry maps to first texture
@group(2) @binding(2)
var texture_panel: texture_2d_array<f32>;
@group(2) @binding(3)
var sampler_panel: sampler;
// WE ASSUME BOTH ARE THE SAME SIZE, THIS MIGHT BE WRONG?
@group(2) @binding(4)
var<uniform> panel_texture_size: vec2<u32>;
//@group(2) @binding(4)
//var<uniform> panels_sample_world: u32;

// Array to keep recording the interestcions. Each thread in fragmanet shader will write to:
// (x + y * column) * 3
// we need to give every x 3 entries.
// WARNING: This needs to be double tested!
@group(3) @binding(0)
var<storage, read_write> m_a_y_buffer: array<u32>;
@group(3) @binding(1)
var<storage, read_write> m_a_x_buffer: array<u32>;
@group(3) @binding(2)
var<storage, read_write> m_b_y_buffer: array<u32>;
@group(3) @binding(3)
var<storage, read_write> m_b_x_buffer: array<u32>;
@group(3) @binding(4)
var<storage, read_write> m_t_y_buffer: array<u32>;
@group(3) @binding(5)
var<storage, read_write> m_t_x_buffer: array<u32>;

@group(4) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;

// Deals with camera history
// We need to declare a maximum in order to handle this. 10 seems like a correct amount?
@group(5) @binding(0) var<uniform> camera_positions: array<vec3<f32>, 10>;
@group(5) @binding(1) var<uniform> camera_count: u32;


@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.x, GlobalInvocationID.y);
    let panel_projection = panels[1];

    if screen_pos.x < panel_projection.pixel_count.x && screen_pos.y < panel_projection.pixel_count.y {

        let pixel_location = pixel_to_world_location(panel_projection, screen_pos);

        for (var camera_index = 0u; camera_index < camera_count; camera_index++) {
            let observer = camera_positions[camera_index];

            let direction = normalize(pixel_location - observer);

            // Build the ray
            let ray = Ray(observer, direction);

        }

    }

}
fn single_intersection(ray: Ray, current_pixel: vec2<u32>, observer_index: u32) {

    let ray_index_x = current_pixel.x + (panels[1].pixel_count.x * observer_index);
    let ray_index_y = current_pixel.y + (panels[1].pixel_count.y * observer_index);
    let ray_index = vec2u(ray_index_x, ray_index_y);
    var color = background_color;

    var trig_1_intersection = intersection_panel(ray, true, scene);
    var trig_2_intersection = intersection_panel(ray, false, scene);

    let pixel_count = scene.pixel_count;

    if trig_1_intersection.hit {
        let pixel_coords = trig_1_intersection.pixel_coords;
        let hit_relative = vec2f(f32(pixel_coords.x) / f32(pixel_count.x), f32(pixel_coords.y) / f32(pixel_count.y));

        color = vec4f(1.0, 0.0, 1.0, 1.0);

        record_hit_T(ray_index, current_pixel, observer_index);
        record_hit_B(ray_index, trig_1_intersection.pixel_coords, observer_index);

    //color = vec4f(hit_relative, 0.0, 1.0);
    }

    if trig_2_intersection.hit {

        let pixel_coords = trig_2_intersection.pixel_coords;
        let hit_relative = vec2f(f32(pixel_coords.x) / f32(pixel_count.x), f32(pixel_coords.y) / f32(pixel_count.y));

        if index == 0 {
            color = vec4f(1.0, 1.0, 0.0, 1.0);

            record_hit_A(current_pixel, trig_2_intersection.pixel_coords, observer_index);

            record_hit_T(ray_index, current_pixel, observer_index);
        } else {
            color = vec4f(1.0, 0.0, 1.0, 1.0);

            record_hit_T(ray_index, current_pixel, observer_index);
            record_hit_B(current_pixel, trig_2_intersection.pixel_coords, observer_index);
        }

    }

}


struct TargetIntersection {
    hit: bool,
    pixel_coords: vec2u,
}
;
fn intersection_panel(ray: Ray, abc: bool, target: Target) -> TargetIntersection {

    var hit = false;
    var border = false;
    var pixel_coords = vec2u(0, 0);
    var a = target.quad.a;
    var b = target.quad.b;
    var c = target.quad.c;
    if !abc {
        a = target.quad.b;
        b = target.quad.c;
        c = target.quad.d;
    }

    var e1 = b - a;
    var e2 = c - a;
    var rey_cross_e2 = cross(e2,(ray).direction);
    var det = dot(rey_cross_e2, e1);

    if det > -eps && det < eps {
        return TargetIntersection(hit, border, pixel_coords);
    }
    var inv_det = 1.0 / det;
    var s = (ray).origin - a;
    var u = inv_det * dot(rey_cross_e2, s);

    if (u < 0.0 && abs(u) > eps) || (u > 1.0 && abs(u - 1.0) > eps) {
        return TargetIntersection(hit, border, pixel_coords);
    }
    var s_cross_e1 = cross(e1, s);
    var v = inv_det * dot(s_cross_e1,(ray).direction);
    var w = 1.0 - v - u;

    if v < 0.0 || u + v > 1.0 {
        return TargetIntersection(hit, border, pixel_coords);
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    var t = inv_det * dot(s_cross_e1, e2);

    if t > eps {
        hit = true;
        let bary_coords = vec3f(u, v, w);

        // target definition
        // a ==== b
        // |      |
        // |      |
        // c ==== d
        // Two triangles
        // ABC, BCD

        // A, B, C
        if abc {
            //w === u
            //|   /
            //|  /
            //| /
            //v
            let tex_coords = array(vec2f(1.0, 0.0), vec2f(0.0, 1.0), vec2f(0.0, 0.0));

            //
            let pixels_target = pixel_hit(bary_coords, tex_coords, target);
            let pixels = pixels_target.pixel;

            if pixels.x == 0 || pixels.x == target.pixel_count.x - 1 || pixels.y == 0 || pixels.y == target.pixel_count.y - 1 {
                border = true;
                return TargetIntersection(hit, border, pixels);
            } else {
                return TargetIntersection(hit, border, pixels);
            //return vec4f(u, v, w, 1.0);

            //return miss_color;
            }
        } else {
            // B, C, D
            //       w
            //     / |
            //    /  |
            //   /   |
            //  /    |
            // u === v

            let tex_coords = array(vec2f(0.0, 1.0), vec2f(1.0, 1.0), vec2f(1.0, 0.0));

            let pixels_target = pixel_hit(bary_coords, tex_coords, target);
            let pixels = pixels_target.pixel;
            if pixels.x == 0 || pixels.x == target.pixel_count.x - 1 || pixels.y == 0 || pixels.y == target.pixel_count.y - 1 {

                border = true;
                return TargetIntersection(hit, border, pixels);
            } else {

                return TargetIntersection(hit, border, pixels);
            }
        }

    //return vec4f(0.0, 0.5, 0.5, 1.0);
    }

    return TargetIntersection(hit, border, pixel_coords);

}


struct Pixel_Target {
    pixel: vec2u,

}
;
fn pixel_hit(bary_coords: vec3f, relative_tex_coords: array<vec2f, 3>, target: Target) -> Pixel_Target {
    // Relative Coordinates
    let x_coord = (bary_coords.x * relative_tex_coords[0].x + bary_coords.y * relative_tex_coords[1].x + bary_coords.z * relative_tex_coords[2].x);
    let y_coord = (bary_coords.x * relative_tex_coords[0].y + bary_coords.y * relative_tex_coords[1].y + bary_coords.z * relative_tex_coords[2].y);
    // From Relative coordinates to pixel
    // Casting
    // Cast pixel count into a f32 to multiply, then into u32 to round
    let x_pixel = u32(x_coord * f32(target.pixel_count.x));
    let y_pixel = u32(y_coord * f32(target.pixel_count.y));

    // Messy code, need to write this out on paper?
    let pixel = vec2u(x_pixel, y_pixel);
    return Pixel_Target(pixel);
}


fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + (t * (b - a));

}

fn LerpPoint(a: vec3f, b: vec3f, t: f32) -> vec3f {
    return vec3f(lerp(a.x, b.x, t), lerp(a.y, b.y, t), lerp(a.z, b.z, t));

}
fn pixel_to_world_location(source: Panel, pixel_pos: vec2<u32>) -> vec3<f32> {

    let pixel_size = source.size / vec2f(source.pixel_count);

    // Pixel to Pixel Ceneter
    let x_pixel_center = f32(pixel_pos.x) + 0.5;
    let y_pixel_center = f32(pixel_pos.y) + 0.5;
    // Pixel Ceneter to relative coordinates
    let x_relative = x_pixel_center / f32(source.pixel_count.x);
    let y_relative = y_pixel_center / f32(source.pixel_count.y);
    let p = LerpPoint(source.quad.a, source.quad.b, x_relative);
    let q = LerpPoint(source.quad.c, source.quad.d, x_relative);
    return LerpPoint(p, q, y_relative);

// Add A to it to give World coordinates

}
