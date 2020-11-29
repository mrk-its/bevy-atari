use bevy::prelude::{info, warn};

const RANDOM: usize = 0x0A;

pub struct Pokey {

}

impl Default for Pokey {
    fn default() -> Self {
        Self {

        }
    }
}

impl Pokey {
    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0xf;
        let value = match addr {
            RANDOM => rand::random(),
            _ => 0xff
        };
        //warn!("POKEY read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&self, addr: usize, value: u8) {
        let addr = addr & 0xf;
        //warn!("POKEY write: {:02x}: {:02x}", addr, value);
    }
}