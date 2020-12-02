use once_cell::sync::Lazy;
use parking_lot::RwLock;
use wasm_bindgen::prelude::wasm_bindgen;

#[allow(dead_code)]
pub static ARRAY: Lazy<RwLock<Vec<JoyState>>> = Lazy::new(|| RwLock::new(vec![]));

#[derive(Default, Debug)]
pub struct JoyState {
    pub port: usize,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub fire: bool,
}

#[allow(dead_code)]
#[wasm_bindgen]
pub fn set_joystick(port: usize, up: bool, down: bool, left: bool, right: bool, fire: bool) {
    let mut guard = ARRAY.write();
    guard.push(JoyState {
        port,
        left,
        right,
        up,
        down,
        fire,
    });
}
