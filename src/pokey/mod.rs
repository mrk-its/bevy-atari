use std::cell::RefCell;
use std::sync::Arc;

use crate::EmulatorConfig;
pub use bevy::prelude::*;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
#[cfg(target_arch = "wasm32")]
#[path = "web.rs"]
mod audio;

#[cfg(not(target_arch = "wasm32"))]
#[path = "native.rs"]
mod audio;

const RANDOM: usize = 0x0A;
const KBCODE: usize = 0x09;
const SKCTL: usize = 0x0f;
const SKSTAT: usize = 0x0f;
const IRQST: usize = 0x0e;
const IRQEN: usize = 0x0e;

pub const CLOCK_177: f32 = 1778400.0;
pub const DIVIDER_64K: u32 = 28;
pub const DIVIDER_15K: u32 = 114;

bitflags! {
    #[derive(Default)]
    pub struct AUDCTL: u8 {
        const CLOCK_15 = 1;
        const CH2_HIGH_PASS = 2;
        const CH1_HIGH_PASS = 4;
        const CH34_LINKED_CNT = 8;
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

bitflags! {
    pub struct IRQ: u8 {
        const BRK = 0x80;
        const KEY = 0x40;
        const SIN = 0x20;
        const SOUT = 0x10;
        const SCMP = 0x08;
        const T4 = 0x04;
        const T2 = 0x02;
        const T1 = 0x01;
    }
}

pub struct PokeyRegWrite {
    index: u8,
    value: u8,
    timestamp: u64,
}

#[derive(Default)]
pub struct PokeyRegQueue {
    pub stereo: bool,
    queue: [Vec<PokeyRegWrite>; 2],
    pub total_cycles: u64,
}

impl PokeyRegQueue {
    pub fn write(&mut self, index: u8, value: u8) {
        let pokey_index = if !self.stereo {
            0
        } else {
            ((index >> 4) & 1) as usize
        };
        self.queue[pokey_index].push(PokeyRegWrite {
            index: index & 0xf,
            value,
            timestamp: self.total_cycles,
        })
    }
}
pub struct Pokey {
    audio_context: audio::Context,
    muted: bool,
    kbcode: u8,
    skstat: u8,
    irqst: u8,
    pub irqen: IRQ,
    rng: SmallRng,
    pub pokey_reg_queue: Arc<RefCell<PokeyRegQueue>>,
    pub delta_t: f64,
}

impl Default for Pokey {
    fn default() -> Self {
        let rng = SmallRng::from_seed([0; if cfg!(target_arch = "wasm32") { 16 } else { 32 }]);

        Self {
            muted: false,
            rng,
            kbcode: 0xff,
            skstat: 0xff,
            irqst: 0xff,
            irqen: IRQ::from_bits_truncate(0xff),
            pokey_reg_queue: Default::default(),
            delta_t: 0.0,
            audio_context: Default::default(),
        }
    }
}
unsafe impl Send for Pokey {}
unsafe impl Sync for Pokey {}

impl Pokey {
    pub fn read(&mut self, addr: usize) -> u8 {
        let addr = addr & 0xf;
        let value = match addr {
            RANDOM => self.rng.gen(),
            KBCODE => self.kbcode,
            IRQST => self.irqst,
            SKSTAT => self.skstat,
            _ => 0xff,
        };
        // warn!("POKEY read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn mute(&mut self, muted: bool) {
        self.muted = muted;
    }
    // const IDLE_DELAY: usize = 2;

    pub fn send_regs(&mut self) {
        if self.muted || !self.audio_context.is_running() {
            // skipping writes this way may lead to bad pokey state
            // for example some channels may still generate sound
            // or we may have wrong audctl value

            // TODO - reset POKEY (or at least mute all channels) on resume?

            // but typically all pokey registers are frequenty updated
            // so let's ignore it for a while
            return;
        }
        let mut reg_queue = self.pokey_reg_queue.borrow_mut();

        let audio_context_time = self.audio_context.current_time();

        let atari_time = reg_queue.total_cycles as f64 / (312.0 * 114.0 * 50.0);

        let time_diff = atari_time - self.delta_t - audio_context_time;
        if time_diff.abs() >= 0.05 {
            self.delta_t = atari_time - audio_context_time;
            info!("too big time diff: {}, syncing", time_diff,);
        }

        let reg_queues = if reg_queue.stereo {
            &reg_queue.queue[..]
        } else {
            &reg_queue.queue[..1]
        };

        self.audio_context.send_regs(reg_queues, self.delta_t);
        reg_queue.queue[0].clear();
        reg_queue.queue[1].clear();
    }

    pub fn scanline_tick(&mut self, _scanline: usize) {}

    pub fn write(&mut self, addr: usize, value: u8) {
        if addr & 0xf <= 8 {
            self.pokey_reg_queue.borrow_mut().write(addr as u8, value)
        }

        let addr = addr & 0xf;
        match addr {
            SKCTL => {
                if value & 3 == 0 {
                    // info!("POKEY reset!");
                }
            }
            IRQEN => {
                self.irqen = IRQ::from_bits_truncate(value);
                self.irqst |= !self.irqen.bits;
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
        config: &EmulatorConfig,
    ) -> bool {
        let mut is_ctl = is_ctl;
        let mut is_shift = is_shift;

        let kbcode = match *event {
            KeyCode::Key1 => 0x1f,
            KeyCode::Key2 => {
                if is_shift {
                    0x35
                } else {
                    0x1e
                }
            }
            KeyCode::Key3 => 0x1a,
            KeyCode::Key4 => 0x18,
            KeyCode::Key5 => 0x1d,
            KeyCode::Key6 => {
                if is_shift {
                    0x07
                } else {
                    0x1b
                }
            }
            KeyCode::Key7 => {
                if is_shift {
                    0x1b
                } else {
                    0x33
                }
            }
            KeyCode::Key8 => {
                if is_shift {
                    is_shift = false;
                    0x7
                } else {
                    0x35
                }
            }
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
            KeyCode::Capital => 0x3c,
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
            KeyCode::Comma => {
                if is_shift {
                    is_shift = false;
                    0x36
                } else {
                    0x20
                }
            }
            KeyCode::Period => {
                if is_shift {
                    is_shift = false;
                    0x37
                } else {
                    0x22
                }
            }
            KeyCode::Semicolon => 0x02,
            KeyCode::Slash => 0x26,
            KeyCode::Tab => 0x2c,
            KeyCode::Minus => 0x0e,
            KeyCode::LBracket => {
                is_shift = true;
                0x20
            }
            KeyCode::RBracket => {
                is_shift = true;
                0x22
            }
            KeyCode::Equals => {
                if is_shift {
                    is_shift = false;
                    0x06
                } else {
                    0x0f
                }
            }
            KeyCode::Apostrophe => {
                if is_shift {
                    0x1e
                } else {
                    is_shift = true;
                    0x33
                }
            }
            KeyCode::Backslash => {
                if is_shift {
                    0x0f
                } else {
                    is_shift = true;
                    0x06
                }
            }
            KeyCode::F1 => 0x11,
            KeyCode::F7 => {
                // break
                if is_pressed {
                    self.irqst &= !0x80;
                    self.skstat = 0xff;
                    self.kbcode = 0xff;
                }
                return is_pressed;
            }
            // KeyCode::Capital => 0x3c,
            KeyCode::Up => {
                is_ctl = (is_ctl || config.arrows_force_ctl) ^ config.arrows_neg_ctl;
                0x0e
            }
            KeyCode::Down => {
                is_ctl = (is_ctl || config.arrows_force_ctl) ^ config.arrows_neg_ctl;
                0x0f
            }
            KeyCode::Left => {
                is_ctl = (is_ctl || config.arrows_force_ctl) ^ config.arrows_neg_ctl;
                0x06
            }
            KeyCode::Right => {
                is_ctl = (is_ctl || config.arrows_force_ctl) ^ config.arrows_neg_ctl;
                0x07
            }
            _ => return false,
        };
        self.kbcode = self.kbcode & 0x3f | ((is_shift as u8) << 6) | ((is_ctl as u8) << 7);
        self.skstat = self.skstat & !(0xc) | ((!is_shift as u8) << 3) | ((!is_pressed as u8) << 2);
        if is_pressed {
            self.kbcode = self.kbcode & !0x3f | kbcode & 0x3f;
            self.irqst &= !0x40;
        };
        // info!("kbcode: {:?}, is_pressed: {:?}", self.kbcode, is_pressed);
        is_pressed
    }
}
