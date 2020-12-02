use std::fmt::UpperExp;

use bevy::prelude::{info, warn};
pub struct PIA {
    regs: [u8; 4],
}
const PORTA: usize = 0;
const PACTL: usize = 2;
const PBCTL: usize = 3;

impl Default for PIA {
    fn default() -> Self {
        Self {
            regs: [0xff, 0xff, 0x3f, 0x3f],
        }
    }
}

impl PIA {
    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0x3;
        let value = self.regs[addr];
        //warn!("PIA read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0x3;
        //warn!("PIA write: {:02x}: {:02x}", addr, value);
    }
    pub fn write_port(&mut self, port: usize, mask: u8, value: u8) {
        let index = PORTA + port & 1;
        self.regs[index] = self.regs[index] & mask | value;
        info!("pia write: {:02x}", self.regs[index]);
    }
}