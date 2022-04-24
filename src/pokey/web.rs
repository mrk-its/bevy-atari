use bevy::prelude::info;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};

pub struct Context {
    audio_context: Option<web_sys::AudioContext>,
}

impl Context {
    const LATENCY: f64 = 0.05;

    pub fn is_running(&self) -> bool {
        match self.audio_context.as_ref() {
            Some(ctx) => ctx.state() == web_sys::AudioContextState::Running,
            None => false,
        }
    }

    pub fn current_time(&self) -> f64 {
        match self.audio_context.as_ref() {
            Some(ctx) => ctx.current_time(),
            None => 0.0,
        }
    }

    pub fn send_regs(&mut self, regs: &[Vec<super::PokeyRegWrite>], delta_t: f64) {
        let js_arr = regs
            .iter()
            .map(|reg_writes| {
                let js_arr = reg_writes
                    .iter()
                    .flat_map(|r| {
                        [
                            r.index as f64,
                            r.value as f64,
                            r.timestamp as f64 / (312.0 * 114.0 * 50.0) - delta_t + Self::LATENCY,
                        ]
                    })
                    .map(|f| JsValue::from_f64(f))
                    .collect::<js_sys::Array>();
                JsValue::from(js_arr)
            })
            .collect::<js_sys::Array>();
        let js_value = JsValue::from(js_arr);
        crate::js_api::pokey_post_message(&js_value)
    }
}

impl Default for Context {
    fn default() -> Self {
        let audio_context = {
            let window = web_sys::window().expect("no global `window` exists");
            js_sys::Reflect::get(&window, &"audio_context".into())
                .expect("no window.audio_context")
                .dyn_into::<web_sys::AudioContext>()
                .ok()
        };
        Self { audio_context }
    }
}
