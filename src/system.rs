use crate::atari800_state::Atari800State;
pub use crate::{antic, gtia};
pub use crate::{antic::Antic, gtia::Gtia, pia::PIA, pokey::Pokey};
pub use bevy::prelude::*;
pub use emulator_6502::{Interface6502, MOS6502};
pub use std::{cell::RefCell, rc::Rc};
use crate::atr::ATR;

bitflags! {
    #[derive(Default)]
    pub struct PORTB: u8 {
        const OSROM_ENABLED = 0x01;
        const BASIC_DISABLED = 0x02;
        const SELFTEST_DISABLED = 0x80;
    }
}

pub struct AtariSystem {
    portb: PORTB,
    ram: [u8; 0x20000],
    osrom: [u8; 0x4000],
    basic: [u8; 0x2000],
    pub antic: Antic,
    pub gtia: Gtia,
    pub pokey: Pokey,
    pub pia: PIA,
    pub disk_1: Option<ATR>,
}

const OSROM: &[u8] = include_bytes!("../assets/altirraos-xl.rom");
const BASIC: &[u8] = include_bytes!("../assets/atbasic.bin");

impl AtariSystem {
    pub fn new() -> AtariSystem {
        // initialize RAM with all 0xFFs
        let ram = [0x0; 0x20000];
        let mut osrom = [0x00; 0x4000];
        osrom.copy_from_slice(OSROM);
        let mut basic = [0x00; 0x2000];
        basic.copy_from_slice(BASIC);
        let antic = Antic::default();
        let pokey = Pokey::default();
        let gtia = Gtia::default();
        let pia = PIA::default();
        let disk_1 = None;

        AtariSystem {
            portb: PORTB::from_bits_truncate(0xff),
            ram,
            osrom,
            basic,
            antic,
            gtia,
            pokey,
            pia,
            disk_1,
        }
    }

    #[inline(always)]
    fn bank_offset(&self, addr: usize, antic: bool) -> usize {
        if !antic && !self.portb.contains(PORTB::CPU_SELECT_NEG) || antic && !self.portb.contains(PORTB::ANITC_SELECT_NEG) {
            (addr & 0x3fff) + 0x10000 + (((self.portb & PORTB::BANK_MASK).bits as usize) << 12)
        } else {
            addr
        }
    }

    fn _read(&mut self, addr: u16, antic: bool) -> u8 {
        // all reads return RAM values directly
        let addr = addr as usize;
        match addr >> 8 {
            0x50..=0x57 => {
                if !(self.portb.contains(PORTB::OSROM_ENABLED) && !self.portb.contains(PORTB::SELFTEST_DISABLED)) {
                    self.ram[self.bank_offset(addr, antic)]
                } else {
                    self.osrom[0x1000 + (addr & 0x7ff)]
                }
            }
            0xA0..=0xBF => {
                if !self.portb.contains(PORTB::BASIC_DISABLED) {
                    self.basic[addr & 0x1fff]
                } else {
                    self.ram[addr]
                }
            }
            0xD0 => self.gtia.read(addr),
            0xD1 => 0xff,
            0xD2 => self.pokey.read(addr),
            0xD3 => self.pia.read(addr),
            0xD4 => self.antic.read(addr),
            0xC0..=0xFF => {
                if self.portb.contains(PORTB::OSROM_ENABLED) {
                    self.osrom[addr & 0x3fff]
                } else {
                    self.ram[addr]
                }
            }
            0x40..=0x7f => self.ram[self.bank_offset(addr, antic)],
            _ => self.ram[addr],
        }
    }
    fn _write(&mut self, addr: u16, value: u8, antic: bool) {
        let addr = addr as usize;
        match addr >> 8 {
            0x50..=0x5F => {
                if !(self.portb.contains(PORTB::OSROM_ENABLED) && !self.portb.contains(PORTB::SELFTEST_DISABLED)) {
                    self.ram[self.bank_offset(addr, antic)] = value
                }
            }
            0xA0..=0xBF => {
                if self.portb.contains(PORTB::BASIC_DISABLED) {
                    self.ram[addr] = value
                }
            }
            0xD0 => self.gtia.write(addr, value),
            0xD2 => self.pokey.write(addr, value),
            0xD3 => {
                self.pia.write(addr, value);
                if addr & 0xff == 1 {
                    self.portb = PORTB::from_bits_truncate(value);
                }
            },
            0xD4 => self.antic.write(addr, value),
            0xC0..=0xFF => {
                if !self.portb.contains(PORTB::OSROM_ENABLED) {
                    self.ram[addr] = value
                }
            }
            0x40..=0x7f => self.ram[self.bank_offset(addr, antic)] = value,
            _ => self.ram[addr] = value,
        }
    }

    pub fn set_osrom(&mut self, data: Option<Vec<u8>>) {
        let data = if let Some(data) = data.as_ref() {
            data
        } else {
            OSROM
        };
        self.osrom.copy_from_slice(data);
    }

