use bevy::prelude::{info, warn};
use crate::color_set::{atari_color, ColorSet};

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
        // warn!("GTIA write: {:02x}: {:02x}", addr, value);
        self.reg[addr] = value;
        // if addr >= COLPF0 && addr <= COLBK {
        //     warn!("GTIA color write: {:02x}: {:02x}", addr, value);
        // }

    }
    pub fn get_color_set(&self) -> ColorSet {
        ColorSet {
            c0: atari_color(self.reg[COLPF2]),
            c1: atari_color(self.reg[COLPF2] & 0xf0 | self.reg[COLPF1] & 0x0f),
            c0_0: atari_color(self.reg[COLBK]),
            c1_0: atari_color(self.reg[COLPF0]),
            c2_0: atari_color(self.reg[COLPF1]),
            c3_0: atari_color(self.reg[COLPF2]),
            c0_1: atari_color(self.reg[COLBK]),
            c1_1: atari_color(self.reg[COLPF0]),
            c2_1: atari_color(self.reg[COLPF1]),
            c3_1: atari_color(self.reg[COLPF3]),
        }
    }
}
