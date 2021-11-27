// use crate::render_resources::GTIARegs;
use bevy_atari_antic::CollisionsData;

use bevy_atari_antic::GTIARegs;

// WRITE
pub const HPOSP0: usize = 0x00;
pub const HPOSP1: usize = 0x01;
pub const HPOSP2: usize = 0x02;
pub const HPOSP3: usize = 0x03;
pub const HPOSM0: usize = 0x04;
pub const HPOSM1: usize = 0x05;
pub const HPOSM2: usize = 0x06;
pub const HPOSM3: usize = 0x07;
pub const SIZEP0: usize = 0x08;
pub const SIZEP1: usize = 0x09;
pub const SIZEP2: usize = 0x0a;
pub const SIZEP3: usize = 0x0b;
pub const SIZEM: usize = 0x0c;
pub const GRAFP0: usize = 0x0d;
pub const GRAFP1: usize = 0x0e;
pub const GRAFP2: usize = 0x0f;
pub const GRAFP3: usize = 0x10;
pub const GRAFM: usize = 0x11;
pub const COLPM0: usize = 0x12;
pub const COLPM1: usize = 0x13;
pub const COLPM2: usize = 0x14;
pub const COLPM3: usize = 0x15;
pub const COLPF0: usize = 0x16;
pub const COLPF1: usize = 0x17;
pub const COLPF2: usize = 0x18;
pub const COLPF3: usize = 0x19;
pub const COLBK: usize = 0x1a;
pub const PRIOR: usize = 0x1b;
pub const VDELAY: usize = 0x1c;
pub const _GRACTL: usize = 0x1d; // TODO - move consts to submodule
pub const HITCLR: usize = 0x1e;

pub const CONSOL: usize = 0x1f; // RW  bits 0-2:  Start/Select/Option

// READ
pub const M0PF: usize = 0x00;
pub const M1PF: usize = 0x01;
pub const M2PF: usize = 0x02;
pub const M3PF: usize = 0x03;
pub const P0PF: usize = 0x04;
pub const P1PF: usize = 0x05;
pub const P2PF: usize = 0x06;
pub const P3PF: usize = 0x07;
pub const M0PL: usize = 0x08; // ok
pub const M1PL: usize = 0x09;
pub const M2PL: usize = 0x0a;
pub const M3PL: usize = 0x0b;
pub const P0PL: usize = 0x0c; // ok
pub const P1PL: usize = 0x0d;
pub const P2PL: usize = 0x0e;
pub const P3PL: usize = 0x0f;
pub const TRIG0: usize = 0x10;
pub const TRIG1: usize = 0x11;
pub const TRIG2: usize = 0x12;
pub const TRIG3: usize = 0x13;
pub const PAL: usize = 0x14;

bitflags! {
    #[derive(Default)]
    pub struct GRACTL: u8 {
        const MISSILE_DMA = 0x01;
        const PLAYER_DMA = 0x02;
        const TRIGGER_LATCH = 0x04;
    }
}

pub struct Gtia {
    pub scan_line: usize,
    pub collision_update_scanline: usize,
    pub regs: GTIARegs,
    collisions: [u8; 0x16], // R
    pub trig: [u8; 4],      // R
    pub gractl: GRACTL,
    pub consol: u8,
    pub consol_mask: u8,
    pub consol_force_mask: u8,
}

impl Default for Gtia {
    fn default() -> Self {
        Self {
            regs: GTIARegs::default(),
            collisions: [0x00; 0x16],
            trig: [0xff, 0xff, 0xff, 0x00],
            gractl: GRACTL::from_bits_truncate(0),
            consol: 0x7,
            consol_mask: 0x7,
            consol_force_mask: 0x7, // force option on start;
            scan_line: 0,
            collision_update_scanline: 0,
        }
    }
}

