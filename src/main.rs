#[macro_use]
extern crate bitflags;
use std::time::Duration;
pub mod antic;
mod atari800_state;
// pub mod atari_text;
pub mod atr;
mod cartridge;
pub mod config;
pub mod gamepad;
pub mod gdb;
pub mod gtia;
use config::EmulatorConfig;
#[cfg(target_arch = "wasm32")]
mod js_api;
pub mod messages;
pub mod multiplexer;
pub mod pia;
pub mod platform;
pub mod pokey;

pub mod resources;
#[cfg(feature = "egui")]
mod ui;
use gdb::GdbMessage;
use platform::FileSystem;
use resources::UIConfig;

pub mod focus;

mod hooks;
mod system;
pub mod time_used_plugin;

use crate::cartridge::Cartridge;

include!(concat!(env!("OUT_DIR"), "/build_config.rs"));

#[allow(unused_imports)]
use bevy::{render::view::Visibility, tasks::IoTaskPool, window::WindowResized};
#[cfg(feature = "egui")]
use bevy_egui::{EguiContext, EguiPlugin};

#[allow(unused_imports)]
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    log::{Level, LogSettings},
    prelude::*,
    render::view::Msaa,
    winit::WinitConfig,
    DefaultPlugins,
};

use bevy::{
    render::{camera::OrthographicCameraBundle, renderer::RenderDevice, texture::Image},
    sprite::SpriteBundle,
};
use bevy_atari_antic::AtariAnticPlugin;
use emulator_6502::{Interface6502, MOS6502};
// use render::ANTIC_DATA_HANDLE;
// use render_resources::{AnticData, CustomTexture, SimpleMaterial};
use bevy_atari_antic::AnticData;
use focus::Focused;
use system::{Antic, AtariSystem};

// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum EmulatorState {
    Idle,
    Running,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakPoint {
    PC(u16),
    IndirectPC(u16),
    NotPC(u16),
    ScanLine(usize),
}

#[derive(Debug, PartialEq, Eq)]
enum Step {
    None,
    Into,
    Over { sp: u8 },
    NextScanline { scanline: usize },
    NextFrame,
}

impl Default for Step {
    fn default() -> Self {
        Step::None
    }
}

#[derive(Debug, Component, Default)]
pub struct Debugger {
    pub paused: bool,
    breakpoints: Vec<BreakPoint>,
    gdb_sender: Option<gdb::GdbSender>,
    step: Step,
}

