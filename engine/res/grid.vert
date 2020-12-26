#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_tex_coords;

layout(location = 0) out vec2 v_tex_coords;
layout(location = 1) out vec4 v_view_position;

layout(set = 0, binding = 0)
uniform Uniforms {
    vec3 u_view_position;
    mat4 u_view_proj;
    mat4 u_view;
};

layout(set = 1, binding = 0)
uniform Grid {
    float grid_scale;
    vec4 grid_color;
};

void main() {
    gl_Position = u_view_proj * vec4(a_position, 1);
    v_tex_coords = a_tex_coords / grid_scale;
    v_view_position = u_view * vec4(a_position, 1);
}