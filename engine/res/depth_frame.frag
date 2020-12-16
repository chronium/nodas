#version 450

layout(location=0) in vec2 v_tex_coords;

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_depth;
layout(set = 0, binding = 1) uniform samplerShadow s_depth;

float linearize_depth(float d,float zNear,float zFar)
{
    return zNear * zFar / (zFar + d * (zNear - zFar));
}

void main() {
    float near = 0.1;
    float far = 100.0;
    float depth = texture(sampler2DShadow(t_depth, s_depth), vec3(v_tex_coords, 1));
    float r = linearize_depth(depth, near, far);

    f_color = vec4(vec3(r), 1);
}