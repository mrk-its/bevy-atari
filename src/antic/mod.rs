use crate::render::COLLISIONS_BUFFER;
use crate::render::{self, CollisionsBufferNode};
use crate::render_resources::{AnticData, AtariPalette, CustomTexture};
use crate::system::AtariSystem;
use crate::{gtia, render::AnticRendererGraphBuilder};
use bevy::render::{render_graph::base::node::MAIN_PASS, renderer::RenderResourceContext};
use bevy::{prelude::*, render::render_graph::RenderGraph};
use bevy::{reflect::TypeUuid, render::render_graph::AssetRenderResourcesNode};
use bevy::{render::pipeline::PipelineDescriptor, sprite::collide_aabb};
use emulator_6502::{Interface6502, MOS6502};

pub const ATARI_PALETTE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(AtariPalette::TYPE_UUID, 5197421896076365082);

pub const ANTIC_DATA_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(AnticData::TYPE_UUID, 11338886280454987747);

pub const COLLISIONS_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6758940903835595297);

pub const DEBUG_COLLISIONS_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 12701505191960931865);

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

#[allow(dead_code)]
const PAL_SCAN_LINES: usize = 312;
#[allow(dead_code)]
const NTSC_SCAN_LINES: usize = 262;

pub const MAX_SCAN_LINES: usize = PAL_SCAN_LINES;
pub const SCAN_LINE_CYCLES: usize = 114;
bitflags! {
    #[derive(Default)]
    pub struct DMACTL: u8 {
        const EMPTY = 0x00;
        const NARROW_PLAYFIELD = 0x01;
        const NORMAL_PLAYFIELD = 0x02;
        const WIDE_PLAYFIELD = 0x03;
        const PLAYFIELD_WIDTH_MASK = 0x03;
        const MISSILE_DMA = 0x04;
        const PLAYER_DMA = 0x08;
        const PM_HIRES = 0x10;
        const DLIST_DMA = 0x20;
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
    #[allow(non_camel_case_types)]
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
    ir: u8,
    pub line_height: usize,
    pub n_bytes: usize,
    pub line_voffset: usize,
    pub start_scan_line: usize,
    pub next_scan_line: usize,
    pub dmactl: DMACTL,
    pub nmist: NMIST,
    pub nmien: NMIEN,
    pub chactl: u8,
    pub chbase: u8,
    pub hscrol: u8,
    pub vscrol: u8,
    pub pmbase: u8,
    pub dlist: u16,
    nmireq: bool,
    pub cycle: usize,
    pub total_cycles: usize,
    visible_cycle: usize,
    dma_cycles: usize,
    pub scan_line: usize,
    pub vcount: u8,
    pub video_memory: usize,
    wsync: bool,
    is_visible: bool,
    is_vscroll: bool,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct AnticModeDescr {
    pub height: usize,
    pub n_bytes: usize,
}

const ANTIC_MODES: [AnticModeDescr; 16] = [
    AnticModeDescr {
        height: 1,
        n_bytes: 0,
    },
    AnticModeDescr {
        height: 1,
        n_bytes: 0,
    },
    AnticModeDescr {
        height: 8,
        n_bytes: 40,
    },
    AnticModeDescr {
        height: 10,
        n_bytes: 40,
    },
    AnticModeDescr {
        height: 8,
        n_bytes: 40,
    },
    AnticModeDescr {
        height: 16,
        n_bytes: 40,
    },
    AnticModeDescr {
        height: 8,
        n_bytes: 20,
    },
    AnticModeDescr {
        height: 16,
        n_bytes: 20,
    },
    AnticModeDescr {
        height: 8,
        n_bytes: 10,
    },
    AnticModeDescr {
        height: 4,
        n_bytes: 10,
    },
    AnticModeDescr {
        height: 4,
        n_bytes: 20,
    },
    AnticModeDescr {
        height: 2,
        n_bytes: 20,
    },
    AnticModeDescr {
        height: 1,
        n_bytes: 20,
    },
    AnticModeDescr {
        height: 2,
        n_bytes: 40,
    },
    AnticModeDescr {
        height: 1,
        n_bytes: 40,
    },
    AnticModeDescr {
        height: 1,
        n_bytes: 40,
    },
];

#[derive(Debug)]
pub struct ModeLineDescr {
    pub opts: MODE_OPTS,
    pub mode: u8,
    pub scan_line: usize,
    pub width: usize,
    pub height: usize,
    pub n_bytes: usize,
    pub line_voffset: usize,
    pub data_offset: usize,
    pub chbase: u8,
    pub pmbase: u8,
    pub hscrol: u8,
    pub video_memory_offset: usize,
    pub charset_memory_offset: usize,
}

impl ModeLineDescr {
    pub fn next_mode_line(&self) -> usize {
        return self.scan_line + self.height;
    }
    pub fn charset_size(&self) -> usize {
        match self.mode {
            2..=5 => 1024,
            6..=7 => 512,
            _ => 0,
        }
    }
}

const MODE_25_STEALED_CYCLES_FIRST_LINE: [(usize, &[usize; 8]); 4] = [
    (29, &[9, 9, 9, 9, 9, 9, 9, 9]),
    (25, &[66, 66, 66, 66, 66, 66, 66, 66]),
    (18 - 2, &[81, 81, 81, 81, 81, 81, 82, 81]), // TODO investigate this -2 correction required for last squadron
    (10, &[96, 95, 94, 93, 92, 91, 90, 89]),
];

const MODE_25_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (29, &[9, 9, 9, 9, 9, 9, 9, 9]),
    (29, &[41, 41, 41, 41, 41, 41, 41, 41]),
    (21, &[49, 49, 49, 49, 49, 49, 49, 48]),
    (13, &[56, 55, 55, 54, 54, 53, 53, 53]),
];

const MODE_67_STEALED_CYCLES_FIRST_LINE: [(usize, &[usize; 8]); 4] = [
    (29, &[9, 9, 9, 9, 9, 9, 9, 9]),
    (25, &[41, 41, 41, 41, 41, 41, 41, 41]),
    (18, &[49, 49, 49, 49, 49, 49, 49, 48]),
    (10, &[57, 56, 56, 56, 55, 54, 54, 54]),
];

const MODE_67_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (29, &[9, 9, 9, 9, 9, 9, 9, 9]),
    (29, &[25, 25, 25, 25, 25, 25, 25, 25]),
    (21, &[29, 29, 29, 29, 29, 29, 29, 29]),
    (13, &[33, 32, 32, 32, 32, 31, 31, 31]),
];

