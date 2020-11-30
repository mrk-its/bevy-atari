use bevy::prelude::{info, warn};
use bevy::prelude::Color;
use crate::palette::default::PALETTE;
use crate::render_resources::GTIAColors;

pub const COLPF0: usize = 0x16;
pub const COLPF1: usize = 0x17;
pub const COLPF2: usize = 0x18;
pub const COLPF3: usize = 0x19;
pub const COLBK: usize = 0x1a;

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
            0x13 => 0,
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
    pub fn get_colors(&self) -> GTIAColors {
        GTIAColors {
            colbk: atari_color(self.reg[COLBK]),
            colpf0: atari_color(self.reg[COLPF0]),
            colpf1: atari_color(self.reg[COLPF1]),
            colpf2: atari_color(self.reg[COLPF2]),
            colpf3: atari_color(self.reg[COLPF3]),
        }
    }
}

pub fn atari_color(index: u8) -> Color {
    let index = index as usize;
    Color::rgb(PALETTE[index][0] as f32 / 255.0, PALETTE[index][1] as f32 / 255.0, PALETTE[index][2] as f32 / 255.0)
}
