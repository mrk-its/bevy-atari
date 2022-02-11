#[macro_use]
extern crate bitflags;
use std::{io::prelude::*, time::Duration};
pub mod antic;
mod atari800_state;
// pub mod atari_text;
pub mod atr;
mod cartridge;
pub mod gamepad;
pub mod gtia;
mod js_api;
pub mod multiplexer;
pub mod pia;
pub mod platform;
pub mod pokey;

pub mod resources;
#[cfg(feature = "egui")]
mod ui;
use platform::FileSystem;
use resources::UIConfig;

pub mod focus;

pub mod sio;
mod system;
pub mod time_used_plugin;
use crate::cartridge::Cartridge;

use bevy::{render::view::Visibility, utils::HashSet, window::WindowResized, tasks::IoTaskPool};
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
    step: Step,
}

impl Debugger {
    // #[allow(dead_code)]
    // fn set_breakpoint(&mut self, break_point: BreakPoint) {
    //     self.paused = false;
    //     // self.break_point = Some(break_point);
    // }
    fn pause(&mut self) {
        self.paused = true;
        // self.break_point = None;
    }
    fn step_into(&mut self) {
        self.paused = false;
        self.step = Step::Into;
    }
    fn step_over(&mut self, cpu: &MOS6502) {
        self.paused = false;
        self.step = Step::Over {
            sp: cpu.get_stack_pointer(),
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
    debugger: Debugger,
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
    mut query: Query<(&mut Debugger, &CPU, &AtariSystem), With<Focused>>,
    mut auto_repeat: Local<KeyAutoRepeater>,
    keyboard: Res<Input<KeyCode>>,
) {
    if let Some((mut debugger, cpu, system)) = query.iter_mut().next() {
        for key_code in auto_repeat.pressed(&keyboard) {
            match key_code {
                KeyCode::F6 => debugger.paused = !debugger.paused,
                KeyCode::F7 => debugger.step_into(),
                KeyCode::F8 => debugger.step_over(&cpu.cpu),
                KeyCode::F9 => debugger.next_scanline(&system.antic),
                KeyCode::F10 => debugger.next_frame(),
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
    fs: Res<FileSystem>,
    keyboard: Res<Input<KeyCode>>,
    render_device: Res<RenderDevice>,
) {
    for (focused, mut atari_system, mut cpu, mut debugger, antic_data_handle) in query.iter_mut() {
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
                break;
            }

            cpu.cycle(&mut *atari_system);
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
        atari_system.store_disks(&fs);
    }
}

// pub const SCANLINE_MESH_HANDLE: HandleUntyped =
//     HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6039053558161382807);

fn set_binary(
    atari_system: &mut AtariSystem,
    cpu: &mut CPU,
    key: &str,
    path: &str,
    data: Option<Vec<u8>>,
) {
    info!("set_binary: {} {} {:?}", key, path, data);
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
            atari_system.disks[n] = data.map(|data| atr::ATR::new(path, &data));
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

fn events(
    mut query: Query<(&AtariSlot, &mut AtariSystem, &mut CPU, &mut Debugger)>,
    mut state: ResMut<State<EmulatorState>>,
    mut windows: ResMut<Windows>,
) {
    let mut _messages = js_api::MESSAGES.write();
    for (atari_slot, mut atari_system, mut cpu, mut debugger) in query.iter_mut() {
        let mut messages = (*_messages).clone();
        for event in messages.drain(..) {
            match event {
                js_api::Message::SetResolution { width, height } => {
                    let window = windows.get_primary_mut().unwrap();
                    window.set_resolution(width, height);
                }
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
                    key,
                    data,
                    slot,
                    path,
                } => {
                    if slot.is_none() || Some(atari_slot.0) == slot {
                        set_binary(&mut atari_system, &mut cpu, &key, &path, data);
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
                                debugger.breakpoints.push(BreakPoint::PC(pc));
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
                _ => {
                    info!("not handled: {:?}", event);
                    continue;
                }
            }
        }
    }
    _messages.clear();
}

fn fs_events(
    mut query: Query<(&AtariSlot, &mut AtariSystem, &mut CPU, &mut Debugger), With<Focused>>,
    mut events: EventReader<platform::FsEvent>,
) {
    for event in events.iter() {
        for (atari_slot, mut atari_system, mut cpu, mut debugger) in query.iter_mut() {
            match event {
                platform::FsEvent::AttachBinary { key, path, data } => {
                    set_binary(&mut atari_system, &mut cpu, &key, path, Some(data.clone()));
                    // TODO - get rid of clone
                }
                _ => continue,
            }
        }
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
    fs: Res<platform::FileSystem>,
    config: Res<EmulatorConfig>,
    #[cfg(feature = "egui")] mut egui_context: ResMut<EguiContext>,
) {
    fs.attach_binary("osrom", "os.rom");
    bevy::utils::tracing::info!("HERE!!!");
    for y in 0..config.wall_size.1 {
        for x in 0..config.wall_size.0 {
            let slot = y * config.wall_size.0 + x;

            let main_image_handle = bevy_atari_antic::create_main_image(&mut *images);
            let antic_data =
                AnticData::new(&render_device, main_image_handle.clone(), config.collisions);
            let antic_data_handle = antic_data_assets.add(antic_data);
            #[cfg(feature = "egui")]
            egui_context.set_egui_texture(slot as u64, main_image_handle.clone());
            let mut atari_bundle = AtariBundle {
                slot: AtariSlot(slot),
                antic_data_handle,
                ..Default::default()
            };
            atari_bundle.system.pokey.mute(config.is_multi());
            // atari_bundle
            //     .debugger
            //     .breakpoints
            //     .push(BreakPoint::IndirectPC(0x2e0));
            // atari_bundle
            //     .debugger
            //     .breakpoints
            //     .push(BreakPoint::IndirectPC(0x2e2));
            // atari_bundle
            //     .debugger
            //     .breakpoints
            //     .push(BreakPoint::IndirectPC(0x2e2));
            atari_bundle
                .system
                .reset(&mut atari_bundle.cpu.cpu, true, true);
            let mut entity_commands = commands.spawn();
            entity_commands.insert_bundle(atari_bundle);
            if !config.is_multi() {
                entity_commands.insert(Focused);
            }
            if slot == 0 {
                let mut full_screen_sprite = SpriteBundle {
                    texture: main_image_handle,
                    visibility: Visibility { is_visible: true },
                    transform: Transform {
                        translation: Vec3::new(
                            -400.0 / 2.0 * (config.wall_size.0 - 1) as f32 + (400 * x) as f32,
                            -(-256.0 / 2.0 * (config.wall_size.1 - 1) as f32 + (256 * y) as f32),
                            0.0,
                        ),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                full_screen_sprite.transform.scale =
                    compute_atari_screen_scale(window_descriptor.width, window_descriptor.height);
                commands.spawn_bundle(full_screen_sprite).insert(FullScreen);
            }
        }
    }

    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    // camera_bundle.transform.scale = Vec3::new(1.0 / config.scale, 1.0 / config.scale, 1.0);
    camera_bundle.transform.scale = Vec3::new(1.0, 1.0, 1.0);
    camera_bundle.transform.translation = Vec3::new(0.0, 0.0, 0.0);
    commands.spawn_bundle(camera_bundle);

    // atari_bundle.state.break_point = Some(BreakPoint::PC(0x7100));
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
    let config = EmulatorConfig {
        collisions: cfg!(any(not(target_arch = "wasm32"), feature = "webgl")),
        wall_size: (1, 1),
        scale: 2.0,
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
    app.insert_resource(UIConfig::default());
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

    app.add_plugins(DefaultPlugins);

    let task_pool = app.world.get_resource::<IoTaskPool>().expect("IoTaskPool").0.clone();

    app.insert_resource(platform::FileSystem::new(task_pool.clone()));
    app.add_event::<platform::FsEvent>();
    app.add_system(platform::pump_fs_events);
    app.add_system(fs_events);

    #[cfg(feature = "egui")]
    app.add_plugin(EguiPlugin).add_system(ui::show_ui.system());
    app.add_system(resized_events.system());

    app.add_plugin(AtariAnticPlugin {
        collisions: config.collisions,
    });

    app.add_plugin(time_used_plugin::TimeUsedPlugin);
    app.insert_resource(WinitConfig {
        // force_fps: Some(50.0),
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
        .add_system(debug_keyboard.system())

        .run();
}
