use crate::palette::default::PALETTE;
use crate::render_resources::GTIARegs;
use bevy::prelude::*;

// WRITE
pub const HPOSP0: usize = 0x00;
pub const HPOSP1: usize = 0x01;
pub const HPOSP2: usize = 0x02;
pub const HPOSP3: usize = 0x03;
pub const HPOSM0: usize = 0x04;
pub const HPOSM1: usize = 0x05;
pub const HPOSM2: usize = 0x06;
pub const HPOSM3: usize = 0x07;
pub const SIZEP0: usize = 0x08;
pub const SIZEP1: usize = 0x09;
pub const SIZEP2: usize = 0x0a;
pub const SIZEP3: usize = 0x0b;
pub const SIZEM: usize = 0x0c;
pub const GRAFP0: usize = 0x0d;
pub const GRAFP1: usize = 0x0e;
pub const GRAFP2: usize = 0x0f;
pub const GRAFP3: usize = 0x10;
pub const GRAFM: usize = 0x11;
pub const COLPM0: usize = 0x12;
pub const COLPM1: usize = 0x13;
pub const COLPM2: usize = 0x14;
pub const COLPM3: usize = 0x15;
pub const COLPF0: usize = 0x16;
pub const COLPF1: usize = 0x17;
pub const COLPF2: usize = 0x18;
pub const COLPF3: usize = 0x19;
pub const COLBK: usize = 0x1a;
pub const PRIOR: usize = 0x1b;
pub const VDELAY: usize = 0x1c;
pub const _GRACTL: usize = 0x1d; // TODO - move consts to submodule
pub const HITCLR: usize = 0x1e;

pub const CONSOL: usize = 0x1f; // RW  bits 0-2:  Start/Select/Option

// READ
pub const M0PF: usize = 0x00;
pub const M1PF: usize = 0x01;
pub const M2PF: usize = 0x02;
pub const M3PF: usize = 0x03;
pub const P0PF: usize = 0x04;
pub const P1PF: usize = 0x05;
pub const P2PF: usize = 0x06;
pub const P3PF: usize = 0x07;
pub const M0PL: usize = 0x08; // ok
pub const M1PL: usize = 0x09;
pub const M2PL: usize = 0x0a;
pub const M3PL: usize = 0x0b;
pub const P0PL: usize = 0x0c; // ok
pub const P1PL: usize = 0x0d;
pub const P2PL: usize = 0x0e;
pub const P3PL: usize = 0x0f;
pub const TRIG0: usize = 0x10;
pub const TRIG1: usize = 0x11;
pub const TRIG2: usize = 0x12;
pub const TRIG3: usize = 0x13;
pub const PAL: usize = 0x14;

bitflags! {
    #[derive(Default)]
    pub struct GRACTL: u8 {
        const MISSILE_DMA = 0x01;
        const PLAYER_DMA = 0x02;
        const TRIGGER_LATCH = 0x04;
    }
}

pub struct Gtia {
    pub scan_line: usize,
    pub regs: GTIARegs,
    collisions: [u8; 0x16], // R
    trig: [u8; 4],          // R
    pub gractl: GRACTL,
    pub consol: u8,
    pub consol_mask: u8,
    pub consol_force_mask: u8,
}

impl Default for Gtia {
    fn default() -> Self {
        Self {
            regs: GTIARegs::default(),
            collisions: [0x00; 0x16],
            trig: [0xff, 0xff, 0xff, 0],
            gractl: GRACTL::from_bits_truncate(0),
            consol: 0x7,
            consol_mask: 0x7,
            consol_force_mask: 0x7, // force option on start;
            scan_line: 0,
        }
    }
}

impl Gtia {
    pub fn read(&mut self, addr: usize) -> u8 {
        let addr = addr & 0x1f;
        let value = match addr {
            0x0..=0xf => self.collisions[addr],
            CONSOL => self.consol & self.consol_mask & self.consol_force_mask,
            TRIG0..=TRIG3 => self.trig[addr - TRIG0],
            PAL => 0x01, // 0x01 - PAL, 0x0f - NTSC
            _ => 0x0f,
        };
        // warn!("GTIA read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0x1f;

        let _size_pm = |x| match x & 3 {
            1 => 32,
            3 => 64,
            _ => 16,
        };

        match addr {
            COLBK => self.regs.colors[0] = value as u32,
            COLPF0..=COLPF3 => self.regs.colors[1 + addr - COLPF0] = value as u32,
            COLPM0..=COLPM3 => self.regs.colors_pm[addr - COLPM0] = value as u32,
            GRAFP0..=GRAFP3 => self.regs.grafp[addr - GRAFP0] = value as u32,
            GRAFM => self.regs.grafm = value as u32,
            PRIOR => self.regs.prior = value as u32,
            HPOSP0..=HPOSP3 => self.regs.hposp[addr - HPOSP0] = value as u32,
            HPOSM0..=HPOSM3 => self.regs.hposm[addr - HPOSM0] = value as u32,
            SIZEP0..=SIZEP3 => self.regs.player_size[addr - SIZEP0] = _size_pm(value),
            SIZEM => self.regs.sizem = _size_pm(value) / 4,
            _GRACTL => self.gractl = GRACTL::from_bits_truncate(value),
            CONSOL => self.consol_mask = 0x7 & !value,
            HITCLR => {
                // info!("resetting collisions, scan_line: {:?}", self.scan_line);
                self.collisions.iter_mut().for_each(|v| *v = 0);
            }
            _ => (),
        }
    }
    pub fn set_trig(&mut self, n: usize, is_pressed: bool) {
        self.trig[n] = if is_pressed { 0 } else { 0xff };
    }
    pub fn update_collisions(&mut self, data: [u32; 4]) {
        // info!(
        //     "update collisions: {:?}, scanline: {:?}",
        //     data, self.scan_line
        // );

        self.collisions[M0PF] |= (data[0] & 0xf) as u8;
        self.collisions[M1PF] |= ((data[0] >> 4) & 0xf) as u8;
        self.collisions[M2PF] |= ((data[0] >> 8) & 0xf) as u8;
        self.collisions[M3PF] |= ((data[0] >> 12) & 0xf) as u8;

        self.collisions[P0PF] |= (data[1] & 0xf) as u8;
        self.collisions[P1PF] |= ((data[1] >> 4) & 0xf) as u8;
        self.collisions[P2PF] |= ((data[1] >> 8) & 0xf) as u8;
        self.collisions[P3PF] |= ((data[1] >> 12) & 0xf) as u8;

        self.collisions[M0PL] |= (data[2] & 0xf) as u8;
        self.collisions[M1PL] |= ((data[2] >> 4) & 0xf) as u8;
        self.collisions[M2PL] |= ((data[2] >> 8) & 0xf) as u8;
        self.collisions[M3PL] |= ((data[2] >> 12) & 0xf) as u8;

        self.collisions[P0PL] |= (data[3] & 0xf) as u8;
        self.collisions[P1PL] |= ((data[3] >> 4) & 0xf) as u8;
        self.collisions[P2PL] |= ((data[3] >> 8) & 0xf) as u8;
        self.collisions[P3PL] |= ((data[3] >> 12) & 0xf) as u8;

        // fred
        // if data[1] > 0 {
        //     self.collisions[P2PF] |= 0xf; // collision with any playfield color
        //     self.collisions[P3PF] |= 0xf;
        // }
    }
}

pub fn atari_color(index: u8) -> Color {
    let index = index as usize;
    Color::rgb(
        PALETTE[index][0] as f32 / 255.0,
        PALETTE[index][1] as f32 / 255.0,
        PALETTE[index][2] as f32 / 255.0,
    )
}
