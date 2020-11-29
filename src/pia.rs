use bevy::prelude::{info, warn};
pub struct PIA {

}

impl Default for PIA {
    fn default() -> Self {
        Self {

        }
    }
}

impl PIA {
    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0x3;
        let value = 0xff;
        //warn!("PIA read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&self, addr: usize, value: u8) {
        let addr = addr & 0x3;
        //warn!("PIA write: {:02x}: {:02x}", addr, value);
    }
}