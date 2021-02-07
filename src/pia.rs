use crate::system::PORTB;
pub struct PIA {
    porta_dir: u8, // 0 - input  1 - output
    portb_dir: u8,
    porta_ctl: u8,
    portb_ctl: u8,
    portb_out: u8,
    porta_out: u8,
    portb_in: u8,
    porta_in: u8,
}

const PORTA_ADDR: usize = 0;
const PORTB_ADDR: usize = 1;
const PACTL_ADDR: usize = 2;
const PBCTL_ADDR: usize = 3;

const DIR: u8 = 4;

impl Default for PIA {
    fn default() -> Self {
        Self {
            porta_dir: 0,
            portb_dir: 0,
            porta_ctl: 0,
            portb_ctl: 0,
            porta_out: 0xff,
            portb_out: 0xff,
            porta_in: 0xff,
            portb_in: 0xff,
        }
    }
}

impl PIA {
    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 3;
        match addr {
            PACTL_ADDR => self.porta_ctl,
            PBCTL_ADDR => self.portb_ctl,
            PORTA_ADDR => {
                if self.porta_ctl & DIR == 0 {
                    self.porta_dir
                } else {
                    self.porta_out & self.porta_dir | self.porta_in & !self.porta_dir
                }
            }
            PORTB_ADDR => {
                if self.portb_ctl & DIR == 0 {
                    self.portb_dir
                } else {
                    self.portb_out & self.portb_dir | self.portb_in & !self.portb_dir
                }
            }
            _ => panic!("impossible"),
        }
    }

    pub fn write(&mut self, addr: usize, value: u8) {
        match addr & 3 {
            PACTL_ADDR => self.porta_ctl = value,
            PBCTL_ADDR => self.portb_ctl = value,
            PORTA_ADDR => {
                if self.porta_ctl & DIR == 0 {
                    self.porta_dir = value
                } else {
                    self.porta_out = value
                }
            }
            PORTB_ADDR => {
                if self.portb_ctl & DIR == 0 {
                    self.portb_dir = value
                } else {
                    self.portb_out = value
                }
            }
            _ => panic!("impossible"),
        }
    }
    pub fn set_port_a_input(&mut self, mask: u8, value: u8) {
        self.porta_in = self.porta_in & mask | value;
    }
    pub fn portb_out(&self) -> PORTB {
        return PORTB::from_bits_truncate(self.portb_out);
    }
    pub fn set_portb_out(&mut self, value: u8) {
        self.portb_out = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_defaults() {
        let pia = PIA::default();
        assert_eq!(pia.read(PORTA_ADDR), 0x0);
        assert_eq!(pia.read(PORTB_ADDR), 0x0);
        assert_eq!(pia.read(PACTL_ADDR), 0x0);
        assert_eq!(pia.read(PBCTL_ADDR), 0x0);

        assert_eq!(pia.porta_out, 0xff);
        assert_eq!(pia.portb_out, 0xff);
    }

    #[test]
    fn test_porta_input() {
        let mut pia = PIA::default();
        assert_eq!(pia.read(PORTA_ADDR), 0);
        pia.write(PACTL_ADDR, DIR);
        assert_eq!(pia.read(PACTL_ADDR), DIR);
        assert_eq!(pia.read(PORTA_ADDR), 0xff);

        pia.set_port_a_input(0xf0, 0x0a);
        assert_eq!(pia.read(PORTA_ADDR), 0xfa);
    }
}
