use super::{AUDC, AUDCTL, CLOCK_177};
pub struct AudioBackend {
}

impl AudioBackend {
    pub fn new() -> Result<AudioBackend, ()> {
        Ok(AudioBackend {})
    }

    pub fn resume(&mut self) {
    }

    /// Sets the gain for this oscillator, between 0.0 and 1.0.
    pub fn set_gain(&self, channel: usize, gain: f32) {
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
    }

    //AUDC = 132
    //AUDF = 35
    //AUDCTL = 0

    pub fn setup_channel(
        &mut self,
        channel: usize,
        audctl: AUDCTL,
        ctl: AUDC,
        divider: u32,
        clock_divider: u32,
        freq: f32,
    ) {
    }
}
