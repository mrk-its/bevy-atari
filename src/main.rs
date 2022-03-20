#[macro_use]
extern crate bitflags;
use std::time::Duration;
pub mod antic;
mod atari800_state;
// pub mod atari_text;
pub mod atr;
mod cartridge;
pub mod gamepad;
pub mod gdb;
pub mod gtia;
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

pub mod sio;
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
    mut query: Query<(&mut Debugger, &CPU, &mut AtariSystem), With<Focused>>,
    mut auto_repeat: Local<KeyAutoRepeater>,
    keyboard: Res<Input<KeyCode>>,
) {
    if let Some((mut debugger, cpu, mut system)) = query.iter_mut().next() {
        for key_code in auto_repeat.pressed(&keyboard) {
            match key_code {
                KeyCode::F6 => debugger.paused = !debugger.paused,
                KeyCode::F7 => debugger.step_into(),
                KeyCode::F8 => debugger.step_over(&mut system, &cpu.cpu),
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
    }
}

// function xex2atr(data) {
//     let n_sectors = Math.floor((data.length + 127) / 128) + 3;
//     let size = n_sectors * 128 / 16; // size in paragraphs;
//     let size_h = Math.floor(size / 256);
//     let size_l = size % 256;
//     let atr_buf = new Uint8Array(n_sectors * 128 + 16);
//     atr_buf.set(k_file_header, 0);
//     atr_buf.set(data, k_file_header.length);
//     atr_buf[2] = size_l;
//     atr_buf[3] = size_h;
//     atr_buf[25] = data.length % 256;
//     atr_buf[26] = Math.floor(data.length / 256);
//     return atr_buf;
//   }

const K_FILE_HEADER: [u8; 400] = [
    150, 2, 96, 17, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 7, 20, 7, 76, 20, 7, 116, 137,
    0, 0, 169, 70, 141, 198, 2, 208, 254, 160, 0, 169, 107, 145, 88, 32, 217, 7, 176, 238, 32, 196,
    7, 173, 122, 8, 13, 118, 8, 208, 227, 165, 128, 141, 224, 2, 165, 129, 141, 225, 2, 169, 0,
    141, 226, 2, 141, 227, 2, 32, 235, 7, 176, 204, 160, 0, 145, 128, 165, 128, 197, 130, 208, 6,
    165, 129, 197, 131, 240, 8, 230, 128, 208, 2, 230, 129, 208, 227, 173, 118, 8, 208, 175, 173,
    226, 2, 141, 112, 7, 13, 227, 2, 240, 14, 173, 227, 2, 141, 113, 7, 32, 255, 255, 173, 122, 8,
    208, 19, 169, 0, 141, 226, 2, 141, 227, 2, 32, 174, 7, 173, 122, 8, 208, 3, 76, 60, 7, 169, 0,
    133, 128, 133, 129, 133, 130, 133, 131, 173, 224, 2, 133, 10, 133, 12, 173, 225, 2, 133, 11,
    133, 13, 169, 1, 133, 9, 169, 0, 141, 68, 2, 108, 224, 2, 32, 235, 7, 133, 128, 32, 235, 7,
    133, 129, 165, 128, 201, 255, 208, 16, 165, 129, 201, 255, 208, 10, 32, 235, 7, 133, 128, 32,
    235, 7, 133, 129, 32, 235, 7, 133, 130, 32, 235, 7, 133, 131, 96, 32, 235, 7, 201, 255, 208, 9,
    32, 235, 7, 201, 255, 208, 2, 24, 96, 56, 96, 173, 9, 7, 13, 10, 7, 13, 11, 7, 240, 121, 172,
    121, 8, 16, 80, 238, 119, 8, 208, 3, 238, 120, 8, 169, 49, 141, 0, 3, 169, 1, 141, 1, 3, 169,
    82, 141, 2, 3, 169, 64, 141, 3, 3, 169, 128, 141, 4, 3, 169, 8, 141, 5, 3, 169, 31, 141, 6, 3,
    169, 128, 141, 8, 3, 169, 0, 141, 9, 3, 173, 119, 8, 141, 10, 3, 173, 120, 8, 141, 11, 3, 32,
    89, 228, 173, 3, 3, 201, 2, 176, 34, 160, 0, 140, 121, 8, 185, 128, 8, 170, 173, 9, 7, 208, 11,
    173, 10, 7, 208, 3, 206, 11, 7, 206, 10, 7, 206, 9, 7, 238, 121, 8, 138, 24, 96, 160, 1, 140,
    118, 8, 56, 96, 160, 1, 140, 122, 8, 56, 96, 0, 3, 0, 128, 0, 0, 0, 0, 0, 0,
];

fn xex2atr(data: &[u8]) -> Vec<u8> {
    let n_sectors = (data.len() + 127) / 128 + 3;
    let size = n_sectors * 128 / 16; // size in paragraphs;
    let size_h = (size / 256) as u8;
    let size_l = (size % 256) as u8;

    let mut atr_buf = vec![0; n_sectors * 128 + 16];

    atr_buf[0..400].copy_from_slice(&K_FILE_HEADER);
    atr_buf[400..400 + data.len()].copy_from_slice(data);
    atr_buf[2] = size_l;
    atr_buf[3] = size_h;
    atr_buf[25] = (data.len() % 256) as u8;
    atr_buf[26] = (data.len() / 256) as u8;
    atr_buf
}

// pub const SCANLINE_MESH_HANDLE: HandleUntyped =
//     HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6039053558161382807);

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
            atari_system.disks[n] = data.map(|data| atr::ATR::new(path, &data));
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
    gdb_channel: ResMut<gdb::GdbChannel>,
    config: Res<EmulatorConfig>,
    #[cfg(feature = "egui")] mut egui_context: ResMut<EguiContext>,
) {
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
                debugger: Debugger {
                    gdb_sender: Some(gdb_channel.0.clone()),
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

    #[allow(unused_mut)]
    let mut log_filter = "wgpu=warn".to_string();
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
        if let Ok(Some(_log_filter)) = local_storage.get_item("log") {
            log_filter = _log_filter;
        }
    }
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)));
    app.insert_resource(UIConfig::default());
    app.insert_resource(LogSettings {
        filter: log_filter,
        level: Level::INFO,
    });
    app.insert_resource(config.clone());
    app.insert_resource(Msaa { samples: 1 });
    app.insert_resource(WindowDescriptor {
        title: WINDOW_TITLE.to_string(),
        width: window_size.x,
        height: window_size.y,
        scale_factor_override: Some(1.0),
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

    // let task_pool = app
    //     .world
    //     .get_resource::<IoTaskPool>()
    //     .expect("IoTaskPool")
    //     .0
    //     .clone();

    // app.add_event::<platform::FsEvent>();
    // app.add_system(platform::pump_fs_events);
    // app.add_system(fs_events);

    // let fs = platform::FileSystem::new(task_pool.clone());
    // fs.attach_binary("osrom", "os.rom");
    // fs.attach_binary("car", "flob.1.0.3b.car");

    // app.insert_resource(fs);

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
