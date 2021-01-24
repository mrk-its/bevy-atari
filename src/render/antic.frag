#version 300 es

precision highp float;
precision highp int;

in vec2 v_Uv;
layout(location = 0) out vec4 o_ColorTarget;
layout(location = 1) out uvec4 o_CollisionsTarget;

layout(std140) uniform AnticLine_antic_line_descr {  // set = 1 binding = 1
    float line_width;
    int mode;
    float hscrol;
    float line_height;
    float line_voffset;
    float scan_line;
};

layout(std140) uniform AnticLine_data { // set = 1 binding = 2
    uvec4 data[3];  // 48 bytes
};

struct GTIA {
    ivec4 color_regs[2];
    ivec4 colpm;
    ivec4 hposp;
    ivec4 hposm;
    ivec4 player_size;
    ivec4 grafp;
    ivec4 prior;  // [prior, sizem, grafm, unused]
};

layout(std140) uniform AnticLine_gtia_regs_array { // set = 1 binding = 3
    //ivec4 color_regs[2]; // [[bak, pf0, pf1, pf2], [bak, pf0, pf1, pf3]]
    GTIA gtia[8];
};

layout(std140) uniform AnticLine_charset { // set = 1 binding = 4
    uvec4 charset[64];
};

layout(std140) uniform AtariPalette_palette { // set=2 binding = 1
    vec4 palette[256];
};

#define get_color_reg(line, k) gtia[line].color_regs[k>>2][k&3]

#define get_byte(data, offset) (int(data[offset >> 4][(offset >> 2) & 3] >> ((offset & 3) << 3)) & 255)
#define get_player_byte(_player, offset) (int(_player[(offset >> 2) & 3] >> ((offset & 3) << 3)) & 255)

vec4 encodeSRGB(vec4 linearRGB_in) {
    vec3 linearRGB = linearRGB_in.rgb;
    vec3 a = 12.92 * linearRGB;
    vec3 b = 1.055 * pow(linearRGB, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), linearRGB);
    return vec4(mix(a, b, c), linearRGB_in.a);
}

// #define encodeColor(x) encodeSRGB(x)
#define encodeColor(x) (x)

bool get_player_pixel(int n, float px, int y, vec4 hpos) {
    if (px >= hpos[n] && px < hpos[n] + float(gtia[y].player_size[n])) {
        int pl_bit = 7 - int((px - hpos[n]) / float(gtia[y].player_size[n]) * 8.0);
        int byte = gtia[y].grafp[n];
        // int byte = get_player_byte(player[n], y);
        return ((byte >> pl_bit) & 1) > 0;
    }
    return false;
}

bool get_missile_pixel(int n, float px, int y, vec4 hpos) {
    float sizem = float(gtia[y].prior[1]);
    if (px >= hpos[n] && px < hpos[n] + sizem) {
        int bit = 1 - int((px - hpos[n]) / sizem * 2.0);
        // int byte = get_player_byte(missiles, y) >> (n * 2);
        int byte = gtia[y].prior[2] >> (n * 2);
        return ((byte >> bit) & 1) > 0;
    }
    return false;
}

