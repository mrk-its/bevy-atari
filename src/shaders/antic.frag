#version 300 es

precision highp float;

in vec2 v_Uv;
out vec4 o_Target;

layout(std140) uniform AnticLine_line_width { // set = 1 binding = 1
    int line_width;
};

layout(std140) uniform AnticLine_mode { // set = 1 binding = 2
    int mode;
};

layout(std140) uniform AnticLine_data { // set = 1 binding = 3
    uvec4 data[3];  // 48 bytes
};

layout(std140) uniform AnticLine_gtia_colors { // set = 1 binding = 4
    ivec4 color_regs[2]; // [[bak, pf0, pf1, pf2], [bak, pf0, pf1, pf3]]
};

layout(std140) uniform AnticLine_charset { // set = 1 binding = 5
    uvec4 charset[64];
};

layout(std140) uniform AnticLine_hscrol { // set = 1 binding = 6
    int hscrol;
};

layout(std140) uniform AtariPalette_palette { // set=2 binding = 1
    vec4 palette[256];
};

#define get_color_reg(s, k) color_regs[s][k]

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
    if(mode == 0x2) {
        float w = v_Uv[0] * float(line_width / 8);
        int n = int(w);
        float frac = w - float(n);
        int x = 7 - int(frac * 8.0);
        int y = int(v_Uv[1] * 7.9);

        int c = get_byte(data, n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_byte(charset, offs);

        int index = (((byte >> x) & 1) ^ inv);
        int bg_index = get_color_reg(0, 3);
        int fg_index = get_color_reg(0, 2);
        fg_index = (fg_index & 0xf) | (bg_index & 0xf0);
        int colors[2] = int[](bg_index, fg_index);
        o_Target = encodeColor(palette[colors[index]]);
        return;
    } else if(mode == 0x04) {
        float px = v_Uv[0] * float(line_width) + float(hscrol); // pixel x pos
        float w = px / 8.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 6 - int(frac * 4.0) * 2;
        int y = int(v_Uv[1] * 7.9);

        int c = get_byte(data, n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_byte(charset, offs);

        int index = (byte >> x) & 3;
        o_Target = encodeColor(palette[get_color_reg(inv, index)]);
        return;
    } else if(mode == 0xa) {
        float w = v_Uv[0] * float(line_width / 16);
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 6-int(frac * 4.0) * 2; // bit offset in byte

        int byte = get_byte(data, n);
        int index = (byte >> bit_offs) & 3;
        o_Target = encodeColor(palette[get_color_reg(0, index)]);
        // o_Target = vec4(1.0, 1.0, 0.0, 1.0);
        return;
    } else if(mode == 0x0c) {
        float w = v_Uv[0] * float(line_width / 16);
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 7-int(frac * 8.0); // bit offset in byte

        int byte = get_byte(data, n);
        int index = (byte >> bit_offs) & 1;
        o_Target = encodeColor(palette[get_color_reg(0, index)]);
        // o_Target = vec4(1.0, 1.0, 0.0, 1.0);
        return;
    } else if(mode == 0x0d) {
        float w = v_Uv[0] * float(line_width / 8);
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 6-int(frac * 4.0) * 2; // bit offset in byte

        int byte = get_byte(data, n);
        int index = (byte >> bit_offs) & 3;
        o_Target = encodeColor(palette[get_color_reg(0, index)]);
        // o_Target = vec4(1.0, 1.0, 0.0, 1.0);
        return;
    }

    o_Target = vec4(0.0, 1.0, 0.0, 1.0);
}