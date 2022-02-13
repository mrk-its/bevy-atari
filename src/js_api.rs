use crate::{AtariSlot, AtariSystem, BreakPoint, Debugger, EmulatorState, CPU};
use bevy::prelude::*;
use emulator_6502::Interface6502;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[allow(dead_code)]
pub static MESSAGES: Lazy<RwLock<Vec<Message>>> = Lazy::new(|| RwLock::new(vec![]));

#[derive(Clone, Debug)]
pub enum Message {
    JoyState {
        port: usize,
        dirs: u8,
        fire: bool,
    },
    SetConsol {
        state: u8,
    },
    BinaryData {
        key: String,
        path: String,
        data: Option<Vec<u8>>,
        slot: Option<i32>,
    },
    Command {
        cmd: String,
    },
    Reset {
        cold: bool,
        disable_basic: bool,
    },
    SetState(String),
    SetResolution {
        width: f32,
        height: f32,
    },
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_joystick(port: usize, dirs: u8, fire: bool) {
    bevy::utils::tracing::info!("set_joystick: {:?} {:?} {:?}", port, dirs, fire);
    let mut messages = MESSAGES.write();
    messages.push(Message::JoyState { port, dirs, fire });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_consol(state: u8) {
    bevy::utils::tracing::info!("set_consol: {:?}", state);
    let mut messages = MESSAGES.write();
    messages.push(Message::SetConsol { state });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_binary_data(key: String, path: String, data: Vec<u8>, slot: Option<i32>) {
    let mut messages = MESSAGES.write();
    let data = if data.len() > 0 { Some(data) } else { None };
    messages.push(Message::BinaryData {
        key,
        path,
        data,
        slot,
    });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn cmd(cmd: String) {
    let mut messages = MESSAGES.write();
    messages.push(Message::Command { cmd });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_state(state: String) {
    let mut messages = MESSAGES.write();
    messages.push(Message::SetState(state));
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn reset(cold: bool, disable_basic: bool) {
    let mut messages = MESSAGES.write();
    messages.push(Message::Reset {
        cold,
        disable_basic,
    });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_resolution(width: f32, height: f32) {
    let mut messages = MESSAGES.write();
    messages.push(Message::SetResolution { width, height });
}

#[wasm_bindgen(catch)]
extern "C" {
    pub fn pokey_post_message(a: &JsValue);
    pub fn sio_get_status(device: u8, unit: u8, data: &mut [u8]) -> u8;
    pub fn sio_get_sector(device: u8, unit: u8, sector: u16, data: &mut [u8]) -> u8;
    pub fn sio_put_sector(device: u8, unit: u8, sector: u16, data: &[u8]) -> u8;

    #[wasm_bindgen(catch)]
    pub async fn ls(path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn readFile(path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn writeFile(path: &str, contents: &[u8]) -> Result<(), JsValue>;
}

pub fn events(
    mut query: Query<(&AtariSlot, &mut AtariSystem, &mut CPU, &mut Debugger)>,
    mut state: ResMut<State<EmulatorState>>,
    mut windows: ResMut<Windows>,
) {
    let mut _messages = MESSAGES.write();
    for (atari_slot, mut atari_system, mut cpu, mut debugger) in query.iter_mut() {
        let mut messages = (*_messages).clone();
        for event in messages.drain(..) {
            match event {
                Message::SetResolution { width, height } => {
                    let window = windows.get_primary_mut().unwrap();
                    window.set_resolution(width, height);
                }
                Message::Reset {
                    cold,
                    disable_basic,
                } => {
                    atari_system.reset(&mut cpu.cpu, cold, disable_basic);
                }
                Message::SetState(new_state) => {
                    let result = match new_state.as_ref() {
                        "running" => state.set(EmulatorState::Running),
                        "idle" => state.set(EmulatorState::Idle),
                        _ => panic!("invalid state requested"),
                    }
                    .ok()
                    .is_some();
                    info!("set_state: {:?}: {:?}", new_state, result);
                }
                Message::JoyState { port, dirs, fire } => {
                    atari_system.set_joystick(1, port, dirs, fire)
                }
                Message::SetConsol { state } => {
                    atari_system.update_consol(1, state);
                }
                Message::BinaryData {
                    key,
                    data,
                    slot,
                    path,
                } => {
                    if slot.is_none() || Some(atari_slot.0) == slot {
                        let data = match data.as_ref() {
                            Some(data) => Some(&data[..]),
                            None => None,
                        };
                        crate::set_binary(&mut atari_system, &mut cpu, &key, &path, data);
                    }
                }
                Message::Command { cmd } => {
                    let parts = cmd.split(" ").collect::<Vec<_>>();
                    match parts[0] {
                        "mem" => {
                            if let Ok(start) = u16::from_str_radix(parts[1], 16) {
                                let mut data = [0 as u8; 256];
                                atari_system.copy_to_slice(start, &mut data);
                                info!("{:x?}", data);
                            }
                        }
                        "write" => {
                            if let Ok(addr) = u16::from_str_radix(parts[1], 16) {
                                if let Ok(value) = u8::from_str_radix(parts[2], 16) {
                                    atari_system.write(addr, value);
                                    info!("write {:04x} <- {:02x}", addr, value);
                                }
                            }
                        }
                        "pc" => {
                            if let Ok(pc) = u16::from_str_radix(parts[1], 16) {
                                cpu.cpu.set_program_counter(pc)
                            }
                        }
                        "brk" => {
                            if let Ok(pc) = u16::from_str_radix(parts[1], 16) {
                                debugger.breakpoints.push(BreakPoint::PC(pc));
                                info!("breakpoint set on pc={:04x}", pc);
                            }
                        }
                        "trainer_init" => {
                            atari_system.trainer_init();
                        }
                        "trainer_changed" => {
                            let cnt = atari_system.trainer_changed(true);
                            info!("matched: {}", cnt);
                        }
                        "trainer_unchanged" => {
                            let cnt = atari_system.trainer_changed(false);
                            info!("matched: {}", cnt);
                        }
                        _ => (),
                    }
                }
            }
        }
    }
    _messages.clear();
}
