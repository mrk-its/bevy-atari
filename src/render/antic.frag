#version 300 es
precision highp float;
precision highp int;
precision highp usampler2D;

in vec3 v_Position;
in vec2 v_Uv;
flat in vec4 v_Custom;

layout(location = 0) out vec4 o_ColorTarget;
layout(location = 1) out uvec4 o_CollisionsTarget;

layout(std140) uniform Camera {
    mat4 ViewProj;
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

layout(std140) uniform AtariPalette_palette { // set=1 binding = 0
    vec4 palette[256];
};

layout(std140) uniform AnticData_gtia_regs { // set=3 binding = 0
    GTIA gtia_regs[240];
};

layout(std140) uniform StandardMaterial_albedo { // set = 4, binding = 0
    vec4 Albedo;
};

//#ifdef STANDARDMATERIAL_ALBEDO_TEXTURE
uniform usampler2D StandardMaterial_albedo_texture;  // set = 4, binding = 1
//#endif

#define get_color_reg(line, k) gtia_regs[line].color_regs[k>>2][k&3]
#define uint_byte(i, k) int((i >> (8 * k)) & uint(0xff))

vec4 encodeSRGB(vec4 linearRGB_in)
{
    vec3 linearRGB = linearRGB_in.rgb;
    vec3 a = 12.92 * linearRGB;
    vec3 b = 1.055 * pow(linearRGB, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), linearRGB);
    return vec4(mix(a, b, c), linearRGB_in.a);
}

#define get_byte(data, offset) (int(data[(offset) / 16][((offset) / 4) & 3] >> (((offset) & 3) * 8)) & 255)
#define _get_video_memory(offset) get_byte(video_memory, video_memory_offset + offset)
#define _get_charset_memory(offset) get_byte(charset_memory, charset_memory_offset + offset)
// #define encodeColor(x) encodeSRGB(x)
#define encodeColor(x) (x)

#define get_texture_byte(offset) ((int(texelFetch(StandardMaterial_albedo_texture, ivec2(((offset)>>2) & 0xff, (offset >> 10)), 0)[0]) >> (((offset) & 3) * 8)) & 0xff)
#define get_charset_memory(offset) get_texture_byte(charset_memory_offset + offset)
#define get_video_memory(offset) get_texture_byte(video_memory_offset + offset)

bool get_player_pixel(int n, float px, int scan_line, vec4 hpos) {
    if (px >= hpos[n] && px < hpos[n] + float(gtia_regs[scan_line].player_size[n])) {
        int pl_bit = 7 - int((px - hpos[n]) / float(gtia_regs[scan_line].player_size[n]) * 8.0);
        int byte = gtia_regs[scan_line].grafp[n];
        return ((byte >> pl_bit) & 1) > 0;
    }
    return false;
}

bool get_missile_pixel(int n, float px, int scan_line, vec4 hpos) {
    float sizem = float(gtia_regs[scan_line].prior[1]);
    if (px >= hpos[n] && px < hpos[n] + sizem) {
        int bit = 1 - int((px - hpos[n]) / sizem * 2.0);
        int byte = gtia_regs[scan_line].prior[2] >> (n * 2);
        return ((byte >> bit) & 1) > 0;
    }
    return false;
}

