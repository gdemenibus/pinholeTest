#version 140
in vec3 position;
in vec2 tex_coords;

uniform mat4 view_proj;
uniform vec4 color_in;

void main() {
    // Place object in space
    vec4 model_space = vec4(position, 1.0);
    // 
    gl_Position = view_proj * model_space;
}
