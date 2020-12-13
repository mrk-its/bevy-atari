use bevy::reflect::TypeUuid;
use bevy::{
    asset::{AssetLoader, LoadedAsset},
    prelude::*,
};
use emulator_6502::MOS6502;


#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Test {
    a: u8,
    b: u8,
    c: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Atari800 {
    is_pal: bool,
    machine_size: u8,

    builtin_basic: bool,
    keyboard_leds: u8,
    f_keys: u8,
    jumper: u8,
    builtin_game: bool,
    keyboard_detached: bool,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Antic {
    pub dmactl: u8,
    pub chactl: u8,
    pub hscrol: u8,
    pub vscrol: u8,
    pub pmbase: u8,
    pub chbase: u8,
    pub nmien: u8,
    pub nmist: u8,
    pub ir: u8,
    pub anticmode: u8,
    pub dctr: u8,
    pub lastline: u8,
    pub need_dl: u8,
    pub vscrol_off: u8,
    pub dlist: u16,
    pub screenaddr: u16,
    pub xops: u32,
    pub xpos_limit: u32,
    pub ypos: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Cartridge {
    pub saved_type: u32,
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C, packed)]
pub struct CPU {
    pub reg_a: u8,
    pub reg_p: u8,
    pub reg_s: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub irq: u8,
    pub pc: u16,
}

#[derive(Default)]
pub struct Memory<'a> {
    pub data: &'a [u8],
    pub attrib: &'a [u8],
    pub basic: &'a [u8],
    pub cart0bf: &'a [u8],
    pub os: &'a [u8],
    pub under_atarixl_os: &'a [u8],
    pub xegame: &'a [u8],
    pub num_xe_banks: u32,
    pub atarixe_memory: &'a [u8],
    pub portb: u8,
    pub cart_a0bf_enabled: [u8; 4],
    pub enable_mapram: [u8; 4],
}
#[derive(Debug, Default, Clone, Copy)]
#[repr(C, packed)]
pub struct PIA {
    pub pactl: u8,
    pub pbctl: u8,
    pub porta: u8,
    pub portb: u8,
    pub porta_mask: u8,
    pub portb_mask: u8,
    pub ca2: i32,
    pub ca2_negpending: i32,
    pub ca2_pospending: i32,
    pub cb2: i32,
    pub cb2_negpending: i32,
    pub cb2_pospending: i32,
}
#[derive(Debug, Default, Clone, Copy)]
#[repr(C, packed)]
pub struct POKEY {
    pub kbcode: u8,
    pub irqst: u8,
    pub irqen: u8,
    pub skctl: u8,
    pub shift_key: i32,
    pub keypressed: i32,
    pub delayed_serin_irq: i32,
    pub delayed_serout_irq: i32,
    pub delayed_xmtdone_irq: i32,
    pub audf: [u8; 4],
    pub audc: [u8; 4],
    pub audctl: u8,
    pub divnirq: [i32; 4],
    pub divnmax: [i32; 4],
    pub base_mult: i32,
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C, packed)]
pub struct GTIA {
    pub hposp0: u8,
    pub hposp1: u8,
    pub hposp2: u8,
    pub hposp3: u8,
    pub hposm0: u8,
    pub hposm1: u8,
    pub hposm2: u8,
    pub hposm3: u8,

    pub pf0pm: u8,
    pub pf1pm: u8,
    pub pf2pm: u8,
    pub pf3pm: u8,

    pub m0pl: u8,
    pub m1pl: u8,
    pub m2pl: u8,
    pub m3pl: u8,
    pub p0pl: u8,
    pub p1pl: u8,
    pub p2pl: u8,
    pub p3pl: u8,

    pub sizep0: u8,
    pub sizep1: u8,
    pub sizep2: u8,
    pub sizep3: u8,
    pub sizem: u8,

    pub grafp0: u8,
    pub grafp1: u8,
    pub grafp2: u8,
    pub grafp3: u8,
    pub grafm: u8,

    pub colpm0: u8,
    pub colpm1: u8,
    pub colpm2: u8,
    pub colpm3: u8,
    pub colpf0: u8,
    pub colpf1: u8,
    pub colpf2: u8,
    pub colpf3: u8,
    pub colbk: u8,

    pub prior: u8,
    pub vdelay: u8,
    pub gractl: u8,
    pub consol_mask: u8,
    pub speaker: i32,
    pub next_console_value: u32,
    pub trig_latch: u32,
}

pub fn read<T>(data: &[u8]) -> (&T, &[u8]) {
    let (head, body, _) = unsafe { data.align_to::<T>() };
    assert!(head.is_empty());
    (&body[0], &data[std::mem::size_of::<T>()..])
}

