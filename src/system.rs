use crate::atr::ATR;
use crate::cartridge::Cartridge;
use crate::multiplexer::Multiplexer;
pub use crate::{antic, gtia};
pub use crate::{antic::Antic, gtia::Gtia, pia::PIA, pokey::Pokey};
use crate::{atari800_state::Atari800State, pokey};
pub use bevy::prelude::*;
pub use emulator_6502::{Interface6502, MOS6502};
pub use std::{cell::RefCell, rc::Rc};

bitflags! {
    #[derive(Default)]
    pub struct PORTB: u8 {
        const OSROM_ENABLED = 0x01;
        const BASIC_DISABLED = 0x02;
        const SELFTEST_DISABLED = 0x80;
        const UNUSED = 0x40;
        const ANITC_SELECT_NEG = 0x20;
        const CPU_SELECT_NEG = 0x10;
        const BANK_MASK = 0b1100;
        const BANK_SELECT_NEG = 0x30;
    }
}

pub struct AtariSystem {
    consol: Multiplexer<u8>,
    joystick: [Multiplexer<u8>; 2],
    ram: Vec<u8>,
    ram_copy: Vec<u8>,
    ram_mask: Vec<u8>,
    osrom: [u8; 0x4000],
    basic: Option<[u8; 0x2000]>,
    pub antic: Antic,
    pub gtia: Gtia,
    pub pokey: Pokey,
    pub pia: PIA,
    pub disk_1: Option<ATR>,
    ticks: usize,
    pub cart: Option<Box<dyn Cartridge>>,
}

impl AtariSystem {
    pub fn new() -> AtariSystem {
        // initialize RAM with all 0xFFs
        let mut ram: Vec<u8> = Vec::new();
        ram.resize_with(320 * 1024, || 0);
        let osrom = [0x00; 0x4000];
        let basic = None;
        let antic = Antic::default();
        let pokey = Pokey::default();
        let gtia = Gtia::default();
        let pia = PIA::default();
        let disk_1 = None;
        let consol = Multiplexer::new(2);
        let joystick = [Multiplexer::new(3), Multiplexer::new(3)];

        AtariSystem {
            consol,
            joystick,
            ram,
            ram_copy: Vec::new(),
            ram_mask: Vec::new(),
            osrom,
            basic,
            antic,
            gtia,
            pokey,
            pia,
            disk_1,
            ticks: 0,
            cart: None,
        }
    }

    pub fn set_cart(&mut self, cart: Option<Box<dyn Cartridge>>) {
        self.cart = cart;
        self.gtia.trig[3] = if self.cart.is_some() { 1 } else { 0 };
    }

    pub fn trainer_init(&mut self) {
        self.ram_copy = self.ram.clone();
        self.ram_mask = Vec::new();
        self.ram_mask.resize_with(self.ram.len(), || 0xff);
    }

    pub fn trainer_changed(&mut self, changed: bool) -> usize {
        let mut cnt = 0;
        for (i, c) in self.ram.iter().enumerate() {
            if self.ram_mask[i] == 0xff
                && (changed && *c == self.ram_copy[i] || !changed && *c != self.ram_copy[i])
            {
                self.ram_mask[i] = 0;
            }
            if self.ram_mask[i] == 0xff {
                cnt += 1;
                info!("addr: {:04x?}", i);
            }
        }
        self.ram_copy = self.ram.clone();
        cnt
    }

    #[inline(always)]
    fn bank_offset(&self, addr: usize, antic: bool) -> usize {
        if !antic && !self.pia.portb_out().contains(PORTB::CPU_SELECT_NEG)
            || antic && !self.pia.portb_out().contains(PORTB::ANITC_SELECT_NEG)
        {
            let portb = self.pia.portb_out().bits;
            // let bank_nr = (((portb & 0b1100) + ((portb & 0xc0) >> 2)) as usize) >> 2;
            let bank_nr = (portb & 0b1100) as usize >> 2;
            (addr & 0x3fff) + 0x10000 + (bank_nr * 16384)
        } else {
            addr
        }
    }

