use std::str::Chars;

use crate::render_resources::{AnticLine, AtariPalette};
use crate::render_resources::{Charset, GTIARegsArray, LineData};
use crate::system::AtariSystem;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::pipeline::RenderPipeline;
use bevy::{
    prelude::{Handle, Mesh},
    render::pipeline::PipelineDescriptor,
    sprite::QUAD_HANDLE,
};

pub const ATARI_PALETTE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 13714196555738289155);

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
use wasm_bindgen::prelude::*;

const PAL_SCAN_LINES: usize = 312;
#[allow(dead_code)]
const NTSC_SCAN_LINES: usize = 262;

pub const MAX_SCAN_LINES: usize = PAL_SCAN_LINES;
pub const SCAN_LINE_CYCLES: usize = 114;

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
    pub dlist_data: [u8; 3],
    pub vblank: bool,
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
    pub scan_line: usize,
    pub vcount: u8,
    pub video_memory: usize,
    pub wsync: bool,
    enable_log: bool,
    pub is_vscroll: bool,
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
    pub line_data: LineData,
    pub charset: Charset,
    pub gtia_regs_array: GTIARegsArray,
}

impl ModeLineDescr {
    pub fn next_mode_line(&self) -> usize {
        return self.scan_line + self.height;
    }
}

#[derive(Default)]
pub struct AnticResources {
    pub pipeline_handle: Handle<PipelineDescriptor>,
    pub palette_handle: Handle<AtariPalette>,
}

const MODE_25_STEALED_CYCLES_FIRST_LINE: [(usize, &[usize; 8]); 4] = [
    (0, &[0, 0, 0, 0, 0, 0, 0, 0]),
    (25, &[66, 66, 66, 66, 66, 66, 66, 66]),
    (18, &[81, 81, 81, 81, 81, 81, 82, 81]),
    (10, &[96, 95, 94, 93, 92, 91, 90, 89]),
];

const MODE_25_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (0, &[0, 0, 0, 0, 0, 0, 0, 0]),
    (29, &[41, 41, 41, 41, 41, 41, 41, 41]),
    (21, &[49, 49, 49, 49, 49, 49, 49, 48]),
    (13, &[56, 55, 55, 54, 54, 53, 53, 53]),
];

const MODE_67_STEALED_CYCLES_FIRST_LINE: [(usize, &[usize; 8]); 4] = [
    (0, &[0, 0, 0, 0, 0, 0, 0, 0]),
    (25, &[41, 41, 41, 41, 41, 41, 41, 41]),
    (18, &[49, 49, 49, 49, 49, 49, 49, 48]),
    (10, &[57, 56, 56, 56, 55, 54, 54, 54]),
];

const MODE_67_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (0, &[0, 0, 0, 0, 0, 0, 0, 0]),
    (29, &[25, 25, 25, 25, 25, 25, 25, 25]),
    (21, &[29, 29, 29, 29, 29, 29, 29, 29]),
    (13, &[33, 32, 32, 32, 32, 31, 31, 31]),
];

const MODE_89_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (0, &[0, 0, 0, 0, 0, 0, 0, 0]),
    (0, &[17, 17, 17, 17, 17, 17, 17, 17]),
    (0, &[19, 19, 19, 19, 19, 19, 19, 19]),
    (0, &[21, 21, 21, 21, 21, 21, 20, 20]),
];

const MODE_AC_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (0, &[0, 0, 0, 0, 0, 0, 0, 0]),
    (0, &[25, 25, 25, 25, 25, 25, 25, 25]),
    (0, &[29, 29, 29, 29, 29, 29, 29, 29]),
    (0, &[33, 33, 32, 32, 32, 32, 31, 31]),
];

const MODE_DF_STEALED_CYCLES: [(usize, &[usize; 8]); 4] = [
    (0, &[0, 0, 0, 0, 0, 0, 0, 0]),
    (0, &[41, 41, 41, 41, 41, 41, 41, 41]),
    (0, &[49, 49, 49, 49, 49, 49, 49, 49]),
    (0, &[56, 56, 55, 55, 54, 54, 53, 53]),
];

