use super::{AUDC, AUDCTL, CLOCK_177};
use bevy::utils::tracing::info;
use lru::LruCache;
use wasm_bindgen::prelude::*;
use web_sys::{AudioBuffer, AudioContext, OscillatorType};

const MIN_SAMPLE_RATE: f32 = 8000.0;
const MAX_SAMPLE_RATE: f32 = 96000.0;
const SAMPLE_DUR: f32 = 0.2;

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct NoiseDescr {
    pub divider: u32,
    pub clock_divider: u32,
    pub ctl: AUDC,
    pub audctl: AUDCTL,
}

pub struct AudioBackend {
    ctx: AudioContext,
    poly_4: &'static [u8; 15],
    poly_5: &'static [u8; 31],
    poly_9: &'static [u8; 511],
    poly_17: &'static [u8; 131071],
    oscillator: [web_sys::OscillatorNode; 4],
    oscillator_gain: [web_sys::GainNode; 4],
    oscillator_is_started: [bool; 4],
    buffer_source: [Option<web_sys::AudioBufferSourceNode>; 4],
    white_noise: web_sys::AudioBuffer,
    gain: [web_sys::GainNode; 4],
    noise_buffer_cache: LruCache<NoiseDescr, AudioBuffer>,
}

impl Drop for AudioBackend {
    fn drop(&mut self) {
        let _ = self.ctx.close();
    }
}

impl AudioBackend {
    pub fn new() -> Result<AudioBackend, JsValue> {
        let ctx = web_sys::AudioContext::new()?;

        let oscillator = [
            ctx.create_oscillator()?,
            ctx.create_oscillator()?,
            ctx.create_oscillator()?,
            ctx.create_oscillator()?,
        ];
        let oscillator_gain = [
            ctx.create_gain()?,
            ctx.create_gain()?,
            ctx.create_gain()?,
            ctx.create_gain()?,
        ];
        let gain = [
            ctx.create_gain()?,
            ctx.create_gain()?,
            ctx.create_gain()?,
            ctx.create_gain()?,
        ];
        let buffer_source = [None, None, None, None];

        let white_noise = AudioBackend::create_white_noise_buffer(&ctx)?;

        for i in 0..4 {
            oscillator[i].set_type(OscillatorType::Square);
            oscillator[i].frequency().set_value(0.0);
            oscillator[i].connect_with_audio_node(&oscillator_gain[i])?;
            oscillator_gain[i].connect_with_audio_node(&gain[i])?;
            gain[i].gain().set_value(0.0);
            gain[i].connect_with_audio_node(&ctx.destination())?;
            oscillator[i].start()?;
        }
        let oscillator_is_started = [false; 4];
        let mut backend = AudioBackend {
            ctx,
            poly_4: include_bytes!("poly_4.dat"),
            poly_5: include_bytes!("poly_5.dat"),
            poly_9: include_bytes!("poly_9.dat"),
            poly_17: include_bytes!("poly_17.dat"),
            oscillator_is_started,
            oscillator_gain,
            buffer_source,
            oscillator,
            gain,
            white_noise,
            noise_buffer_cache: LruCache::new(500),
        };
        for i in 0..4 {
            backend.oscillator_enable(i, true);
        }
        Ok(backend)
    }

    fn oscillator_enable(&mut self, channel: usize, enable: bool) {
        if self.oscillator_is_started[channel] != enable {
            self.oscillator_gain[channel]
                .gain()
                .set_value(if enable { 1.0 } else { 0.0 });
            self.oscillator_is_started[channel] = enable;
        }
    }
    pub fn create_noise_buffer(&mut self, noise_descr: &NoiseDescr) -> Option<&AudioBuffer> {
        if !self.noise_buffer_cache.contains(noise_descr) {
            info!(
                "create new audio buffer, total: {}",
                self.noise_buffer_cache.len()
            );
            let mut position = 0;

            let noise_data = if noise_descr.ctl.contains(AUDC::NOISE_4BIT) {
                &self.poly_4[..]
            } else if noise_descr.audctl.contains(AUDCTL::POLY_9BIT) {
                &self.poly_9[..]
            } else {
                &self.poly_17[..]
            };
            // frequency of fetching bits from poly_data buffer
            // we are going to use this sample rate for playing data from AudioBuffer
            // if rate is lower than minimal we are going to do upsampling by multiplier

            let mut sample_rate =
                CLOCK_177 / noise_descr.clock_divider as f32 / noise_descr.divider as f32;
            let multiplier = if sample_rate < MIN_SAMPLE_RATE {
                Some((MIN_SAMPLE_RATE / sample_rate).ceil() as u32)
            } else if sample_rate > MAX_SAMPLE_RATE {
                None
            } else {
                Some(1)
            };
            if let Some(multiplier) = multiplier {
                // warn!("sample_rate: {:?} {:?}", sample_rate, noise_descr);
                // 1 second is `sample_rate` of samples playing with `sample_rate`, so:
                let n_samples = (sample_rate as f32 * SAMPLE_DUR) as u32;

                sample_rate *= multiplier as f32;
                let mut data = Vec::with_capacity((n_samples * multiplier) as usize);
                for _ in 0..n_samples {
                    let mask = if !noise_descr.ctl.contains(AUDC::NOT_5BIT)
                        && self.poly_5[position % self.poly_5.len()] == 0
                    {
                        0
                    } else {
                        1
                    };
                    let sample = (mask & noise_data[position % noise_data.len()]) as f32 * 2.0 - 1.0;
                    position += (noise_descr.divider * noise_descr.clock_divider) as usize;
                    for _ in 0..multiplier {
                        data.push(sample);
                    }
                }
                let buffer = self
                    .ctx
                    .create_buffer(1, n_samples * multiplier, sample_rate)
                    .unwrap();
                buffer.copy_to_channel(&mut data, 0).unwrap();
                self.noise_buffer_cache.put(noise_descr.clone(), buffer);
            }
        }
        self.noise_buffer_cache.get(noise_descr)
    }

