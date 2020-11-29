pub use crate::{antic::Antic, gtia::Gtia, pia::PIA, pokey::Pokey};
pub use bevy::prelude::{info, warn};
pub use std::{cell::RefCell, rc::Rc};
pub use w65c02s::*;

pub struct AtariSystem {
    pub ram: [u8; 65536],
    pub antic: Antic,
    pub gtia: Gtia,
    pub pokey: Pokey,
    pub pia: PIA,
}

impl AtariSystem {
    pub fn new() -> AtariSystem {
        // initialize RAM with all 0xFFs
        let ram = [0xFF; 65536];
        let antic = Antic::default();
        let pokey = Pokey::default();
        let gtia = Gtia::default();
        let pia = PIA::default();
        AtariSystem {
            ram,
            antic,
            gtia,
            pokey,
            pia,
        }
    }
}

impl Default for AtariSystem {
    fn default() -> Self {
        AtariSystem::new()
    }
}

impl System for AtariSystem {
    fn read(&mut self, _cpu: &mut W65C02S, addr: u16) -> u8 {
        // all reads return RAM values directly
        let addr = addr as usize;
        match addr >> 8 {
            0xD0 => self.gtia.read(addr),
            0xD2 => self.pokey.read(addr),
            0xD3 => self.pia.read(addr),
            0xD4 => self.antic.read(addr),
            _ => self.ram[addr],
        }
    }
    fn write(&mut self, _cpu: &mut W65C02S, addr: u16, value: u8) {
        let addr = addr as usize;
        match addr >> 8 {
            0xD0 => self.gtia.write(addr, value),
            0xD2 => self.pokey.write(addr, value),
            0xD3 => self.pia.write(addr, value),
            0xD4 => self.antic.write(addr, value),
            _ => self.ram[addr] = value,
        }
    }
}
