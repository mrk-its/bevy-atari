#version 300 es
precision highp float;
precision highp int;
precision highp usampler2D;

in vec3 v_Position;
in vec2 v_Uv;

out vec4 o_Target;

layout(std140) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(std140) uniform SimpleMaterial_base_color {  // set = 1, binding = 1
    vec4 color;
};

uniform usampler2D SimpleMaterial_base_color_texture;  // set = 1, binding = 2

void main() {
    int px = int(v_Uv[0] * 384.0);
    int py = int(v_Uv[1] * 240.0);
    uvec4 v = texelFetch(SimpleMaterial_base_color_texture, ivec2(px, py), 0);
    o_Target = vec4(v[0] > uint(0) ? 1.0 : 0.0, v[1] > uint(1.0) ? 1.0 : 0.0, v[2] > uint(1.0) ? 1.0 : 0.0, 1.0);
}
