#[macro_use]
extern crate bitflags;
use std::io::prelude::*;
pub mod antic;
mod atari800_state;
// pub mod atari_text;
pub mod atr;
mod cartridge;
mod debug;
pub mod gamepad;
pub mod gtia;
mod js_api;
pub mod multiplexer;
pub mod pia;
pub mod pokey;

pub mod focus;

pub mod sio;
mod system;
pub mod time_used_plugin;
use crate::cartridge::Cartridge;

use bevy::render2::color::Color;
use bevy_egui::egui;
use bevy_egui::{EguiContext, EguiPlugin};

use bevy::{
    diagnostic::Diagnostics,
    render2::{camera::OrthographicCameraBundle, renderer::RenderDevice, texture::Image},
    sprite2::{PipelinedSpriteBundle, Sprite},
};
#[allow(unused_imports)]
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    log::{Level, LogSettings},
    prelude::*,
    render2::view::Msaa,
    winit::WinitConfig,
    PipelinedDefaultPlugins,
};
use bevy_atari_antic::AtariAnticPlugin;
use emulator_6502::{Interface6502, MOS6502};
// use render::ANTIC_DATA_HANDLE;
// use render_resources::{AnticData, CustomTexture, SimpleMaterial};
use bevy_atari_antic::AnticData;
use focus::Focused;
use system::AtariSystem;
use time_used_plugin::TimeUsedPlugin;

// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(PartialEq, Component, Copy, Clone, Default)]
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

#[derive(Debug, Component, Default)]
pub struct Debugger {
    paused: bool,
    break_point: Option<BreakPoint>,
}

impl Debugger {
    #[allow(dead_code)]
    fn set_breakpoint(&mut self, break_point: BreakPoint) {
        self.paused = false;
        self.break_point = Some(break_point);
    }
    fn clear_break_point(&mut self) {
        self.paused = true;
        self.break_point = None;
    }
}

#[derive(Clone)]
pub struct EmulatorConfig {
    collisions: bool,
    wall_size: (i32, i32),
    scale: f32,
}

impl EmulatorConfig {
    fn is_multi(&self) -> bool {
        self.wall_size != (1, 1)
    }
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            collisions: true,
            wall_size: (1, 1),
            scale: 0.5,
        }
    }
}

#[derive(Component, Default)]
pub struct CPU {
    cpu: MOS6502,
}

#[derive(Component, Default)]
pub struct AtariSlot(pub i32);

#[derive(Bundle, Default)]
pub struct AtariBundle {
    slot: AtariSlot,
    system: AtariSystem,
    state: Debugger,
    cpu: CPU,
    antic_data_handle: Handle<AnticData>,
    texture: Handle<Image>,
}

fn gunzip(data: &[u8]) -> Vec<u8> {
    let mut decoder = flate2::read::GzDecoder::new(&data[..]);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result).unwrap();
    result
}

