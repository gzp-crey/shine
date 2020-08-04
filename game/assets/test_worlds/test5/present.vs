#version 450

layout(location=0) out vec2 v_tex_coords;

void main() {
    vec2 position = vec2(gl_VertexIndex, (gl_VertexIndex & 1) * 2) - 1;
    v_tex_coords = position.yx;
    gl_Position = vec4(position.yx, 0.0, 1.0);
}