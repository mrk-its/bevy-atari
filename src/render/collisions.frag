#version 300 es
precision highp float;
precision highp int;
precision highp usampler2D;

in vec3 v_Position;
in vec2 v_Uv;

out uvec4 o_Target;

layout(std140) uniform Camera {
    mat4 ViewProj;
};

layout(std140) uniform CustomTexture_color {  // set = 1, binding = 1
    vec4 color;
};

uniform usampler2D CustomTexture_texture;  // set = 1, binding = 2

void main() {
    int px = int(v_Uv[0] * 384.0);
    uvec4 x = uvec4(0, 0, 0, 0);
    for(int y=0; y < 240; y++) {
        x |= texelFetch(CustomTexture_texture, ivec2(px, y), 0);
    }
    o_Target = x;
}