impl Debugger {
    // #[allow(dead_code)]
    // fn set_breakpoint(&mut self, break_point: BreakPoint) {
    //     self.paused = false;
    //     // self.break_point = Some(break_point);
    // }
    fn pause(&mut self) {
        if !self.paused {
            self.paused = true;
            self.send_message(GdbMessage::Paused);
            // self.break_point = None;
        }
    }
    fn cont(&mut self) {
        self.paused = false;
    }
    fn pause_resume(&mut self) {
        if self.paused {
            self.cont()
        } else {
            self.pause()
        }
    }
    fn step_into(&mut self) {
        self.paused = false;
        self.step = Step::Into;
    }
    fn step_over(&mut self, system: &mut AtariSystem, cpu: &MOS6502) {
        self.paused = false;
        self.step = if system.read(cpu.get_program_counter()) == 0x20 {
            Step::Over {
                sp: cpu.get_stack_pointer(),
            }
        } else {
            Step::Into
        };
    }
    fn next_scanline(&mut self, antic: &Antic) {
        self.paused = false;
        self.step = Step::NextScanline {
            scanline: antic.scan_line,
        }
    }
    fn next_frame(&mut self) {
        self.paused = false;
        self.step = Step::NextFrame;
    }
    fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }
    fn add_breakpoint(&mut self, bp: BreakPoint) {
        info!("add_breakpoint {:x?}", bp);
        self.breakpoints.push(bp);
    }
    fn del_breakpoint(&mut self, bp: BreakPoint) {
        info!("del_breakpoint {:x?}", bp);
        let mut index = 0;
        while index < self.breakpoints.len() {
            if self.breakpoints[index] == bp {
                self.breakpoints.swap_remove(index);
                continue;
            }
            index += 1;
        }
    }
    fn send_message(&mut self, msg: GdbMessage) {
        if let Some(sender) = self.gdb_sender.as_ref() {
            sender.send(msg).unwrap(); // TODO
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
    debugger: Debugger,
    cpu: CPU,
    antic_data_handle: Handle<AnticData>,
    texture: Handle<Image>,
}

// fn gunzip(data: &[u8]) -> Vec<u8> {
//     let mut decoder = flate2::read::GzDecoder::new(&data[..]);
//     let mut result = Vec::new();
//     decoder.read_to_end(&mut result).unwrap();
//     result
// }

#[derive(Default)]
struct KeyAutoRepeater {
    timer: Timer,
}

impl KeyAutoRepeater {
    pub fn pressed<'a>(&mut self, input: &'a Input<KeyCode>) -> impl Iterator<Item = &'a KeyCode> {
        let just_pressed = input.get_just_pressed().collect::<Vec<_>>();
        let pressed = input.get_pressed();
        if just_pressed.len() > 0 {
            self.timer = Timer::new(Duration::from_secs_f32(0.4), false);
        }
        let finished = self.timer.finished();
        self.timer.tick(Duration::from_secs_f32(0.02));
        pressed.filter(move |v| just_pressed.contains(v) || finished)
    }
}

fn debug_keyboard(
    mut query: Query<(&mut Debugger, &mut CPU, &mut AtariSystem), With<Focused>>,
    mut auto_repeat: Local<KeyAutoRepeater>,
    keyboard: Res<Input<KeyCode>>,
    mut config: ResMut<UIConfig>,
) {
    let is_shift = keyboard.any_pressed([KeyCode::LShift, KeyCode::RShift]);
    if let Some((mut debugger, mut cpu, mut system)) = query.iter_mut().next() {
        for key_code in auto_repeat.pressed(&keyboard) {
            match key_code {
                KeyCode::F5 => system.reset(&mut cpu.cpu, false, !config.basic),
                KeyCode::F8 => debugger.paused = !debugger.paused,
                KeyCode::F10 => debugger.step_over(&mut system, &cpu.cpu),
                KeyCode::F11 => debugger.step_into(),
                KeyCode::F12 => {
                    if is_shift {
                        debugger.next_frame()
                    } else {
                        debugger.next_scanline(&system.antic)
                    }
                }
                _ => (),
            }
        }
    }
}

