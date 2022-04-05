use bevy::prelude::info;
use emulator_6502::{Interface6502, MOS6502};

use crate::system::AtariSystem;

enum Status {
    BlockLoaded,
    Finished,
}

pub fn load_block(atari_system: &mut AtariSystem, cpu: &mut MOS6502) {
    let mut flags = cpu.get_status_register() & !0x81;
    match load_block_inner(atari_system) {
        Some(Status::BlockLoaded) => (),      // N=0 C=0
        Some(Status::Finished) => flags |= 1, // N=0 C=1
        None => flags |= 0x81,                // N=1 C=1
    }
    cpu.set_status_register(flags);
    super::hook_rts(atari_system, cpu);
}

fn load_block_inner(atari_system: &mut AtariSystem) -> Option<Status> {
    let mut read24 = |offs| {
        atari_system.read(offs) as usize
            + ((atari_system.read(offs + 1) as usize) << 8)
            + ((atari_system.read(offs + 2) as usize) << 16)
    };

    let mut offs = read24(0x780 - 3);
    let xex_len = read24(0x780 - 6);

    if offs >= xex_len {
        info!("xex successfully loaded, len: {} offs: {}", xex_len, offs);
        return Some(Status::Finished);
    }

    if let Some(atr) = atari_system.disks[0].as_ref() {
        let read = |start, end| {
            if end <= xex_len {
                atr.get_data(start + 128, end + 128)
            } else {
                None
            }
        };
        let readw = |start| read(start, start + 2).map(|r| (r[0] as u16) + (r[1] as u16) * 256);
        let is_first = offs == 0;

        let mut start = readw(offs)?;
        offs += 2;
        if start == 0xffff {
            start = readw(offs)?;
            offs += 2;
        }
        let end = readw(offs)?;
        offs += 2;
        info!("xex block {:04x} - {:04x}", start, end);
        let len = (end - start + 1) as usize;
        let data = read(offs, offs + len)?.to_owned();
        offs += len;

        if is_first {
            atari_system.write(0x2e0, (start & 0xff) as u8);
            atari_system.write(0x2e1, (start >> 8) as u8);
        }
        atari_system.copy_from_slice(start, &data);
        atari_system.write(0x780 - 3, (offs & 0xff) as u8);
        atari_system.write(0x780 - 2, ((offs >> 8) & 0xff) as u8);
        atari_system.write(0x780 - 1, ((offs >> 16) & 0xff) as u8);
        Some(Status::BlockLoaded)
    } else {
        None
    }
}
