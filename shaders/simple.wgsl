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

struct Target {
    quad: Quad,
    pixel_count: vec2u,
    size: vec2f,
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

const eps = 0.00001;
const scene_size: u32 = 1;
const miss_color: vec4f = vec4(0.0, 0.0, 0.0, 0.0);
const border_color: vec4f = vec4(1.0, 0.0, 0.0, 1.0);
const background_color: vec4f = vec4(1.0, 1.0, 1.3, 1.0);

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
@group(2) @binding(5)
// SUPER HACKY, can't pass bool. Treat 0 as false, rest as true??
var<uniform> distort_rays: u32;
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
struct VertexOutput {
    // Like gl_position
    // Gives us the pixel that we are drawing for
    // y = 0 is the top of the screen
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,

}
;

//@group(4) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}


struct FragmentOutput {
    @location(0) color0: vec4<f32>,
    @location(1) color1: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {

    // THE OLD FAITHFUL

    // Clip position tells us the fragment location
    let position: vec2<f32> = in.clip_position.xy;

    var f_x = position.x;
    var f_y = position.y;

    var ray_dir = rt.p_1_m + rt.q_x * (f_x - 1.0) + rt.q_y * (f_y - 1.0);

    var ray = Ray(rt.ray_origin, ray_dir);

    ray_dir = normalize(ray_dir);

    // If panels are involved, means we are going to hit something
    //
    if panels_use_texture != 0 {
        let panel_hit = panels_texture_hit(&ray);
        if panel_hit.hit == true {
            if panel_hit.border {
                return FragmentOutput(border_color, border_color);
            }
            return FragmentOutput(
                panel_hit.color * background_color,
                panel_hit.color * background_color,
            );

        }
    }

    // Check if first panel is hit
    // Check if second panel is hit
    // Update if so
    // Return:
    // Is hit (bool)
    // Is border (bool)
    // where (location)
    // Check if border (then black)
    // Else build
    //
    //
    //var panel = intersection(ray_dir, scene[index].a, scene[index].b, scene[index].c, true)
    // Loop through all the geometry in scene! (for now, very simple)
    // Rely on CPU to give us the correct order?
    var hit_first = false;
    var hit_first_location = vec3f(0.0, 0.0, 0.0);
    var coordinates_first_relative_pixel = vec2u(0, 0);
    var hit_second = false;
    var hit_second_location = vec3f(0.0, 0.0, 0.0);
    var coordinates_second_relative_pixel = vec2u(0, 0);

    for (var index = 0u; index < 2; index++) {

        let panel = panels[index];

        let trig_1 = intersection_panel(&ray, panel.quad.a, panel.quad.b, panel.quad.c, true, panel);
        let trig_2 = intersection_panel(&ray, panel.quad.b, panel.quad.c, panel.quad.d, false, panel);

        if trig_1.border {
            clean_up_record(position);
            return FragmentOutput(border_color, border_color);
        }

        if trig_2.border {

            clean_up_record(position);
            return FragmentOutput(border_color, border_color);
        }
    }

    for (var index = 0u; index < 2; index++) {

        let panel = panels[index];

        let trig_1 = intersection_panel(&ray, panel.quad.a, panel.quad.b, panel.quad.c, true, panel);
        let trig_2 = intersection_panel(&ray, panel.quad.b, panel.quad.c, panel.quad.d, false, panel);

        if trig_1.border {
            clean_up_record(position);
            return FragmentOutput(border_color, border_color);
        }

        if trig_2.border {

            clean_up_record(position);
            return FragmentOutput(border_color, border_color);
        }

        if index == 0 {
            hit_first = trig_1.hit || trig_2.hit;
            if trig_1.hit {
                hit_first_location = trig_1.pixel_center_model_space;

                coordinates_first_relative_pixel = trig_1.pixel_coords;
            }
            if trig_2.hit {
                hit_first_location = trig_2.pixel_center_model_space;
                coordinates_first_relative_pixel = trig_2.pixel_coords;
            }
            if hit_first {
                let new_direction = hit_first_location - ray.origin;
                light_field_distortion(&ray, ray.origin, new_direction);
            }
        }
        if index == 1 {
            hit_second = trig_1.hit || trig_2.hit;

            if trig_1.hit {
                hit_second_location = trig_1.pixel_center_model_space;
                coordinates_second_relative_pixel = trig_1.pixel_coords;
            }
            if trig_2.hit {
                hit_second_location = trig_2.pixel_center_model_space;
                coordinates_second_relative_pixel = trig_2.pixel_coords;
            }
        }

    }

    // We have hit both
    if hit_first && hit_second {
        // The new origin
        let new_origin = hit_first_location;
        // the new directoin
        let new_direction = hit_second_location - new_origin;
        // the new ray
        light_field_distortion(&ray, new_origin, new_direction);
    } else if hit_first {
        let new_direction = hit_first_location - ray.origin;
        light_field_distortion(&ray, ray.origin, new_direction);
    } else if hit_second {

        let new_direction = hit_second_location - ray.origin;
        light_field_distortion(&ray, ray.origin, new_direction);
    }

    var target_intersection = intersection(&ray, scene.quad.a, scene.quad.b, scene.quad.c, true);
    var color = target_intersection.color;

    if color.a != 0.0 {
        if hit_first || hit_second {

            let gray_scale = color.r * 0.299 + 0.587 * color.g + 0.114 * color.b;

            var out = vec4f(gray_scale, gray_scale, gray_scale, color.a);
            out = out * background_color;
            return FragmentOutput(out, out);

        }

        return FragmentOutput(color, color);
    }
    target_intersection = intersection(&ray, scene.quad.b, scene.quad.c, scene.quad.d, false);
    color = target_intersection.color;

    if color.a != 0.0 {

        if hit_first || hit_second {

            let gray_scale = color.r * 0.299 + 0.587 * color.g + 0.114 * color.b;

            var out = vec4f(gray_scale, gray_scale, gray_scale, color.a);
            out = out * background_color;

            return FragmentOutput(out, out);
        }
        return FragmentOutput(color, color);
    }

    clean_up_record(position);
    return FragmentOutput(background_color, background_color);
}

struct PanelHit {
    hit: bool,
    border: bool,
    color: vec4f,

}

fn panels_texture_hit(ray: ptr<function, Ray>) -> PanelHit {

    // WE DO THIS THE 'ARD WAY

    var color = vec4f(1.0, 1.0, 1.0, 1.0);
    var hit = false;

    for (var index = 0u; index < 2; index++) {

        let panel = panels[index];

        let trig_1 = intersection_panel(ray, panel.quad.a, panel.quad.b, panel.quad.c, true, panel);
        hit = hit || trig_1.hit;
        if trig_1.border {
            return PanelHit(hit, true, border_color);
        }
        if trig_1.hit {
            let pixel_coords = trig_1.pixel_coords - vec2u(1, 1);
            let coordinates = vec2f(f32(pixel_coords.x) / f32(panel_texture_size.x), f32(pixel_coords.y) / f32(panel_texture_size.y));

            let sample = textureSample(texture_panel, sampler_panel, coordinates, index);
            let opacity = 1.0 - sample.r;
            color = sample * color;
        }

        let trig_2 = intersection_panel(ray, panel.quad.b, panel.quad.c, panel.quad.d, false, panel);

        hit = hit || trig_2.hit;
        if trig_2.border {
            return PanelHit(hit, true, border_color);
        }
        if trig_2.hit {
            let pixel_coords = trig_2.pixel_coords - vec2u(1, 1);
            let coordinates = vec2f(f32(pixel_coords.x) / f32(panel_texture_size.x), f32(pixel_coords.y) / f32(panel_texture_size.y));

            let sample = textureSample(texture_panel, sampler_panel, coordinates, index);

            color = sample * color;
        }
        ;
    }

    //color.a = 1 - color.r;

    return PanelHit(hit, false, color);
}


fn record_light_field_sample(position: vec2<f32>, panel_1_coords: vec2<u32>, panel_2_coords: vec2<u32>, targets_coords: vec2<u32>, sample: f32) {
    // First location
    //
    // (x + y * column) * 3
    //
    let array_coordination = (u32(position.x) + u32(position.y) * 2560) * 3;
    // There is padding from the border. The pixel count should be 2 less
    // And remove one from pixel panel coordinates

    //  1.0 means there was an intersection.
    //  0.0 means we didn't hit

    // ROW, COLUMN, ENTRY
    // There is an unusual problem, sometimes we record hits that are outside the panel?
    m_a_y_buffer[array_coordination] = min(targets_coords.y, scene.pixel_count.y);
    m_a_y_buffer[array_coordination + 1] = min(panel_1_coords.y, panels[0].pixel_count.y);
    m_a_y_buffer[array_coordination + 2] = 1;

    m_a_x_buffer[array_coordination] = min(targets_coords.x, scene.pixel_count.x);
    m_a_x_buffer[array_coordination + 1] = min(panel_1_coords.x, panels[0].pixel_count.x);
    m_a_x_buffer[array_coordination + 2] = 1;

    m_b_y_buffer[array_coordination] = min(targets_coords.y, scene.pixel_count.y);
    m_b_y_buffer[array_coordination + 1] = min(panel_2_coords.y, panels[1].pixel_count.y);
    m_b_y_buffer[array_coordination + 2] = 1;

    m_b_x_buffer[array_coordination] = min(targets_coords.x, scene.pixel_count.x);
    m_b_x_buffer[array_coordination + 1] = min(panel_2_coords.x, panels[1].pixel_count.x);
    m_b_x_buffer[array_coordination + 2] = 1;
}
fn clean_up_record(position: vec2<f32>) {
    let array_coordination = (u32(position.x) + u32(position.y) * 2560) * 3;
    m_a_y_buffer[array_coordination] = 0;
    m_a_y_buffer[array_coordination + 1] = 0;
    m_a_y_buffer[array_coordination + 2] = 0;

    m_a_x_buffer[array_coordination] = 0;
    m_a_x_buffer[array_coordination + 1] = 0;
    m_a_x_buffer[array_coordination + 2] = 0;

    m_b_y_buffer[array_coordination] = 0;
    m_b_y_buffer[array_coordination + 1] = 0;
    m_b_y_buffer[array_coordination + 2] = 0;

    m_b_x_buffer[array_coordination] = 0;
    m_b_x_buffer[array_coordination + 1] = 0;
    m_b_x_buffer[array_coordination + 2] = 0;
}
// Distortion of Ray caused by limits of the panel
// Change the Ray that will be used for other intersections
fn light_field_distortion(ray: ptr<function, Ray>, new_origin: vec3f, new_direction: vec3f) {
    if distort_rays != 0 {
        (*ray).origin = new_origin;
        (*ray).direction = new_direction;
    }
}
;
struct Pixel_Panel {
    pixel: vec2u,
    model_coords: vec3f,

}
;
// From the Barycentric coordinates, give us the pixel coordinates
fn pixel_hit(bary_coords: vec3f, relative_tex_coords: array<vec2f, 3>, panel: Panel) -> Pixel_Panel {

    // Relative Coordinates
    let x_coord = (bary_coords.x * relative_tex_coords[0].x + bary_coords.y * relative_tex_coords[1].x + bary_coords.z * relative_tex_coords[2].x);
    let y_coord = (bary_coords.x * relative_tex_coords[0].y + bary_coords.y * relative_tex_coords[1].y + bary_coords.z * relative_tex_coords[2].y);
    // From Relative coordinates to pixel
    // Casting
    // Cast pixel count into a f32 to multiply, then into u32 to round
    let x_pixel = u32(x_coord * f32(panel.pixel_count.x));
    let y_pixel = u32(y_coord * f32(panel.pixel_count.y));

    let pixel_size = panel.size / vec2f(panel.pixel_count);

    // Get the pixel center
    let center_x_pixel = (f32(x_pixel) * pixel_size.x) + (pixel_size.x / 2.0);
    let center_y_pixel = (f32(y_pixel) * pixel_size.y) + (pixel_size.y / 2.0);

    //
    let x_comp = panel.quad.b - panel.quad.a;
    let y_comp = panel.quad.c - panel.quad.a;

    //
    let x_vec = x_comp * center_x_pixel;

    let y_vec = y_comp * center_y_pixel;

    // Does this seem correct, I think
    let new_position = x_vec + y_vec + panel.quad.a;
    // Messy code, need to write this out on paper?
    let pixel = vec2u(x_pixel, y_pixel);
    return Pixel_Panel(pixel, new_position);
}
;
// Use this to get the pixel that was hit, x/y coordinates. Then record!
fn target_texel(bary_coords: vec3f, relative_tex_coords: array<vec2f, 3>) -> vec2u {

    let x_coord = (bary_coords.x * relative_tex_coords[0].x + bary_coords.y * relative_tex_coords[1].x + bary_coords.z * relative_tex_coords[2].x);
    let y_coord = (bary_coords.x * relative_tex_coords[0].y + bary_coords.y * relative_tex_coords[1].y + bary_coords.z * relative_tex_coords[2].y);
    // From Relative coordinates to pixel
    // Casting
    // Cast pixel count into a f32 to multiply, then into u32 to round
    let x_pixel = u32(floor(x_coord * f32(scene.pixel_count.x)));
    let y_pixel = u32(floor(y_coord * f32(scene.pixel_count.y)));
    return vec2(x_pixel, y_pixel);
}

/// Struct to capture the possible hits
struct PanelIntersection {
    hit: bool,
    border: bool,
    pixel_coords: vec2u,
    pixel_center_model_space: vec3f,
}
;
fn intersection_panel(ray: ptr<function, Ray>, a: vec3f, b: vec3f, c: vec3f, abc: bool, panel: Panel) -> PanelIntersection {
    var hit = false;
    var border = false;
    var pixel_center_model_space = vec3f(0.0, 0.0, 0.0);
    var pixel_coords = vec2u(0, 0);

    var e1 = b - a;
    var e2 = c - a;
    var rey_cross_e2 = cross(e2,(*ray).direction);
    var det = dot(rey_cross_e2, e1);

    if det > -eps && det < eps {
        return PanelIntersection(hit, border, pixel_coords, pixel_center_model_space);
    }
    var inv_det = 1.0 / det;
    var s = (*ray).origin - a;
    var u = inv_det * dot(rey_cross_e2, s);

    if (u < 0.0 && abs(u) > eps) || (u > 1.0 && abs(u - 1.0) > eps) {
        return PanelIntersection(hit, border, pixel_coords, pixel_center_model_space);
    }
    var s_cross_e1 = cross(e1, s);
    var v = inv_det * dot(s_cross_e1,(*ray).direction);
    var w = 1.0 - v - u;

    if v < 0.0 || u + v > 1.0 {
        return PanelIntersection(hit, border, pixel_coords, pixel_center_model_space);
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
            pixel_center_model_space = pixels_panel.model_coords;

            if pixels.x == 0 || pixels.x == panel.pixel_count.x - 1 || pixels.y == 0 || pixels.y == panel.pixel_count.y - 1 {
                border = true;
                return PanelIntersection(hit, border, pixels, pixel_center_model_space);
            } else {
                //Distort Camera!
                return PanelIntersection(hit, border, pixels, pixel_center_model_space);
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
            pixel_center_model_space = pixels_panel.model_coords;
            if pixels.x == 0 || pixels.x == panel.pixel_count.x - 1 || pixels.y == 0 || pixels.y == panel.pixel_count.y - 1 {

                border = true;
                return PanelIntersection(hit, border, pixels, pixel_center_model_space);
            } else {

                return PanelIntersection(hit, border, pixels, pixel_center_model_space);
            }
        }

    //return vec4f(0.0, 0.5, 0.5, 1.0);
    }

    return PanelIntersection(hit, border, pixel_coords, pixel_center_model_space);
}
;
struct TargetIntersection {
    color: vec4<f32>,
    texel: vec2<u32>,
}

fn intersection(ray: ptr<function, Ray>, a: vec3f, b: vec3f, c: vec3f, abc: bool) -> TargetIntersection {
    var e1 = b - a;
    var e2 = c - a;
    var rey_cross_e2 = cross(e2,(*ray).direction);
    var det = dot(rey_cross_e2, e1);

    if det > -eps && det < eps {
        return TargetIntersection(
            miss_color,
            vec2u(0, 0),
        );
    }
    var inv_det = 1.0 / det;
    var s = (*ray).origin - a;
    var u = inv_det * dot(rey_cross_e2, s);

    if (u < 0.0 && abs(u) > eps) || (u > 1.0 && abs(u - 1.0) > eps) {
        return TargetIntersection(
            miss_color,
            vec2u(0, 0),
        );
    }
    var s_cross_e1 = cross(e1, s);
    var v = inv_det * dot(s_cross_e1,(*ray).direction);
    var w = 1.0 - v - u;

    if v < 0.0 || u + v > 1.0 {
        return TargetIntersection(
            miss_color,
            vec2u(0, 0),
        );
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    var t = inv_det * dot(s_cross_e1, e2);

    if t > eps {
        let bary_coords = vec3f(u, v, w);

        // Tex coordinates
        // a -> 0.0, 0.0
        // b -> 1.0, 0.0
        // c = 0.0, 1.0
        // d = 1.0, 1.0
        // a ==== b
        // |      |
        // |      |
        // c ==== d

        // A, B, C
        if abc {

            //w === u
            //|   /
            //|  /
            //| /
            //v
            let tex_coords = array(vec2f(1.0, 0.0), vec2f(0.0, 1.0), vec2f(0.0, 0.0));
            let color = sample_texture(bary_coords, tex_coords[0], tex_coords[1], tex_coords[2], scene.pixel_count);
            //let color = vec4f(u, v, w, 1.0);
            let texel = target_texel(bary_coords, tex_coords);
            return TargetIntersection(
                color,
                texel,
            );
        } else {
            // B, C, D

            //
            //       w
            //     / |
            //    /  |
            //   /   |
            //  /    |
            // u === v
            let tex_coords = array(vec2f(0.0, 1.0), vec2f(1.0, 1.0), vec2f(1.0, 0.0));
            let color = sample_texture(bary_coords, tex_coords[0], tex_coords[1], tex_coords[2], scene.pixel_count);

            //let color = vec4f(u, v, w, 1.0);
            let texel = target_texel(bary_coords, tex_coords);
            return TargetIntersection(
                color,
                texel,
            );
        }

    //return vec4f(0.0, 0.5, 0.5, 1.0);
    }

    return TargetIntersection(
        miss_color,
        vec2u(0, 0),
    );
}

// Texture is upside down for Shader? why?
fn sample_texture(bary_coords: vec3f, tex_coord_0: vec2f, tex_coord1: vec2f, tex_coord2: vec2f, texture_size: vec2u) -> vec4f {
    // sample expects between [0,0] and [1,1]
    var x_coord = (bary_coords.x * tex_coord_0.x + bary_coords.y * tex_coord1.x + bary_coords.z * tex_coord2.x);
    // 0,0 is bottom left, not top left. Flip the Y axis to get the right image
    var y_coord = (bary_coords.x * tex_coord_0.y + bary_coords.y * tex_coord1.y + bary_coords.z * tex_coord2.y);
    // Express coordinate as ratio
    x_coord = x_coord * (f32(texture_size.x) / f32(tex_size.x));
    y_coord = y_coord * (f32(texture_size.y) / f32(tex_size.y));

    let coordinates = vec2f(x_coord, y_coord);

    return textureSample(t_diffuse, s_diffuse, coordinates);
}

fn collapse(in: vec4<bool>) -> bool {
    return in.x || in.y || in.z && in.w;
}
