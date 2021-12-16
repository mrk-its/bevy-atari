use bevy::prelude::*;
use emulator_6502::{Interface6502, MOS6502};

use crate::js_api;
use crate::system::AtariSystem;

#[allow(dead_code)]
mod consts {
    pub const DDEVIC: u16 = 0x300;
    pub const DUNIT: u16 = 0x301;
    pub const DCMND: u16 = 0x302;
    pub const DSTATS: u16 = 0x303;
    pub const DBUFA: u16 = 0x304; // buffer address
    pub const DAUX1: u16 = 0x30a; // number of sectors
    pub const DAUX2: u16 = 0x30b;
    pub const DBYT: u16 = 0x308;
}
use consts::*;

fn set_sio_status(cpu: &mut MOS6502, atari_system: &mut AtariSystem, status: u8) {
    cpu.set_status_register((cpu.get_status_register() & 0x7f) | (status & 0x80));
    cpu.set_y_register(status);
    atari_system.write(0x303, status);
}

pub fn sio_exit(atari_system: &mut AtariSystem, cpu: &mut MOS6502, status: u8) {
    set_sio_status(cpu, atari_system, status);

    let sp = cpu.get_stack_pointer();
    let fp = sp as u16 + 0x100;
    let pc = atari_system.read(fp + 1) as u16 + 256 * atari_system.read(fp + 2) as u16 + 1;
    cpu.set_stack_pointer(sp.wrapping_add(2));
    cpu.set_program_counter(pc);
}

pub fn sioint_hook(atari_system: &mut AtariSystem, cpu: &mut MOS6502) {
    let device = atari_system.read(DDEVIC);
    let unit = atari_system.read(DUNIT);
    let cmd = atari_system.read(DCMND);
    let addr = atari_system.readw(DBUFA);
    let len = atari_system.readw(DBYT);
    let sector = atari_system.readw(DAUX1);

    let drive = (device + unit - 49 - 1) as usize;
    let status = match cmd {
        0x53 => {
            info!("SIO status: addr: {:04x}, len: {:x}", addr, len);
            atari_system.get_status(drive, addr, len)
        }
        0x52 => {
            // read
            info!(
                "SIO read: addr: {:04x}, sector: {:x}, len: {:x}",
                addr, sector, len
            );
            atari_system.get_sector(drive, sector as usize, addr, len)
        }
        0x50 | 0x57 => {
            info!(
                "SIO write: addr: {:04x}, sector: {:x}, len: {:x}",
                addr, sector, len
            );
            atari_system.put_sector(drive, sector as usize, addr, len)
        }
        _ => {
            warn!("unknown SIO command: {:02x}", cmd);
            0xff
        }
    };
    sio_exit(atari_system, cpu, status);
}
