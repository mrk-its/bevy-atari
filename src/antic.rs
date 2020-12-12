use bevy::log::*;
mod consts {
    pub const DMACTL: usize = 0x00; // bit3 - player DMA, bit2 - missile DMA, bit4 - 1-PM hires, 0: PM lores, AHRM page 72
    pub const CHACTL: usize = 0x01;
    pub const DLIST_L: usize = 0x02;
    pub const DLIST_H: usize = 0x03;
    pub const HSCROL: usize = 0x04;
    pub const VSCROL: usize = 0x05;
    pub const PMBASE: usize = 0x07;
    pub const CHBASE: usize = 0x09;
    pub const WSYNC: usize = 0x0A;
    pub const VCOUNT: usize = 0x0B;
    pub const NMIEN: usize = 0x0E;
    pub const NMIST: usize = 0x0f;
    pub const NMIRES: usize = 0x0f;
}

bitflags! {
    #[derive(Default)]
    pub struct DMACTL: u8 {
        const EMPTY = 0;
        const NARROW_PLAYFIELD = 1;
        const NORMAL_PLAYFIELD = 2;
        const WIDE_PLAYFIELD = 3;
        const PLAYFIELD_WIDTH_MASK = 3;
        const MISSILE_DMA = 4;
        const PLAYER_DMA = 8;
        const PM_HIRES = 16;
        const DLIST_DMA = 32;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct NMIST: u8 {
        const DLI = 128;
        const VBI = 64;
        const SYSTEM_RESET = 32;  // 400/800 only
        const UNUSED = 0x1f;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct NMIEN: u8 {
        const DLI = 128;
        const VBI = 64;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct MODE_OPTS: u8 {
        const DLI = 0x80;
        const LMS = 0x40;
        const VSCROL = 0x20;
        const HSCROL = 0x10;
    }
}

#[derive(Default)]
pub struct Antic {
    pub dmactl: DMACTL,
    pub nmist: NMIST,
    pub nmien: NMIEN,
    pub chactl: u8,
    pub chbase: u8,
    pub hscrol: u8,
    pub pmbase: u8,
    pub dlist: u16,
    pub scan_line: usize,
    pub video_memory: usize,
    pub wsync: bool,
    enable_log: bool,
}

#[derive(Debug)]
pub struct ModeLineDescr {
    pub opts: MODE_OPTS,
    pub mode: u8,
    pub scan_line: usize,
    pub width: usize,
    pub height: usize,
    pub n_bytes: usize,
    pub data_offset: usize,
    pub chbase: u8,
    pub pmbase: u8,
    pub hscrol: u8,
}

const MODE_25_STEALED_CYCLES_FIRST_LINE: [&[usize; 8]; 4] = [
    &[0, 0, 0, 0, 0, 0, 0, 0],
    &[66, 66, 66, 66, 66, 66, 66, 66],
    &[81, 81, 81, 81, 81, 81, 82, 81],
    &[96, 95, 94, 93, 92, 91, 90, 89],
];
const MODE_25_STEALED_CYCLES: [&[usize; 8]; 4] = [
    &[0, 0, 0, 0, 0, 0, 0, 0],
    &[41, 41, 41, 41, 41, 41, 41, 41],
    &[49, 49, 49, 49, 49, 49, 49, 48],
    &[56, 55, 55, 54, 54, 53, 53, 53],
];

const MODE_67_STEALED_CYCLES_FIRST_LINE: [&[usize; 8]; 4] = [
    &[0, 0, 0, 0, 0, 0, 0, 0],
    &[41, 41, 41, 41, 41, 41, 41, 41],
    &[49, 49, 49, 49, 49, 49, 49, 48],
    &[57, 56, 56, 56, 55, 54, 54, 54],
];
const MODE_67_STEALED_CYCLES: [&[usize; 8]; 4] = [
    &[0, 0, 0, 0, 0, 0, 0, 0],
    &[25, 25, 25, 25, 25, 25, 25, 25],
    &[29, 29, 29, 29, 29, 29, 29, 29],
    &[33, 32, 32, 32, 32, 31, 31, 31],
];


const MODE_89_STEALED_CYCLES: [&[usize; 8]; 4] = [
    &[0, 0, 0, 0, 0, 0, 0, 0],
    &[17, 17, 17, 17, 17, 17, 17, 17],
    &[19, 19, 19, 19, 19, 19, 19, 19],
    &[21, 21, 21, 21, 21, 21, 20, 20],
];

const MODE_AC_STEALED_CYCLES: [&[usize; 8]; 4] = [
    &[0, 0, 0, 0, 0, 0, 0, 0],
    &[25, 25, 25, 25, 25, 25, 25, 25],
    &[29, 29, 29, 29, 29, 29, 29, 29],
    &[33, 33, 32, 32, 32, 32, 31, 31],
];

const MODE_DF_STEALED_CYCLES: [&[usize; 8]; 4] = [
    &[0, 0, 0, 0, 0, 0, 0, 0],
    &[41, 41, 41, 41, 41, 41, 41, 41],
    &[49, 49, 49, 49, 49, 49, 49, 49],
    &[56, 56, 55, 55, 54, 54, 53, 53],
];

impl Antic {
    fn playfield_width_index(&self, hscroll: bool) -> usize {
        match (hscroll, self.dmactl & DMACTL::PLAYFIELD_WIDTH_MASK) {
            (false, DMACTL::EMPTY) => 0,
            (false, DMACTL::NARROW_PLAYFIELD) => 1,
            (false, DMACTL::NORMAL_PLAYFIELD) => 2,
            (false, DMACTL::WIDE_PLAYFIELD) => 3,
            (true, DMACTL::EMPTY) => 0,
            (true, DMACTL::NARROW_PLAYFIELD) => 2,
            (true, DMACTL::NORMAL_PLAYFIELD) => 3,
            (true, DMACTL::WIDE_PLAYFIELD) => 3,
            _ => panic!("imposssible!"),
        }
    }

    fn playfield_width(&self, fetch_width: bool, hscroll: bool) -> usize {
        match (
            hscroll,
            fetch_width,
            self.dmactl & DMACTL::PLAYFIELD_WIDTH_MASK,
        ) {
            (false, _, DMACTL::NARROW_PLAYFIELD) => 256,
            (false, _, DMACTL::NORMAL_PLAYFIELD) => 320,
            (false, _, DMACTL::WIDE_PLAYFIELD) => 384,

            (true, false, DMACTL::NARROW_PLAYFIELD) => 256,
            (true, false, DMACTL::NORMAL_PLAYFIELD) => 320,
            (true, false, DMACTL::WIDE_PLAYFIELD) => 320,

            (true, true, DMACTL::NARROW_PLAYFIELD) => 320,
            (true, true, DMACTL::NORMAL_PLAYFIELD) => 384,
            (true, true, DMACTL::WIDE_PLAYFIELD) => 384,
            _ => 0,
        }
    }
    pub fn set_vbi(&mut self) {
        self.nmist.insert(NMIST::VBI);
        self.nmist.remove(NMIST::DLI);
    }
    pub fn set_dli(&mut self) {
        self.nmist.insert(NMIST::DLI);
        self.nmist.remove(NMIST::VBI);
    }

    pub fn get_dma_cycles(&self, current_line: &ModeLineDescr) -> usize {
        let is_hscrol = current_line.opts.contains(MODE_OPTS::HSCROL);
        let is_first_mode_line = current_line.scan_line == self.scan_line;
        let hscrol = if is_hscrol {
            self.hscrol as usize / 2
        } else {
            0
        };
        let playfield_width_index = self.playfield_width_index(is_hscrol);
        let mode = current_line.mode;
        let mut n_cycles = match mode {
            0x2..=0x5 => {
                if is_first_mode_line {
                    MODE_25_STEALED_CYCLES_FIRST_LINE[playfield_width_index][hscrol]
                } else {
                    MODE_25_STEALED_CYCLES[playfield_width_index][hscrol]
                }
            }
            0x6..=0x7 => {
                if is_first_mode_line {
                    MODE_67_STEALED_CYCLES_FIRST_LINE[playfield_width_index][hscrol]
                } else {
                    MODE_67_STEALED_CYCLES[playfield_width_index][hscrol]
                }
            },
            0x8..=0x9 => MODE_89_STEALED_CYCLES[playfield_width_index][hscrol],
            0xa..=0xc => MODE_AC_STEALED_CYCLES[playfield_width_index][hscrol],
            0xd..=0xf => MODE_DF_STEALED_CYCLES[playfield_width_index][hscrol],

            _ => 0,
        };
        if self.dmactl.contains(DMACTL::PLAYER_DMA) {
            n_cycles += 5;
        }
        if is_first_mode_line {
            if mode == 1 {
                n_cycles += 3; // DL with ADDR
            } else {
                n_cycles += 1;
            }
        }
        n_cycles
    }

    fn create_mode_line(
        &self,
        opts: MODE_OPTS,
        mode: u8,
        height: usize,
        n_bytes: usize,
    ) -> ModeLineDescr {
        let is_hscrol = opts.contains(MODE_OPTS::HSCROL);
        let hscrol = if is_hscrol { 32 - self.hscrol * 2 } else { 0 };

        let hscrol_line_width = n_bytes * self.playfield_width(true, is_hscrol) / 320;

        ModeLineDescr {
            mode,
            opts,
            height,
            n_bytes: hscrol_line_width,
            scan_line: self.scan_line,
            width: self.playfield_width(false, is_hscrol),
            data_offset: self.video_memory,
            chbase: self.chbase,
            pmbase: self.pmbase,
            hscrol,
        }
    }
    pub fn inc_dlist(&mut self, k: u8) {
        self.dlist = self.dlist.overflowing_add(k as u16).0;
    }

    pub fn create_next_mode_line(&mut self, dlist: &[u8]) -> Option<ModeLineDescr> {
        let opts = MODE_OPTS::from_bits_truncate(dlist[0]);
        let mode = dlist[0] & 0xf;
        self.inc_dlist(1);
        if opts.contains(MODE_OPTS::LMS) && mode > 1 {
            self.video_memory = dlist[1] as usize + (dlist[2] as usize * 256);
            self.inc_dlist(2);
        };
        let mode_line = match mode {
            0x0 => self.create_mode_line(opts, mode, ((dlist[0] >> 4) & 7) as usize + 1, 0),
            0x1 => {
                self.dlist = dlist[1] as u16 | ((dlist[2] as u16) << 8);
                if opts.contains(MODE_OPTS::LMS) {
                    return None;
                }
                self.create_mode_line(opts, mode, 1, 0)
            }
            0x2 => self.create_mode_line(opts, mode, 8, 40),
            0x4 => self.create_mode_line(opts, mode, 8, 40),
            0xa => self.create_mode_line(opts, mode, 4, 20),
            0xc => self.create_mode_line(opts, mode, 1, 20),
            0xd => self.create_mode_line(opts, mode, 2, 40),
            0xe => self.create_mode_line(opts, mode, 1, 40),
            0xf => self.create_mode_line(opts, mode, 1, 40),
            _ => {
                warn!("unsupported antic vide mode {:?}", mode);
                self.create_mode_line(opts, mode, 1, 0)
            }
        };
        self.video_memory += mode_line.n_bytes;
        Some(mode_line)
    }
    pub fn wsync(&mut self) -> bool {
        if self.wsync {
            self.wsync = false;
            true
        } else {
            false
        }
    }

    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0xf;
        let value = match addr {
            consts::NMIST => self.nmist.bits | 0x1f,
            consts::VCOUNT => (self.scan_line >> 1) as u8,
            _ => 0x00,
        };
        // bevy::log::warn!("ANTIC read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0xf;
        if self.enable_log {
            warn!(
                "ANTIC write: {:02x}: {:02x}, scanline: {}",
                addr, value, self.scan_line
            );
        }
        match addr {
            consts::DMACTL => self.dmactl = DMACTL::from_bits_truncate(value),
            consts::CHACTL => self.chactl = value,
            consts::PMBASE => self.pmbase = value,
            consts::CHBASE => self.chbase = value,
            consts::NMIEN => self.nmien = NMIEN::from_bits_truncate(value),
            consts::NMIRES => self.nmist.bits = NMIST::UNUSED.bits,
            consts::HSCROL => self.hscrol = value,
            consts::DLIST_L => self.dlist = self.dlist & 0xff00 | value as u16,
            consts::DLIST_H => self.dlist = self.dlist & 0xff | ((value as u16) << 8),
            consts::WSYNC => self.wsync = true, // TODO
            consts::VSCROL => (),               // TODO
            _ => bevy::log::warn!("unsupported antic write reg: {:x?}", addr),
        }
    }
    pub fn enable_log(&mut self, enable: bool) {
        self.enable_log = enable;
    }
}
