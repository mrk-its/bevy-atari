use bevy::prelude::info;
mod utils;
use utils::{FIRFilter, Filter, Poly17, Poly4, Poly5, Poly9, PolyGenerator, FIR_37_TO_1};

use web_audio_api::context::{
    AudioContext, AudioContextLatencyCategory, AudioContextOptions, AudioContextRegistration,
    AudioParamId, BaseAudioContext,
};
use web_audio_api::node::{AudioNode, ChannelConfig, ChannelConfigOptions};
use web_audio_api::param::{AudioParam, AudioParamDescriptor, AutomationRate};
use web_audio_api::render::{AudioParamValues, AudioProcessor, AudioRenderQuantum};
use web_audio_api::SampleRate;

pub struct Context {
    audio_context: Option<AudioContext>,
    pokey_node: PokeyNode,
}

impl Context {
    const LATENCY: f64 = 0.05;

    pub fn is_running(&self) -> bool {
        true // TODO
    }

    pub fn current_time(&self) -> f64 {
        match self.audio_context.as_ref() {
            Some(ctx) => ctx.current_time(),
            None => 0.0,
        }
    }

    pub fn send_regs(&mut self, regs: &Vec<super::PokeyRegWrite>, delta_t: f64) {
        for r in regs {
            let index = r.index & 0xf;
            if index > 9 {
                continue;
            }
            let t = r.timestamp as f64 / (312.0 * 114.0 * 50.0) - delta_t + Self::LATENCY;
            self.pokey_node.regs[index as usize].set_value_at_time(r.value as f32, t);
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        let opts = AudioContextOptions {
            sample_rate: Some(44100),
            latency_hint: Some(AudioContextLatencyCategory::Balanced),
            channels: Some(2),
        };
        let context = AudioContext::new(Some(opts));
        info!("sample_rate: {}", context.sample_rate());
        let pokey_node = PokeyNode::new(&context);
        pokey_node.connect(&context.destination());
        Self {
            audio_context: Some(context),
            pokey_node,
        }
    }
}

struct PokeyNode {
    /// handle to the audio context, required for all audio nodes
    registration: AudioContextRegistration,
    /// channel configuration (for up/down-mixing of inputs), required for all audio nodes
    channel_config: ChannelConfig,
    pub regs: Vec<AudioParam>,
}

// implement required methods for AudioNode trait
impl AudioNode for PokeyNode {
    fn registration(&self) -> &AudioContextRegistration {
        &self.registration
    }

    fn channel_config_raw(&self) -> &ChannelConfig {
        &self.channel_config
    }

    // source nodes take no input
    fn number_of_inputs(&self) -> u32 {
        0
    }

    // emit a single output
    fn number_of_outputs(&self) -> u32 {
        1
    }
}

impl PokeyNode {
    /// Construct a new PokeyNode
    fn new<C: BaseAudioContext>(context: &C) -> Self {
        context.base().register(move |registration| {
            // setup the amplitude audio param
            let mut params = Vec::with_capacity(9);
            let mut param_ids = Vec::with_capacity(9);

            for _ in 0..9 {
                let param_opts = AudioParamDescriptor {
                    min_value: 0.,
                    max_value: 255.,
                    default_value: 0.,
                    automation_rate: AutomationRate::A,
                };

                let (param, param_id) = context
                    .base()
                    .create_audio_param(param_opts, registration.id());
                params.push(param);
                param_ids.push(param_id);
            }

            // setup the processor, this will run in the render thread
            let render = PokeyProcessor {
                regs: param_ids,
                pokey: Pokey::new(context.sample_rate() as usize),
            };

            // setup the audio node, this will live in the control thread (user facing)
            let node = PokeyNode {
                registration,
                channel_config: ChannelConfigOptions::default().into(),
                regs: params,
            };

            (node, Box::new(render))
        })
    }