fn atari_system(
    mut query: Query<(
        Option<&Focused>,
        &mut AtariSystem,
        &mut CPU,
        &mut Debugger,
        &Handle<AnticData>,
    )>,
    mut antic_data_assets: ResMut<Assets<AnticData>>,
    mut keyboard: ResMut<Input<KeyCode>>,
    render_device: Res<RenderDevice>,
    config: Res<EmulatorConfig>,
) {
    for (focused, mut atari_system, mut cpu, mut debugger, antic_data_handle) in query.iter_mut() {
        atari_system.configure(&config);
        let mut cpu = &mut cpu.cpu;
        let antic_data = antic_data_assets.get_mut(antic_data_handle).unwrap();

        antic_data.config.debug_scan_line = atari_system.antic.scan_line as i32 - 8;
        if debugger.paused {
            continue;
        }

        if let Some(ref collisions_data) = antic_data.collisions_data {
            collisions_data.read_collisions(&*render_device);
        }

        loop {
            if (atari_system.antic.scan_line, atari_system.antic.cycle) == (0, 0) {
                if focused.is_some()
                    && atari_system.handle_keyboard(&mut keyboard, &mut cpu, &config)
                {
                    cpu.interrupt_request();
                }
            };

            antic::tick(&mut *atari_system, &mut cpu, &mut *antic_data);

            if debugger.paused {
                break;
            }

            cpu.cycle(&mut *atari_system);
            hooks::hook(&mut cpu, &mut atari_system);

            let finished_instr = cpu.get_remaining_cycles() == 0;
            if finished_instr {
                antic::post_instr_tick(&mut *atari_system, &antic_data.collisions_data);
            }
            atari_system.inc_cycle();

            if finished_instr {
                let mut pause = false;
                for breakpoint in &debugger.breakpoints {
                    match breakpoint {
                        BreakPoint::PC(pc) => {
                            if cpu.get_program_counter() == *pc {
                                pause = true;
                                break;
                            }
                        }
                        BreakPoint::NotPC(pc) => {
                            if cpu.get_program_counter() != *pc {
                                pause = true;
                                break;
                            }
                        }
                        BreakPoint::IndirectPC(addr) => {
                            let pc = atari_system.read(*addr) as u16
                                + atari_system.read(addr + 1) as u16 * 256;
                            if pc != 0 && cpu.get_program_counter() == pc {
                                pause = true;
                                break;
                            }
                        }
                        _ => (),
                    }
                }
                if !pause {
                    match debugger.step {
                        Step::Into => {
                            pause = true;
                            debugger.step = Step::None
                        }
                        Step::Over { sp } => {
                            if cpu.get_stack_pointer() == sp {
                                pause = true;
                                debugger.step = Step::None;
                            }
                        }
                        Step::NextScanline { scanline } => {
                            if atari_system.antic.scan_line != scanline {
                                pause = true;
                                debugger.step = Step::None;
                            }
                        }
                        _ => (),
                    }
                }
                if pause {
                    debugger.pause();
                }
            }
            if atari_system.antic.cycle == 0 {
                // if let Some(BreakPoint::ScanLine(scan_line)) = &debugger.break_point {
                //     if *scan_line == atari_system.antic.scan_line {
                //         debugger.paused = true;
                //         debugger.break_point = None;
                //         display_config.debug = true;
                //         break;
                //     }
                // }
                if atari_system.antic.scan_line == 248 {
                    atari_system.pokey.send_regs();
                    if debugger.step == Step::NextFrame {
                        debugger.step = Step::None;
                        debugger.pause();
                    }
                    break;
                }
            }
            if debugger.paused && !atari_system.antic.wsync() {
                break;
            }
        }
    }
}

const XEX_LOADER: &[u8; 144] = include_bytes!("../xex_loader/xex_loader.atr");

fn xex2atr(data: &[u8]) -> Vec<u8> {
    let n_sectors = (data.len() + 127) / 128 + 1;
    let size = n_sectors * 128 / 16; // size in paragraphs;
    let size_h = (size / 256) as u8;
    let size_l = (size % 256) as u8;

    let mut atr_buf = vec![0; n_sectors * 128 + 16];

    atr_buf[0..144].copy_from_slice(XEX_LOADER);
    atr_buf[144..144 + data.len()].copy_from_slice(data);
    atr_buf[2] = size_l;
    atr_buf[3] = size_h;

    // the last 6 bytes of sector have special meaning
    // first 3 is lenght of xex file
    // remaining ones is xex reading offset, zeroed on start
    atr_buf[144 - 6] = (data.len() & 0xff) as u8;
    atr_buf[144 - 5] = ((data.len() >> 8) & 0xff) as u8;
    atr_buf[144 - 4] = ((data.len() >> 16) & 0xff) as u8;
    atr_buf
}