fn atari_system(
    mut query: Query<(
        Option<&Focused>,
        &mut AtariSystem,
        &mut CPU,
        &mut Debugger,
        &Handle<AnticData>,
    )>,
    mut display_config: ResMut<DisplayConfig>,
    mut antic_data_assets: ResMut<Assets<AnticData>>,
    keyboard: Res<Input<KeyCode>>,
    render_device: Res<RenderDevice>,
) {
    for (focused, mut atari_system, mut cpu, mut debugger, antic_data_handle) in query.iter_mut() {
        let mut cpu = &mut cpu.cpu;
        if debugger.paused {
            continue;
        }
        let mut prev_pc = 0;
        let antic_data = antic_data_assets.get_mut(antic_data_handle).unwrap();

        if let Some(ref collisions_data) = antic_data.collisions_data {
            collisions_data.read_collisions(&*render_device);
        }

        loop {
            if (atari_system.antic.scan_line, atari_system.antic.cycle) == (0, 0) {
                if focused.is_some() && atari_system.handle_keyboard(&keyboard, &mut cpu) {
                    cpu.interrupt_request();
                }
            };

            antic::tick(&mut *atari_system, &mut cpu, &mut *antic_data);

            match cpu.get_program_counter() {
                0xe459 => sio::sioint_hook(&mut *atari_system, &mut *cpu),
                _ => (),
            }

            if debugger.paused {
                continue;
            }

            cpu.cycle(&mut *atari_system);

            if cpu.get_remaining_cycles() == 0 {
                antic::post_instr_tick(&mut *atari_system, &antic_data.collisions_data);
                match debugger.break_point {
                    Some(BreakPoint::PC(pc)) => {
                        if cpu.get_program_counter() == pc {
                            debugger.clear_break_point();
                            display_config.debug = true;
                        }
                    }
                    Some(BreakPoint::NotPC(pc)) => {
                        if cpu.get_program_counter() != pc {
                            debugger.clear_break_point();
                            display_config.debug = true;
                        }
                    }
                    Some(BreakPoint::IndirectPC(addr)) => {
                        let pc = atari_system.read(addr) as u16
                            + atari_system.read(addr + 1) as u16 * 256;
                        if prev_pc != pc {
                            prev_pc = pc;
                            info!("run addr: {:x?}", pc);
                        }
                        if cpu.get_program_counter() == pc {
                            // frame.clear_break_point();
                            debugger.paused = true;
                            display_config.debug = true;
                        }
                    }
                    _ => (),
                }
            }
            atari_system.inc_cycle();
            if atari_system.antic.cycle == 0 {
                if let Some(BreakPoint::ScanLine(scan_line)) = &debugger.break_point {
                    if *scan_line == atari_system.antic.scan_line {
                        debugger.paused = true;
                        debugger.break_point = None;
                        display_config.debug = true;
                        break;
                    }
                }
                if atari_system.antic.scan_line == 248 {
                    atari_system.pokey.send_regs();
                    break;
                }
            }
            if debugger.paused && !atari_system.antic.wsync() {
                break;
            }
        }
    }
}

// pub const SCANLINE_MESH_HANDLE: HandleUntyped =
//     HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6039053558161382807);

