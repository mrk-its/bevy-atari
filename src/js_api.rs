use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::messages::{send_message, Message};

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_joystick(port: usize, dirs: u8, fire: bool) {
    bevy::utils::tracing::info!("set_joystick: {:?} {:?} {:?}", port, dirs, fire);
    send_message(Message::JoyState { port, dirs, fire });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_consol(state: u8) {
    bevy::utils::tracing::info!("set_consol: {:?}", state);
    send_message(Message::SetConsol { state });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_binary_data(key: String, path: String, data: Vec<u8>, slot: Option<i32>) {
    let data = if data.len() > 0 { Some(data) } else { None };
    send_message(Message::BinaryData {
        key,
        path,
        data,
        slot,
    });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn cmd(cmd: String) {
    send_message(Message::Command { cmd });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_state(state: String) {
    send_message(Message::SetState(state));
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn reset(cold: bool, disable_basic: bool) {
    send_message(Message::Reset {
        cold,
        disable_basic,
    });
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_resolution(width: f32, height: f32) {
    send_message(Message::SetResolution { width, height });
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
