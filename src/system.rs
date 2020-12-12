use crate::atari800_state::Atari800State;
pub use crate::{gtia, antic};
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

    pub fn load_atari800_state(&mut self, atari800_state: &Atari800State) {
        self.ram.copy_from_slice(atari800_state.memory.data);
        let gtia = atari800_state.gtia;
        let antic = atari800_state.antic;
        let pokey = atari800_state.pokey;

        self.gtia.write(gtia::COLBK, gtia.colbk);
        self.gtia.write(gtia::COLPF0, gtia.colpf0);
        self.gtia.write(gtia::COLPF1, gtia.colpf1);
        self.gtia.write(gtia::COLPF2, gtia.colpf2);
        self.gtia.write(gtia::COLPF3, gtia.colpf3);

        self.gtia.write(gtia::COLPM0, gtia.colpm0);
        self.gtia.write(gtia::COLPM1, gtia.colpm1);
        self.gtia.write(gtia::COLPM2, gtia.colpm2);
        self.gtia.write(gtia::COLPM3, gtia.colpm3);
        self.gtia.write(gtia::HPOSP0, gtia.hposp0);
        self.gtia.write(gtia::HPOSP1, gtia.hposp1);
        self.gtia.write(gtia::HPOSP2, gtia.hposp2);
        self.gtia.write(gtia::HPOSP3, gtia.hposp3);
        self.gtia.write(gtia::SIZEP0, gtia.sizep0);
        self.gtia.write(gtia::SIZEP1, gtia.sizep1);
        self.gtia.write(gtia::SIZEP2, gtia.sizep2);
        self.gtia.write(gtia::SIZEP3, gtia.sizep3);
        self.gtia.write(gtia::P0PL, gtia.p0pl);
        self.gtia.write(gtia::P1PL, gtia.p1pl);
        self.gtia.write(gtia::P2PL, gtia.p2pl);
        self.gtia.write(gtia::P3PL, gtia.p3pl);
        self.gtia.write(gtia::M0PL, gtia.m0pl);
        self.gtia.write(gtia::M1PL, gtia.m1pl);
        self.gtia.write(gtia::M2PL, gtia.m2pl);
        self.gtia.write(gtia::M3PL, gtia.m3pl);
        self.gtia.write(gtia::M0PF, 0);
        self.gtia.write(gtia::M1PF, 0);
        self.gtia.write(gtia::M2PF, 0);
        self.gtia.write(gtia::M3PF, 0);
        self.gtia.write(gtia::P0PF, 0);
        self.gtia.write(gtia::P1PF, 0);
        self.gtia.write(gtia::P2PF, 0);
        self.gtia.write(gtia::P3PF, 0);

        self.antic.dmactl = antic::DMACTL::from_bits_truncate(antic.dmactl);
        self.antic.chactl = antic.chactl;
        self.antic.chbase = antic.chbase;
        self.antic.pmbase = antic.pmbase;

        self.antic.dlist = antic.dlist;
        self.antic.nmien = antic::NMIEN::from_bits_truncate(antic.nmien);
        self.antic.nmist = antic::NMIST::from_bits_truncate(antic.nmist);
        self.antic.pmbase = antic.pmbase;

        self.pokey.write(0x08, pokey.audctl);
        for i in 0..4 {
            self.pokey.write(i * 2, pokey.audf[i]);
            self.pokey.write(i * 2 + 1, pokey.audc[i]);
        }

        // self.pokey.write(0x08, 0);
        // self.pokey.write(0, 34);
        // self.pokey.write(1, 132);

        let dlist = self.antic.dlist as usize;
        info!(
            "DLIST: addr: {:04x} data: {:x?}",
            dlist,
            &self.ram[dlist..dlist + 64]
        );
    }

    pub fn handle_keyboard(&mut self, keyboard: &Res<Input<KeyCode>>) -> bool {
        let mut irq = false;
        let is_shift = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
        let is_ctl = keyboard.pressed(KeyCode::LControl) || keyboard.pressed(KeyCode::RControl);
        let mut joy_changed = false;
        for ev in keyboard.get_just_pressed() {
            if !is_shift && !is_ctl {
                self.pokey.resume();
            }
            irq = irq || self.pokey.key_press(ev, true, is_shift, is_ctl);
            joy_changed = joy_changed
                || *ev == KeyCode::LShift
                || *ev == KeyCode::RShift
                || *ev == KeyCode::Up
                || *ev == KeyCode::Down
                || *ev == KeyCode::Left
                || *ev == KeyCode::Right;
        }

        for ev in keyboard.get_just_released() {
            self.pokey.key_press(ev, false, is_shift, is_ctl);
            joy_changed = joy_changed
                || *ev == KeyCode::LShift
                || *ev == KeyCode::RShift
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
        // info!("set_joystick {} {} {} {} {}", up, down, left, right, fire);
        self.gtia.set_trig(port, fire);
        let up = up as u8;
        let down = down as u8 * 2;
        let left = left as u8 * 4;
        let right = right as u8 * 8;
        self.pia
            .write_port(0, 0xf0, (up | down | left | right) ^ 0xf);
    }
    pub fn tick(&mut self) {
        self.pokey.tick()
    }
    pub fn enable_log(&mut self, enable: bool) {
        self.gtia.enable_log(enable);
        self.antic.enable_log(enable);
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
