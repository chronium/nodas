#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 1) in vec4 v_view_position;

layout(location = 0) out vec4 f_color;

layout(set = 1, binding = 0)
uniform Grid {
    float grid_scale;
    vec4 grid_color;
};

const float FADE_NEAR = 10;
const float FADE_FAR = 100;
const float N = 25.0;

float invGridAlpha(in vec2 p, in vec2 ddx, in vec2 ddy) {
    vec2 w = max(abs(ddx), abs(ddy)) + 0.01;
    vec2 a = p + 0.5 * w;                        
    vec2 b = p - 0.5 * w;

    vec2 i = (floor(a) + min(fract(a) * N, 1.0)
             -floor(b) - min(fract(b) * N, 1.0))
             /(N * w);

    return (1.0 - i.x) * (1.0 - i.y);
}

void main() {
    float grid = (1 - invGridAlpha(v_tex_coords, dFdx(v_tex_coords), dFdy(v_tex_coords)));
    float dist = length(v_view_position);
	float fade_factor = (FADE_FAR - dist) / (FADE_FAR - FADE_NEAR);
	fade_factor = clamp(fade_factor, 0.0, 1.0);
    f_color = grid_color * vec4(1.0, 1.0, 1.0, grid * fade_factor); 
}