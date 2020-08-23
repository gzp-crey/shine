#version 450

layout(location=0) in vec2 v_tex_coords;
layout(location=0) out vec4 f_color;

layout(set = 1, binding = 1) uniform texture2D t_diffuse;
layout(set = 1, binding = 2) uniform sampler s_diffuse;

void main() {
    f_color = texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords);
}