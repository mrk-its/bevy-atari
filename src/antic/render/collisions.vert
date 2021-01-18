#version 300 es

precision highp float;
precision mediump int;

in vec3 Vertex_Position;
in vec2 Vertex_Uv;

out vec2 v_Uv;

layout(std140) uniform Camera {
    mat4 ViewProj;
};
layout(std140) uniform Transform { // set = 2, binding = 0
    mat4 Model;
};

void main() {
    gl_PointSize = 30.0;
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
    v_Uv = Vertex_Uv;
}
