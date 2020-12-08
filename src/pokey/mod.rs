mod web_audio;
pub use bevy::prelude::*;
use web_audio::AudioBackend;

const RANDOM: usize = 0x0A;
const KBCODE: usize = 0x09;
const SKSTAT: usize = 0x0f;
const IRQST: usize = 0x0e;

pub const CLOCK_177: f32 = 1778400.0;
pub const DIVIDER_64K: u32 = 28;
pub const DIVIDER_15K: u32 = 114;

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
bitflags! {
    pub struct AUDC: u8 {
        const NOT_5BIT = 128;
        const NOISE_4BIT = 64;
        const NOT_NOISE = 32;
        const VOL_ONLY = 16;
        const VOL_MASK = 15;
    }
}

pub struct Pokey {
    delay: [usize; 4],
    clock_divider: [u32; 4],
    freq: [u8; 4],
    ctl: [AUDC; 4],
    audctl: AUDCTL,
    kbcode: u8,
    skstat: u8,
    irqst: u8,
    backend: AudioBackend,
}

impl Default for Pokey {
    fn default() -> Self {
        Self {
            delay: [0; 4],
            ctl: [AUDC::from_bits_truncate(0); 4],
            freq: [0; 4],
            clock_divider: [DIVIDER_64K; 4],
            kbcode: 0xff,
            skstat: 0xff,
            irqst: 0xff,
            backend: AudioBackend::new().unwrap(),
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

    const IDLE_DELAY: usize = 100;
    pub fn tick(&mut self) {
        for channel in 0..4 {
            if self.delay[channel] == 0 {
                continue;
            }
            self.delay[channel] -= 1;
            if self.delay[channel] == 0 {
                self.setup_channel(channel);
            }
        }
    }

    pub fn setup_channel(&mut self, channel: usize) {
        let is_linked_01 = self.audctl.contains(AUDCTL::CH12_LINKED_CNT);
        let is_linked_23 = self.audctl.contains(AUDCTL::CH34_LINKED_CNT);
        let (divider, clock_divider) = if channel == 1 && is_linked_01 || channel == 3 && is_linked_23 {
            let divider = 7 + (self.freq[channel-1] as u32) + (self.freq[channel] as u32) * 256;
            let clock_divider = self.clock_divider[channel-1];
            (divider, clock_divider)
        } else {
            let divider = 1 + (self.freq[channel] as u32);
            let clock_divider = self.clock_divider[channel];
            (divider, clock_divider)
        };
        assert!(clock_divider>0);
        assert!(divider>0);
        let freq = CLOCK_177 / clock_divider as f32 / divider as f32 / 2.0;

        self.backend.setup_channel(
            channel,
            self.audctl,
            self.ctl[channel],
            divider,
            clock_divider,
            freq,
        );
        let gain = 0.25 * (self.ctl[channel] & AUDC::VOL_MASK).bits as f32 / 15.0;
        self.backend.set_gain(channel, gain);
        let is_linked = is_linked_01 && channel == 1 || is_linked_23 && channel == 3;
        if is_linked {
            self.backend.set_gain(channel - 1, 0.0);
        }
        // warn!(
        //     "setup channel {}, linked: {}, divider: {:04x}, freq: {}Hz",
        //     channel, is_linked, divider, freq
        // );
    }

    pub fn reset_idle(&mut self, channel: usize) {
        let linked_channel = channel & 0x2; // 0 or 2
        if linked_channel == 0 && self.audctl.contains(AUDCTL::CH12_LINKED_CNT)
            || linked_channel == 2 && self.audctl.contains(AUDCTL::CH34_LINKED_CNT)
        {
            self.delay[linked_channel + 1] = Pokey::IDLE_DELAY;
        } else {
            self.delay[channel] = Pokey::IDLE_DELAY;
        }
    }

    pub fn update_freq(&mut self, channel: usize, value: u8) {
        self.reset_idle(channel);
        self.freq[channel] = value;
        // warn!("FREQ: channel: {} value: {:02x}", channel, value);
    }
    // ch34 linked, ch3 fast clock
    // freq: e605, 586.15686Hz
    // ctl: ch3: c7
    // ctl: ch2: 00

    pub fn update_ctl(&mut self, channel: usize, value: u8) {
        self.reset_idle(channel);
        self.ctl[channel] = AUDC::from_bits_truncate(value);
        // info!("update_ctl channel: {}, value: {:02x}, {:?}", channel, value, self.ctl[channel]);
        // warn!("CTL: channel: {} value: {:02x}", channel, value);

        // let is_linked_01 = self.audctl.contains(AUDCTL::CH12_LINKED_CNT);
        // let is_linked_23 = self.audctl.contains(AUDCTL::CH34_LINKED_CNT);

        // let gain = 0.5 * (value & 0xf) as f32 / 15.0;

        // if is_linked_01 && channel == 1 || is_linked_23 && channel == 3 {
        //     self.backend
        //         .set_noise(channel, value & 0x20 == 0, self.audctl, value);
        //     self.backend.set_gain(channel, gain);
        //     self.backend.set_gain(channel - 1, 0.0);
        // } else {
        //     self.backend
        //         .set_noise(channel, value & 0x20 == 0, self.audctl, value);
        //     self.backend.set_gain(channel, gain);
        // }
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
                    DIVIDER_15K
                } else {
                    DIVIDER_64K
                };
                self.clock_divider[0] = if self.audctl.contains(AUDCTL::CH1_FAST_CLOCK) {
                    1
                } else {
                    slow_clock
                };
                self.clock_divider[1] = slow_clock;
                self.clock_divider[2] = if self.audctl.contains(AUDCTL::CH3_FAST_CLOCK) {
                    1
                } else {
                    slow_clock
                };
                self.clock_divider[3] = slow_clock;
                // warn!("AUDCTL: {:?}", self.audctl);
            }
            _ => (),
        }
    }
    pub fn resume(&mut self) {
        self.backend.resume()
    }
    pub fn key_press(
        &mut self,
        event: &KeyCode,
        is_pressed: bool,
        is_shift: bool,
        is_ctl: bool,
    ) -> bool {
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
