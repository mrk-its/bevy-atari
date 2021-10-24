pub use bevy::prelude::*;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use wasm_bindgen::{JsCast, JsValue};

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
    timestamp: usize,
}

pub struct Pokey {
    audio_context: web_sys::AudioContext,
    freq: [u8; 4],
    ctl: [AUDC; 4],
    audctl: AUDCTL,
    kbcode: u8,
    skstat: u8,
    irqst: u8,
    pub irqen: IRQ,
    rng: SmallRng,
    pub total_cycles: usize,
    pub reg_writes: Vec<PokeyRegWrite>,
    pub delta_t: f64,
}

impl Default for Pokey {
    fn default() -> Self {
        let rng = SmallRng::from_seed([0; 16]);
        let window = web_sys::window().expect("no global `window` exists");
        let audio_context = unsafe {
            js_sys::Reflect::get(&window, &"audio_context".into())
                .expect("no window.audio_context")
                .dyn_into::<web_sys::AudioContext>()
                .expect("cannot cast to AudioContext")
        };
        Self {
            audio_context,
            rng,
            ctl: [AUDC::from_bits_truncate(0); 4],
            freq: [0; 4],
            kbcode: 0xff,
            skstat: 0xff,
            irqst: 0xff,
            irqen: IRQ::from_bits_truncate(0xff),
            audctl: AUDCTL::from_bits_truncate(0),
            total_cycles: 0,
            reg_writes: Vec::new(),
            delta_t: 0.0,
        }
    }
}
unsafe impl Send for Pokey {}
unsafe impl Sync for Pokey {}

impl Pokey {
    const LATENCY: f64 = 0.05;
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

    // const IDLE_DELAY: usize = 2;

    #[cfg(target_arch = "wasm32")]
    pub fn send_regs(&mut self) {

        // let window = web_sys::window().expect("no global `window` exists");

        let state = self.audio_context.state();
        if state != web_sys::AudioContextState::Running {
            // skipping writes this way may lead to bad pokey state
            // for example some channels may still generate sound
            // or we may have wrong audctl value

            // TODO - reset POKEY (or at least mute all channels) on resume?

            // but typically all pokey registers are frequenty updated
            // so let's ignore it for a while
            return;
        }

        let audio_context_time = self.audio_context.current_time();

        let atari_time = self.total_cycles as f64 / (312.0 * 114.0 * 50.0);

        let time_diff = atari_time - self.delta_t - audio_context_time;
        if time_diff.abs() >= 0.05 {
            self.delta_t = atari_time - audio_context_time;
            warn!("too big time diff: {}, syncing", time_diff,);
        }

        // #[allow(unused_unsafe)]
        // let port = unsafe {
        //     js_sys::Reflect::get(&window, &"pokey_port".into())
        //         .expect("no pokey_port exists")
        //         .dyn_into::<web_sys::MessagePort>()
        //         .expect("cannot cast to MessagePort")
        // };
        let regs = std::mem::take(&mut self.reg_writes);

        let js_regs = regs
            .iter()
            .flat_map(|r| {
                [
                    r.index as f64,
                    r.value as f64,
                    r.timestamp as f64 / (312.0 * 114.0 * 50.0) - self.delta_t + Self::LATENCY,
                ]
            })
            .map(|f| JsValue::from_f64(f))
            .collect::<js_sys::Array>();
        let js_regs = JsValue::from(js_regs);

        unsafe { crate::js_api::pokey_post_message(&js_regs) };

        // port.post_message(&js_regs).expect("cannot post_message");
        // info!("pokey regs: {:?} {:?}", regs, port);
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn send_regs(&mut self) {}

    pub fn scanline_tick(&mut self, _scanline: usize) {}

    pub fn update_freq(&mut self, channel: usize, value: u8) {
        self.freq[channel] = value;
    }

    pub fn update_audctl(&mut self, value: u8) {
        self.audctl = AUDCTL::from_bits_truncate(value);
    }

    pub fn update_ctl(&mut self, channel: usize, value: u8) {
        self.ctl[channel] = AUDC::from_bits_truncate(value);
    }

    pub fn write(&mut self, addr: usize, value: u8) {
        if addr & 0xf <= 8 {
            self.reg_writes.push(PokeyRegWrite {
                index: addr as u8,
                value,
                timestamp: self.total_cycles,
            })
        }

        let addr = addr & 0xf;
        let channel = addr / 2;
        if addr <= 8 {
            self.reg_writes.push(PokeyRegWrite {
                index: addr as u8,
                value,
                timestamp: self.total_cycles,
            })
        }
        match addr {
            0 | 2 | 4 | 6 => {
                self.update_freq(channel, value);
            }
            1 | 3 | 5 | 7 => {
                self.update_ctl(channel, value);
            }
            8 => {
                self.update_audctl(value);
            }
            SKCTL => {
                if value & 3 == 0 {
                    // info!("POKEY reset!");
                }
            }
            IRQEN => self.irqen = IRQ::from_bits_truncate(value),
            _ => (),
        }
    }

    pub fn key_press(
        &mut self,
        event: &KeyCode,
        is_pressed: bool,
        mut is_shift: bool,
        mut is_ctl: bool,
    ) -> bool {
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
            KeyCode::Capital => 0x3c,
            KeyCode::Up => {
                is_ctl = true;
                0x0e
            }
            KeyCode::Down => {
                is_ctl = true;
                0x0f
            }
            KeyCode::Left => {
                is_ctl = true;
                0x06
            }
            KeyCode::Right => {
                is_ctl = true;
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
