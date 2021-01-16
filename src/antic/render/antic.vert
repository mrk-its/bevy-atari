#version 300 es

precision highp float;
precision mediump int;

in vec3 Vertex_Position;
in vec2 Vertex_Uv;

out vec2 v_Uv;

layout(std140) uniform Camera {
    mat4 ViewProj;
};
layout(std140) uniform Transform { // set = 1 binding = 0
    mat4 Model;
};

#ifdef COLLISIONS
layout(std140) uniform AnticLine_antic_line_descr {  // set = 1 binding = 1
    float line_width;
    int mode;
    float hscrol;
    float line_height;
    float line_voffset;
    float scan_line;
};
#endif

void main() {
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
#ifdef ___COLLISIONS
    gl_Position = gl_Position + vec4(0.0, 2.0 - 2.0 * scan_line / 240.0, 0.0, 0.0);
#endif
    v_Uv = Vertex_Uv;
}
