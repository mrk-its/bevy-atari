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
    DraggedFileData {
        data: Vec<u8>,
    },
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
pub fn set_state(data: Vec<u8>) {
    let mut guard = ARRAY.write();
    guard.push(Message::DraggedFileData { data: data });
}
