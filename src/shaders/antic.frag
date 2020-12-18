#version 300 es

precision highp float;

in vec2 v_Uv;
out vec4 o_Target;

layout(std140) uniform AnticLine_line_width { // set = 1 binding = 1
    float line_width;
};

layout(std140) uniform AnticLine_mode { // set = 1 binding = 2
    int mode;
};

layout(std140) uniform AnticLine_data { // set = 1 binding = 3
    uvec4 data[3];  // 48 bytes
    uvec4 player[4]; // players - 4 * 16 bytes (scanlines) max
};
struct GTIA {
    ivec4 color_regs[2];
    ivec4 colpm;
    vec4 player_pos;
    vec4 player_size;
    ivec4 prior;
};
layout(std140) uniform AnticLine_gtia_regs_array { // set = 1 binding = 4
    //ivec4 color_regs[2]; // [[bak, pf0, pf1, pf2], [bak, pf0, pf1, pf3]]
    GTIA gtia[8];
};

layout(std140) uniform AnticLine_charset { // set = 1 binding = 5
    uvec4 charset[64];
};

layout(std140) uniform AnticLine_hscrol { // set = 1 binding = 6
    float hscrol;
};

layout(std140) uniform AtariPalette_palette { // set=2 binding = 1
    vec4 palette[256];
};

#define get_color_reg(line, k) gtia[line].color_regs[k>>2][k&3]

#define get_byte(data, offset) (int(data[offset >> 4][(offset >> 2) & 3] >> ((offset & 3) << 3)) & 255)
#define get_player_byte(_player, offset) (int(_player[(offset >> 2) & 3] >> ((offset & 3) << 3)) & 255)

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

bool get_player_pixel(int n, float px, int y, vec4 player_pos) {
    if (px >= player_pos[n] && px < player_pos[n] + gtia[0].player_size[n]) {
        int pl_bit = 7 - int((px - player_pos[n]) / gtia[0].player_size[n] * 8.0);
        int byte = get_player_byte(player[n], y);
        return ((byte >> pl_bit) & 1) > 0;
    }
    return false;
}

void main() {
    float px = v_Uv[0] * line_width;
    float px_scrolled = px + float(hscrol);  // pixel x position
    vec4 player_pos = gtia[0].player_pos * 2.0 + vec4(line_width / 2.0 - 256.0);
    vec4 player_pos_end = player_pos + gtia[0].player_size;
    int y = 0;
    bool hires=false;

    int color_reg_index = 0; // bg_color
    if(mode == 0x0) {
        y = int(v_Uv[1] * 7.9);
        o_Target = encodeColor(palette[get_color_reg(y, 0)]);
    } else if(mode == 0x2) {
        float w = px_scrolled / 8.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 7 - int(frac * 8.0);
        y = int(v_Uv[1] * 7.9);

        int c = get_byte(data, n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_byte(charset, offs);

        int pixel_val = (((byte >> x) & 1) ^ inv);
        color_reg_index = 3 - pixel_val;
        hires = true;
    } else if(mode == 0x04) {
        float w = px_scrolled / 8.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 6 - int(frac * 4.0) * 2;
        y = int(v_Uv[1] * 7.99);

        int c = get_byte(data, n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_byte(charset, offs);

        color_reg_index = (byte >> x) & 3;
        if(inv != 0 && color_reg_index == 3) {
            color_reg_index = 4;
        };
    } else if(mode == 0xa) {
        float w = px_scrolled / 16.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 6-int(frac * 4.0) * 2; // bit offset in byte

        int byte = get_byte(data, n);
        color_reg_index = (byte >> bit_offs) & 3;
    } else if(mode == 0x0c) {
        float w = px_scrolled / 16.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 7-int(frac * 8.0); // bit offset in byte

        int byte = get_byte(data, n);
        color_reg_index = (byte >> bit_offs) & 1;
    } else if(mode == 0x0d) {
        float w = px_scrolled / 8.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 6-int(frac * 4.0) * 2; // bit offset in byte

        int byte = get_byte(data, n);
        color_reg_index = (byte >> bit_offs) & 3;
    } else if(mode == 0x0e) {
        float w = px_scrolled / 8.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 6-int(frac * 4.0) * 2; // bit offset in byte

        int byte = get_byte(data, n);
        color_reg_index = (byte >> bit_offs) & 3;
    } else if(mode == 0x0f) {

        float w = px_scrolled / 8.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 7-int(frac * 8.0); // bit offset in byte

        int byte = get_byte(data, n);
        int pixel_val = (byte >> bit_offs) & 1;
        color_reg_index = 3 - pixel_val;
        hires = true;
    };

    int color_index = get_color_reg(y, color_reg_index);

    if(hires && color_reg_index == 2) {
        color_index = (color_index & 0xf) | (get_color_reg(y, 3) & 0xf0);
    }

    o_Target = encodeColor(palette[color_index]);
    // TODO - implement real priorities
    // Robbo hack (ovewrite bg with players color)
    if(gtia[0].prior[0]==4 && color_reg_index>0) return;

    // sprites!

    int color_reg = 0;
    if(get_player_pixel(0, px, y, player_pos)) {
        color_reg |= gtia[0].colpm[0];
    }
    if(get_player_pixel(1, px, y, player_pos)) {
        color_reg |= gtia[0].colpm[1];
    }
    if(get_player_pixel(2, px, y, player_pos)) {
        color_reg |= gtia[0].colpm[2];
    };
    if(get_player_pixel(3, px, y, player_pos)) {
        color_reg |= gtia[0].colpm[3];
    };
    if(color_reg>0) {
        o_Target = encodeColor(palette[color_reg]);
    };
}