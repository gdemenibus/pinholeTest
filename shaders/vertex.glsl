#version 460
in vec3 position;
in vec2 tex_coords;
out vec2 v_tex_coords;

uniform mat4 view_proj;
uniform mat4 model;

void main() {
    v_tex_coords = tex_coords;
    // Place object in space
    vec4 model_space = model * vec4(position, 1.0);
    // 
    gl_Position = view_proj * model_space;
}
