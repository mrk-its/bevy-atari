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

layout(std140) uniform CustomTexture_color {  // set = 1, binding = 10
    vec4 color;
};

uniform usampler2D CustomTexture_texture;  // set = 1, binding = 11

vec4 encodeSRGB(vec4 linearRGB_in)
{
    vec3 linearRGB = linearRGB_in.rgb;
    vec3 a = 12.92 * linearRGB;
    vec3 b = 1.055 * pow(linearRGB, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), linearRGB);
    return vec4(mix(a, b, c), linearRGB_in.a);
}

void main() {
    // uvec4 x = texture(
    //     CustomTexture_texture,
    //     v_Uv
    // );
    int px = int(v_Uv[0] * 384.0);
    uvec4 x = uvec4(0, 0, 0, 0);
    for(int y=0; y < 240; y++) {
        uvec4 v = texelFetch(CustomTexture_texture, ivec2(px, y), 0);
        x |= v;
        x[0] |= uint(1);
    }
    o_Target = x;
    // o_Target = vec4(x[0] > uint(0) ? 1.0 : 0.0, x[1] > uint(0) ? 1.0 : 0.0, x[2] > uint(0) ? 1.0 : 0.0, 1.0);

    // o_Target = color;

    // o_Target = vec4(1.0, 0.0, 0.0, 1.0);
    // o_Target = texture(
    //     CustomTexture_texture,
    //     v_Uv
    // );
}
