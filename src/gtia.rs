use crate::palette::default::PALETTE;
use crate::render_resources::GTIARegs;
use bevy::prelude::Color;
use bevy::utils::tracing::*;

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
pub const GRACTL: usize = 0x1d;
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

pub struct Gtia {
    reg: [u8; 0x20],
    pub player_graphics: [u8; 4],
    pub missile_graphics: u8,
    collisions: [u8; 0x16], // R
    trig: [u8; 4],          // R
    prior: u8,
    p0pos: u8,
    enable_log: bool,
}

impl Default for Gtia {
    fn default() -> Self {
        Self {
            reg: [0x00; 0x20],
            collisions: [0x00; 0x16],
            missile_graphics: 0x00,
            player_graphics: [0x00; 4],
            trig: [0xff, 0xff, 0xff, 0],
            prior: 0,
            p0pos: 0,
            enable_log: false,
        }
    }
}

impl Gtia {
    pub fn read(&mut self, addr: usize) -> u8 {
        let addr = addr & 0x1f;
        let value = match addr {
            0x0..=0xf => {
                //info!("reading collisions at {:x}", addr);
                if addr == 6 || addr == 7 {
                    // Player2/3 collisions with playfield, for Fred
                    0xff
                } else {
                    self.collisions[addr]
                    // if self.p0pos == 0x60 {
                    //     self.p0pos = 0;
                    //     0xff
                    // } else {
                    //     0
                    // }
                }
            }
            CONSOL => 0x0f,
            TRIG0..=TRIG3 => self.trig[addr - TRIG0],
            PAL => 0x01, // 0x01 - PAL, 0x0f - NTSC
            _ => self.reg[addr],
        };
        // warn!("GTIA read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0x1f;
        self.reg[addr] = value;

        match addr {
            GRAFP0..=GRAFP3 => self.player_graphics[addr - GRAFP0] = value,
            GRAFM => self.missile_graphics = value,
            PRIOR => self.prior = value,
            HPOSP0 => {
                if value >= 0x40 && value < 0xc0 {
                    self.p0pos = value;
                }
            }
            // HITCLR => {
            //     for i in 0..=0xf {
            //         self.reg[i] = 0;
            //     }
            // }
            _ => (),
        }
        // if addr == HPOSP0 || addr == HPOSP1 || addr == HPOSP2 || addr == HPOSP1 {
        //     warn!(
        //         "player positions: {:02x} {:02x} {:02x} {:02x}",
        //         self.reg[HPOSP0], self.reg[HPOSP1], self.reg[HPOSP2], self.reg[HPOSP3],
        //     );
        // }
        if self.enable_log {
            warn!("GTIA write: {:02x}: {:02x}", addr, value);
        }
        // if addr >= COLPF0 && addr <= COLBK {
        //     warn!("GTIA color write: {:02x}: {:02x}", addr, value);
        // }
    }
    pub fn set_trig(&mut self, n: usize, is_pressed: bool) {
        self.trig[n] = if is_pressed { 0 } else { 0xff };
    }
    pub fn get_colors(&self) -> GTIARegs {
        // HPOSP0-HPOSP3 [D000-D003]
        // HPOSM0-HPOSM3 [D004-D007]
        // SIZEP0-SIZEP3 [D008-D00B]

        GTIARegs::new(
            self.reg[COLBK],
            self.reg[COLPF0],
            self.reg[COLPF1],
            self.reg[COLPF2],
            self.reg[COLPF3],
            self.reg[COLPM0],
            self.reg[COLPM1],
            self.reg[COLPM2],
            self.reg[COLPM3],
            self.reg[HPOSP0],
            self.reg[HPOSP1],
            self.reg[HPOSP2],
            self.reg[HPOSP3],
            self.reg[SIZEP0],
            self.reg[SIZEP1],
            self.reg[SIZEP2],
            self.reg[SIZEP3],
            self.prior,
        )
    }
    pub fn enable_log(&mut self, enable: bool) {
        self.enable_log = enable;
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