    // /// The Amplitude AudioParam
    // fn amplitude(&self) -> &AudioParam {
    //     &self.amplitude
    // }
}

struct PokeyProcessor {
    regs: Vec<AudioParamId>,
    pokey: Pokey,
}

impl AudioProcessor for PokeyProcessor {
    fn process(
        &mut self,
        _inputs: &[AudioRenderQuantum],
        outputs: &mut [AudioRenderQuantum],
        params: AudioParamValues,
        _timestamp: f64,
        _sample_rate: SampleRate,
    ) -> bool {
        // single output node
        let output = &mut outputs[0];
        let buf = output.channel_data_mut(0);

        let regs = (0..9)
            .map(|i| params.get(&self.regs[i]))
            .collect::<Vec<_>>();
        for (n, b) in buf.iter_mut().enumerate() {
            for i in 0..4 {
                self.pokey.set_audf(i, regs[i * 2][n] as u8);
                self.pokey.set_audc(i, regs[i * 2 + 1][n] as u8);
            }
            self.pokey.set_audctl(regs[8][n] as u8);
            *b = self.pokey.get();
        }

        true // source node will always be active
    }
}

struct Pokey {
    divider: usize,
    filter: Box<dyn Filter + Send>,
    clock_cnt: isize,
    cycle_cnt: usize,
    audf: [u8; 4],
    audc: [u8; 4],
    cnt: [i16; 4],
    square_output: [bool; 4],
    output: [bool; 4],
    audctl: u8,
    fast_1: bool,
    fast_3: bool,
    link12: bool,
    link34: bool,
    clock_period: usize,
    hipass1: bool,
    hipass2: bool,
    hipass1_flipflop: bool,
    hipass2_flipflop: bool,
    poly_4: Vec<bool>,
    poly_5: Vec<bool>,
    poly_9: Vec<bool>,
    poly_17: Vec<bool>,
}

impl Pokey {
    fn new(sample_rate: usize) -> Self {
        let (divider, filter) = match sample_rate {
            // 44100 => 40,
            48000 => (37, Box::new(FIRFilter::new(FIR_37_TO_1))),
            // 56000 => 32,
            _ => panic!("sample rate {} is not supported", sample_rate),
        };
        let mut pokey = Self {
            divider,
            filter,
            clock_cnt: Default::default(),
            cycle_cnt: Default::default(),
            audf: Default::default(),
            audc: Default::default(),
            cnt: Default::default(),
            square_output: Default::default(),
            output: Default::default(),
            audctl: 0x00,
            fast_1: Default::default(),
            fast_3: Default::default(),
            link12: Default::default(),
            link34: Default::default(),
            clock_period: Default::default(),
            hipass1: Default::default(),
            hipass2: Default::default(),
            hipass1_flipflop: Default::default(),
            hipass2_flipflop: Default::default(),
            poly_4: Poly4::as_vec(),
            poly_5: Poly5::as_vec(),
            poly_9: Poly9::as_vec(),
            poly_17: Poly17::as_vec(),
        };
        pokey.set_audctl(0);
        pokey
    }
}

impl Pokey {
    fn set_audctl(&mut self, value: u8) {
        self.audctl = value;
        self.fast_1 = (value & 0x40) > 0;
        self.fast_3 = (value & 0x20) > 0;
        self.link12 = (value & 0x10) > 0;
        self.link34 = (value & 0x8) > 0;
        self.clock_period = if value & 1 > 0 { 114 } else { 28 };
        self.hipass1 = (value >> 2) & 1 > 0;
        self.hipass2 = (value >> 1) & 1 > 0;
        self.hipass1_flipflop |= !self.hipass1;
        self.hipass2_flipflop |= !self.hipass2;
    }

    #[inline(always)]
    fn set_audf(&mut self, index: usize, value: u8) {
        self.audf[index] = value;
    }

    #[inline(always)]
    fn set_audc(&mut self, index: usize, value: u8) {
        self.audc[index] = value;
    }

    #[inline(always)]
    fn get_poly_output(&self, k: usize, poly: &[bool]) -> bool {
        return poly[(self.cycle_cnt + k) % poly.len()];
    }

