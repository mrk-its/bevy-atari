
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, OscillatorType};

/// Converts a midi note to frequency
///
/// A midi note is an integer, generally in the range of 21 to 108
pub fn midi_to_freq(note: u8) -> f32 {
    27.5 * 2f32.powf((note as f32 - 21.0) / 12.0)
}

pub struct FmOsc {
    ctx: AudioContext,
    oscilators: [web_sys::OscillatorNode; 4],
    gains: [web_sys::GainNode; 4],

    resumed: bool,
}

impl Drop for FmOsc {
    fn drop(&mut self) {
        let _ = self.ctx.close();
    }
}

impl FmOsc {
    pub fn new() -> Result<FmOsc, JsValue> {
        let ctx = web_sys::AudioContext::new()?;

        let oscilators = [ctx.create_oscillator()?, ctx.create_oscillator()?, ctx.create_oscillator()?, ctx.create_oscillator()?];
        let gains = [ctx.create_gain()?, ctx.create_gain()?, ctx.create_gain()?, ctx.create_gain()?];

        for i in 0..4 {
            oscilators[i].set_type(OscillatorType::Square);
            oscilators[i].frequency().set_value(440.0);
            gains[i].gain().set_value(0.0);
            oscilators[i].connect_with_audio_node(&gains[i])?;
            gains[i].connect_with_audio_node(&ctx.destination())?;
            oscilators[i].start()?;
        }

        Ok(FmOsc {
            ctx,
            oscilators,
            gains,
            resumed: false,
        })
    }
    pub fn resume(&mut self) {
        self.ctx.resume().ok();
        self.resumed = true
    }

    /// Sets the gain for this oscillator, between 0.0 and 1.0.
    pub fn set_gain(&self, channel: usize, gain: f32) {
        self.gains[channel].gain().set_value(gain);
    }

    pub fn set_frequency(&self, channel: usize, freq: f32) {
        self.oscilators[channel].frequency().set_value(freq);
    }

    pub fn set_note(&self, channel: usize, note: u8) {
        let freq = midi_to_freq(note);
        self.set_frequency(channel, freq);
    }
}