void main() {
    // float px = v_Uv[0] * line_width;
    float px = 384.0 * (v_Uv[0] - 0.5) + line_width / 2.0;

    float px_scrolled = px + float(hscrol);  // pixel x position
    int cy = int(v_Uv[1] * line_height * 0.99);
    int y = cy + int(line_voffset);
    bool hires = false;

    vec4 hposp = vec4(gtia[cy].hposp) * 2.0 + vec4(line_width / 2.0 - 256.0);
    vec4 hposm = vec4(gtia[cy].hposm) * 2.0 + vec4(line_width / 2.0 - 256.0);

    int color_reg_index = 0; // bg_color
    if(mode == 0x0 || px < 0.0 || px >= line_width) {
        color_reg_index = 0;
    } else if(mode == 0x2 || mode == 0x3) { // TODO - proper support for 0x3
        float w = px_scrolled / 8.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 7 - int(frac * 8.0);

        int c = get_byte(data, n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_byte(charset, offs);

        int pixel_val = (((byte >> x) & 1) ^ inv);

        color_reg_index = 3 - pixel_val;  // pf2 pf1
        hires = true;
    } else if(mode == 0x04 || mode == 0x05) {
        float w = px_scrolled / 8.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 6 - int(frac * 4.0) * 2;

        int c = get_byte(data, n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_byte(charset, offs);

        color_reg_index = (byte >> x) & 3;
        if(inv != 0 && color_reg_index == 3) {
            color_reg_index = 4;
        };
    } else if(mode == 0x6 || mode == 0x7) {
        float w = px_scrolled / 16.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 7 - int(frac * 8.0);

        int c = get_byte(data, n);
        int cc = c >> 6;
        int offs = (c & 0x3f) * 8 + (mode == 6 ? y : y / 2);
        int byte = get_byte(charset, offs);

        if(((byte >> x) & 1) > 0) {
            color_reg_index = cc + 1;
        } else {
            color_reg_index = 0;
        };
    } else if(mode == 0xa) {
        float w = px_scrolled / 16.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 6-int(frac * 4.0) * 2; // bit offset in byte

        int byte = get_byte(data, n);
        color_reg_index = (byte >> bit_offs) & 3;
    } else if(mode == 0xb || mode == 0xc) {
        float w = px_scrolled / 16.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 7-int(frac * 8.0); // bit offset in byte

        int byte = get_byte(data, n);
        color_reg_index = (byte >> bit_offs) & 1;
    } else if(mode == 0x0d || mode == 0xe) {
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

    int prior = gtia[cy].prior[0];
    bool pri0 = (prior & 1) > 0;
    bool pri1 = (prior & 2) > 0;
    bool pri2 = (prior & 4) > 0;
    bool pri3 = (prior & 8) > 0;

    bool pri01 = pri0 || pri1;
    bool pri12 = pri1 || pri2;
    bool pri23 = pri2 || pri3;
    bool pri03 = pri0 || pri3;

    bool m0 = get_missile_pixel(0, px, cy, hposm);
    bool m1 = get_missile_pixel(1, px, cy, hposm);
    bool m2 = get_missile_pixel(2, px, cy, hposm);
    bool m3 = get_missile_pixel(3, px, cy, hposm);

    bool p5 = (prior & 0x10) > 0;

    bool p0 = get_player_pixel(0, px, cy, hposp) || !p5 && m0;
    bool p1 = get_player_pixel(1, px, cy, hposp) || !p5 && m1;
    bool p2 = get_player_pixel(2, px, cy, hposp) || !p5 && m2;
    bool p3 = get_player_pixel(3, px, cy, hposp) || !p5 && m3;

    bool pf0 = color_reg_index == 1;
    bool pf1 = !hires && color_reg_index == 2;
    bool pf2 = hires || color_reg_index == 3;
    bool pf3 = color_reg_index == 4 || p5 && (m0 || m1 || m2 || m3);

    bool p01 = p0 || p1;
    bool p23 = p2 || p3;
    bool pf01 = pf0 || pf1;
    bool pf23 = pf2 || pf3;

    bool multi = (prior & 0x20) > 0;

    bool sp0 = p0 && !(pf01 && pri23) && !(pri2 && pf23);
    bool sp1 = p1  &&  !(pf01 && pri23) && !(pri2 && pf23)  &&  (!p0 || multi);
    bool sp2 = p2  &&  !p01  &&  !(pf23 && pri12) && !(pf01 && !pri0);
    bool sp3 = p3  &&  !p01  &&  !(pf23 && pri12) && !(pf01 && !pri0)  &&  (!p2 || multi);
    bool sf3 = pf3  &&  !(p23 && pri03)  &&  !(p01 && !pri2);
    bool sf0 = pf0  &&  !(p23 && pri0)  &&  !(p01 && pri01)  &&  !sf3;
    bool sf1 = pf1  &&  !(p23 && pri0)  &&  !(p01 && pri01)  &&  !sf3;
    bool sf2 = pf2  &&  !(p23 && pri03)  &&  !(p01 && !pri2)  &&  !sf3;
    bool sb = !p01  &&  !p23  &&  !pf01  &&  !pf23;


    int color_reg = 0;
    if(sp0) color_reg |= gtia[cy].colpm[0];
    if(sp1) color_reg |= gtia[cy].colpm[1];
    if(sp2) color_reg |= gtia[cy].colpm[2];
    if(sp3) color_reg |= gtia[cy].colpm[3];
    if(sf0) color_reg |= get_color_reg(cy, 1);
    if(sf1) color_reg |= get_color_reg(cy, 2);
    if(sf2) color_reg |= get_color_reg(cy, 3);
    if(sf3) color_reg |= get_color_reg(cy, 4);
    if(sb) color_reg |= get_color_reg(cy, 0);
    if(hires && color_reg_index == 2) {
        color_reg = color_reg & 0xf0 | (get_color_reg(cy, 2) & 0xf);
    }
    o_ColorTarget = encodeColor(palette[color_reg]);
    // o_Target = vec4((pf0 || true) && (p0 || p1) ? 1.0 : 0.0, pf1 && (p0 || p1) ? 1.0 : 0.0, pf2 && (p0 || p1) ? 1.0 : 0.0, 1.0);

    int pf_bits = (pf0 ? 1 : 0) | (pf1 ? 2 : 0) | (pf2 ? 4 : 0) | (pf3 ? 8 : 0);

    int p0pf = p0 ? pf_bits : 0;
    int p1pf = p1 ? pf_bits << 4 : 0;
    int p2pf = p2 ? pf_bits << 8 : 0;
    int p3pf = p3 ? pf_bits << 12 : 0;

    int m0pf = m0 ? pf_bits : 0;
    int m1pf = m1 ? pf_bits << 4 : 0;
    int m2pf = m2 ? pf_bits << 8 : 0;
    int m3pf = m3 ? pf_bits << 12 : 0;

    int player_bits = int(p0) | (int(p1) << 1) | (int(p2) << 2) | (int(p3) << 3);

    int m0pl = m0 ? player_bits : 0;
    int m1pl = m1 ? player_bits << 4 : 0;
    int m2pl = m2 ? player_bits << 8 : 0;
    int m3pl = m3 ? player_bits << 12 : 0;

    int p0pl = p0 ? player_bits & ~1 : 0;
    int p1pl = p1 ? (player_bits & ~2) << 4 : 0;
    int p2pl = p2 ? (player_bits & ~4) << 8 : 0;
    int p3pl = p3 ? (player_bits & ~8) << 12 : 0;

    o_CollisionsTarget = uvec4(
        uint(m0pf | m1pf | m2pf | m3pf) | (uint(p0pf | p1pf | p2pf | p3pf) << 16),
        uint(m0pl | m1pl | m2pl | m3pl) | (uint(p0pl | p1pl | p2pl | p3pl) << 16),
        0,
        0
    );
}
