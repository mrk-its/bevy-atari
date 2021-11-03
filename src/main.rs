#[macro_use]
extern crate bitflags;
use std::io::prelude::*;
pub mod antic;
mod atari800_state;
// pub mod atari_text;
pub mod atr;
mod cartridge;
// mod debug;
pub mod gtia;
mod js_api;
pub mod keyboard;
pub mod multiplexer;
mod palette;
pub mod pia;
pub mod pokey;

pub mod render;

pub mod sio;
mod system;
pub mod time_used_plugin;
use crate::cartridge::Cartridge;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::log::{Level, LogSettings};
use bevy::render2::view::Msaa;
#[allow(unused_imports)]
use bevy::winit::WinitConfig;
use bevy::{prelude::*, PipelinedDefaultPlugins};
use emulator_6502::{Interface6502, MOS6502};
// use render::ANTIC_DATA_HANDLE;
// use render_resources::{AnticData, CustomTexture, SimpleMaterial};
use render::{AnticData, ANTIC_DATA_HANDLE};
use system::AtariSystem;

#[derive(PartialEq, Copy, Clone, Default)]
pub struct DisplayConfig {
    pub fps: bool,
    pub debug: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum EmulatorState {
    Idle,
    Running,
}

#[allow(dead_code)]
#[derive(Debug)]
enum BreakPoint {
    PC(u16),
    IndirectPC(u16),
    NotPC(u16),
    ScanLine(usize),
}

#[derive(Default)]
pub struct ClearCollisions(pub bool);

#[derive(Debug, Default)]
pub struct FrameState {
    paused: bool,
    break_point: Option<BreakPoint>,
}

impl FrameState {
    fn set_breakpoint(&mut self, break_point: BreakPoint) {
        self.paused = false;
        self.break_point = Some(break_point);
    }
    fn clear_break_point(&mut self) {
        self.paused = true;
        self.break_point = None;
    }
}

fn gunzip(data: &[u8]) -> Vec<u8> {
    let mut decoder = flate2::read::GzDecoder::new(&data[..]);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result).unwrap();
    result
}

fn atari_system(
    mut display_config: ResMut<DisplayConfig>,
    mut frame: ResMut<FrameState>,
    mut cpu: ResMut<MOS6502>,
    mut atari_system: ResMut<AtariSystem>,
    mut antic_data_assets: ResMut<Assets<AnticData>>,
    keyboard: Res<Input<KeyCode>>,
) {
    if frame.paused {
        return;
    }
    let mut prev_pc = 0;
    let antic_data = antic_data_assets.get_mut(ANTIC_DATA_HANDLE).unwrap();

    loop {
        if (atari_system.antic.scan_line, atari_system.antic.cycle) == (0, 0) {
            if atari_system.handle_keyboard(&keyboard, &mut *cpu) {
                cpu.interrupt_request();
            }
        };

        antic::tick(&mut *atari_system, &mut *cpu, &mut *antic_data);

        match cpu.get_program_counter() {
            0xe459 => sio::sioint_hook(&mut *atari_system, &mut *cpu),
            _ => (),
        }

        if frame.paused {
            return;
        }

        cpu.cycle(&mut *atari_system);

        if cpu.get_remaining_cycles() == 0 {
            antic::post_instr_tick(&mut *atari_system);
            match frame.break_point {
                Some(BreakPoint::PC(pc)) => {
                    if cpu.get_program_counter() == pc {
                        frame.clear_break_point();
                        display_config.debug = true;
                    }
                }
                Some(BreakPoint::NotPC(pc)) => {
                    if cpu.get_program_counter() != pc {
                        frame.clear_break_point();
                        display_config.debug = true;
                    }
                }
                Some(BreakPoint::IndirectPC(addr)) => {
                    let pc =
                        atari_system.read(addr) as u16 + atari_system.read(addr + 1) as u16 * 256;
                    if prev_pc != pc {
                        prev_pc = pc;
                        info!("run addr: {:x?}", pc);
                    }
                    if cpu.get_program_counter() == pc {
                        // frame.clear_break_point();
                        frame.paused = true;
                        display_config.debug = true;
                    }
                }
                _ => (),
            }
        }
        atari_system.inc_cycle();
        if atari_system.antic.cycle == 0 {
            if let Some(BreakPoint::ScanLine(scan_line)) = &frame.break_point {
                if *scan_line == atari_system.antic.scan_line {
                    frame.paused = true;
                    frame.break_point = None;
                    display_config.debug = true;
                    break;
                }
            }
            if atari_system.antic.scan_line == 248 {
                atari_system.pokey.send_regs();
                break;
            }
        }
        if frame.paused && !atari_system.antic.wsync() {
            break;
        }
    }
}

// pub const SCANLINE_MESH_HANDLE: HandleUntyped =
//     HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6039053558161382807);

