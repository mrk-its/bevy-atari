#version 300 es
precision highp float;
precision highp int;
precision highp usampler2D;

in vec3 v_Position;
in vec2 v_Uv;

out uvec4 o_Target;

layout(std140) uniform CameraViewProj {  // set = 0, binding = 0
    mat4 ViewProj;
};

layout(std140) uniform CustomTexture_color {  // set = 1, binding = 1
    vec4 color;
};

uniform usampler2D CustomTexture_texture;  // set = 1, binding = 2


// collision aggregation texture is 384.0 x 15.0
// 240 / 15 = 16
// so we want to aggregate 15 strips 384.0 x 16.0

// or

// collision aggregation texture is 16.0 x 240.0
// 384 / 16 = 24
// so we want to aggregate 16 strips 24.0 x 240.0 px

// #define HORIZONTAL_TEXTURE

void main() {
    uvec4 v = uvec4(0, 0, 0, 0);

    #ifdef HORIZONTAL_TEXTURE
        const int TEXTURE_HEIGHT = 16;
        const int STRIP_HEIGHT = 240 / TEXTURE_HEIGHT;
        int px = int(v_Uv[0] * 384.0);
        int py = int(v_Uv[1] * float(TEXTURE_HEIGHT)) * STRIP_HEIGHT;

        for(int y=0; y < STRIP_HEIGHT; y++) {
            v |= texelFetch(CustomTexture_texture, ivec2(px, py + y), 0);
        }
    #else
        const int TEXTURE_WIDTH = 16;
        const int STRIP_WIDTH = 384 / TEXTURE_WIDTH;
        int px = int(v_Uv[0] * float(TEXTURE_WIDTH)) * STRIP_WIDTH;
        int py = int(v_Uv[1] * 240.0);

        for(int x=0; x < STRIP_WIDTH; x++) {
            v |= texelFetch(CustomTexture_texture, ivec2(px + x, py), 0);
        }
    #endif

    o_Target = v;
}
