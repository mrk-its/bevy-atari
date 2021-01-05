use once_cell::sync::Lazy;
use parking_lot::RwLock;
use wasm_bindgen::prelude::wasm_bindgen;

#[allow(dead_code)]
pub static ARRAY: Lazy<RwLock<Vec<Message>>> = Lazy::new(|| RwLock::new(vec![]));

pub enum Message {
    JoyState {
        port: usize,
        up: bool,
        down: bool,
        left: bool,
        right: bool,
        fire: bool,
    },
    BinaryData {
        key: String,
        filename: String,
        data: Option<Vec<u8>>,
    },
    Command {
        cmd: String,
    },
    Reset {
        cold: bool,
        disable_basic: bool,
    },
    SetState(String),
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_joystick(port: usize, up: bool, down: bool, left: bool, right: bool, fire: bool) {
    let mut guard = ARRAY.write();
    guard.push(Message::JoyState {
        port,
        left,
        right,
        up,
        down,
        fire,
    });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_binary_data(key: String, filename: String, data: Vec<u8>) {
    let mut guard = ARRAY.write();
    let data = if data.len() > 0 { Some(data) } else { None };
    guard.push(Message::BinaryData {
        key,
        filename,
        data,
    });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn cmd(cmd: String) {
    let mut guard = ARRAY.write();
    guard.push(Message::Command { cmd });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_state(state: String) {
    let mut guard = ARRAY.write();
    guard.push(Message::SetState(state));
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn reset(cold: bool, disable_basic: bool) {
    let mut guard = ARRAY.write();
    guard.push(Message::Reset { cold, disable_basic });
}
