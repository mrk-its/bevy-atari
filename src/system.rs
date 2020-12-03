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
    pub fn handle_keyboard(&mut self, keyboard: &Res<Input<KeyCode>>) -> bool {
        let mut irq = false;
        let is_shift = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
        let is_ctl = keyboard.pressed(KeyCode::LControl) || keyboard.pressed(KeyCode::RControl);
        let mut joy_changed = false;
        for ev in keyboard.get_just_pressed() {
            irq = irq || self.pokey.key_press(ev, true, is_shift, is_ctl);
            joy_changed = joy_changed
                || *ev == KeyCode::LShift
                || *ev == KeyCode::Up
                || *ev == KeyCode::Down
                || *ev == KeyCode::Left
                || *ev == KeyCode::Right;
        }

        for ev in keyboard.get_just_released() {
            self.pokey.key_press(ev, false, is_shift, is_ctl);
            joy_changed = joy_changed
                || *ev == KeyCode::LShift
                || *ev == KeyCode::Up
                || *ev == KeyCode::Down
                || *ev == KeyCode::Left
                || *ev == KeyCode::Right;
        }
        if joy_changed {
            let fire = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
            let up = keyboard.pressed(KeyCode::Up);
            let down = keyboard.pressed(KeyCode::Down);
            let left = keyboard.pressed(KeyCode::Left);
            let right = keyboard.pressed(KeyCode::Right);
            self.set_joystick(0, up, down, left, right, fire);
            irq = false;
        }
        irq
    }
    pub fn set_joystick(
        &mut self,
        port: usize,
        up: bool,
        down: bool,
        left: bool,
        right: bool,
        fire: bool,
    ) {
        info!("set_joystick {} {} {} {} {}", up, down, left, right, fire);
        self.gtia.set_trig(port, fire);
        let up = up as u8;
        let down = down as u8 * 2;
        let left = left as u8 * 4;
        let right = right as u8 * 8;
        self.pia.write_port(0, 0xf0, (up | down | left | right)^0xf);
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