impl Antic {
    pub fn set_scan_line(&mut self, scan_line: usize, cycle: usize) {
        self.scan_line = scan_line;
        let scan_line = if cycle >= 110 {
            (scan_line + 1) % MAX_SCAN_LINES
        } else {
            scan_line
        };
        self.vcount = (scan_line / 2) as u8;
        if self.scan_line < 8 || self.scan_line >= 248 {
            self.next_scan_line = 8;
            self.is_vscroll = false;
            self.line_voffset = 0;
        }
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
    pub fn set_vbi(&mut self) {
        self.nmist.insert(NMIST::VBI);
        self.nmist.remove(NMIST::DLI);
    }
    pub fn set_dli(&mut self) {
        self.nmist.insert(NMIST::DLI);
        self.nmist.remove(NMIST::VBI);
    }

    pub fn is_vbi(&mut self) -> bool {
        let vbi = self.scan_line == 248 && self.nmien.contains(NMIEN::VBI);
        if vbi {
            self.set_vbi();
        }
        vbi
    }

    pub fn is_dli(&mut self) -> bool {
        let opts = MODE_OPTS::from_bits_truncate(self.dlist_data[0]);
        if opts.contains(MODE_OPTS::DLI) && self.nmien.contains(NMIEN::DLI) && self.scan_line >= 8 && self.scan_line < 248 {
            if self.scan_line == self.start_scan_line + self.line_height - 1 {
                self.set_dli();
                return true;
            }
        }
        false
    }

    pub fn get_dma_cycles(&self) -> (usize, usize, usize) {
        if self.scan_line < 8 || self.scan_line >= 248 {
            return (0, 0, 0);
        }
        // TODO - take hscroll into account for steal start value
        let is_first_mode_line = self.scan_line == self.start_scan_line;
        let mode = self.dlist_data[0] & 0x0f;

        let (line_start_cycle, dma_cycles) = if (self.dmactl & DMACTL::PLAYFIELD_WIDTH_MASK).bits() > 0 {
            let opts: MODE_OPTS = MODE_OPTS::from_bits_truncate(self.dlist_data[0]);

            let is_hscrol = opts.contains(MODE_OPTS::HSCROL);
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

                _ => (0, &[0, 0, 0, 0, 0, 0, 0, 0]),
            };
            (line_start_cycle, dma_cycles_arr[hscrol])
        } else {
            (0, 0)
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
        (
            start_dma_cycles,
            line_start_cycle.max(start_dma_cycles),
            dma_cycles,
        )
    }

    fn create_mode_line(
        &self,
        opts: MODE_OPTS,
        mode: u8,
        height: usize,
        n_bytes: usize,
        scan_line: usize,
    ) -> ModeLineDescr {
        let is_hscrol = opts.contains(MODE_OPTS::HSCROL);
        let hscrol = if is_hscrol { 32 - self.hscrol * 2 } else { 0 };

        let hscrol_line_width = n_bytes * self.playfield_width(true, is_hscrol) / 320;
        let width = if mode > 1 {
            self.playfield_width(false, is_hscrol)
        } else {
            320
        };
        ModeLineDescr {
            mode,
            opts,
            height: self.line_height,
            line_voffset: self.line_voffset,
            n_bytes: hscrol_line_width,
            scan_line: scan_line,
            width,
            data_offset: self.video_memory,
            chbase: self.chbase,
            pmbase: self.pmbase,
            hscrol,
            line_data: LineData::default(),
            charset: Charset::default(),
            gtia_regs_array: GTIARegsArray::default(),
        }
    }

    pub fn dlist_offset(&self, k: u8) -> u16 {
        return self.dlist & 0xfc00 | self.dlist.overflowing_add(k as u16).0 & 0x3ff;
    }

    pub fn inc_dlist(&mut self, k: u8) {
        self.dlist = self.dlist_offset(k);
    }

    pub fn prefetch_dlist(&self, ram: &[u8]) -> Option<[u8; 3]> {
        if self.dmactl.contains(DMACTL::DLIST_DMA)
            && (self.scan_line == 8 || self.scan_line == self.next_scan_line)
        {
            let mut data: [u8; 3] = [0; 3];
            data[0] = ram[self.dlist_offset(0) as usize];
            data[1] = ram[self.dlist_offset(1) as usize];
            data[2] = ram[self.dlist_offset(2) as usize];
            Some(data)
        } else {
            None
        }
    }

    pub fn set_dlist_data(&mut self, dlist_data: [u8; 3]) {
        self.dlist_data = dlist_data;
        let mode = self.dlist_data[0] & 0xf;
        let opts = MODE_OPTS::from_bits_truncate(self.dlist_data[0]);
        self.inc_dlist(1);
        if opts.contains(MODE_OPTS::LMS) && mode > 1 {
            self.video_memory = self.dlist_data[1] as usize + (self.dlist_data[2] as usize * 256);
            // info!("LMS: {:04x}", self.video_memory);
            self.inc_dlist(2);
        }
        if mode == 1 {
            self.dlist = self.dlist_data[1] as u16 | ((self.dlist_data[2] as u16) << 8);
            if opts.contains(MODE_OPTS::LMS) {
                // info!("dlist restart");
                self.start_scan_line = self.scan_line;
                self.next_scan_line = 8;
                return;
            }
        }
        let current_mode = &ANTIC_MODES[mode as usize];
        self.line_height = current_mode.height;
        self.n_bytes = current_mode.n_bytes;
        if mode == 0 {
            self.line_height = ((self.dlist_data[0] >> 4) & 7) as usize + 1;
        }
        let is_vscroll = mode > 0 && opts.contains(MODE_OPTS::VSCROL);
        self.line_voffset = 0;
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
        // info!(
        //     "mode: {:?} opts: {:?} {:?} scan_line: {} next: {}",
        //     mode, opts, current_mode, self.start_scan_line, self.next_scan_line
        // );
    }

    pub fn create_next_mode_line(&mut self) -> Option<ModeLineDescr> {
        let scan_line = self.scan_line;
        let opts = MODE_OPTS::from_bits_truncate(self.dlist_data[0]);
        let mode = self.dlist_data[0] & 0xf;
        let mode_line = match mode {
            0x0 => self.create_mode_line(
                opts,
                mode,
                ((self.dlist_data[0] >> 4) & 7) as usize + 1,
                0,
                scan_line,
            ),
            0x1 => {
                if opts.contains(MODE_OPTS::LMS) {
                    return None;
                }
                self.create_mode_line(opts, mode, 1, 0, scan_line)
            }
            0x2 => self.create_mode_line(opts, mode, 8, 40, scan_line),
            0x4 => self.create_mode_line(opts, mode, self.line_height, 40, scan_line),
            0xa => self.create_mode_line(opts, mode, 4, 20, scan_line),
            0xc => self.create_mode_line(opts, mode, 1, 20, scan_line),
            0xd => self.create_mode_line(opts, mode, 2, 40, scan_line),
            0xe => self.create_mode_line(opts, mode, 1, 40, scan_line),
            0xf => self.create_mode_line(opts, mode, 1, 40, scan_line),
            _ => {
                warn!("unsupported antic vide mode {:?}", mode);
                self.create_mode_line(opts, mode, 1, 0, scan_line)
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
            consts::VCOUNT => self.vcount,
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
            consts::HSCROL => self.hscrol = value & 0xf,
            consts::VSCROL => self.vscrol = value & 0xf,
            consts::DLIST_L => self.dlist = self.dlist & 0xff00 | value as u16,
            consts::DLIST_H => self.dlist = self.dlist & 0xff | ((value as u16) << 8),
            consts::WSYNC => self.wsync = true, // TODO
            _ => bevy::log::warn!("unsupported antic write reg: {:x?}", addr),
        }
    }
    pub fn enable_log(&mut self, enable: bool) {
        self.enable_log = enable;
    }
}

pub fn create_line_data(
    system: &AtariSystem,
    scan_line: usize,
    pmbase: u8,
    data_offset: usize,
) -> LineData {
    let pm_hires = system.antic.dmactl.contains(DMACTL::PM_HIRES);
    // TODO - check if PM DMA is working, page 114 of AHRM
    // if DMA is disabled display data from Graphics Data registers, p. 114
    // TODO - add suppor for low-res sprites

    let pl_mem = |n: usize| {
        if system.antic.dmactl.contains(DMACTL::PLAYER_DMA) {
            let beg = if pm_hires {
                0x400 + n * 0x100 + scan_line + (pmbase & 0b11111000) as usize * 256
            } else {
                0x200 + n * 0x80 + scan_line / 2 + (pmbase & 0b11111100) as usize * 256
            };
            system.ram[beg..beg + 16].to_owned()
        } else {
            let v = system.gtia.player_graphics[n];
            vec![v, v, v, v, v, v, v, v, v, v, v, v, v, v, v, v]
        }
    };

    LineData::new(
        &system.ram[data_offset..data_offset + 48],
        &pl_mem(0),
        &pl_mem(1),
        &pl_mem(2),
        &pl_mem(3),
    )
}

pub fn create_mode_line(
    commands: &mut Commands,
    resources: &AnticResources,
    mode_line: &ModeLineDescr,
    y_extra_offset: f32,
) {
    // info!("drawing: {:?}", mode_line);
    commands
        .spawn(MeshBundle {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                resources.pipeline_handle.clone_weak(),
            )]),
            // visible: Visible {
            //     is_transparent: true,
            //     is_visible: true,
            // },
            transform: Transform::from_translation(Vec3::new(
                0.0,
                120.0
                    - (mode_line.scan_line as f32)
                    - y_extra_offset
                    - mode_line.height as f32 / 2.0
                    + 8.0,
                0.0,
            ))
            .mul_transform(Transform::from_scale(Vec3::new(
                mode_line.width as f32,
                mode_line.height as f32,
                1.0,
            ))),
            ..Default::default()
        })
        .with(AnticLine {
            // chbase: mode_line.chbase as u32,
            mode: mode_line.mode as u32,
            gtia_regs_array: mode_line.gtia_regs_array,
            line_width: mode_line.width as f32,
            line_height: mode_line.height as f32,
            line_voffset: mode_line.line_voffset as f32,
            hscrol: mode_line.hscrol as f32,
            data: mode_line.line_data,
            charset: mode_line.charset,
            start_scan_line: mode_line.scan_line,
            end_scan_line: mode_line.next_mode_line(),
        })
        .with(resources.palette_handle.clone_weak());
}