fn set_binary(
    atari_system: &mut AtariSystem,
    _cpu: &mut CPU,
    key: &str,
    path: &str,
    data: Option<&[u8]>,
) {
    match key {
        "basic" => {
            atari_system.set_basic(data);
        }
        "osrom" => {
            info!("loading osrom, len: {:?}", data.as_ref().map(|v| v.len()));
            atari_system.set_osrom(data);
        }
        "disk_1" | "disk_2" | "disk_3" | "disk_4" => {
            let n = (key.bytes().nth(5).unwrap() - 48 - 1) as usize;
            atari_system.set_disk(n, data.map(|data| atr::ATR::new(path, &data)));
        }
        "xex" => {
            let data = data.map(xex2atr);
            set_binary(
                atari_system,
                _cpu,
                "disk_1",
                path,
                data.as_ref().map(|v| v.as_slice()),
            );
        }
        "car" => {
            atari_system.set_cart(
                data.map(|data| <dyn Cartridge>::from_bytes(&data).ok())
                    .flatten(),
            );
        }
        // "state" => {
        //     if let Some(data) = data {
        //         let data = gunzip(&data);
        //         let a800_state = atari800_state::Atari800State::new(&data);
        //         a800_state.reload(&mut *atari_system, &mut cpu.cpu);
        //         // *frame = FrameState::default();
        //         state.set(EmulatorState::Running).ok();
        //         info!("LOADED! {:?}", *state);
        //     }
        // }
        _ => {
            warn!("unknown binary");
        }
    }
}

#[allow(dead_code)]
fn fs_events(
    mut query: Query<(&AtariSlot, &mut AtariSystem, &mut CPU, &mut Debugger), With<Focused>>,
    mut events: EventReader<platform::FsEvent>,
    fs: Res<FileSystem>,
) {
    for (_atari_slot, mut atari_system, mut cpu, mut _debugger) in query.iter_mut() {
        for event in events.iter() {
            match event {
                platform::FsEvent::AttachBinary { key, path, data } => {
                    set_binary(&mut atari_system, &mut cpu, &key, path, Some(data));
                    atari_system.reset(&mut cpu.cpu, true, true)
                }
                _ => continue,
            }
        }
        atari_system.store_disks(&fs);
    }
}

fn compute_atari_screen_scale(window_width: f32, window_height: f32) -> bevy::math::Vec3 {
    let scale = (window_width / 384.0).min(window_height / 240.0);
    Vec3::new(scale, scale, 1.0)
}

#[derive(Component)]
pub struct FullScreen;

fn setup(
    window_descriptor: Res<WindowDescriptor>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut antic_data_assets: ResMut<Assets<AnticData>>,
    render_device: Res<RenderDevice>,
    gdb_channel: Res<Option<gdb::GdbChannel>>,
    config: Res<EmulatorConfig>,
    #[cfg(feature = "egui")] mut egui_context: ResMut<EguiContext>,
) {
    let slot = 0;

    let main_image_handle = bevy_atari_antic::create_main_image(&mut *images);
    let antic_data = AnticData::new(&render_device, main_image_handle.clone(), config.collisions);
    let antic_data_handle = antic_data_assets.add(antic_data);
    #[cfg(feature = "egui")]
    egui_context.set_egui_texture(slot as u64, main_image_handle.clone());
    let mut atari_bundle = AtariBundle {
        slot: AtariSlot(slot),
        antic_data_handle,
        debugger: Debugger {
            gdb_sender: (*gdb_channel).as_ref().map(|c| c.0.clone()),
            ..Default::default()
        },
        ..Default::default()
    };
    atari_bundle.system.pokey.mute(config.is_multi());

    #[cfg(not(target_arch = "wasm32"))]
    embed_binaries(&mut atari_bundle.system, &mut atari_bundle.cpu);

    atari_bundle
        .system
        .reset(&mut atari_bundle.cpu.cpu, true, true);
    // atari_bundle.debugger.breakpoints.push(BreakPoint::IndirectPC(0x2e0));

    let mut entity_commands = commands.spawn();
    entity_commands.insert_bundle(atari_bundle);
    if !config.is_multi() {
        entity_commands.insert(Focused);
    }
    if slot == 0 {
        let mut full_screen_sprite = SpriteBundle {
            texture: main_image_handle,
            visibility: Visibility { is_visible: true },
            ..Default::default()
        };
        full_screen_sprite.transform.scale =
            compute_atari_screen_scale(window_descriptor.width, window_descriptor.height);
        commands.spawn_bundle(full_screen_sprite).insert(FullScreen);
    }

    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    // camera_bundle.transform.scale = Vec3::new(1.0 / config.scale, 1.0 / config.scale, 1.0);
    camera_bundle.transform.scale = Vec3::new(1.0, 1.0, 1.0);
    camera_bundle.transform.translation = Vec3::new(0.0, 0.0, 0.0);
    commands.spawn_bundle(camera_bundle);
}

