pub use bevy::prelude::*;

const RANDOM: usize = 0x0A;
const KBCODE: usize = 0x09;
const SKSTAT: usize = 0x0f;
const IRQST: usize = 0x0e;

pub struct Pokey {
    kbcode: u8,
    skstat: u8,
    irqst: u8,
}

impl Default for Pokey {
    fn default() -> Self {
        Self {
            kbcode: 0xff,
            skstat: 0xff,
            irqst: 0xff,
        }
    }
}

impl Pokey {
    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0xf;
        let value = match addr {
            RANDOM => rand::random(),
            KBCODE => self.kbcode,
            IRQST => {
                self.irqst
            },
            SKSTAT => {
                self.skstat
            },
            _ => 0xff
        };
        //warn!("POKEY read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&self, addr: usize, value: u8) {
        let addr = addr & 0xf;
        //warn!("POKEY write: {:02x}: {:02x}", addr, value);
    }

    pub fn key_press(&mut self, event: &KeyCode, is_pressed: bool, is_shift: bool, is_ctl: bool) {
        let kbcode = match *event {
            KeyCode::Key1 => 0x1f,
            KeyCode::Key2 => 0x1e,
            KeyCode::Key3 => 0x1a,
            KeyCode::Key4 => 0x18,
            KeyCode::Key5 => 0x1d,
            KeyCode::Key6 => 0x1b,
            KeyCode::Key7 => 0x33,
            KeyCode::Key8 => 0x35,
            KeyCode::Key9 => 0x30,
            KeyCode::Key0 => 0x32,
            KeyCode::A => 0x3f,
            KeyCode::B => 0x15,
            KeyCode::C => 0x12,
            KeyCode::D => 0x3a,
            KeyCode::E => 0x2a,
            KeyCode::F => 0x38,
            KeyCode::G => 0x3d,
            KeyCode::H => 0x39,
            KeyCode::I => 0x0d,
            KeyCode::J => 0x01,
            KeyCode::K => 0x05,
            KeyCode::L => 0x00,
            KeyCode::M => 0x25,
            KeyCode::N => 0x23,
            KeyCode::O => 0x08,
            KeyCode::P => 0x0a,
            KeyCode::Q => 0x2f,
            KeyCode::R => 0x28,
            KeyCode::S => 0x3e,
            KeyCode::T => 0x2d,
            KeyCode::U => 0x0b,
            KeyCode::V => 0x10,
            KeyCode::W => 0x2e,
            KeyCode::X => 0x16,
            KeyCode::Y => 0x2b,
            KeyCode::Z => 0x17,
            KeyCode::Escape => 0x1c,
            // KeyCode::LControl => {}
            // KeyCode::LShift => {}
            // KeyCode::RControl => {}
            // KeyCode::RShift => {}
            KeyCode::Back => 0x34,
            KeyCode::Return => 0x0c,
            KeyCode::Space => 0x21,
            KeyCode::Asterisk => 0x07,
            KeyCode::Plus => 0x06,
            KeyCode::Colon => 0x02,
            KeyCode::Comma => 0x20,
            KeyCode::Period => 0x22,
            KeyCode::Semicolon => 0x02,
            KeyCode::Slash => 0x26,
            KeyCode::Tab => 0x2c,
            _ => return,
        };
        if is_pressed {
            self.kbcode = kbcode | ((is_shift as u8) << 6) | ((is_ctl as u8) << 7);
            info!("kbcode: {:02x}", kbcode);
            self.skstat = 0xff - 4;
            self.irqst = 0xff - 0x40;
        } else {
            self.skstat = 0xff;
        }
    }
}