fn events(
    mut state: ResMut<State<EmulatorState>>,
    mut atari_system: ResMut<AtariSystem>,
    mut frame: ResMut<FrameState>,
    mut cpu: ResMut<MOS6502>,
) {
    let mut messages = js_api::MESSAGES.write();
    for event in messages.drain(..) {
        match event {
            js_api::Message::Reset {
                cold,
                disable_basic,
            } => {
                atari_system.reset(&mut *cpu, cold, disable_basic);
            }
            js_api::Message::SetState(new_state) => {
                match new_state.as_ref() {
                    "running" => info!("{:?}", state.set(EmulatorState::Running)),
                    "idle" => {
                        state.set(EmulatorState::Idle).ok();
                    }
                    _ => panic!("invalid state requested"),
                };
                info!("set_state: {:?}", new_state);
            }
            js_api::Message::JoyState { port, dirs, fire } => {
                atari_system.set_joystick(1, port, dirs, fire)
            }
            js_api::Message::SetConsol { state } => {
                atari_system.update_consol(1, state);
            }
            js_api::Message::BinaryData { key, data, .. } => match key.as_str() {
                "basic" => {
                    atari_system.set_basic(data);
                }
                "osrom" => {
                    atari_system.set_osrom(data);
                }
                "disk_1" => {
                    atari_system.disk_1 = data.map(|data| atr::ATR::new(&data));
                }
                "car" => {
                    atari_system.set_cart(
                        data.map(|data| <dyn Cartridge>::from_bytes(&data).ok())
                            .flatten(),
                    );
                }
                "state" => {
                    if let Some(data) = data {
                        let data = gunzip(&data);
                        let a800_state = atari800_state::Atari800State::new(&data);
                        a800_state.reload(&mut *atari_system, &mut *cpu);
                        // *frame = FrameState::default();
                        state.set(EmulatorState::Running).ok();
                        info!("LOADED! {:?}", *state);
                    }
                }
                _ => {
                    warn!("unknown binary");
                }
            },
            js_api::Message::Command { cmd } => {
                let parts = cmd.split(" ").collect::<Vec<_>>();
                match parts[0] {
                    "mem" => {
                        if let Ok(start) = u16::from_str_radix(parts[1], 16) {
                            let mut data = [0 as u8; 256];
                            atari_system.copy_to_slice(start, &mut data);
                            info!("{:x?}", data);
                        }
                    }
                    "write" => {
                        if let Ok(addr) = u16::from_str_radix(parts[1], 16) {
                            if let Ok(value) = u8::from_str_radix(parts[2], 16) {
                                atari_system.write(addr, value);
                                info!("write {:04x} <- {:02x}", addr, value);
                            }
                        }
                    }
                    "pc" => {
                        if let Ok(pc) = u16::from_str_radix(parts[1], 16) {
                            cpu.set_program_counter(pc)
                        }
                    }
                    "brk" => {
                        if let Ok(pc) = u16::from_str_radix(parts[1], 16) {
                            frame.break_point = Some(BreakPoint::PC(pc));
                            info!("breakpoint set on pc={:04x}", pc);
                        }
                    }
                    "trainer_init" => {
                        atari_system.trainer_init();
                    }
                    "trainer_changed" => {
                        let cnt = atari_system.trainer_changed(true);
                        info!("matched: {}", cnt);
                    }
                    "trainer_unchanged" => {
                        let cnt = atari_system.trainer_changed(false);
                        info!("matched: {}", cnt);
                    }
                    _ => (),
                }
            }
        }
    }
}

fn main() {
    let mut log_filter = "bevy_webgl2=warn".to_string();
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
        if let Ok(Some(_log_filter)) = local_storage.get_item("log") {
            log_filter = _log_filter;
        }
    }
    let mut app = App::new();
    app.insert_resource(LogSettings {
        filter: log_filter,
        level: Level::INFO,
    });
    app.insert_resource(Msaa { samples: 1 });
    app.insert_resource(WindowDescriptor {
        title: "GoodEnoughAtariEmulator".to_string(),
        width: 768.0,
        height: 480.0,
        resizable: false,
        scale_factor_override: Some(1.0),
        mode: bevy::window::WindowMode::Windowed,
        #[cfg(target_arch = "wasm32")]
        canvas: Some("#bevy-canvas".to_string()),
        vsync: true,
        ..Default::default()
    });

    app.add_plugins(PipelinedDefaultPlugins);
    app.add_plugin(render::AnticRenderPlugin);
    app.add_plugin(time_used_plugin::TimeUsedPlugin);
    app.insert_resource(WinitConfig {
        force_fps: Some(50.0),
        return_from_run: false,
    });
    // app.add_plugins(DefaultPlugins);

    //    app.add_plugin(atari_text::AtartTextPlugin::default());
    // app.add_asset::<SimpleMaterial>();
    // app.add_asset::<CustomTexture>();
    // app.add_plugin(render::AnticRenderPlugin {
    //     texture_size: Vec2::new(render::ANTIC_TEXTURE_SIZE.0, render::ANTIC_TEXTURE_SIZE.1),
    //     enable_collisions: true,
    //     collision_agg_size: render::COLLISION_AGG_SIZE,
    // });
    app.add_plugin(FrameTimeDiagnosticsPlugin::default());
    app.add_plugin(LogDiagnosticsPlugin::default());

    let mut system = AtariSystem::new();
    let mut cpu = MOS6502::default();

    system.set_osrom(Some(
        include_bytes!("../assets/Atari OS v2 83.10.05.rom").to_vec(),
    ));
    let cart = <dyn Cartridge>::from_bytes(include_bytes!("../assets/flob.1.0.3.car")).unwrap();
    system.set_cart(Some(cart));

    system.reset(&mut cpu, true, true);

    let frame = FrameState::default();
    // frame.break_point = Some(BreakPoint::IndirectPC(0x2e0));
    // frame.break_point = Some(BreakPoint::PC(0x7100));

    app
        // .insert_resource(ClearColor(gtia::atari_color(0)))
        .insert_resource(DisplayConfig {
            fps: true,
            debug: false,
        })
        .insert_resource(system)
        .insert_resource(cpu)
        .insert_resource(frame)
        // .add_startup_system(debug::setup.system())
        .add_state(EmulatorState::Running)
        .add_system_to_stage(CoreStage::PreUpdate, keyboard::system.system())
        .add_system_set(
            SystemSet::on_update(EmulatorState::Running)
                .with_system(atari_system.system().label("run_atari"))
                // .with_system(debug::debug_overlay_system.system().after("run_atari"))
                // .with_system(debug::update_fps.system()),
        )
        // .add_system(debug::update_display_config.system())
        .add_system(events.system())
        .run();
}
