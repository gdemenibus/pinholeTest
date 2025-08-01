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

const eps = 0.00001;
// Scene group
@group(0) @binding(0)
var<uniform> scene: Target;
@group(0) @binding(1)
var<uniform> rt: RayTraceInfo;

@group(0) @binding(2)
var<uniform> background_color: vec4<f32>;
// Panel Group

// Texture group
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(1) @binding(2)
var<uniform> tex_size: vec2<u32>;

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
// If we need more than 10, make this a x, y, z problem instead?
@group(5) @binding(0) var<storage, read> camera_positions: array<vec3<f32>>;
@group(5) @binding(1) var<uniform> camera_count: u32;

@group(6) @binding(0)
var<storage, read_write> a_buffer: array<u32>;
@group(6) @binding(1)
var<storage, read_write> b_buffer: array<u32>;
@group(6) @binding(2)
var<storage, read_write> l_buffer: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {

    var screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.x, 0);
    let camera_index = GlobalInvocationID.y;

    if GlobalInvocationID.z == 1 {
        screen_pos = vec2<u32>(0, GlobalInvocationID.x);
    }

    if screen_pos.x < scene.pixel_count.x && screen_pos.y < scene.pixel_count.y {
        //if true {
        let pixel_location = pixel_to_world_location(scene, screen_pos);
        let origin = pixel_location;
        let observer = camera_positions[camera_index];

        let direction = normalize(observer - pixel_location);

        // Build the ray
        let ray = Ray(origin, direction);
        double_intersection(ray, screen_pos, camera_index);

        let current_pixel_float = vec2f(f32(screen_pos.x) / f32(tex_size.x), f32(screen_pos.y) / f32(tex_size.y));

        //let color = textureSampleLevel(t_diffuse, s_diffuse, current_pixel_float, 0.0);
        let color = vec4f(current_pixel_float.x, current_pixel_float.y, 0.0, 1.0);

        textureStore(color_buffer, screen_pos, color);

    } else {
        let red = vec4f(1.0, 0.0, 0.0, 1.0);

        textureStore(color_buffer, screen_pos, red);

    }
// ensure everyone has written??

}

fn double_intersection(ray: Ray, current_pixel: vec2<u32>, observer_index: u32) {

    let ray_index_x = current_pixel.x + (scene.pixel_count.y * observer_index);
    let ray_index_y = current_pixel.y + (scene.pixel_count.x * observer_index);
    let ray_index = vec2u(ray_index_x, ray_index_y);

    record_hit_T(ray_index, current_pixel);
    intersection_panel_0(ray, ray_index);
    intersection_panel_1(ray, ray_index);
}
fn intersection_panel_0(
    ray: Ray,
    ray_index: vec2<u32>,
) {

    var trig_1_intersection = intersection_panel(ray, true, panels[0]);
    var trig_2_intersection = intersection_panel(ray, false, panels[0]);

    let pixel_count = panels[0].pixel_count;

    if trig_1_intersection.hit {
        let pixel_coords = trig_1_intersection.pixel_coords;

        record_hit_A(ray_index, pixel_coords);

    }

    if trig_2_intersection.hit {

        let pixel_coords = trig_2_intersection.pixel_coords;

        record_hit_A(ray_index, pixel_coords);

    }

}

fn intersection_panel_1(
    ray: Ray,
    ray_index: vec2<u32>,
) {

    var trig_1_intersection = intersection_panel(ray, true, panels[1]);
    var trig_2_intersection = intersection_panel(ray, false, panels[1]);

    let pixel_count = panels[1].pixel_count;

    if trig_1_intersection.hit {
        let pixel_coords = trig_1_intersection.pixel_coords;

        record_hit_B(ray_index, pixel_coords);

    }

    if trig_2_intersection.hit {

        let pixel_coords = trig_2_intersection.pixel_coords;

        record_hit_B(ray_index, pixel_coords);

    }

}


fn draw_red(screen_pos: vec2<u32>) {

    let red = vec4f(1.0, 0.0, 0.0, 1.0);

    textureStore(color_buffer, screen_pos, red);
}

fn record_hit_A(
    ray_index: vec2<u32>,
    a_coords: vec2<u32>,
) {

    m_a_x_buffer[ray_index.x] = a_coords.x;
    m_a_y_buffer[ray_index.y] = a_coords.y;

}

