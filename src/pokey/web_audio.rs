use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, OscillatorType};
use bevy::prelude::{info};
pub struct AudioBackend {
    ctx: AudioContext,
    oscilator: [web_sys::OscillatorNode; 4],
    oscilator_gain: [web_sys::GainNode; 4],
    noise: [web_sys::AudioBufferSourceNode; 4],
    noise_gain: [web_sys::GainNode; 4],
    is_noise: [bool; 4],
    resumed: bool,
}

impl Drop for AudioBackend {
    fn drop(&mut self) {
        let _ = self.ctx.close();
    }
}

impl AudioBackend {
    pub fn new() -> Result<AudioBackend, JsValue> {
        let ctx = web_sys::AudioContext::new()?;

        let oscilator = [
            ctx.create_oscillator()?,
            ctx.create_oscillator()?,
            ctx.create_oscillator()?,
            ctx.create_oscillator()?,
        ];
        let oscilator_gain = [
            ctx.create_gain()?,
            ctx.create_gain()?,
            ctx.create_gain()?,
            ctx.create_gain()?,
        ];
        let noise_gain = [
            ctx.create_gain()?,
            ctx.create_gain()?,
            ctx.create_gain()?,
            ctx.create_gain()?,
        ];
        let noise = [
            AudioBackend::create_noise_source(&ctx)?,
            AudioBackend::create_noise_source(&ctx)?,
            AudioBackend::create_noise_source(&ctx)?,
            AudioBackend::create_noise_source(&ctx)?,
        ];
        let is_noise = [false; 4];

        for i in 0..4 {
            oscilator[i].set_type(OscillatorType::Square);
            oscilator[i].frequency().set_value(0.0);
            oscilator_gain[i].gain().set_value(0.0);
            oscilator[i].connect_with_audio_node(&oscilator_gain[i])?;
            oscilator_gain[i].connect_with_audio_node(&ctx.destination())?;
            noise[i].connect_with_audio_node(&noise_gain[i])?;
            noise_gain[i].connect_with_audio_node(&ctx.destination())?;
            noise_gain[i].gain().set_value(0.0);
            oscilator[i].start()?;
            noise[i].start()?;
        }
        Ok(AudioBackend {
            ctx,
            oscilator,
            oscilator_gain,
            noise,
            noise_gain,
            is_noise,
            resumed: false,
        })
    }

    pub fn create_noise_source(
        ctx: &web_sys::AudioContext,
    ) -> Result<web_sys::AudioBufferSourceNode, JsValue> {
        const N: usize = 44100;
        let buffer = ctx.create_buffer(1, N as u32, N as f32)?;
        let mut source: [f32; N] = [0.0; N];
        for i in 0..N {
            source[i] = (rand::random::<i32>() & 1) as f32 * 2.0 - 1.0;
        }
        buffer.copy_to_channel(&mut source, 0)?;
        let noise_source = ctx.create_buffer_source()?;
        noise_source.set_buffer(Some(&buffer));
        noise_source.set_loop(true);
        Ok(noise_source)
    }

    pub fn resume(&mut self) {
        self.ctx.resume().ok();
    }

    /// Sets the gain for this oscillator, between 0.0 and 1.0.
    pub fn set_gain(&self, channel: usize, gain: f32) {
        let (enable, disable) = if self.is_noise[channel] {
            (&self.noise_gain[channel], &self.oscilator_gain[channel])
        } else {
            (&self.oscilator_gain[channel], &self.noise_gain[channel])
        };
        enable.gain().set_value(gain);
        disable.gain().set_value(0.0);
    }

    pub fn set_frequency(&self, channel: usize, freq: f32) {
        if !self.is_noise[channel] {
            self.oscilator[channel].frequency().set_value(freq);
        } else {
            self.noise[channel].playback_rate().set_value(1.0 * freq / 22050.0)
        }
    }
    pub fn set_noise(&mut self, channel: usize, enable: bool) {
        self.is_noise[channel] = enable
    }
}
