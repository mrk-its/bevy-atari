#version 300 es
precision highp float;
precision highp int;
precision highp usampler2D;

in vec3 v_Position;
in vec2 v_Uv;
flat in vec4 v_Custom;

layout(location = 0) out vec4 o_ColorTarget;
layout(location = 1) out uvec4 o_CollisionsTarget;

layout(std140) uniform CameraViewProj {
    mat4 ViewProj;
};

struct GTIA {
    ivec4 color_regs[2];
    ivec4 colpm;
    ivec4 hposp;
    ivec4 hposm;
    ivec4 player_size;
    ivec4 missile_size;
    ivec4 grafp;
    ivec4 prior;  // [prior, unused, grafm, unused]
};

layout(std140) uniform AtariPalette_palette { // set=1 binding = 0
    vec4 palette[256];
};

layout(std140) uniform AnticData_gtia_regs { // set=3 binding = 0
    GTIA gtia_regs[240];
};

uniform usampler2D SimpleMaterial_base_color_texture;  // set = 4, binding = 0

#define get_color_reg(k) gtia_regs[scan_line].color_regs[k>>2][k&3]
#define get_gtia_colpm(k) gtia_regs[scan_line].colpm[k]
#define get_gtia_prior() gtia_regs[scan_line].prior[0]
#define get_gtia_hposp_vec4() vec4(gtia_regs[scan_line].hposp)
#define get_gtia_hposm_vec4() vec4(gtia_regs[scan_line].hposm)
#define get_gtia_player_size_vec4() vec4(gtia_regs[scan_line].player_size)
#define get_gtia_missile_size_vec4() vec4(gtia_regs[scan_line].missile_size)
#define get_gtia_grafp_ivec4() gtia_regs[scan_line].grafp
#define get_gtia_grafm() gtia_regs[scan_line].prior[2]


#define uint_byte(i, k) int((i >> (8 * k)) & uint(0xff))

#define get_texture_byte(offset) ((int(texelFetch(SimpleMaterial_base_color_texture, ivec2(((offset)>>4) & 0xff, (offset >> 12)), 0)[(offset >> 2) & 3]) >> (((offset) & 3) * 8)) & 0xff)
#define get_charset_memory(offset) get_texture_byte(charset_memory_offset + offset)
#define get_video_memory(offset) get_texture_byte(video_memory_offset + offset)

vec4 encodeSRGB(vec4 linearRGB_in)
{
    vec3 linearRGB = linearRGB_in.rgb;
    vec3 a = 12.92 * linearRGB;
    vec3 b = 1.055 * pow(linearRGB, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), linearRGB);
    return vec4(mix(a, b, c), linearRGB_in.a);
}

// #define encodeColor(x) encodeSRGB(x)
#define encodeColor(x) (x)

vec4 get_player_pixels(vec4 px, int scan_line, vec4 hpos) {
    vec4 psize = get_gtia_player_size_vec4();
    vec4 cond = vec4(greaterThanEqual(px, hpos)) * vec4(lessThan(px, hpos + psize));
    ivec4 pl_bit = ivec4(mix(vec4(7.9999), vec4(0.0), (px - hpos) / psize));
    ivec4 byte = get_gtia_grafp_ivec4();
    return mix(vec4(0.0), vec4(greaterThan((byte >> pl_bit) & 1, ivec4(0))), cond);
}

const ivec4 missile_shift = ivec4(0, 2, 4, 6);

vec4 get_missile_pixels(vec4 px, int scan_line, vec4 hpos) {
    vec4 msize = get_gtia_missile_size_vec4();
    vec4 cond = vec4(greaterThanEqual(px, hpos)) * vec4(lessThan(px, hpos + msize));
    ivec4 bit = ivec4(mix(vec4(1.9999), vec4(0.0), (px - hpos) / msize));
    ivec4 byte = ivec4(get_gtia_grafm()) >> missile_shift;
    return mix(vec4(0.0), vec4(greaterThan((byte >> bit) & 1, ivec4(0))), cond);
}

