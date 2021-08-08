use std::iter::FromIterator;
pub trait Cartridge: Sync + Send {
    fn is_enabled(&self) -> bool;
    fn read(&self, addr: usize) -> u8;
    fn write(&mut self, addr: usize, value: u8);
}

impl Cartridge {
    pub fn from_bytes(bytes: &[u8]) -> Box<dyn Cartridge> {
        assert!(std::str::from_utf8(&bytes[0..7]) == Ok("CART\0\0\0"));
        assert!((bytes.len() - 16) & 0x1fff == 0);
        let cart_type = bytes[7];
        match cart_type {  // TODO: cast bytes[4..8] to u32
            42 => Box::new(AtariMax {
                data: Vec::copy_from_slice(bytes[16..]),
                cart_bank: 0,
            }),
            _ => panic!("unsupported {} cartridge type", cart_type),
        }
    }
}

pub struct AtariMax {
    data: Vec<u8>,
    cart_bank: usize,
}

impl Cartridge for AtariMax {
    fn is_enabled(&self) -> bool {
        self.cart_bank < 128
    }

    fn read(&self, addr: usize) -> u8 {
        self.data[(self.cart_bank & 0x7f) * 0x2000 + (addr & 0x1fff)]
    }
    fn write(&mut self, addr: usize, value: u8) {
        match addr >> 8 {
            0xD5 => self.cart_bank = (addr & 0xff) as usize,
            _ => (),
        }
    }
}
