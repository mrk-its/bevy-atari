#version 300 es

precision highp float;

in vec2 v_Uv;
out vec4 o_Target;

layout(std140) uniform TextArea_width { // set = 1 binding = 0
    float width;
};

layout(std140) uniform TextArea_height { // set = 1 binding = 1
    float height;
};

layout(std140) uniform TextArea_fg_color { // set=1 binding = 2
    vec4 fg_color;
};

layout(std140) uniform TextArea_bg_color { // set=1 binding = 3
    vec4 bg_color;
};

layout(std140) uniform TextArea_data { // set = 1 binding = 4
    uvec4 data[64];  // 64 * 16 = 1024 bytes
};

layout(std140) uniform TextArea_charset { // set = 1 binding = 5
    uvec4 charset[64];
};

#define get_byte(data, offset) (int(data[offset >> 4][(offset >> 2) & 3] >> ((offset & 3) << 3)) & 255)

vec4 encodeSRGB(vec4 linearRGB_in)
{
    vec3 linearRGB = linearRGB_in.rgb;
    vec3 a = 12.92 * linearRGB;
    vec3 b = 1.055 * pow(linearRGB, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), linearRGB);
    return vec4(mix(a, b, c), linearRGB_in.a);
}

#define encodeColor(x) encodeSRGB(x)
// #define encodeColor(x) (x)

void main() {
    int y = 0;
    float x_pos = v_Uv[0] * width;
    float y_pos = v_Uv[1] * height;
    int c = get_byte(data, int(y_pos) * int(width) + int(x_pos));
    float x_frac = x_pos - float(int(x_pos));
    float y_frac = y_pos - float(int(y_pos));

    int inv = c >> 7;
    int charset_offset = (c & 0x7f) * 8 + int(y_frac * 7.9);
    int byte = get_byte(charset, charset_offset);

    int x_bit = 7-int(x_frac * 8.0);

    int pixel_val = (((byte >> x_bit) & 1) ^ inv);
    if(pixel_val > 0) {
        o_Target = encodeColor(fg_color);
    } else {
        o_Target = encodeColor(bg_color);
    }
    // o_Target = vec4(1.0, 0.0, 0.0, 1.0);
}