void main() {
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

    vec4 hpos_offs = vec4(line_width / 2.0 - 256.0);
    vec4 hposp = get_gtia_hposp_vec4() * 2.0 + hpos_offs;
    vec4 hposm = get_gtia_hposm_vec4() * 2.0 + hpos_offs;

    int color_reg_index = 0; // bg_color
    int prior = get_gtia_prior();
    int gtia_mode = prior >> 6;
    int color_reg = 0;

    if(mode == 0x0 || px < 0.0 || px >= line_width) {

    } else if(mode == 0x2 || mode == 0x3) { // TODO - proper support for 0x3
        float w = px_scrolled / 8.0;
        int n = int(w);
        float frac = w - float(n);

        int c = get_video_memory(n);
        int inv = c >> 7;
        int offs = (c & 0x7f) * 8 + y;
        int byte = get_charset_memory(offs);

        if(gtia_mode == 0) {
            int bit_offs = 7 - int(frac * 8.0);
            int pixel_val = (((byte >> bit_offs) & 1) ^ inv);
            color_reg_index = 3 - pixel_val;  // pf2 pf1
            hires = true;
        } else {
            int bit_offs = 4-int(frac * 2.0) * 4; // nibble offset
            int value = (byte >> bit_offs) & 0xf;
            if(gtia_mode == 1) {
                color_reg = value | get_color_reg(0) & 0xf0;
            } else if(gtia_mode == 3) {
                color_reg = value << 4;
                if(color_reg>0) color_reg |= get_color_reg(0) & 0xf;
            } else if(gtia_mode == 2) {
                if(value < 4) {
                    color_reg_index = value + 1;
                } else if(value < 8) {
                    color_reg = get_gtia_colpm(value - 4);
                } else {
                    color_reg = get_color_reg(0);
                }
            };
        };
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
        int byte = get_video_memory(n);

        if(gtia_mode == 0) {
            int bit_offs = 7-int(frac * 8.0); // bit offset in byte
            int pixel_val = (byte >> bit_offs) & 1;
            color_reg_index = 3 - pixel_val;
            hires = true;
        } else {
            int bit_offs = 4-int(frac * 2.0) * 4; // nibble offset
            int value = (byte >> bit_offs) & 0xf;
            if(gtia_mode == 1) {
                color_reg = value | get_color_reg(0) & 0xf0;
            } else if(gtia_mode == 3) {
                color_reg = value << 4;
                if(color_reg>0) color_reg |= get_color_reg(0) & 0xf;
            } else if(gtia_mode == 2) {
                if(value < 4) {
                    color_reg_index = value + 1;
                } else if(value < 8) {
                    color_reg = get_gtia_colpm(value - 4);
                } else {
                    color_reg = get_color_reg(0);
                }
            };
        };
    };

    bool pri0 = (prior & 1) > 0;
    bool pri1 = (prior & 2) > 0;
    bool pri2 = (prior & 4) > 0;
    bool pri3 = (prior & 8) > 0;

    bool pri01 = pri0 || pri1;
    bool pri12 = pri1 || pri2;
    bool pri23 = pri2 || pri3;
    bool pri03 = pri0 || pri3;

    vec4 vpx = vec4(px);
    vec4 m = get_missile_pixels(vpx, scan_line, hposm);
    bool m0 = m[0] > 0.0;
    bool m1 = m[1] > 0.0;
    bool m2 = m[2] > 0.0;
    bool m3 = m[3] > 0.0;

    bool p5 = (prior & 0x10) > 0;

    vec4 p = get_player_pixels(vpx, scan_line, hposp);

    bvec4 _p = bvec4(p + float(!p5) * m);
    bool p0 = _p[0];
    bool p1 = _p[1];
    bool p2 = _p[2];
    bool p3 = _p[3];

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


    if(sp0) color_reg |= get_gtia_colpm(0);
    if(sp1) color_reg |= get_gtia_colpm(1);
    if(sp2) color_reg |= get_gtia_colpm(2);
    if(sp3) color_reg |= get_gtia_colpm(3);
    if(sf0) color_reg |= get_color_reg(1);
    if(sf1) color_reg |= get_color_reg(2);
    if(sf2) color_reg |= get_color_reg(3);
    if(sf3) color_reg |= get_color_reg(4);
    if(sb && gtia_mode == 0) color_reg |= get_color_reg(0);

    if(hires && color_reg_index == 2) {
        color_reg = color_reg & 0xf0 | (get_color_reg(2) & 0xf);
    }

    o_ColorTarget = encodeColor(palette[color_reg]);
    // if(p3) o_ColorTarget = encodeColor(vec4(1.0, 0.0, 0.0, 1.0));

    // TODO - do not check collisions on HBLANK

    p0 = bool(p[0]);
    p1 = bool(p[1]);
    p2 = bool(p[2]);
    p3 = bool(p[3]);

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
        uint(m0pf | m1pf | m2pf | m3pf) | uint(p0pf | p1pf | p2pf | p3pf) << 16,
        uint(m0pl | m1pl | m2pl | m3pl) | uint(p0pl | p1pl | p2pl | p3pl) << 16,
        0,
        0
    );
}
