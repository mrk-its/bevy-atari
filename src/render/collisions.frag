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

// collision aggregation texture is 384.0 x 15.0
// 240 / 15 = 16
// so we want to aggregate 15 strips 384.0 x 16.0

const int TEXTURE_HEIGHT = 15;
const int STRIP_HEIGHT = 240 / TEXTURE_HEIGHT;

void main() {
    int px = int(v_Uv[0] * 384.0);
    int py = int(v_Uv[1] * float(TEXTURE_HEIGHT)) * STRIP_HEIGHT;

    uvec4 x = uvec4(0, 0, 0, 0);
    for(int y=0; y < STRIP_HEIGHT; y++) {
        x |= texelFetch(CustomTexture_texture, ivec2(px, py + y), 0);
    }
    o_Target = x;
}
