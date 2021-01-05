pub struct PIA {
    regs: [u8; 4],
}
const PORTA: usize = 0;
#[allow(dead_code)]
const PACTL: usize = 2;
#[allow(dead_code)]
const PBCTL: usize = 3;

bitflags! {
    #[derive(Default)]
    pub struct PORTB: u8 {
        const OSROM_ENABLED = 0x01;
        const BASIC_DISABLED = 0x02;
        const SELFTEST_DISABLED = 0x80;
    }
}

impl Default for PIA {
    fn default() -> Self {
        Self {
            regs: [0xff, 0xff, 0xff, 0xff],
        }
    }
}

impl PIA {
    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0x3;
        self.regs[addr]
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0x3;
        if addr != PORTA {
            // TODO
            self.regs[addr] = value;
        }
    }
    pub fn write_port_a(&mut self, mask: u8, value: u8) {
        self.regs[PORTA] = self.regs[PORTA] & mask | value;
    }
}
