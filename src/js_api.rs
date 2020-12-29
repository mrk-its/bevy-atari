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
        filename: String,
    },
    Command {
        cmd: String,
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
pub fn load_file(data: Vec<u8>, filename: String) {
    let mut guard = ARRAY.write();
    guard.push(Message::DraggedFileData { data, filename});
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn cmd(cmd: String) {
    let mut guard = ARRAY.write();
    guard.push(Message::Command { cmd});
}
