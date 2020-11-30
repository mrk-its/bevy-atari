pub use crate::{antic::Antic, gtia::Gtia, pia::PIA, pokey::Pokey};
pub use bevy::prelude::*;
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
    pub fn handle_keyboard(&mut self, keyboard: &Res<Input<KeyCode>>) {
        let is_shift = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
        let up = keyboard.pressed(KeyCode::Up) as u8;
        let down = keyboard.pressed(KeyCode::Down) as u8 * 2;
        let left = keyboard.pressed(KeyCode::Left) as u8 * 4;
        let right = keyboard.pressed(KeyCode::Right) as u8 * 8;

        self.gtia.set_trig(is_shift);
        self.pia.set_joystick(0, (up | down | left | right) ^ 0x0f);
    }
}

impl Default for AtariSystem {
    fn default() -> Self {
        AtariSystem::new()
    }
}

impl w65c02s::System for AtariSystem {
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