    fn _read(&mut self, addr: u16, antic: bool) -> u8 {
        // all reads return RAM values directly
        let addr = addr as usize;
        match addr >> 8 {
            0x50..=0x57 => {
                let portb = self.pia.portb_out();
                if portb.contains(PORTB::OSROM_ENABLED)
                    && !portb.contains(PORTB::SELFTEST_DISABLED)
                    && (portb & PORTB::BANK_SELECT_NEG) == PORTB::BANK_SELECT_NEG
                {
                    self.osrom[0x1000 + (addr & 0x7ff)]
                } else {
                    self.ram[self.bank_offset(addr, antic)]
                }
            }
            0xA0..=0xBF => {
                if let Some(cart) = &self.cart {
                    if self.gtia.trig[3] > 0 && cart.is_enabled() {
                        return cart.read(addr);
                    }
                }
                if !self.pia.portb_out().contains(PORTB::BASIC_DISABLED) {
                    match self.basic {
                        Some(basic) => basic[addr & 0x1fff],
                        None => self.ram[addr],
                    }
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
                if self.pia.portb_out().contains(PORTB::OSROM_ENABLED) {
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
                let portb = self.pia.portb_out();
                if !(portb.contains(PORTB::OSROM_ENABLED)
                    && !portb.contains(PORTB::SELFTEST_DISABLED)
                    && (portb & PORTB::BANK_SELECT_NEG) == PORTB::BANK_SELECT_NEG)
                {
                    let offs = self.bank_offset(addr, antic);
                    self.ram[offs] = value
                }
            }
            0xA0..=0xBF => {
                if self.pia.portb_out().contains(PORTB::BASIC_DISABLED) {
                    self.ram[addr] = value
                }
            }
            0xD0 => self.gtia.write(addr, value),
            0xD2 => self.pokey.write(addr, value),
            0xD3 => {
                self.pia.write(addr, value);
            }
            0xD4 => self.antic.write(addr, value),
            0xD5 => match &mut self.cart {
                Some(cart) => cart.write(addr, value),
                _ => (),
            },
            0xC0..=0xFF => {
                if !self.pia.portb_out().contains(PORTB::OSROM_ENABLED) {
                    self.ram[addr] = value
                }
            }
            0x40..=0x7f => {
                let offs = self.bank_offset(addr, antic);
                self.ram[offs] = value;
            }
            _ => self.ram[addr] = value,
        }
    }

    pub fn set_osrom(&mut self, data: Option<Vec<u8>>) {
        let data: &[u8] = if let Some(data) = data.as_ref() {
            data
        } else {
            &[0; 0x4000]
        };
        self.osrom.copy_from_slice(data);
    }

    pub fn set_basic(&mut self, data: Option<Vec<u8>>) {
        self.basic = data.map(|data| {
            let mut basic = [0; 0x2000];
            basic.copy_from_slice(&data);
            basic
        });
    }

    pub fn copy_from_slice(&mut self, offs: usize, data: &[u8]) {
        for (i, b) in data.iter().enumerate() {
            self.write((i + offs) as u16, *b);
        }
    }
    pub fn copy_to_slice(&mut self, offs: u16, data: &mut [u8]) {
        for (i, b) in data.iter_mut().enumerate() {
            *b = self.read(i as u16 + offs);
        }
    }
    pub fn antic_copy_to_slice(&mut self, offs: u16, data: &mut [u8]) {
        for (i, b) in data.iter_mut().enumerate() {
            *b = self._read(i as u16 + offs, true);
        }
    }
    pub fn readw(&mut self, addr: u16) -> u16 {
        self.read(addr) as u16 + 256 * self.read(addr + 1) as u16
    }

    pub fn load_atari800_state(&mut self, atari800_state: &Atari800State) {
        self.pia.set_portb_out(atari800_state.memory.portb);
        self.ram[0..0x10000].copy_from_slice(atari800_state.memory.data);
        // self.ram2.copy_from_slice(atari800_state.memory.under_atarixl_os);
        self.osrom.copy_from_slice(atari800_state.memory.os);
        self.basic = Some([0; 0x2000]);
        self.basic
            .as_mut()
            .unwrap()
            .copy_from_slice(atari800_state.memory.basic);
        if self.pia.portb_out().contains(PORTB::OSROM_ENABLED) {
            self.ram[0xc000..0x10000].copy_from_slice(atari800_state.memory.under_atarixl_os);
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

    pub fn reset(&mut self, cpu: &mut MOS6502, cold: bool, disable_basic: bool) {
        let disable_basic = disable_basic || self.basic.is_none();
        self.write(0xd301, 0xff); // turn on osrom
        info!(
            "atari_system reset, cold: {:?}, disable_basic: {:?}",
            cold, disable_basic
        );
        if cold {
            self.write(0x244, 255);
        }
        self.antic = Antic::default();
        cpu.reset(self);
        self.ticks = 0;
        self.gtia.consol_force_mask = if disable_basic { 0x03 } else { 0x07 };
        if let Some(cart) = &mut self.cart {
            cart.reset();
        }
    }

    pub fn update_consol(&mut self, index: usize, value: u8) {
        self.consol.set_input(index, value);
        self.gtia.consol = !self.consol.get_output() & 7;
    }

    pub fn handle_keyboard(&mut self, keyboard: &Res<Input<KeyCode>>, cpu: &mut MOS6502) -> bool {
        let mut irq = false;
        let start = keyboard.pressed(KeyCode::F2);
        let select = keyboard.pressed(KeyCode::F3);
        let option = keyboard.pressed(KeyCode::F4);
        self.update_consol(0, (start as u8) | (select as u8) << 1 | (option as u8) << 2);

        let is_shift = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
        let is_ctl = keyboard.pressed(KeyCode::LControl) || keyboard.pressed(KeyCode::RControl);
        let mut joy_changed = false;
        let map_joy = true;
        for ev in keyboard.get_just_pressed() {
            if *ev == KeyCode::F5 {
                self.reset(cpu, false, false);
            }
            if map_joy {
                joy_changed = joy_changed
                    || *ev == KeyCode::LShift
                    || *ev == KeyCode::RShift
                    || *ev == KeyCode::Up
                    || *ev == KeyCode::Down
                    || *ev == KeyCode::Left
                    || *ev == KeyCode::Right;
            }
            if !joy_changed || is_ctl {
                irq = irq || self.pokey.key_press(ev, true, is_shift, is_ctl);
            }
        }

        for ev in keyboard.get_just_released() {
            if map_joy {
                joy_changed = joy_changed
                    || *ev == KeyCode::LShift
                    || *ev == KeyCode::RShift
                    || *ev == KeyCode::Up
                    || *ev == KeyCode::Down
                    || *ev == KeyCode::Left
                    || *ev == KeyCode::Right;
            }
            if !joy_changed || is_ctl {
                self.pokey.key_press(ev, false, is_shift, is_ctl);
            }
        }
        if !is_ctl && joy_changed {
            let fire = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
            let up = keyboard.pressed(KeyCode::Up) as u8;
            let down = keyboard.pressed(KeyCode::Down) as u8 * 2;
            let left = keyboard.pressed(KeyCode::Left) as u8 * 4;
            let right = keyboard.pressed(KeyCode::Right) as u8 * 8;
            self.set_joystick(2, 0, up | down | left | right, fire);
            irq = false;
        }
        return irq && self.pokey.irqen.contains(pokey::IRQ::KEY);
    }

    pub fn set_joystick(&mut self, input: usize, port: usize, dirs: u8, fire: bool) {
        self.joystick[port].set_input(input, dirs | (fire as u8) << 4);
        let ports = [self.joystick[0].get_output(), self.joystick[1].get_output()];
        self.pia
            .set_port_a_input(0, (ports[0] & 0xf | (ports[1] & 0xf) << 4) ^ 0xff);
        self.gtia.set_trig(port, (ports[port] & 0x10) > 0);
    }

    pub fn scanline_tick(&mut self, scanline: usize) {
        self.pokey.scanline_tick(scanline);
        self.ticks += 1;
        if self.ticks == 15600 {
            // ~1sek
            self.gtia.consol_force_mask = 0x07;
        }
    }
    pub fn inc_cycle(&mut self) {
        self.antic.inc_cycle();
        self.pokey.total_cycles = self.antic.total_cycles;
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
