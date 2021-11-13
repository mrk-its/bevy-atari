use once_cell::sync::Lazy;
use parking_lot::RwLock;
use wasm_bindgen::prelude::wasm_bindgen;

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
        filename: String,
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
pub fn set_binary_data(key: String, filename: String, data: Vec<u8>, slot: Option<i32>) {
    let mut messages = MESSAGES.write();
    let data = if data.len() > 0 { Some(data) } else { None };
    bevy::log::info!("slot: {:?}", slot);
    messages.push(Message::BinaryData {
        key,
        filename,
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
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    pub fn pokey_post_message(a: &JsValue);
}
