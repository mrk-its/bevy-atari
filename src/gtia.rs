use crate::palette::default::PALETTE;
use crate::render_resources::GTIAColors;
use bevy::prelude::Color;
use bevy::prelude::{info, warn};

pub const COLPF0: usize = 0x16;
pub const COLPF1: usize = 0x17;
pub const COLPF2: usize = 0x18;
pub const COLPF3: usize = 0x19;
pub const COLBK: usize = 0x1a;
pub const HPOSP0: usize = 0x00;
pub const HPOSP1: usize = 0x01;
pub const HPOSP2: usize = 0x02;
pub const HPOSP3: usize = 0x03;

pub const TRIG0: usize = 0x10;
pub const TRIG1: usize = 0x11;
pub const TRIG2: usize = 0x12;
pub const TRIG3: usize = 0x13;

pub struct Gtia {
    reg: [u8; 0x20],
}

impl Default for Gtia {
    fn default() -> Self {
        let reg = [0xFF; 0x20];
        Self { reg }
    }
}

impl Gtia {
    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0x1f;
        let value = match addr {
            TRIG0 => self.reg[addr],
            TRIG3 => 0,
            _ => self.reg[addr],
        };
        // warn!("GTIA read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0x1f;
        self.reg[addr] = value;
        // warn!("GTIA write: {:02x}: {:02x}", addr, value);
        // if addr >= COLPF0 && addr <= COLBK {
        //     warn!("GTIA color write: {:02x}: {:02x}", addr, value);
        // }
    }
    pub fn set_trig(&mut self, is_pressed: bool) {
        self.reg[TRIG0] = if is_pressed {0} else {0xff};
    }
    pub fn get_colors(&self) -> GTIAColors {
        // HPOSP0-HPOSP3 [D000-D003]
        // HPOSM0-HPOSM3 [D004-D007]
        // SIZEP0-SIZEP3 [D008-D00B]
        let overwrite_robbo_bg = self.reg[HPOSP0] == 0x40
            && self.reg[HPOSP1] == 0x60
            && self.reg[HPOSP2] == 0x80
            && self.reg[HPOSP3] == 0xa0;
        let bgcol_idx = if !overwrite_robbo_bg { COLBK } else { 0x12 };
        GTIAColors::new(
            self.reg[bgcol_idx],
            self.reg[COLPF0],
            self.reg[COLPF1],
            self.reg[COLPF2],
            self.reg[COLPF3],
        )
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