impl Gtia {
    pub fn read(&mut self, addr: usize) -> u8 {
        let addr = addr & 0x1f;
        let value = match addr {
            0x0..=0xf => {
                let v = self.collisions[addr];
                // info!("reading collisions {:x?}: {:x?}, scan_line: {:?}", addr, v, self.scan_line);
                v
            }
            CONSOL => self.consol & self.consol_mask & self.consol_force_mask,
            TRIG0..=TRIG3 => self.trig[addr - TRIG0],
            PAL => 0x01, // 0x01 - PAL, 0x0f - NTSC
            _ => 0x0f,
        };
        // warn!("GTIA read: {:02x}: {:02x}", addr, value);
        value
    }
    pub fn write(&mut self, addr: usize, value: u8) {
        let addr = addr & 0x1f;

        let _size_pm = |x: u8| match x & 3 {
            1 => 1,
            3 => 2,
            _ => 0,
        };

        match addr {
            COLPM0..=COLBK => self.regs.col[addr - COLPM0] = value,
            GRAFP0..=GRAFP3 => self.regs.grafp[addr - GRAFP0] = value,
            GRAFM => self.regs.grafm = value,
            PRIOR => self.regs.prior = value,
            HPOSP0..=HPOSP3 => self.regs.hposp[addr - HPOSP0] = value,
            HPOSM0..=HPOSM3 => self.regs.hposm[addr - HPOSM0] = value,
            SIZEP0..=SIZEP3 => self.regs.sizep[addr - SIZEP0] = _size_pm(value),
            SIZEM => {
                self.regs.sizem = _size_pm(value)
                    | (_size_pm(value >> 2) << 2)
                    | (_size_pm(value >> 4) << 4)
                    | (_size_pm(value >> 6) << 6)
            }
            _GRACTL => self.gractl = GRACTL::from_bits_truncate(value),
            CONSOL => self.consol_mask = 0x7 & !value,
            HITCLR => {
                // info!("resetting collisions, scan_line: {:?}", self.scan_line);
                self.collisions.iter_mut().for_each(|v| *v = 0);
                // self.scan_line is set by antic to completed (displayed) scanline
                // set collision_update_scanline to next scanline
                // so we will do next collision update when next scanline is complete
                self.collision_update_scanline = self.scan_line + 1;
                // RiverRaid: Activision logo ends on 237 line and generates collisions
                // clear is done like below:
                // B76D: STA WSYNC - # end of scanline 237
                // B779: STA HITCLR
            }
            _ => (),
        }
    }
    pub fn set_trig(&mut self, n: usize, is_pressed: bool) {
        self.trig[n] = if is_pressed { 0 } else { 0x01 };
    }

    pub fn update_collisions_for_scanline(&mut self, collisions: &CollisionsData) {
        // this is called when scan_line is complete
        if self.scan_line > self.collision_update_scanline
            && self.scan_line >= 8
            && self.scan_line < 248
        {
            let collision_array = collisions.inner.read();
            for i in self.collision_update_scanline.max(8)..self.scan_line {
                self.update_collisions(collision_array.data[i - 8]);
            }
            self.collision_update_scanline = self.scan_line;
        }
    }

    pub fn update_collisions(&mut self, data: u64) {
        // if data > 0 {
        //     info!(
        //         "update collisions: {:?}, scanline: {:?}",
        //         data, self.scan_line
        //     );
        // }
        // if self.scan_line > 216 {
        //     return
        // }
        let data0 = data & 0xffff;
        let data1 = (data >> 16) & 0xffff;
        let data2 = (data >> 32) & 0xffff;
        let data3 = (data >> 48) & 0xffff;

        self.collisions[M0PF] |= (data0 & 0xf) as u8;
        self.collisions[M1PF] |= ((data0 >> 4) & 0xf) as u8;
        self.collisions[M2PF] |= ((data0 >> 8) & 0xf) as u8;
        self.collisions[M3PF] |= ((data0 >> 12) & 0xf) as u8;

        self.collisions[P0PF] |= (data1 & 0xf) as u8;
        self.collisions[P1PF] |= ((data1 >> 4) & 0xf) as u8;
        self.collisions[P2PF] |= ((data1 >> 8) & 0xf) as u8;
        self.collisions[P3PF] |= ((data1 >> 12) & 0xf) as u8;

        self.collisions[M0PL] |= (data2 & 0xf) as u8;
        self.collisions[M1PL] |= ((data2 >> 4) & 0xf) as u8;
        self.collisions[M2PL] |= ((data2 >> 8) & 0xf) as u8;
        self.collisions[M3PL] |= ((data2 >> 12) & 0xf) as u8;

        self.collisions[P0PL] |= (data3 & 0xf) as u8;
        self.collisions[P1PL] |= ((data3 >> 4) & 0xf) as u8;
        self.collisions[P2PL] |= ((data3 >> 8) & 0xf) as u8;
        self.collisions[P3PL] |= ((data3 >> 12) & 0xf) as u8;

        // fred
        // if data[1] > 0 {
        //     self.collisions[P2PF] |= 0xf; // collision with any playfield color
        //     self.collisions[P3PF] |= 0xf;
        // }
    }
}
