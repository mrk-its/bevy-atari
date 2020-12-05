pub use crate::fm_osc::FmOsc;
use bevy::input::keyboard::KeyboardInput;
pub use bevy::prelude::*;

const RANDOM: usize = 0x0A;
const KBCODE: usize = 0x09;
const SKSTAT: usize = 0x0f;
const IRQST: usize = 0x0e;

bitflags! {
    #[derive(Default)]
    pub struct AUDCTL: u8 {
        const CLOCK_15 = 1;
        const CH2_HIGH_PASS = 2;
        const CH1_HIGH_PASS = 4;
        const CH34_LINKED_CNT = 8;  // bug in Altirra Manual?
        const CH12_LINKED_CNT = 16;
        const CH3_FAST_CLOCK = 32;
        const CH1_FAST_CLOCK = 64;
        const POLY_9BIT = 128;
    }
}

pub struct Pokey {
    clocks: [f32; 4],
    freq: [u8; 4],
    ctl: [u8; 4],
    audctl: AUDCTL,
    kbcode: u8,
    skstat: u8,
    irqst: u8,
    osc: FmOsc,
}

impl Default for Pokey {
    fn default() -> Self {
        Self {
            ctl: [0; 4],
            freq: [0; 4],
            clocks: [0.0; 4],
            kbcode: 0xff,
            skstat: 0xff,
            irqst: 0xff,
            osc: FmOsc::new().unwrap(),
            audctl: AUDCTL::from_bits_truncate(0),
        }
    }
}
unsafe impl Send for Pokey {}
unsafe impl Sync for Pokey {}

impl Pokey {
    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0xf;
        let value = match addr {
            RANDOM => rand::random(),
            KBCODE => self.kbcode,
            IRQST => self.irqst,
            SKSTAT => self.skstat,
            _ => 0xff,
        };
        //warn!("POKEY read: {:02x}: {:02x}", addr, value);
        value
    }

    pub fn update_freq(&mut self, channel: usize, value: u8) {
        self.freq[channel] = value;
        let linked_channel = channel & 0x2; // 0 or 2

        let is_linked_01 = self.audctl.contains(AUDCTL::CH12_LINKED_CNT);
        let is_linked_23 = self.audctl.contains(AUDCTL::CH34_LINKED_CNT);

        let div = if linked_channel == 0 && is_linked_01 || linked_channel == 2 && is_linked_23 {
            2.0 * (7.0
                + self.freq[linked_channel] as f32
                + self.freq[linked_channel + 1] as f32 * 256.0)
        } else {
            2.0 * (1.0 + self.freq[channel] as f32)
        };
        if is_linked_01 && linked_channel == 0 || is_linked_23 && linked_channel == 2 {
            self.osc.set_frequency(linked_channel, self.clocks[linked_channel] / div);
            self.osc.set_gain(linked_channel + 1, 0.0);
        } else {
            self.osc.set_frequency(channel, self.clocks[channel] / div)
        }
    }

    pub fn update_ctl(&mut self, channel: usize, value: u8) {
        self.ctl[channel] = value;

        let opts = value & 0xf0;
        self.osc.set_noise(channel, value & 0x20 == 0);

        let linked_channel = channel & 0x2; // 0 or 2

        let is_linked_01 = self.audctl.contains(AUDCTL::CH12_LINKED_CNT);
        let is_linked_23 = self.audctl.contains(AUDCTL::CH34_LINKED_CNT);

        let gain = 0.1 * (value & 0xf) as f32 / 15.0;

        if is_linked_01 && channel == 0 || is_linked_23 && channel == 2 {
            self.osc.set_gain(linked_channel, gain);
            self.osc.set_gain(linked_channel + 1, 0.0);
        } else {
            self.osc.set_gain(channel, gain);
        }
    }

    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0xf;
        let channel = addr / 2;
        match addr {
            0 | 2 | 4 | 6 => {
                self.update_freq(channel, value);
            }
            1 | 3 | 5 | 7 => {
                self.update_ctl(channel, value);
            }
            8 => {
                self.audctl = AUDCTL::from_bits_truncate(value);
                let slow_clock = if self.audctl.contains(AUDCTL::CLOCK_15) {
                    15600.0
                } else {
                    63514.29
                };
                self.clocks[0] = if self.audctl.contains(AUDCTL::CH1_FAST_CLOCK) {
                    1778400.0
                } else {
                    slow_clock
                };
                self.clocks[1] = slow_clock;
                self.clocks[2] = if self.audctl.contains(AUDCTL::CH3_FAST_CLOCK) {
                    1778400.0
                } else {
                    slow_clock
                };
                self.clocks[3] = slow_clock;
                warn!("AUDCTL: {:?}", self.audctl);
            }
            _ => (),
        }
    }

    pub fn key_press(
        &mut self,
        event: &KeyCode,
        is_pressed: bool,
        is_shift: bool,
        is_ctl: bool,
    ) -> bool {
        let kbcode = match *event {
            KeyCode::F10 => {
                self.osc.resume();
                return false;
            }
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
            _ => return false,
        };
        if is_pressed {
            self.kbcode = kbcode | ((is_shift as u8) << 6) | ((is_ctl as u8) << 7);
            info!("kbcode: {:02x}", kbcode);
            self.skstat = 0xff - 4;
            self.irqst = 0xff - 0x40;
            true
        } else {
            self.skstat = 0xff;
            false
        }
    }
}
