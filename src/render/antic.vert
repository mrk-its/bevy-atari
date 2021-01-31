#version 300 es
precision highp float;
precision highp int;

in vec3 Vertex_Position;
in vec2 Vertex_Uv;
in uvec4 Vertex_Custom;

out vec3 v_Position;
out vec2 v_Uv;
flat out uvec4 v_Custom;

layout(std140) uniform Camera {
    mat4 ViewProj;
};

layout(std140) uniform Transform {  // set = 2, binding = 0
    mat4 Model;
};

void main() {
    v_Uv = Vertex_Uv;
    v_Custom = Vertex_Custom;
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
}
