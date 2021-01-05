use bevy::prelude::*;
use emulator_6502::{Interface6502, MOS6502};

use crate::system::AtariSystem;

#[allow(dead_code)]
mod consts {
    pub const DDEVIC: u16 = 0x300;
    pub const DUINT: u16 = 0x301;
    pub const DCMND: u16 = 0x302;
    pub const DSTATS: u16 = 0x303;
    pub const DBUFA: u16 = 0x304; // buffer address
    pub const DAUX1: u16 = 0x30a; // number of sectors
    pub const DAUX2: u16 = 0x30b;
    pub const DBYT: u16 = 0x308;
}
use consts::*;

fn set_sio_status(cpu: &mut MOS6502, atari_system: &mut AtariSystem, status: u8) {
    cpu.status_register = (cpu.status_register & 0x7f) | (status & 0x80);
    cpu.y_register = status;
    atari_system.write(0x303, status);
}

pub fn sioint_hook(atari_system: &mut AtariSystem, cpu: &mut MOS6502) {
    let device = atari_system.read(DDEVIC);
    let cmd = atari_system.read(DCMND);
    let status = if device == 0x31 {
        match cmd {
            0x53 => {
                // status
                // info!("dskint: read status");
                if let Some(_) = atari_system.disk_1 {
                    0x01
                } else {
                    0xff
                }
                // 0x01
            }
            0x52 => {
                // read
                let addr = atari_system.readw(DBUFA);
                let sector = atari_system.readw(DAUX1);
                let len = atari_system.readw(DBYT);
                info!(
                    "SIO read: addr: {:04x}, sector: {:x}, len: {:x}",
                    addr, sector, len
                );
                // TODO: unnecessary copy
                if let Some(data) = atari_system
                    .disk_1
                    .as_ref()
                    .and_then(|atr| atr.get_sector(sector as usize).map(|f| f.to_owned()))
                {
                    assert!(data.len() == len as usize);
                    atari_system.copy_from_slice(addr as usize, &data);
                    0x01
                } else {
                    0xff
                }
            }
            _ => {
                warn!("unknown SIO command: {:02x}", cmd);
                0xff
            }
        }
    } else {
        0xff
    };
    set_sio_status(cpu, atari_system, status);

    let fp = cpu.stack_pointer as u16 + 0x100;
    let pc = atari_system.read(fp + 1) as u16 + 256 * atari_system.read(fp + 2) as u16 + 1;
    cpu.stack_pointer += 2;
    cpu.program_counter = pc;
}
