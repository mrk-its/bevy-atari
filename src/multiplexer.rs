use core::ops::BitOrAssign;
pub struct Multiplexer<T> {
    inputs: Vec<T>,
}

impl<T: Default + BitOrAssign + Copy> Multiplexer<T> {
    pub fn new(size: usize) -> Self {
        let mut inputs = Vec::with_capacity(size);
        inputs.resize_with(size, Default::default);
        Multiplexer { inputs }
    }
    pub fn set_input(&mut self, i: usize, v: T) {
        self.inputs[i] = v;
    }

    pub fn get_output(&self) -> T {
        let mut out: T = Default::default();
        for v in self.inputs.iter() {
            out |= *v;
        }
        out
    }
}
