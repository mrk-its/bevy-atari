pub const DMACTL: usize = 0x00;
pub const CHACTL: usize = 0x01;
pub const DLIST: usize = 0x02;
pub const HSCROL: usize = 0x04;
pub const VSCROL: usize = 0x05;
pub const PMBASE: usize = 0x07;
pub const CHBASE: usize = 0x09;
pub const WSYNC: usize = 0x0A;
pub const VCOUNT: usize = 0x0B;
pub const NMIEN: usize = 0x0E;
pub const NMIST: usize = 0x0f;
pub const NMIRES: usize = 0x0f;

#[derive(Default)]
pub struct Antic {
    pub regs: [u8; 0x10],
    pub scan_line: usize,
    pub chactl: u8,
    pub video_memory: usize,
}

#[derive(Debug)]
pub struct ModeLineDescr {
    pub dli: bool,
    pub mode: u8,
    pub scan_line: usize,
    pub width: usize,
    pub height: usize,
    pub n_bytes: usize,
    pub data_offset: usize,
    pub chbase: u8,
}

impl Antic {
    fn playfield_width(&self) -> usize {
        match self.regs[DMACTL] & 3 {
            1 => 256,
            2 => 320,
            3 => 384,
            _ => 0,
        }
    }
    pub fn set_vbi(&mut self) {
        self.regs[NMIST] &= 0xff - 0x80;  // clear DLI status
        self.regs[NMIST] |= 0x40;
    }
    pub fn set_dli(&mut self) {
        self.regs[NMIST] &= 0xff - 0x40;  // clear VBI status
        self.regs[NMIST] |= 0x80;
    }
    fn create_mode_line(
        &self,
        dli: bool,
        mode: u8,
        height: usize,
        n_bytes: usize,
    ) -> ModeLineDescr {
        ModeLineDescr {
            dli,
            mode,
            height,
            n_bytes: n_bytes * self.playfield_width() / 320,
            scan_line: self.scan_line,
            width: self.playfield_width(),
            data_offset: self.video_memory,
            chbase: self.regs[CHBASE],
        }
    }
    pub fn dlist(&self) -> usize {
        (self.regs[DLIST] as usize) | ((self.regs[DLIST + 1] as usize) << 8)
    }
    pub fn inc_dlist(&mut self, k: u8) {
        let (v, c) = self.regs[DLIST].overflowing_add(k);
        self.regs[DLIST] = v;
        if c {
            self.regs[DLIST + 1] = self.regs[DLIST + 1].overflowing_add(1).0;
        }
    }

    pub fn create_next_mode_line(&mut self, dlist: &[u8]) -> Option<ModeLineDescr> {
        let op = dlist[0];
        self.inc_dlist(1);
        let mods = op & 0xf0;
        let mode = op & 0x0f;
        let dli = (op & 0x80) > 0;
        if (mods & 0x40 > 0) && mode > 1 {
            self.video_memory = dlist[1] as usize + (dlist[2] as usize * 256);
            self.inc_dlist(2);
        };
        let mode_line = match mode {
            0x0 => self.create_mode_line(dli, mode, ((mods >> 4) & 7) as usize + 1, 0),
            0x1 => {
                let addr = self.dlist();
                self.regs[DLIST] = (addr & 0xff) as u8;
                self.regs[DLIST + 1] = (addr >> 8) as u8;
                self.inc_dlist(2);
                if mods & 0x40 > 0 {
                    return None;
                }
                self.create_mode_line(dli, mode, 1, 0)
            }
            0x2 => self.create_mode_line(dli, mode, 8, 40),
            0x4 => self.create_mode_line(dli, mode, 8, 40),
            0xa => self.create_mode_line(dli, mode, 4, 20),
            0xc => self.create_mode_line(dli, mode, 1, 20),
            _ => panic!("unsupported antic video mode {:x}", mode),
        };
        self.video_memory += mode_line.n_bytes;
        Some(mode_line)
    }

    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0xf;
        let value = match addr {
            NMIST => self.regs[addr] | 0x1f,
            0x0b => (self.scan_line >> 1) as u8,
            _ => self.regs[addr],
        };
        // warn!("ANTIC read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0xf;
        // bevy::log::warn!(
        //     "ANTIC write: {:02x}: {:02x}, scanline: {}",
        //     addr, value, self.scan_line
        // );
        match addr {
            NMIRES => self.regs[NMIST] = 0x1f,
            _ => self.regs[addr] = value,
        }
    }
}
