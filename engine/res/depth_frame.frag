#version 450

layout(location=0) in vec2 v_tex_coords;
layout(location=1) in mat4 v_inv_proj;

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_depth;
layout(set = 0, binding = 1) uniform samplerShadow s_depth;

void main() {
    float x = (v_tex_coords.x * 2.0) - 1.0;
    float y = (v_tex_coords.y * 2.0) - 1.0;
    float z = texture(sampler2DShadow(t_depth, s_depth), vec3(v_tex_coords, 1));

    vec4 buff = v_inv_proj * vec4(x, y, z, 1.0);
    //vec3 spos = buff.xyz / buff.w;
    vec3 spos = buff.xyz;

    f_color = vec4(spos, 1.0);
}