void main() {
    uvec4 t = texelFetch(StandardMaterial_albedo_texture, ivec2(1, 0), 0);
    // vec4 output_color = Albedo;
    int mode = uint_byte(uint(v_Custom[0]), 0);
    int start_scan_line = uint_byte(uint(v_Custom[0]), 1);
    int line_height = uint_byte(uint(v_Custom[0]), 2);

    int hscrol = uint_byte(uint(v_Custom[1]), 0);
    int line_voffset = uint_byte(uint(v_Custom[1]), 1);
    float line_width = float(uint_byte(uint(v_Custom[1]), 2)) * 2.0;

    int video_memory_offset = int(v_Custom[2]);
    int charset_memory_offset = int(v_Custom[3]);

    float x = v_Uv[0] * 384.0;
    float px = x - 192.0 + line_width / 2.0;

    float px_scrolled = px + float(hscrol);  // pixel x position
    int cy = int(v_Uv[1] * float(line_height) * 0.99);
    int y = cy + line_voffset;
    bool hires = false;

    int scan_line = start_scan_line + cy;

    vec4 hposp = vec4(gtia_regs[scan_line].hposp) * 2.0 + vec4(line_width / 2.0 - 256.0);
    vec4 hposm = vec4(gtia_regs[scan_line].hposm) * 2.0 + vec4(line_width / 2.0 - 256.0);

    int color_reg_index = 0; // bg_color

    if(mode == 0x0 || px < 0.0 || px >= line_width) {

    } else if(mode == 0x2 || mode == 0x3) { // TODO - proper support for 0x3
        float w = px_scrolled / 8.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 7 - int(frac * 8.0);

        int c = get_video_memory(n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_charset_memory(offs);

        int pixel_val = (((byte >> x) & 1) ^ inv);

        color_reg_index = 3 - pixel_val;  // pf2 pf1
        hires = true;
    } else if(mode == 0x04 || mode == 0x05) {
        float w = px_scrolled / 8.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 6 - int(frac * 4.0) * 2;

        int c = get_video_memory(n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_charset_memory(offs);

        color_reg_index = (byte >> x) & 3;
        if(inv != 0 && color_reg_index == 3) {
            color_reg_index = 4;
        };
    } else if(mode == 0x6 || mode == 0x7) {
        float w = px_scrolled / 16.0;
        int n = int(w);
        float frac = w - float(n);
        int x = 7 - int(frac * 8.0);

        int c = get_video_memory(n);
        int cc = c >> 6;
        int offs = (c & 0x3f) * 8 + (mode == 6 ? y : y / 2);
        int byte = get_charset_memory(offs);

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

        int byte = get_video_memory(n);
        color_reg_index = (byte >> bit_offs) & 3;
    } else if(mode == 0xb || mode == 0xc) {
        float w = px_scrolled / 16.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 7-int(frac * 8.0); // bit offset in byte

        int byte = get_video_memory(n);
        color_reg_index = (byte >> bit_offs) & 1;
    } else if(mode == 0x0d || mode == 0xe) {
        float w = px_scrolled / 8.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 6-int(frac * 4.0) * 2; // bit offset in byte

        int byte = get_video_memory(n);
        color_reg_index = (byte >> bit_offs) & 3;
    } else if(mode == 0x0f) {

        float w = px_scrolled / 8.0;
        int n = int(w); // byte offset
        float frac = w - float(n);
        int bit_offs = 7-int(frac * 8.0); // bit offset in byte

        int byte = get_video_memory(n);
        int pixel_val = (byte >> bit_offs) & 1;
        color_reg_index = 3 - pixel_val;
        hires = true;
    };

    int prior = gtia_regs[scan_line].prior[0];
    bool pri0 = (prior & 1) > 0;
    bool pri1 = (prior & 2) > 0;
    bool pri2 = (prior & 4) > 0;
    bool pri3 = (prior & 8) > 0;

    bool pri01 = pri0 || pri1;
    bool pri12 = pri1 || pri2;
    bool pri23 = pri2 || pri3;
    bool pri03 = pri0 || pri3;

    bool m0 = get_missile_pixel(0, px, scan_line, hposm);
    bool m1 = get_missile_pixel(1, px, scan_line, hposm);
    bool m2 = get_missile_pixel(2, px, scan_line, hposm);
    bool m3 = get_missile_pixel(3, px, scan_line, hposm);

    bool p5 = (prior & 0x10) > 0;

    bool p0 = get_player_pixel(0, px, scan_line, hposp) || !p5 && m0;
    bool p1 = get_player_pixel(1, px, scan_line, hposp) || !p5 && m1;
    bool p2 = get_player_pixel(2, px, scan_line, hposp) || !p5 && m2;
    bool p3 = get_player_pixel(3, px, scan_line, hposp) || !p5 && m3;

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
    if(sp0) color_reg |= gtia_regs[scan_line].colpm[0];
    if(sp1) color_reg |= gtia_regs[scan_line].colpm[1];
    if(sp2) color_reg |= gtia_regs[scan_line].colpm[2];
    if(sp3) color_reg |= gtia_regs[scan_line].colpm[3];
    if(sf0) color_reg |= get_color_reg(scan_line, 1);
    if(sf1) color_reg |= get_color_reg(scan_line, 2);
    if(sf2) color_reg |= get_color_reg(scan_line, 3);
    if(sf3) color_reg |= get_color_reg(scan_line, 4);
    if(sb) color_reg |= get_color_reg(scan_line, 0);
    if(hires && color_reg_index == 2) {
        color_reg = color_reg & 0xf0 | (get_color_reg(scan_line, 2) & 0xf);
    }
    o_ColorTarget = encodeColor(palette[color_reg]);

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

    if(x >= 0.0) {
        o_CollisionsTarget = uvec4(
            uint(m0pf | m1pf | m2pf | m3pf) | uint(p0pf | p1pf | p2pf | p3pf) << 16,
            uint(m0pl | m1pl | m2pl | m3pl) | uint(p0pl | p1pl | p2pl | p3pl) << 16,
            0,
            0
        );
    } else {
        o_CollisionsTarget = uvec4(0, 0, 0, 0);
    }


    // vec4 output_color = palette[get_color_reg(scan_line, color_reg_index)];
    // // multiply the light by material color
    // o_Target = encodeSRGB(output_color);
    // // o_Target = output_color;
}
