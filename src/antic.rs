use bevy::utils::tracing::{info, warn};

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
    pub pmbase: u8,
    pub hscrol: u8,
}

impl Antic {
    fn playfield_width(&self, hscroll: bool) -> usize {
        if !hscroll {
            match self.dmactl & DMACTL::PLAYFIELD_WIDTH_MASK {
                DMACTL::NARROW_PLAYFIELD => 256,
                DMACTL::NORMAL_PLAYFIELD => 320,
                DMACTL::WIDE_PLAYFIELD => 384,
                _ => 0,
            }
        } else {
            match self.dmactl & DMACTL::PLAYFIELD_WIDTH_MASK {
                DMACTL::NARROW_PLAYFIELD => 320,
                DMACTL::NORMAL_PLAYFIELD => 384,
                DMACTL::WIDE_PLAYFIELD => 384,
                _ => 0,
            }
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
    fn create_mode_line(&self, mods: u8, mode: u8, height: usize, n_bytes: usize) -> ModeLineDescr {
        let dli = (mods & 0x80) > 0;
        let is_hscrol = (mods & 0x10) > 0;
        let hscrol = if is_hscrol {
            32 - self.hscrol * 2
        } else {
            0
        };

        let hscrol_line_width = n_bytes * self.playfield_width(is_hscrol) / 320;

        ModeLineDescr {
            dli,
            mode,
            height,
            n_bytes: hscrol_line_width,
            scan_line: self.scan_line,
            width: self.playfield_width(false),
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
        let op = dlist[0];
        self.inc_dlist(1);
        let mods = op & 0xf0;
        let mode = op & 0x0f;
        if (mods & 0x40 > 0) && mode > 1 {
            self.video_memory = dlist[1] as usize + (dlist[2] as usize * 256);
            self.inc_dlist(2);
        };
        let mode_line = match mode {
            0x0 => self.create_mode_line(mods, mode, ((mods >> 4) & 7) as usize + 1, 0),
            0x1 => {
                self.dlist = dlist[1] as u16 | ((dlist[2] as u16) << 8);
                if mods & 0x40 > 0 {
                    return None;
                }
                self.create_mode_line(mods, mode, 1, 0)
            }
            0x2 => self.create_mode_line(mods, mode, 8, 40),
            0x4 => self.create_mode_line(mods, mode, 8, 40),
            0xa => self.create_mode_line(mods, mode, 4, 20),
            0xc => self.create_mode_line(mods, mode, 1, 20),
            0xd => self.create_mode_line(mods, mode, 2, 40),
            0xe => self.create_mode_line(mods, mode, 1, 40),
            0xf => self.create_mode_line(mods, mode, 1, 40),
            _  => {
                warn!("unsupported antic vide mode {:?}", mode);
                self.create_mode_line(mods, mode, 1, 0)
            }
        };
        self.video_memory += mode_line.n_bytes;
        Some(mode_line)
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
        // bevy::log::warn!(
        //     "ANTIC write: {:02x}: {:02x}, scanline: {}",
        //     addr, value, self.scan_line
        // );
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
            consts::WSYNC => (),  // TODO
            consts::VSCROL => (),  // TODO
            _ => bevy::log::warn!("unsupported antic write reg: {:x?}", addr),
        }
    }
}
