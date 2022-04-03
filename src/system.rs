use crate::atr::ATR;
use crate::cartridge::Cartridge;
use crate::multiplexer::Multiplexer;
use crate::platform::FileSystem;
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

#[derive(Component)]
pub struct AtariSystem {
    consol: Multiplexer<u8>,
    joystick: [Multiplexer<u8>; 2],
    pub read_banks: [*const MemBank; 32],
    write_banks: [*mut MemBank; 32],
    rom_write_bank: Vec<u8>,
    ram: Vec<u8>,
    ram_copy: Vec<u8>,
    ram_mask: Vec<u8>,
    pub osrom: Vec<u8>,
    basic: Option<Vec<u8>>,
    ext_mem_bank_mask: Option<usize>,
    pub antic: Antic,
    pub gtia: Gtia,
    pub pokey: Pokey,
    pub pia: PIA,
    pub disks: [Option<ATR>; 4],
    ticks: usize,
    pub cart: Option<Box<dyn Cartridge>>,
    pub keycodes: Vec<Option<(KeyCode, bool)>>,
}
unsafe impl Send for AtariSystem {}
unsafe impl Sync for AtariSystem {}

type MemBank = [u8; 2048];

#[allow(dead_code)]
const MEM_64: Option<usize> = None;
#[allow(dead_code)]
const MEM_128: Option<usize> = Some(3);
#[allow(dead_code)]
const MEM_320: Option<usize> = Some(15);

impl AtariSystem {
    pub fn new() -> AtariSystem {
        // initialize RAM with all 0xFFs
        let mut ram: Vec<u8> = Vec::new();
        ram.resize_with(320 * 1024, || 0);
        let rom_write_bank = vec![0; 0x800];
        let osrom = vec![0; 0x4000];
        let basic = None;
        let antic = Antic::default();
        let pokey = Pokey::default();
        let gtia = Gtia::default();
        let pia = PIA::default();
        let consol = Multiplexer::new(2);
        let joystick = [Multiplexer::new(3), Multiplexer::new(3)];

        let read_banks = [0 as *const MemBank; 32];
        let write_banks = [0 as *mut MemBank; 32];

        let mut atari_system = AtariSystem {
            ext_mem_bank_mask: MEM_320,
            consol,
            joystick,
            ram,
            read_banks,
            write_banks,
            rom_write_bank,
            ram_copy: Vec::new(),
            ram_mask: Vec::new(),
            osrom,
            basic,
            antic,
            gtia,
            pokey,
            pia,
            disks: Default::default(),
            ticks: 0,
            cart: None,
            keycodes: Vec::new(),
        };
        atari_system.setup_memory_banks();
        atari_system
    }