fn events(
    mut query: Query<(&AtariSlot, &mut AtariSystem, &mut CPU, &mut Debugger)>,
    mut state: ResMut<State<EmulatorState>>,
) {
    let mut _messages = js_api::MESSAGES.write();
    for (atari_slot, mut atari_system, mut cpu, mut debugger) in query.iter_mut() {
        let mut messages = (*_messages).clone();
        for event in messages.drain(..) {
            match event {
                js_api::Message::Reset {
                    cold,
                    disable_basic,
                } => {
                    atari_system.reset(&mut cpu.cpu, cold, disable_basic);
                }
                js_api::Message::SetState(new_state) => {
                    let result = match new_state.as_ref() {
                        "running" => state.set(EmulatorState::Running),
                        "idle" => state.set(EmulatorState::Idle),
                        _ => panic!("invalid state requested"),
                    }
                    .ok()
                    .is_some();
                    info!("set_state: {:?}: {:?}", new_state, result);
                }
                js_api::Message::JoyState { port, dirs, fire } => {
                    atari_system.set_joystick(1, port, dirs, fire)
                }
                js_api::Message::SetConsol { state } => {
                    atari_system.update_consol(1, state);
                }
                js_api::Message::BinaryData {
                    key, data, slot, ..
                } => {
                    if slot.is_none() || Some(atari_slot.0) == slot {
                        match key.as_str() {
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
                                    a800_state.reload(&mut *atari_system, &mut cpu.cpu);
                                    // *frame = FrameState::default();
                                    state.set(EmulatorState::Running).ok();
                                    info!("LOADED! {:?}", *state);
                                }
                            }
                            _ => {
                                warn!("unknown binary");
                            }
                        }
                    }
                }
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
                                cpu.cpu.set_program_counter(pc)
                            }
                        }
                        "brk" => {
                            if let Ok(pc) = u16::from_str_radix(parts[1], 16) {
                                debugger.break_point = Some(BreakPoint::PC(pc));
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
    _messages.clear();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut antic_data_assets: ResMut<Assets<AnticData>>,
    render_device: Res<RenderDevice>,
    config: Res<EmulatorConfig>,
    mut egui_context: ResMut<EguiContext>,
) {
    for y in 0..config.wall_size.1 {
        for x in 0..config.wall_size.0 {
            let slot = y * config.wall_size.0 + x;

            let main_image_handle = bevy_atari_antic::create_main_image(&mut *images);
            let antic_data =
                AnticData::new(&render_device, main_image_handle.clone(), config.collisions);
            let antic_data_handle = antic_data_assets.add(antic_data);
            egui_context.set_egui_texture(slot as u64, main_image_handle.clone());
            let mut atari_bundle = AtariBundle {
                slot: AtariSlot(slot),
                antic_data_handle,
                ..Default::default()
            };
            atari_bundle.system.pokey.mute(config.is_multi());
            atari_bundle
                .system
                .reset(&mut atari_bundle.cpu.cpu, true, true);
            let mut entity_commands = commands.spawn();
            entity_commands
                .insert_bundle(atari_bundle)
                .insert_bundle(PipelinedSpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(0.5, 0.5, 0.5, 1.0),
                        ..Default::default()
                    },
                    texture: main_image_handle,
                    transform: Transform {
                        translation: Vec3::new(
                            -400.0 / 2.0 * (config.wall_size.0 - 1) as f32 + (400 * x) as f32,
                            -(-256.0 / 2.0 * (config.wall_size.1 - 1) as f32 + (256 * y) as f32),
                            0.0,
                        ),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            if !config.is_multi() {
                entity_commands.insert(Focused);
            }
        }
    }

    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.transform.scale = Vec3::new(1.0 / config.scale, 1.0 / config.scale, 1.0);
    camera_bundle.transform.translation = Vec3::new(0.0, 0.0, 0.0);
    commands.spawn_bundle(camera_bundle);

    // atari_bundle.state.break_point = Some(BreakPoint::IndirectPC(0x2e0));
    // atari_bundle.state.break_point = Some(BreakPoint::PC(0x7100));
}

fn main() {
    let config = EmulatorConfig {
        collisions: true,
        wall_size: (1, 1),
        scale: 4.0,
    };
    let window_size = (if !config.is_multi() {
        Vec2::new(384.0, 240.0)
    } else {
        Vec2::new(
            400.0 * config.wall_size.0 as f32,
            256.0 * config.wall_size.1 as f32,
        )
    }) * config.scale;

    let mut log_filter = "wgpu=warn".to_string();
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
    app.insert_resource(config.clone());
    app.insert_resource(Msaa { samples: 1 });
    app.insert_resource(WindowDescriptor {
        title: "GoodEnoughAtariEmulator".to_string(),
        width: window_size.x,
        height: window_size.y,
        resizable: false,
        scale_factor_override: Some(1.0),
        mode: bevy::window::WindowMode::Windowed,
        #[cfg(target_arch = "wasm32")]
        canvas: Some("#bevy-canvas".to_string()),
        vsync: true,
        ..Default::default()
    });

    app.add_plugins(PipelinedDefaultPlugins);
    app.add_plugin(EguiPlugin);
    app.add_plugin(AtariAnticPlugin {
        collisions: config.collisions,
    });
    app.add_plugin(time_used_plugin::TimeUsedPlugin);
    app.insert_resource(WinitConfig {
        force_fps: Some(50.0),
        ..Default::default()
    });

    app.add_plugin(FrameTimeDiagnosticsPlugin::default());
    // app.add_plugin(LogDiagnosticsPlugin::default());

    if config.is_multi() {
        app.add_system(focus::update.system())
            .add_startup_system(focus::setup.system());
    }

    app
        .insert_resource(DisplayConfig {
            fps: true,
            debug: false,
        })
        .add_startup_system(setup)
        // .add_startup_system(debug::setup.system())
        .add_state(EmulatorState::Running)
        .add_system_to_stage(CoreStage::PreUpdate, gamepad::update.system())
        .add_system_set(
            SystemSet::on_update(EmulatorState::Running)
                .with_system(atari_system.system().label("run_atari"))
                // .with_system(debug::debug_overlay_system.system().after("run_atari"))
                // .with_system(debug::update_fps.system()),
        )
        // .add_system(debug::update_display_config.system())
        .add_system(events.system())
        .add_system(debug::frame_stats.system())
        .add_system(debug::regs.system())
        .run();
}