pub fn resized_events(
    mut window_resized_events: EventReader<WindowResized>,
    mut query: Query<(&mut Transform, &mut Visibility), With<FullScreen>>,
    ui_config: Res<resources::UIConfig>,
) {
    for (mut transform, mut visibility) in query.iter_mut() {
        for event in window_resized_events.iter() {
            bevy::log::info!("window resized to {} {}", event.width, event.height);
            transform.scale = compute_atari_screen_scale(event.width, event.height);
        }
        visibility.is_visible = !ui_config.small_screen;
    }
}

// #[bevy_main]
fn main() {
    let mut app = App::new();
    app.add_plugin(config::ConfigPlugin::default());
    let config: EmulatorConfig = app.world.get_resource::<EmulatorConfig>().unwrap().clone();

    let window_size = Vec2::new(384.0, 240.0) * config.scale;

    #[allow(unused_mut)]
    let mut log_filter = "wgpu=warn".to_string();
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
        if let Ok(Some(_log_filter)) = local_storage.get_item("log") {
            log_filter = _log_filter;
        }
    }

    app.insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)));
    app.insert_resource(UIConfig {
        basic: config.basic,
        ..Default::default()
    });
    app.insert_resource(LogSettings {
        filter: log_filter,
        level: Level::INFO,
    });
    app.insert_resource(Msaa { samples: 1 });
    app.insert_resource(WindowDescriptor {
        title: WINDOW_TITLE.to_string(),
        width: window_size.x,
        height: window_size.y,
        // scale_factor_override: Some(1.0),
        // find out how to enable(?) it on mobile
        mode: bevy::window::WindowMode::Windowed,
        #[cfg(target_arch = "wasm32")]
        resizable: false,
        #[cfg(target_arch = "wasm32")]
        canvas: Some("#bevy-canvas".to_string()),
        vsync: false,
        ..Default::default()
    });

    app.add_plugins(DefaultPlugins);

    #[cfg(feature = "egui")]
    app.add_plugin(EguiPlugin).add_system(ui::show_ui.system());
    app.add_system(resized_events.system());

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

    app.add_plugin(gdb::GdbPlugin::default());

    app.add_system(messages::events.system());

    let task_pool = app
        .world
        .get_resource::<IoTaskPool>()
        .expect("IoTaskPool")
        .0
        .clone();

    app.add_event::<platform::FsEvent>();
    app.add_system(platform::pump_fs_events);
    app.add_system(fs_events);

    let fs = platform::FileSystem::new(task_pool.clone());
    // fs.attach_binary("osrom", "os.rom");
    // fs.attach_binary("car", "flob.1.0.3b.car");

    app.insert_resource(fs);

    // #[cfg(target_arch = "wasm32")]
    // app.add_system_to_stage(CoreStage::PreUpdate, local_config);

    app.add_startup_system(setup)
        // .add_startup_system(debug::setup.system())
        .add_state(EmulatorState::Running)
        .add_system_to_stage(CoreStage::PreUpdate, gamepad::update.system())
        .add_system_set(
            SystemSet::on_update(EmulatorState::Running)
                .with_system(atari_system.system().label("run_atari")),
        )
        .add_system(debug_keyboard.system())
        .run();
}
