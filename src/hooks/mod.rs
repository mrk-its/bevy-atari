use crate::system::AtariSystem;
use bevy::prelude::warn;
use emulator_6502::{Interface6502, MOS6502};

mod sio;
mod xex;

pub fn hook(cpu: &mut MOS6502, atari_system: &mut AtariSystem) {
    match cpu.get_program_counter() {
        0xe459 => sio::sioint_hook(&mut *atari_system, &mut *cpu),
        0x01ff => {
            let acc = cpu.get_accumulator();
            match acc {
                1 => xex::load_block(&mut *atari_system, &mut *cpu),
                _ => warn!("unknown hook #{}, ignoring", acc),
            }
        }
        _ => (),
    }
}

pub fn hook_rts(atari_system: &mut AtariSystem, cpu: &mut MOS6502) {
    let sp = cpu.get_stack_pointer();
    let fp = sp as u16 + 0x100;
    let pc = atari_system.read(fp + 1) as u16 + 256 * atari_system.read(fp + 2) as u16 + 1;
    cpu.set_stack_pointer(sp.wrapping_add(2));
    cpu.set_program_counter(pc);
}