pub fn skip_sio(data: &[u8]) -> &[u8] {
    let mut offs: usize = 0;
    for i in 0..8 {
        offs += 4;
        let len = data[offs] as usize + 256 * data[offs + 1] as usize;
        offs += 2 + len;
        info!("SIO {} {}", i, len);
    }
    return &data[offs..];
}

pub fn read_memory(data: &[u8]) -> (Memory, &[u8]) {
    let mut memory = Memory::default();
    let base_ram_kb = data[0];
    let data = &data[4..];
    memory.data = &data[0..0x10000];
    memory.attrib = &data[0x10000..0x20000];
    let data = &data[0x20000..];

    memory.basic = &data[0..0x2000];
    let data = &data[0x2000..];

    memory.cart0bf = &data[0..0x2000];
    let data = &data[0x2000..];

    memory.os = &data[0..0x4000];
    let data = &data[0x4000..];

    memory.under_atarixl_os = &data[0..0x4000];
    let data = &data[0x4000..];

    memory.xegame = &data[0..0x2000];
    let data = &data[0x2000..];

    let num_xe_banks = data[0] as usize + 256 * (data[1] as usize);
    let data = &data[4..];

    let mut ram_size = base_ram_kb as usize + num_xe_banks * 16;
    let data = if ram_size == 320 {
        let xe_type = data[0] as usize + 256 * (data[1] as usize);
        ram_size += xe_type;
        &data[4..]
    } else {
        data
    };
    assert!(ram_size == 64 || ram_size == 128);
    memory.portb = data[0];
    memory.cart_a0bf_enabled.copy_from_slice(&data[1..5]);
    let data = &data[5..];
    let data = if ram_size > 64 {
        let atarixe_memory_size = (1 + (ram_size - 64) / 16) * 16384;
        memory.atarixe_memory = &data[0..atarixe_memory_size];
        &data[atarixe_memory_size..]
    } else {
        data
    };
    let data = if ram_size > 20 {
        memory.enable_mapram.copy_from_slice(&data[0..4]);
        &data[4..]
    } else {
        data
    };
    (memory, data)
}

pub struct Atari800State<'a> {
    pub atari800: &'a Atari800,
    pub cartridge: &'a Cartridge,
    pub antic: &'a Antic,
    pub gtia: &'a GTIA,
    pub pia: &'a PIA,
    pub pokey: &'a POKEY,
    pub cpu: CPU,
    pub memory: Memory<'a>,
}

impl<'a> Atari800State<'a> {
    pub fn reload(&self, atari_system: &mut crate::system::AtariSystem, cpu: &mut MOS6502) {
        atari_system.load_atari800_state(self);
        cpu.program_counter = self.cpu.pc;
        cpu.accumulator = self.cpu.reg_a;
        cpu.x_register = self.cpu.reg_x;
        cpu.y_register = self.cpu.reg_y;
        cpu.status_register = self.cpu.reg_p;
        cpu.stack_pointer = self.cpu.reg_s;
    }

    pub fn new(data: &[u8]) -> Atari800State {
        let (header, data) = data.split_at(8);
        assert!(std::str::from_utf8(header).expect("valid utf") == "ATARI800");
        assert!(data[0] == 8);
        let verbose = data[1] > 0;
        assert!(verbose, "verbose save expected");
        let (_, data) = data.split_at(2);

        let (atari800, data) = read::<Atari800>(data);
        assert!(
            atari800.machine_size == 1,
            "not supported machine size: {}",
            atari800.machine_size
        );

        let (cartridge, data) = read::<Cartridge>(data);
        assert!(
            cartridge.saved_type == 0,
            "reading cartridge is not supported"
        );

        let data = skip_sio(data);

        let (antic, data) = read::<Antic>(data);

        let (cpu, _) = read::<CPU>(data);
        let mut cpu = cpu.clone();
        let data = &data[6..];
        let (memory, data) = read_memory(data);
        cpu.pc = data[0] as u16 + (data[1] as u16) * 256;
        let (gtia, data) = read::<GTIA>(data);

        let (pia, data) = read::<PIA>(data);
        let (pokey, _data) = read::<POKEY>(data);

        info!("cpu: {:?}", cpu);
        info!("gtia: {:?}", gtia);
        info!("pia: {:?}", pia);
        info!("pokey: {:?}", pokey);

        Atari800State {
            atari800,
            cartridge,
            antic,
            gtia,
            pia,
            pokey,
            cpu,
            memory,
        }
    }
}


#[derive(TypeUuid)]
#[uuid = "bc6b887f-3a1e-49f2-b101-8e14ab5ceaff"]
pub struct StateFile{
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct Atari800StateLoader;

impl AssetLoader for Atari800StateLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let state_file = StateFile {
                data: bytes.to_owned(),
            };
            load_context.set_default_asset(LoadedAsset::new(state_file));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["state"]
    }
}
