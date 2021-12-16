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

use wasm_bindgen::JsValue;

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