const MODE_89_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (29, &[9, 9, 9, 9, 9, 9, 9, 9]),
    (29, &[17, 17, 17, 17, 17, 17, 17, 17]),
    (21, &[19, 19, 19, 19, 19, 19, 19, 19]),
    (13, &[21, 21, 21, 21, 21, 21, 20, 20]),
];

const MODE_AC_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (29, &[9, 9, 9, 9, 9, 9, 9, 9]),
    (29, &[25, 25, 25, 25, 25, 25, 25, 25]),
    (21, &[29, 29, 29, 29, 29, 29, 29, 29]),
    (13, &[33, 33, 32, 32, 32, 32, 31, 31]),
];

const MODE_DF_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (29, &[9, 9, 9, 9, 9, 9, 9, 9]),
    (29, &[41, 41, 41, 41, 41, 41, 41, 41]),
    (21, &[49, 49, 49, 49, 49, 49, 49, 49]),
    (13, &[56, 56, 55, 55, 54, 54, 53, 53]),
];

impl Antic {
    #[inline]
    pub fn mode(&self) -> u8 {
        self.ir & 0xf
    }

    #[inline]
    pub fn opts(&self) -> MODE_OPTS {
        MODE_OPTS::from_bits_truncate(self.ir)
    }

    #[inline(always)]
    pub fn ir(&self) -> u8 {
        self.ir
    }

    #[inline(always)]
    pub fn inc_cycle(&mut self) {
        self.total_cycles += 1;
        self.cycle = (self.cycle + 1) % SCAN_LINE_CYCLES;
        if self.cycle == 0 {
            self.scan_line = (self.scan_line + 1) % MAX_SCAN_LINES;
            self.vcount = (self.scan_line / 2) as u8;
        } else if self.cycle >= 110 {
            self.vcount = (((self.scan_line + 1) % MAX_SCAN_LINES) / 2) as u8;
        }
    }

    pub fn get_next_scanline(&self) -> usize {
        return (self.scan_line + 1) % MAX_SCAN_LINES;
    }

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
    #[inline(always)]
    pub fn set_vbi(&mut self) {
        self.nmist.insert(NMIST::VBI);
        self.nmist.remove(NMIST::DLI);
    }

    #[inline(always)]
    pub fn set_dli(&mut self) {
        self.nmist.insert(NMIST::DLI);
        self.nmist.remove(NMIST::VBI);
    }