    pub fn set_basic(&mut self, data: Option<Vec<u8>>) {
        let data = if let Some(data) = data.as_ref() {
            data
        } else {
            BASIC
        };
        self.basic.copy_from_slice(data);
    }

    pub fn copy_from_slice(&mut self, offs: usize, data: &[u8]) {
        for (i, b) in data.iter().enumerate() {
            self.write((i + offs) as u16, *b);
        }
    }
    pub fn copy_to_slice(&mut self, offs: u16, data: &mut[u8]) {
        for (i, b) in data.iter_mut().enumerate() {
            *b = self.read(i as u16 + offs);
        }
    }
    pub fn antic_copy_to_slice(&mut self, offs: u16, data: &mut[u8]) {
        for (i, b) in data.iter_mut().enumerate() {
            *b = self._read(i as u16 + offs, true);
        }
    }
    pub fn readw(&mut self, addr: u16) -> u16 {
        self.read(addr) as u16 + 256 * self.read(addr+1) as u16
    }

    pub fn load_atari800_state(&mut self, atari800_state: &Atari800State) {
        self.portb = PORTB::from_bits_truncate(atari800_state.memory.portb);
        self.ram.copy_from_slice(atari800_state.memory.data);
        // self.ram2.copy_from_slice(atari800_state.memory.under_atarixl_os);
        self.osrom.copy_from_slice(atari800_state.memory.os);
        self.basic.copy_from_slice(atari800_state.memory.basic);
        if self.portb.contains(PORTB::OSROM_ENABLED) {
            self.ram[0xc000..].copy_from_slice(atari800_state.memory.under_atarixl_os);
        }

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
        self.gtia.write(gtia::PRIOR, gtia.prior);
        self.gtia.gractl = gtia::GRACTL::from_bits_truncate(gtia.gractl);

        self.antic.dmactl = antic::DMACTL::from_bits_truncate(antic.dmactl);
        self.antic.chactl = antic.chactl;
        self.antic.chbase = antic.chbase;
        self.antic.pmbase = antic.pmbase;

        self.antic.dlist = antic.dlist;
        self.antic.nmien = antic::NMIEN::from_bits_truncate(antic.nmien);
        self.antic.nmist = antic::NMIST::from_bits_truncate(antic.nmist);
        self.antic.pmbase = antic.pmbase;

        for i in 0..4 {
            self.pokey.write(i * 2, pokey.audf[i]);
            self.pokey.write(i * 2 + 1, pokey.audc[i]);
        }
        self.pokey.write(0x08, pokey.audctl);

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

    pub fn handle_keyboard(&mut self, keyboard: &Res<Input<KeyCode>>, cpu: &mut MOS6502) -> bool {
        let mut irq = false;
        let start = keyboard.pressed(KeyCode::F2);
        let select = keyboard.pressed(KeyCode::F3);
        let option = keyboard.pressed(KeyCode::F4);
        self.gtia.consol = !((start as u8) | (select as u8) << 1 | (option as u8) << 2) & 0x07;

        let is_shift = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
        let is_ctl = keyboard.pressed(KeyCode::LControl) || keyboard.pressed(KeyCode::RControl);
        let mut joy_changed = false;
        for ev in keyboard.get_just_pressed() {
            if *ev == KeyCode::F5 {
                cpu.reset(self)
            }
            self.pokey.resume();
            joy_changed = joy_changed
                || *ev == KeyCode::LShift
                || *ev == KeyCode::RShift
                || *ev == KeyCode::Up
                || *ev == KeyCode::Down
                || *ev == KeyCode::Left
                || *ev == KeyCode::Right;
            if !joy_changed || is_ctl {
                irq = irq || self.pokey.key_press(ev, true, is_shift, is_ctl);
            }
        }

        for ev in keyboard.get_just_released() {
            joy_changed = joy_changed
                || *ev == KeyCode::LShift
                || *ev == KeyCode::RShift
                || *ev == KeyCode::Up
                || *ev == KeyCode::Down
                || *ev == KeyCode::Left
                || *ev == KeyCode::Right;
            if !joy_changed || is_ctl {
                self.pokey.key_press(ev, false, is_shift, is_ctl);
            }
        }
        if !is_ctl && joy_changed {
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
            .write_port_a(0xf0, (up | down | left | right) ^ 0xf);
    }
    pub fn tick(&mut self) {
        self.pokey.tick()
    }
}

impl Default for AtariSystem {
    fn default() -> Self {
        AtariSystem::new()
    }
}

impl Interface6502 for AtariSystem {
    fn read(&mut self, addr: u16) -> u8 {
        self._read(addr, false)
    }
    fn write(&mut self, addr: u16, value: u8) {
        self._write(addr, value, false)
    }
}
