#version 460
layout(origin_upper_left) in vec4 gl_FragCoord;
in vec2 v_tex_coords;
out vec4 color;

uniform sampler2D tex;

uniform struct World {
    vec4 a;
    vec4 b;
    vec4 c;
    vec4 d;
    vec4 e;
    vec4 f;
} world;
uniform vec3 ray_origin;
uniform vec3 p_1_m;
uniform vec3 q_x;
uniform vec3 q_y;

const float eps = 0.00001;

bool intersection(vec3 ray_dir, vec3 a, vec3 b, vec3 c) {
    vec3 direction = ray_dir;
    vec3 origin = ray_origin;
    vec3 e1 = b - a;
    vec3 e2 = c - a;
    vec3 ray_cross_e2 = cross(e2, direction);
    float det = dot(ray_cross_e2, e1);

    if (det > eps && det < eps) {
        return false;
    }
    float inv_det = 1.0 / det;
    vec3 s = origin - a;
    float u = inv_det * dot(ray_cross_e2, s);
    if ((u < 0 && abs(u) > eps) || (u > 1 && abs(u - 1) > eps)) {
        return false;
    }

    vec3 s_cross_e1 = cross(e1, s);
    float v = inv_det * dot(s_cross_e1, direction);
    float w = 1.0 - v - u;

    if (v < 0.0 || u + v > 1.0) {
        return false;
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    float t = inv_det * dot(s_cross_e1, e2);
    if (t > eps) {
        return true;
    }
    return false;
}

void main() {
    float f_x = float(gl_FragCoord.x);
    float f_y = float(gl_FragCoord.y);
    vec3 ray_dir = p_1_m + q_x * (f_x - 1.0) + q_y * (f_y - 1.0);

    ray_dir = normalize(ray_dir);
    bool first_trig = intersection(ray_dir, world.a.xyz, world.b.xyz, world.c.xyz);
    if (first_trig) {
        color = vec4(1.0, 1.0, 1.0, 1.0);
    }
    else {
        // How the f are we here?
        color = vec4(0.0, 0.0, 0.5, 0.0);
    }
    //color = texture(tex, v_tex_coords);
}