fn record_hit_B(
    ray_index: vec2<u32>,
    b_coords: vec2<u32>,
) {

    m_b_x_buffer[ray_index.x] = b_coords.x;
    m_b_y_buffer[ray_index.y] = b_coords.y;

}
fn record_hit_T(
    ray_index: vec2<u32>,
    t_coords: vec2<u32>,
) {

    m_t_x_buffer[ray_index.x] = t_coords.x;

    m_t_y_buffer[ray_index.y] = t_coords.y;

}


struct PanelIntersection {
    hit: bool,
    border: bool,
    pixel_coords: vec2u,
}
;
fn intersection_panel(ray: Ray, abc: bool, panel: Panel) -> PanelIntersection {
    var hit = false;
    var border = false;
    var pixel_coords = vec2u(0, 0);
    var a = panel.quad.a;
    var b = panel.quad.b;
    var c = panel.quad.c;
    if !abc {
        a = panel.quad.b;
        b = panel.quad.c;
        c = panel.quad.d;
    }

    var e1 = b - a;
    var e2 = c - a;
    var rey_cross_e2 = cross(e2,(ray).direction);
    var det = dot(rey_cross_e2, e1);

    if det > -eps && det < eps {
        return PanelIntersection(hit, border, pixel_coords);
    }
    var inv_det = 1.0 / det;
    var s = (ray).origin - a;
    var u = inv_det * dot(rey_cross_e2, s);

    if (u < 0.0 && abs(u) > eps) || (u > 1.0 && abs(u - 1.0) > eps) {
        return PanelIntersection(hit, border, pixel_coords);
    }
    var s_cross_e1 = cross(e1, s);
    var v = inv_det * dot(s_cross_e1,(ray).direction);
    var w = 1.0 - v - u;

    if v < 0.0 || u + v > 1.0 {
        return PanelIntersection(hit, border, pixel_coords);
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    var t = inv_det * dot(s_cross_e1, e2);

    if t > eps {
        hit = true;
        let bary_coords = vec3f(u, v, w);

        // Panel definition
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
            let pixels_panel = pixel_hit(bary_coords, tex_coords, panel);
            let pixels = pixels_panel.pixel;

            if pixels.x == 0 || pixels.x == panel.pixel_count.x - 1 || pixels.y == 0 || pixels.y == panel.pixel_count.y - 1 {
                border = true;
                return PanelIntersection(hit, border, pixels);
            } else {
                return PanelIntersection(hit, border, pixels);
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

            let pixels_panel = pixel_hit(bary_coords, tex_coords, panel);
            let pixels = pixels_panel.pixel;
            if pixels.x == 0 || pixels.x == panel.pixel_count.x - 1 || pixels.y == 0 || pixels.y == panel.pixel_count.y - 1 {

                border = true;
                return PanelIntersection(hit, border, pixels);
            } else {

                return PanelIntersection(hit, border, pixels);
            }
        }

    //return vec4f(0.0, 0.5, 0.5, 1.0);
    }

    return PanelIntersection(hit, border, pixel_coords);
}


struct Pixel_Panel {
    pixel: vec2u,

}
;
fn pixel_hit(bary_coords: vec3f, relative_tex_coords: array<vec2f, 3>, panel: Panel) -> Pixel_Panel {

    // Relative Coordinates
    let x_coord = (bary_coords.x * relative_tex_coords[0].x + bary_coords.y * relative_tex_coords[1].x + bary_coords.z * relative_tex_coords[2].x);
    let y_coord = (bary_coords.x * relative_tex_coords[0].y + bary_coords.y * relative_tex_coords[1].y + bary_coords.z * relative_tex_coords[2].y);
    // From Relative coordinates to pixel
    // Casting
    // Cast pixel count into a f32 to multiply, then into u32 to round
    let x_pixel = u32(x_coord * f32(panel.pixel_count.x));
    let y_pixel = u32(y_coord * f32(panel.pixel_count.y));

    // Messy code, need to write this out on paper?
    let pixel = vec2u(x_pixel, y_pixel);
    return Pixel_Panel(pixel);
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + (t * (b - a));

}

fn LerpPoint(a: vec3f, b: vec3f, t: f32) -> vec3f {
    return vec3f(lerp(a.x, b.x, t), lerp(a.y, b.y, t), lerp(a.z, b.z, t));

}

fn pixel_to_world_location(source: Target, pixel_pos: vec2<u32>) -> vec3<f32> {

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
