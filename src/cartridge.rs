pub trait Cartridge: Sync + Send {
    fn is_enabled(&self) -> bool;
    fn read(&self, addr: usize) -> u8;
    fn write(&mut self, addr: usize, value: u8);
    fn reset(&mut self) {}
}

impl dyn Cartridge {
    pub fn from_bytes(bytes: &[u8]) -> Result<Box<dyn Cartridge>, String> {
        assert!(std::str::from_utf8(&bytes[0..7]) == Ok("CART\0\0\0"));
        assert!((bytes.len() - 16) & 0x1fff == 0, "invalid car file size");
        let cart_type = bytes[7];
        match cart_type {
            // TODO: cast bytes[4..8] to u32
            1 => Ok(Box::new(Standard8k {
                data: bytes[16..].to_vec(),
            })),
            41 => Ok(Box::new(AtariMax128k {
                data: bytes[16..].to_vec(),
                cart_bank: 0,
            })),
            42 => Ok(Box::new(AtariMax1M {
                data: bytes[16..].to_vec(),
                cart_bank: 0,
            })),
            _ => Err(format!("unsupported {} cartridge type", cart_type)),
        }
    }
}
pub struct Standard8k {
    data: Vec<u8>,
}

impl Cartridge for Standard8k {
    fn is_enabled(&self) -> bool {
        true
    }

    fn read(&self, addr: usize) -> u8 {
        self.data[addr & 0x1fff]
    }

    fn write(&mut self, _addr: usize, _value: u8) {
    }
}
pub struct AtariMax1M {
    data: Vec<u8>,
    cart_bank: usize,
}

impl Cartridge for AtariMax1M {
    fn is_enabled(&self) -> bool {
        self.cart_bank < 128
    }

    fn read(&self, addr: usize) -> u8 {
        self.data[(self.cart_bank & 0x7f) * 0x2000 + (addr & 0x1fff)]
    }

    fn write(&mut self, addr: usize, _value: u8) {
        match addr >> 8 {
            0xD5 => self.cart_bank = (addr & 0xff) as usize,
            _ => (),
        }
    }
    
    fn reset(&mut self) {
        self.cart_bank = 0;
    }
}

pub struct AtariMax128k {
    data: Vec<u8>,
    cart_bank: usize,
}

impl Cartridge for AtariMax128k {
    fn is_enabled(&self) -> bool {
        self.cart_bank < 0x10
    }

    fn read(&self, addr: usize) -> u8 {
        self.data[(self.cart_bank & 0x0f) * 0x2000 + (addr & 0x1fff)]
    }

    fn write(&mut self, addr: usize, _value: u8) {
        if addr >= 0xd500 && addr < 0xd520 {
            self.cart_bank = (addr & 0x1f) as usize
        }
    }

    fn reset(&mut self) {
        self.cart_bank = 0;
    }
}
