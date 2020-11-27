pub use w65c02s::*;
pub use bevy::prelude::info;

pub struct AtariSystem {
    ram: [u8; 65536],
}

impl AtariSystem {
    pub fn new() -> AtariSystem {
        // initialize RAM with all 0xFFs
        let mut ram = [0xFF; 65536];
        // initialize the message
        ram[0x0001..0x000F].copy_from_slice(b"Hello World!\n\0");
        // initialize the program
        ram[0x0200..0x0204].copy_from_slice(&[
            op::NOP,
            op::JMP_ABS, 0, 2,
        ]);
        // initialize the reset vector to point to $0200
        ram[0xFFFC..0xFFFE].copy_from_slice(&[0x00, 0x02]);
        AtariSystem { ram }
    }
}

impl Default for AtariSystem {
    fn default() -> Self {
        AtariSystem::new()
    }
}

impl System for AtariSystem {
    fn read(&mut self, _cpu: &mut W65C02S, addr: u16) -> u8 {
        // all reads return RAM values directly
        self.ram[addr as usize]
    }
    fn write(&mut self, _cpu: &mut W65C02S, addr: u16, value: u8) {
        if addr == 0 {
            // writing address $0000 outputs on an ASCII-only "serial port"
            info!("{}", String::from_utf8_lossy(&[value]));
        }
        else {
            // all other writes write to RAM
            self.ram[addr as usize] = value
        }
    }
}