    pub fn set_cart(&mut self, cart: Option<Box<dyn Cartridge>>) {
        self.cart = cart;
        self.gtia.trig[3] = if self.cart.is_some() { 1 } else { 0 };
        self.setup_memory_banks();
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
    fn setup_memory_banks(&mut self) {
        // reduce cost of calling of _bank_ptr ~4x
        for i in 0..32 {
            if (i & 3) == 0 || i == (0x5000 >> 11) {
                // assume 8kB (2kB * 4) banks except of self test one
                // compute address of first block in the bank
                self.read_banks[i] = self._bank_ptr(i << 11, false, false);
                self.write_banks[i] = self._bank_ptr(i << 11, false, true) as *mut MemBank;
            } else {
                // for non-first blocks of 8k bank we computing address using first one
                unsafe {
                    self.read_banks[i] = self.read_banks[i & !3].add(i & 3);
                    self.write_banks[i] = self.write_banks[i & !3].add(i & 3);
                }
            }
        }
    }

    fn bank_offset(&self, addr: usize, antic: bool) -> usize {
        let ext_mem_req = !antic && !self.pia.portb_out().contains(PORTB::CPU_SELECT_NEG)
            || antic && !self.pia.portb_out().contains(PORTB::ANITC_SELECT_NEG);

        if let (true, Some(mask)) = (ext_mem_req, self.ext_mem_bank_mask) {
            let portb = self.pia.portb_out().bits;
            let bank_nr = (((portb & 0b1100) + ((portb & 0xc0) >> 2)) as usize) >> 2;
            let bank_nr = bank_nr & mask;
            // let bank_nr = (portb & 0b1100) as usize >> 2;
            (addr & 0x3fff) + 0x10000 + (bank_nr * 16384)
        } else {
            addr
        }
    }

    #[inline(always)]
    fn _io_read(&mut self, addr: usize, _antic: bool) -> u8 {
        let addr = usize::from(addr);
        match addr >> 8 {
            0xD0 => self.gtia.read(addr),
            0xD1 => 0xff,
            0xD2 => self.pokey.read(addr),
            0xD3 => self.pia.read(addr),
            0xD4 => self.antic.read(addr),
            _ => 0xff, // panic!("wrong io read address!"),
        }
    }
    #[inline(always)]
    fn _io_write(&mut self, addr: usize, value: u8, _antic: bool) {
        match addr >> 8 {
            0xD0 => self.gtia.write(addr, value),
            0xD2 => self.pokey.write(addr, value),
            0xD3 => {
                self.pia.write(addr, value);
                if (addr & 3) == 1 {
                    self.setup_memory_banks();
                };
            }
            0xD4 => self.antic.write(addr, value),
            0xD5 => match &mut self.cart {
                Some(cart) => {
                    cart.write(addr, value);
                    self.setup_memory_banks();
                }
                _ => (),
            },
            _ => (),
        }
    }

    pub fn is_rom_enabled(&self) -> bool {
        self.pia.portb_out().contains(PORTB::OSROM_ENABLED)
    }

    fn _bank_ptr(&mut self, addr: usize, antic: bool, write: bool) -> *const MemBank {
        // 0x00..0x3f - RAM
        // 0x40..0x7f - RAM / EXT_RAM / SELFTEST / ANTIC
        // 0x80..0x9f - RAM

        // 0xa0..0xbf - RAM / BASIC / CART

        // 0xC0..0xff - RAM / ROM (without D0..D7)
        let mem_ref = match addr >> 8 {
            0x50..=0x57 => {
                let portb = self.pia.portb_out();
                if portb.contains(PORTB::OSROM_ENABLED)
                    && !portb.contains(PORTB::SELFTEST_DISABLED)
                    && (portb & PORTB::BANK_SELECT_NEG) == PORTB::BANK_SELECT_NEG
                {
                    if !write {
                        &self.osrom[0x1000 + (addr & 0x7ff)]
                    } else {
                        &self.rom_write_bank[0]
                    }
                } else {
                    &self.ram[self.bank_offset(addr, antic)]
                }
            }
            0xA0..=0xBF => {
                if let Some(cart) = &self.cart {
                    if self.gtia.trig[3] > 0 && cart.is_enabled() {
                        if !write {
                            // info!("enabled cart at {:04x}", addr);
                            return cart.read(addr) as *const u8 as *const MemBank;
                        } else {
                            return &self.rom_write_bank[0] as *const u8 as *const MemBank;
                        }
                    } else {
                        // info!("no cart at {:04x}", addr);
                    }
                }
                if !self.pia.portb_out().contains(PORTB::BASIC_DISABLED) {
                    match &self.basic {
                        Some(basic) => {
                            if !write {
                                &basic[addr & 0x1fff]
                            } else {
                                &self.rom_write_bank[0]
                            }
                        }
                        None => &self.ram[addr],
                    }
                } else {
                    &self.ram[addr]
                }
            }
            0xC0..=0xFF => {
                if self.pia.portb_out().contains(PORTB::OSROM_ENABLED) {
                    if !write {
                        &self.osrom[addr & 0x3fff]
                    } else {
                        &self.rom_write_bank[0]
                    }
                } else {
                    &self.ram[addr]
                }
            }
            0x40..=0x7f => &self.ram[self.bank_offset(addr, antic)],
            _ => &self.ram[addr],
        };
        mem_ref as *const u8 as *const MemBank
    }

    #[inline(always)]
    fn _read(&mut self, addr: u16, antic: bool) -> u8 {
        let addr = addr as usize;
        match addr >> 8 {
            0xd0..=0xd7 => self._io_read(addr, antic),
            _ => unsafe { (*self.read_banks[addr >> 11])[addr & 2047] },
        }
    }
    #[inline(always)]
    fn _write(&mut self, addr: u16, value: u8, antic: bool) {
        let addr = addr as usize;
        match addr >> 8 {
            0xd0..=0xd7 => self._io_write(addr, value, antic),
            _ => unsafe { (*self.write_banks[addr >> 11])[addr & 2047] = value },
        }
    }

    pub fn set_osrom(&mut self, data: Option<&[u8]>) {
        let data: &[u8] = if let Some(data) = data {
            data
        } else {
            &[0; 0x4000]
        };
        self.osrom.copy_from_slice(data);
    }

    pub fn set_basic(&mut self, data: Option<&[u8]>) {
        self.basic = data.map(|data| {
            let mut basic = vec![0; 8192];
            basic.copy_from_slice(data);
            basic
        });
    }

    pub fn copy_from_slice(&mut self, offs: u16, data: &[u8]) {
        for (i, b) in data.iter().enumerate() {
            self.write(offs.wrapping_add(i as u16), *b);
        }
    }
    pub fn copy_to_slice(&mut self, offs: u16, data: &mut [u8]) {
        for (i, b) in data.iter_mut().enumerate() {
            *b = self.read(offs.wrapping_add(i as u16));
        }
    }
    pub fn antic_copy_to_slice(&mut self, offs: u16, data: &mut [u8]) {
        for (i, b) in data.iter_mut().enumerate() {
            *b = self._read(offs.wrapping_add(i as u16), true);
        }
    }
    #[inline(always)]
    pub fn readw(&mut self, addr: u16) -> u16 {
        self.read(addr) as u16 + 256 * self.read(addr + 1) as u16
    }

    pub fn load_atari800_state(&mut self, atari800_state: &Atari800State) {
        self.pia.set_portb_out(atari800_state.memory.portb);
        self.ram[0..0x10000].copy_from_slice(atari800_state.memory.data);
        // self.ram2.copy_from_slice(atari800_state.memory.under_atarixl_os);
        self.osrom.copy_from_slice(atari800_state.memory.os);
        self.basic = Some(vec![0; 8192]);
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

    pub fn keystrokes(&mut self, text: &str) {
        for c in text.chars() {
            let codes = char_to_keycodes(c);
            for c in codes {
                self.keycodes.push(Some((*c, true)));
            }
            for c in codes {
                self.keycodes.push(Some((*c, false)));
                self.keycodes.push(None);
            }
            if c == '\n' {
                for _ in 0..4 {
                    self.keycodes.push(None);
                }
            }
        }
    }

    pub fn handle_keyboard(
        &mut self,
        keyboard: &mut ResMut<Input<KeyCode>>,
        cpu: &mut MOS6502,
    ) -> bool {
        if !self.keycodes.is_empty() && self.ticks >= 15600 * 2 {
            if let Some((keycode, pressed)) = self.keycodes.remove(0) {
                if pressed {
                    keyboard.press(keycode);
                } else {
                    keyboard.release(keycode);
                }
            }
        }

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

    #[inline(always)]
    pub fn scanline_tick(&mut self, scanline: usize) {
        self.pokey.scanline_tick(scanline);
        self.ticks += 1;
        if self.ticks == 15600 {
            // ~1sek
            self.gtia.consol_force_mask = 0x07;
        }
    }
    #[inline(always)]
    pub fn inc_cycle(&mut self) {
        self.antic.inc_cycle();
        self.pokey.total_cycles = self.antic.total_cycles;
    }

    pub fn get_status(&mut self, drive: usize, addr: u16, len: u16) -> u8 {
        if drive >= self.disks.len() || self.disks[drive].is_none() {
            return 0xff;
        }
        let mut data = vec![0; len as usize];
        let ret = self.disks[drive].as_ref().unwrap().get_status(&mut data);
        self.copy_from_slice(addr, &data);
        ret
    }

    pub fn get_sector(&mut self, drive: usize, sector: usize, addr: u16, len: u16) -> u8 {
        if drive >= self.disks.len() || self.disks[drive].is_none() {
            return 0xff;
        }
        let mut data = vec![0; len as usize];
        let ret = self.disks[drive]
            .as_ref()
            .unwrap()
            .get_sector(sector, &mut data);
        self.copy_from_slice(addr, &data);
        ret
    }

    pub fn put_sector(&mut self, drive: usize, sector: usize, addr: u16, len: u16) -> u8 {
        if drive >= self.disks.len() || self.disks[drive].is_none() {
            return 0xff;
        }
        let mut data = vec![0; len as usize];
        self.copy_to_slice(addr, &mut data);
        self.disks[drive]
            .as_mut()
            .unwrap()
            .put_sector(sector, &data)
    }

    pub fn store_disks(&mut self, fs: &FileSystem) {
        for disk in self.disks.iter_mut() {
            if let Some(atr) = disk {
                atr.store(fs);
            }
        }
    }
}

impl Default for AtariSystem {
    fn default() -> Self {
        AtariSystem::new()
    }
}

impl Interface6502 for AtariSystem {
    #[inline(always)]
    fn read(&mut self, addr: u16) -> u8 {
        self._read(addr, false)
    }
    #[inline(always)]
    fn write(&mut self, addr: u16, value: u8) {
        self._write(addr, value, false)
    }
}

fn char_to_keycodes(c: char) -> &'static [KeyCode] {
    match c {
        'A' => &[KeyCode::A],
        'B' => &[KeyCode::B],
        'C' => &[KeyCode::C],
        'D' => &[KeyCode::D],
        'E' => &[KeyCode::E],
        'F' => &[KeyCode::F],
        'G' => &[KeyCode::G],
        'H' => &[KeyCode::H],
        'I' => &[KeyCode::I],
        'J' => &[KeyCode::J],
        'K' => &[KeyCode::K],
        'L' => &[KeyCode::L],
        'M' => &[KeyCode::M],
        'N' => &[KeyCode::N],
        'O' => &[KeyCode::O],
        'P' => &[KeyCode::P],
        'Q' => &[KeyCode::Q],
        'R' => &[KeyCode::R],
        'S' => &[KeyCode::S],
        'T' => &[KeyCode::T],
        'U' => &[KeyCode::U],
        'V' => &[KeyCode::V],
        'W' => &[KeyCode::W],
        'X' => &[KeyCode::X],
        'Y' => &[KeyCode::Y],
        'Z' => &[KeyCode::Z],
        '0' => &[KeyCode::Key0],
        '1' => &[KeyCode::Key1],
        '2' => &[KeyCode::Key2],
        '3' => &[KeyCode::Key3],
        '4' => &[KeyCode::Key4],
        '5' => &[KeyCode::Key5],
        '6' => &[KeyCode::Key6],
        '7' => &[KeyCode::Key7],
        '8' => &[KeyCode::Key8],
        '9' => &[KeyCode::Key9],
        '\'' => &[KeyCode::Apostrophe],
        '.' => &[KeyCode::Period],
        ',' => &[KeyCode::Comma],
        '*' => &[KeyCode::Asterisk],
        '\n' => &[KeyCode::Return],
        ';' => &[KeyCode::Colon],
        '[' => &[KeyCode::LBracket],
        ']' => &[KeyCode::RBracket],
        ' ' => &[KeyCode::Space],
        '+' => &[KeyCode::Plus],
        '-' => &[KeyCode::Minus],
        '_' => &[KeyCode::Underline],
        '=' => &[KeyCode::Equals],
        '/' => &[KeyCode::Slash],
        '\\' => &[KeyCode::Backslash],
        '<' => return &[KeyCode::LShift, KeyCode::Comma],
        '>' => return &[KeyCode::LShift, KeyCode::Period],
        '|' => return &[KeyCode::LShift, KeyCode::Backslash],
        '!' => return &[KeyCode::LShift, KeyCode::Key1],
        '@' => return &[KeyCode::LShift, KeyCode::Key2],
        '#' => return &[KeyCode::LShift, KeyCode::Key3],
        '$' => return &[KeyCode::LShift, KeyCode::Key4],
        '%' => return &[KeyCode::LShift, KeyCode::Key5],
        '^' => return &[KeyCode::LShift, KeyCode::Key6],
        '&' => return &[KeyCode::LShift, KeyCode::Key7],
        // '*' => return vec![KeyCode::LShift, KeyCode::Key8],
        '(' => return &[KeyCode::LShift, KeyCode::Key9],
        ')' => return &[KeyCode::LShift, KeyCode::Key0],
        '?' => return &[KeyCode::LShift, KeyCode::Slash],
        ':' => return &[KeyCode::LShift, KeyCode::Colon],
        '"' => return &[KeyCode::LShift, KeyCode::Apostrophe],
        _ => return &[],
    }
}