    #[inline(always)]
    pub fn is_vbi(&mut self) -> bool {
        self.scan_line == 248
    }

    #[inline(always)]
    pub fn is_dli(&mut self) -> bool {
        let opts = self.opts();
        if opts.contains(MODE_OPTS::DLI) && self.scan_line >= 8 && self.scan_line < 248 {
            if self.scan_line == self.start_scan_line + self.line_height - 1 {
                return true;
                // self.set_dli();
                // return self.nmien.contains(NMIEN::DLI);
            }
        }
        false
    }

    #[inline(always)]
    pub fn gets_visible(&mut self) -> bool {
        let ret = self.cycle >= self.visible_cycle && !self.is_visible;
        self.is_visible |= ret;
        ret
    }

    #[inline(always)]
    pub fn check_nmi(&mut self) {
        self.nmireq |= self.is_vbi() || self.is_dli()
    }

    #[inline(always)]
    pub fn fire_nmi(&mut self) -> bool {
        if self.nmireq && self.cycle >= 5 {
            self.nmireq = false;
            if self.is_vbi() {
                self.set_vbi();
                self.nmien.contains(NMIEN::VBI)
            } else {
                self.set_dli();
                self.nmien.contains(NMIEN::DLI)
            }
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn steal_cycles(&mut self) {
        if self.cycle == self.visible_cycle {
            self.cycle += self.dma_cycles;
            self.total_cycles += self.dma_cycles;
        }
    }

    #[inline(always)]
    pub fn update_dma_cycles(&mut self) {
        self.is_visible = false;
        if self.scan_line < 8 || self.scan_line >= 248 {
            self.cycle = 0;
            self.dma_cycles = 0;
            self.visible_cycle = 0;
            return;
        }
        // TODO - take hscroll into account for steal start value
        let is_first_mode_line = self.scan_line == self.start_scan_line;
        let mode = self.mode();

        let (line_start_cycle, dma_cycles) =
            if (self.dmactl & DMACTL::PLAYFIELD_WIDTH_MASK).bits() > 0 {
                let opts: MODE_OPTS = self.opts();

                let is_hscrol = mode > 1 && opts.contains(MODE_OPTS::HSCROL);
                let hscrol = if is_hscrol {
                    self.hscrol as usize / 2
                } else {
                    0
                };
                let playfield_width_index = self.playfield_width_index(is_hscrol);
                let (line_start_cycle, dma_cycles_arr) = match mode {
                    0x2..=0x5 => {
                        if is_first_mode_line {
                            MODE_25_STEALED_CYCLES_FIRST_LINE[playfield_width_index]
                        } else {
                            MODE_25_STEALED_CYCLES[playfield_width_index]
                        }
                    }
                    0x6..=0x7 => {
                        if is_first_mode_line {
                            MODE_67_STEALED_CYCLES_FIRST_LINE[playfield_width_index]
                        } else {
                            MODE_67_STEALED_CYCLES[playfield_width_index]
                        }
                    }
                    0x8..=0x9 => MODE_89_STEALED_CYCLES[playfield_width_index],
                    0xa..=0xc => MODE_AC_STEALED_CYCLES[playfield_width_index],
                    0xd..=0xf => MODE_DF_STEALED_CYCLES[playfield_width_index],

                    _ => (29, &[9, 9, 9, 9, 9, 9, 9, 9]),
                };
                (line_start_cycle, dma_cycles_arr[hscrol])
            } else {
                (25, 9)
            };

        let mut start_dma_cycles = 0;
        if self.dmactl.contains(DMACTL::PLAYER_DMA) {
            start_dma_cycles += 5;
        }
        if is_first_mode_line && self.dmactl.contains(DMACTL::DLIST_DMA) {
            if mode == 1 {
                start_dma_cycles += 3; // DL with ADDR
            } else {
                start_dma_cycles += 1;
            }
        }
        self.cycle = start_dma_cycles;
        self.visible_cycle = line_start_cycle.max(start_dma_cycles);
        self.dma_cycles = dma_cycles;
    }

    fn create_mode_line(&self, mode: u8, opts: MODE_OPTS) -> ModeLineDescr {
        assert!(self.scan_line >= 8 && self.scan_line < 248);
        let is_hscrol = mode > 1 && opts.contains(MODE_OPTS::HSCROL);
        let hscrol = if is_hscrol { 32 - self.hscrol * 2 } else { 0 };

        let hscrol_line_width = self.n_bytes * self.playfield_width(true, is_hscrol) / 320;
        let width = self.playfield_width(false, is_hscrol);

        let height = if self.scan_line + self.line_height > 248 {
            // clip height if necessary
            248 - self.scan_line
        } else {
            self.line_height
        };
        ModeLineDescr {
            mode,
            opts,
            height,
            line_voffset: self.line_voffset,
            n_bytes: hscrol_line_width,
            scan_line: self.scan_line,
            width,
            data_offset: self.video_memory,
            chbase: self.chbase,
            pmbase: self.pmbase,
            hscrol,
            video_memory_offset: 0,
            charset_memory_offset: 0,
        }
    }

    #[inline(always)]
    pub fn dlist_offset(&self, k: u8) -> u16 {
        return self.dlist & 0xfc00 | self.dlist.overflowing_add(k as u16).0 & 0x3ff;
    }

    #[inline(always)]
    pub fn inc_dlist(&mut self, k: u8) {
        self.dlist = self.dlist_offset(k);
    }

    #[inline(always)]
    pub fn is_new_mode_line(&self) -> bool {
        assert!(self.next_scan_line < 248);
        self.scan_line == 8 || self.scan_line == self.next_scan_line
    }

    #[inline(always)]
    pub fn dlist_dma(&self) -> bool {
        self.dmactl.contains(DMACTL::DLIST_DMA)
            && (self.scan_line == 8 || !(self.mode() == 1 && self.opts().contains(MODE_OPTS::LMS)))
    }

    pub fn set_dlist_data(&mut self, dlist_data: [u8; 3]) {
        self.ir = dlist_data[0];
        let mode = self.mode();
        let opts = self.opts();
        self.inc_dlist(1);

        if opts.contains(MODE_OPTS::LMS) && mode > 1 {
            self.video_memory = dlist_data[1] as usize + (dlist_data[2] as usize * 256);
            // info!("LMS: {:04x}", self.video_memory);
            self.inc_dlist(2);
        } else if mode == 1 {
            self.dlist = dlist_data[1] as u16 | ((dlist_data[2] as u16) << 8);
        }
    }
    pub fn prepare_mode_line(&mut self) {
        let mode = self.mode();
        let opts = self.opts();

        self.line_voffset = 0;

        let current_mode = &ANTIC_MODES[mode as usize];
        self.line_height = current_mode.height;
        self.n_bytes = current_mode.n_bytes;

        if mode == 0 {
            self.line_height = ((self.ir >> 4) & 7) as usize + 1;
        } else if mode == 1 && opts.contains(MODE_OPTS::LMS) {
            self.line_height = 8;
        }

        let is_vscroll = mode > 1 && opts.contains(MODE_OPTS::VSCROL);
        if is_vscroll && !self.is_vscroll {
            self.line_voffset = self.vscrol as usize;
            self.line_height -= self.line_voffset;
        // entering vscroll region
        } else if !is_vscroll && self.is_vscroll {
            self.line_height = self.vscrol as usize + 1;
            // leaving scroll region
        }
        self.is_vscroll = is_vscroll;
        self.start_scan_line = self.scan_line;

        self.next_scan_line = self.scan_line + self.line_height;
        if self.next_scan_line >= 248 {
            self.next_scan_line = 8;
        }
        // info!(
        //     "mode: {:?} opts: {:?} {:?} scan_line: {} next: {}",
        //     mode, opts, current_mode, self.start_scan_line, self.next_scan_line
        // );
    }

    pub fn create_next_mode_line(&mut self) -> ModeLineDescr {
        assert!(self.scan_line >= 8 && self.scan_line < 248);
        let mode = self.mode();
        let opts = self.opts();
        if mode == 1 && opts.contains(MODE_OPTS::LMS) {
            self.create_mode_line(0, MODE_OPTS::empty())
        } else {
            let mode_line = self.create_mode_line(mode, opts);
            self.video_memory += mode_line.n_bytes;
            mode_line
        }
    }

    #[inline(always)]
    pub fn wsync(&mut self) -> bool {
        self.wsync
    }

    #[inline(always)]
    pub fn do_wsync(&mut self) {
        if self.cycle < 104 {
            self.cycle = 104;
            self.clear_wsync();
        } else {
            self.cycle = SCAN_LINE_CYCLES - 1;
        }
    }

    #[inline(always)]
    pub fn clear_wsync(&mut self) {
        self.wsync = false
    }

    pub fn read(&self, addr: usize) -> u8 {
        let addr = addr & 0xf;
        let value = match addr {
            consts::NMIST => self.nmist.bits | 0x1f,
            consts::VCOUNT => self.vcount,
            _ => 0xff,
        };
        // bevy::log::warn!("ANTIC read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0xf;
        match addr {
            consts::DMACTL => self.dmactl = DMACTL::from_bits_truncate(value),
            consts::CHACTL => self.chactl = value,
            consts::PMBASE => self.pmbase = value,
            consts::CHBASE => self.chbase = value,
            consts::NMIEN => self.nmien = NMIEN::from_bits_truncate(value),
            consts::NMIRES => self.nmist.bits = NMIST::UNUSED.bits,
            consts::HSCROL => self.hscrol = value & 0xf,
            consts::VSCROL => self.vscrol = value & 0xf,
            consts::DLIST_L => self.dlist = self.dlist & 0xff00 | value as u16,
            consts::DLIST_H => self.dlist = self.dlist & 0xff | ((value as u16) << 8),
            consts::WSYNC => self.wsync = true, // TODO
            _ => (),
        }
    }
}

pub fn tick(
    atari_system: &mut AtariSystem,
    cpu: &mut MOS6502,
    antic_data: &mut AnticData,
    // data_texture: &mut Texture,
) {
    if atari_system.antic.cycle == 0 {
        if atari_system.antic.scan_line == 0 {
            // antic reset
            atari_system.antic.next_scan_line = 8;
        }
        atari_system.scanline_tick(atari_system.antic.scan_line);

        if atari_system.antic.dmactl.contains(DMACTL::PLAYER_DMA) {
            if atari_system.gtia.gractl.contains(gtia::GRACTL::MISSILE_DMA) {
                let b = get_pm_data(atari_system, 0);
                atari_system.gtia.write(gtia::GRAFM, b);
            }
            if atari_system.gtia.gractl.contains(gtia::GRACTL::PLAYER_DMA) {
                let b = get_pm_data(atari_system, 1);
                atari_system.gtia.write(gtia::GRAFP0, b);
                let b = get_pm_data(atari_system, 2);
                atari_system.gtia.write(gtia::GRAFP1, b);
                let b = get_pm_data(atari_system, 3);
                atari_system.gtia.write(gtia::GRAFP2, b);
                let b = get_pm_data(atari_system, 4);
                atari_system.gtia.write(gtia::GRAFP3, b);
            }
        }

        if atari_system.antic.is_new_mode_line() {
            if atari_system.antic.dlist_dma() {
                let mut dlist_data = [0 as u8; 3];
                let offs = atari_system.antic.dlist_offset(0);
                atari_system.antic_copy_to_slice(offs, &mut dlist_data);
                atari_system.antic.set_dlist_data(dlist_data);
            }
            atari_system.antic.prepare_mode_line();
        }
        atari_system.antic.update_dma_cycles();
        atari_system.antic.check_nmi();
        if atari_system.antic.wsync() {
            atari_system.antic.clear_wsync();
            atari_system.antic.cycle = 105;
        }
    }
    if atari_system.antic.fire_nmi() {
        cpu.non_maskable_interrupt_request();
    }
    if atari_system.antic.gets_visible() {
        if atari_system.antic.scan_line >= 8 && atari_system.antic.scan_line < 248 {
            assert!(antic_data.gtia_regs.regs.len() == 240);

            antic_data.gtia_regs.regs[atari_system.antic.scan_line - 8] = atari_system.gtia.regs;
            if atari_system.antic.scan_line == atari_system.antic.start_scan_line {
                let mut mode_line = atari_system.antic.create_next_mode_line();
                let charset_offset = (mode_line.chbase as usize) * 256;

                mode_line.video_memory_offset = antic_data.push_antic_memory(
                    atari_system,
                    mode_line.data_offset,
                    mode_line.n_bytes,
                );

                // todo: detect charset memory changes

                mode_line.charset_memory_offset = antic_data.push_antic_memory(
                    atari_system,
                    charset_offset,
                    mode_line.charset_size(),
                );

                antic_data.create_mode_line(&mode_line);
            }
        }
    }
    atari_system.antic.steal_cycles();
}

pub fn post_instr_tick(atari_system: &mut AtariSystem) {
    let antic = &mut atari_system.antic;
    if antic.wsync() {
        antic.do_wsync();
    }
    atari_system.gtia.scan_line =
        antic.scan_line - (antic.scan_line > 0 && antic.cycle < 104) as usize;
    atari_system.gtia.update_collisions_for_scanline();
}

pub fn get_pm_data(system: &mut AtariSystem, n: usize) -> u8 {
    let pm_hires = system.antic.dmactl.contains(DMACTL::PM_HIRES);
    let offs = if pm_hires {
        0x300
            + n * 0x100
            + system.antic.scan_line
            + (system.antic.pmbase & 0b11111000) as usize * 256
    } else {
        0x180
            + n * 0x80
            + system.antic.scan_line / 2
            + (system.antic.pmbase & 0b11111100) as usize * 256
    };
    system.read(offs as u16)
}

pub struct AnticPlugin {
    pub texture_size: Vec2,
    pub enable_collisions: bool,
    pub collision_agg_size: Option<(u32, u32)>,
}

#[derive(Default)]
struct CollistionsReadState {
    buffer: Vec<u8>,
}

fn collisions_read(_world: &mut World, resources: &mut Resources) {
    let mut state = resources.get_mut::<CollistionsReadState>().unwrap();
    let render_graph = resources.get_mut::<RenderGraph>().unwrap();
    let render_resource_context = resources.get_mut::<Box<dyn RenderResourceContext>>();
    if let Some(render_resource_context) = render_resource_context {
        let collisions_buffer_node: &CollisionsBufferNode =
            render_graph.get_node(COLLISIONS_BUFFER).unwrap();
        if state.buffer.len() != collisions_buffer_node.buffer_info.size {
            state.buffer = Vec::with_capacity(collisions_buffer_node.buffer_info.size);
            unsafe {
                state
                    .buffer
                    .set_len(collisions_buffer_node.buffer_info.size);
            }
        }
        if let Some(buffer_id) = collisions_buffer_node.buffer_id {
            let atari_system = resources.get::<crate::AtariSystem>().unwrap();
            render_resource_context.read_mapped_buffer(
                buffer_id,
                0..(state.buffer.len() as u64),
                &|data, _| {
                    let data = unsafe { std::mem::transmute::<&[u8], &[u64]>(&data) };
                    // collision texture is RG texture, but we read it in RGBA format (4 * u32)
                    // where only RG components are set. That's why we skip every second u64
                    let len = data.len() / 8;
                    let data = &data[..len];
                    let collision_array = &mut *atari_system.gtia.collision_array.write();
                    let width = len / 240 / 2;
                    let mut index = 0;
                    for i in 0..240 {
                        let mut agg = 0;
                        for _ in 0..width {
                            agg |= data[index];
                            index += 2;
                        }
                        collision_array[i] = agg;
                    }
                },
            );
        }
    }
}

impl Plugin for AnticPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<AnticData>()
            .add_asset::<AtariPalette>()
            .add_asset::<CustomTexture>();
        app.add_system_to_stage(CoreStage::PreUpdate, collisions_read.exclusive_system());
        app.init_resource::<CollistionsReadState>();
        let resources = app.resources_mut();
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        let mut palettes = resources.get_mut::<Assets<AtariPalette>>().unwrap();
        let mut antic_data = resources.get_mut::<Assets<AnticData>>().unwrap();

        pipelines.set_untracked(
            COLLISIONS_PIPELINE_HANDLE,
            render::build_collisions_pipeline(&mut shaders),
        );
        pipelines.set_untracked(
            DEBUG_COLLISIONS_PIPELINE_HANDLE,
            render::build_debug_collisions_pipeline(&mut shaders),
        );
        palettes.set_untracked(ATARI_PALETTE_HANDLE, AtariPalette::default());
        antic_data.set_untracked(ANTIC_DATA_HANDLE, AnticData::default());

        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();

        render_graph.add_system_node(
            "atari_palette",
            AssetRenderResourcesNode::<AtariPalette>::new(false),
        );
        render_graph.add_system_node(
            "antic_data",
            AssetRenderResourcesNode::<AnticData>::new(false),
        );

        render_graph.add_system_node(
            "custom_texture",
            AssetRenderResourcesNode::<CustomTexture>::new(false),
        );
        render_graph
            .add_node_edge("custom_texture", MAIN_PASS)
            .unwrap();
        let size = Vec2::new(self.texture_size.x, self.texture_size.y);
        render_graph.add_antic_graph(
            resources,
            &size,
            self.enable_collisions,
            self.collision_agg_size,
        );
    }
}