    pub fn create_white_noise_buffer(
        ctx: &web_sys::AudioContext,
    ) -> Result<web_sys::AudioBuffer, JsValue> {
        const N: usize = 44100;
        let buffer = ctx.create_buffer(1, N as u32, N as f32)?;
        let mut source: [f32; N] = [0.0; N];
        for i in 0..N {
            // source[i] = (rand::random::<i32>() & 1) as f32 * 2.0 - 1.0;
            source[i] = rand::random::<f32>() * 2.0 - 1.0;
        }
        buffer.copy_to_channel(&mut source, 0)?;
        Ok(buffer)
    }

    pub fn resume(&mut self) {
        self.ctx.resume().ok();
    }

    /// Sets the gain for this oscillator, between 0.0 and 1.0.
    pub fn set_gain(&self, channel: usize, gain: f32) {
        self.gain[channel].gain().set_value(gain);
    }

    pub fn set_noise_source(
        &mut self,
        channel: usize,
        audctl: AUDCTL,
        ctl: AUDC,
        divider: u32,
        clock_divider: u32,
        freq: f32,
    ) {
        let noise_source = if true {
            // warn!("set_noise_source {} {:?}, {:02x}", channel, audctl, ctl);
            let noise_source = self.ctx.create_buffer_source().unwrap();
            let descr = NoiseDescr {
                divider,
                clock_divider,
                ctl: ctl & (AUDC::NOISE_4BIT | AUDC::NOT_5BIT),
                audctl: audctl & AUDCTL::POLY_9BIT,
            };
            let buffer = self.create_noise_buffer(&descr);
            if buffer.is_none() {
                if let Some(current_source) = &self.buffer_source[channel] {
                    current_source.stop().unwrap();
                }
                self.buffer_source[channel] = None;
                return;
            }
            let buffer = buffer.unwrap();
            noise_source.set_buffer(Some(buffer));
            noise_source.set_loop(true);
            Some(noise_source)
        } else {
            if let Some(current_source) = &self.buffer_source[channel] {
                current_source.stop().unwrap();
            }
            self.buffer_source[channel] = None;
            let noise_source = self.ctx.create_buffer_source().unwrap();
            noise_source.set_buffer(Some(&self.white_noise));
            noise_source.set_loop(true);
            noise_source.playback_rate().set_value(1.0 * freq);
            Some(noise_source)
        };
        if let Some(current_source) = &self.buffer_source[channel] {
            current_source.stop().unwrap();
        }
        if let Some(noise_source) = &noise_source {
            noise_source
                .connect_with_audio_node(&self.gain[channel])
                .unwrap();
            noise_source.start().unwrap();
        }
        self.buffer_source[channel] = noise_source
    }

    pub fn setup_channel(
        &mut self,
        channel: usize,
        audctl: AUDCTL,
        ctl: AUDC,
        divider: u32,
        clock_divider: u32,
        freq: f32,
    ) {
        let is_noise = !ctl.contains(AUDC::NOT_NOISE);
        // warn!(
        //     "setup_channel: channel: {:?}, audctl: {:?}, ctl: {:?} div: {:?}, freq: {:?}, noise: {}",
        //     channel, audctl, ctl, divider, freq, is_noise
        // );
        self.oscillator_enable(channel, !is_noise && freq <= 22050.0);
        if !is_noise {
            if let Some(source) = self.buffer_source[channel].take() {
                source.stop().unwrap();
            }
            self.oscillator[channel].frequency().set_value(freq);
        } else {
            self.set_noise_source(channel, audctl, ctl, divider, clock_divider, freq);
        }
    }
}
