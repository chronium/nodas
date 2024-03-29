#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec2 a_tex_coords;

layout(location=0) out vec2 v_tex_coords;
layout(location=1) out mat4 v_inv_proj;

layout(set=1, binding=0) 
uniform Uniforms {
  vec3 u_view_position; 
  mat4 u_view_proj;
};

void main() {
  v_tex_coords = a_tex_coords;
  v_inv_proj = inverse(u_view_proj);

  gl_Position = vec4(a_position, 1.0);
}