    fn get_output(&self, k: usize) -> bool {
        let audc = self.audc[k];
        if audc & 0x20 > 0 {
            self.square_output[k]
        } else {
            if audc & 0x40 > 0 {
                self.get_poly_output(k, &self.poly_4)
            } else {
                if self.audctl & 0x80 > 0 {
                    self.get_poly_output(k, &self.poly_9)
                } else {
                    self.get_poly_output(k, &self.poly_17)
                }
            }
        }
    }

    fn set_output(&mut self, k: usize) {
        if self.audc[k] & 0x80 > 0 || self.get_poly_output(k, &self.poly_5) {
            self.square_output[k] = !self.square_output[k]
        }
        self.output[k] = self.get_output(k)
    }

    fn reload_single(&mut self, k: usize) {
        let fast_delay = if k == 0 && self.fast_1 || k == 2 && self.fast_3 {
            3
        } else {
            0
        };
        self.cnt[k] = self.audf[k] as i16 + fast_delay;
        self.set_output(k)
    }

    fn reload_linked(&mut self, k: usize) {
        let cnt = self.audf[k] as usize + self.audf[k + 1] as usize * 256 + 6;
        self.cnt[k] = (cnt & 0xff) as i16;
        self.cnt[k + 1] = (cnt >> 8) as i16;
        self.set_output(k + 1);
    }

    fn get(&mut self) -> f32 {
        for _ in 0..self.divider {
            self.clock_cnt -= 1;
            let clock_underflow = self.clock_cnt < 0;
            if clock_underflow {
                self.clock_cnt = self.clock_period as isize - 1;
            }

            if !self.link12 {
                if self.fast_1 || clock_underflow {
                    self.cnt[0] -= 1;
                    if self.cnt[0] < 0 {
                        self.reload_single(0)
                    }
                }
                if clock_underflow {
                    self.cnt[1] -= 1;
                    if self.cnt[1] < 0 {
                        self.reload_single(1)
                    }
                }
            } else {
                if self.fast_1 || clock_underflow {
                    self.cnt[0] -= 1;
                    if self.cnt[0] < 0 {
                        self.cnt[0] = 255;
                        self.set_output(0);
                        self.cnt[1] -= 1;
                        if self.cnt[1] < 0 {
                            self.reload_linked(0);
                        }
                    }
                }
            }
            if !self.link34 {
                if self.fast_3 || clock_underflow {
                    self.cnt[2] -= 1;
                    if self.cnt[2] < 0 {
                        self.reload_single(2);
                        if self.hipass1 {
                            self.hipass1_flipflop = self.output[0]
                        }
                    }
                }
                if clock_underflow {
                    self.cnt[3] -= 1;
                    if self.cnt[3] < 0 {
                        self.reload_single(3);
                        if self.hipass2 {
                            self.hipass2_flipflop = self.output[1]
                        }
                    }
                }
            } else {
                if self.fast_3 || clock_underflow {
                    self.cnt[2] -= 1;
                    if self.cnt[2] < 0 {
                        // what about hipass1 / hipass2 here?
                        self.cnt[2] = 255;
                        self.set_output(2);
                        self.cnt[3] -= 1;
                        if self.cnt[3] < 0 {
                            self.reload_linked(2);
                        }
                    }
                }
            }

            self.cycle_cnt += 1;

            let vol_only = |n| ((self.audc[n] >> 4) & 1u8) > 0u8;
            let vol = |n| self.audc[n] & 15;

            let ch1 = (self.hipass1_flipflop ^ self.output[0]) | vol_only(0);
            let ch2 = (self.hipass2_flipflop ^ self.output[1]) | vol_only(1);
            let ch3 = self.output[2] | vol_only(2);
            let ch4 = self.output[3] | vol_only(3);
            // let normalize = |vol: u8| vol as f32 / 60.0;
            let normalize_altirra =
                |vol| (1.0 - (-2.9 * (vol as f32 / 60.0)).exp()) / (1.0 - (-2.9f32).exp());
            let sample = normalize_altirra(
                ch1 as u8 * vol(0) + ch2 as u8 * vol(1) + ch3 as u8 * vol(2) + ch4 as u8 * vol(3),
            );
            self.filter.add_sample(sample);
        }
        self.filter.get()
        // return self.high_pass_filter.get(self.fir_filter.get());
    }
}
