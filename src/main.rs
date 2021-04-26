#[macro_use]
extern crate bitflags;
use std::io::prelude::*;
pub mod antic;
mod atari800_state;
pub mod atari_text;
pub mod atr;
pub mod entities;
pub mod gtia;
mod js_api;
pub mod multiplexer;
mod palette;
pub mod pia;
pub mod pokey;
pub mod render;
mod render_resources;
pub mod sio;
mod system;
pub mod time_used_plugin;
use antic::ANTIC_DATA_HANDLE;
use bevy::{log::{Level, LogSettings}, render::entity::OrthographicCameraBundle};
use bevy::utils::Duration;
use bevy::{
    core::{Time, Timer},
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    reflect::TypeUuid,
};
use bevy::{
    prelude::*,
    render::pipeline::{PipelineDescriptor, RenderPipeline},
};
#[allow(unused_imports)]
use bevy::{
    render::{mesh::shape, render_graph::base::MainPass},
    winit::WinitConfig,
};
use emulator_6502::{Interface6502, MOS6502};
use render_resources::{AnticData, AtariPalette, CustomTexture, SimpleMaterial};
use system::{
    antic::{ATARI_PALETTE_HANDLE, COLLISIONS_PIPELINE_HANDLE, DEBUG_COLLISIONS_PIPELINE_HANDLE},
    AtariSystem,
};
use time_used_plugin::TimeUsedPlugin;

pub const RED_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 11482402499638723727);

pub const ATARI_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 11482402499638723728);

pub const COLLISIONS_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(CustomTexture::TYPE_UUID, 11482402411138723729);

pub const ANTIC_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 16056864393442354012);
pub const ANTIC_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 18422387557214033949);

pub const TEST_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(SimpleMaterial::TYPE_UUID, 18422387557214033950);

pub const DATA_TEXTURE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Texture::TYPE_UUID, 18422387557214033951);

pub const COLLISION_AGG_SIZE: Option<(u32, u32)> = Some((16, 240));

#[derive(Default, Bundle)]
pub struct Parent {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(PartialEq, Copy, Clone, Default)]
pub struct DisplayConfig {
    pub fps: bool,
    pub debug: bool,
}

pub struct MainCamera;
pub struct DebugComponent;
pub struct ScanLine;
pub struct CPUDebug;
pub struct AnticDebug;
pub struct GtiaDebug;
pub struct FPS;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum EmulatorState {
    Idle,
    Running,
}

#[derive(Debug, Default)]
struct PerfMetrics {
    frame_cnt: usize,
    cpu_cycle_cnt: usize,
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
struct FrameState {
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

#[derive(Default)]
struct KeyboarSystemState {
    timer: Timer,
}

fn keyboard_system(
    time: Res<Time>,
    mut display_config: ResMut<DisplayConfig>,
    keyboard: Res<Input<KeyCode>>,
    gamepad_buttons: Res<Input<GamepadButton>>,
    axis: Res<Axis<GamepadAxis>>,
    mut state: Local<KeyboarSystemState>,
    mut frame: ResMut<FrameState>,
    mut atari_system: ResMut<AtariSystem>,
    cpu: ResMut<MOS6502>,
) {
    if state.timer.finished() {
        if keyboard.just_pressed(KeyCode::F7) {
            display_config.fps = !display_config.fps;
        } else if keyboard.just_pressed(KeyCode::F8) {
            display_config.debug = !display_config.debug;
        } else if keyboard.pressed(KeyCode::F9) {
            if !frame.paused {
                frame.set_breakpoint(BreakPoint::ScanLine(248))
            } else {
                // frame.break_point = None;
                frame.paused = false;
            }
        } else if keyboard.pressed(KeyCode::F10) {
            let next_scan_line = atari_system.antic.get_next_scanline();
            frame.set_breakpoint(BreakPoint::ScanLine(next_scan_line));
        } else if keyboard.pressed(KeyCode::F11) {
            if atari_system.read(cpu.get_program_counter()) == 0x20 {
                // JSR
                frame.set_breakpoint(BreakPoint::PC(cpu.get_program_counter() + 3));
            } else {
                frame.set_breakpoint(BreakPoint::NotPC(cpu.get_program_counter()));
            }
        } else if keyboard.pressed(KeyCode::F12) {
            frame.set_breakpoint(BreakPoint::NotPC(cpu.get_program_counter()));
        }
    }
    for _ in keyboard.get_just_pressed() {
        state.timer.set_duration(Duration::from_secs_f32(0.2));
        state.timer.set_repeating(false);
        state.timer.reset();
        break;
    }
    for _ in keyboard.get_just_released() {
        state.timer.set_duration(Duration::default());
        state.timer.reset();
        break;
    }
    state.timer.tick(time.delta());

    let mut consol = 0;
    let axis_threshold = 0.5;
    for idx in 0..2 {
        let pad = Gamepad(idx);
        let stick_x = axis
            .get(GamepadAxis(pad, GamepadAxisType::LeftStickX))
            .unwrap_or_default();
        let stick_y = axis
            .get(GamepadAxis(pad, GamepadAxisType::LeftStickY))
            .unwrap_or_default();

        let up = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::DPadUp))
            || stick_y >= axis_threshold;
        let down = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::DPadDown))
            || stick_y <= -axis_threshold;
        let left = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::DPadLeft))
            || stick_x <= -axis_threshold;
        let right = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::DPadRight))
            || stick_x >= axis_threshold;
        let dirs = up as u8 | down as u8 * 2 | left as u8 * 4 | right as u8 * 8;
        let fire = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::East))
            || gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::LeftTrigger))
            || gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::RightTrigger));

        atari_system.set_joystick(0, idx, dirs, fire);
        consol |= gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::South)) as u8
            + gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::North)) as u8 * 2
            + gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::West)) as u8 * 4;
    }
    atari_system.update_consol(1, consol);
}

struct FPSState(Timer);

impl Default for FPSState {
    fn default() -> Self {
        FPSState(Timer::new(Duration::from_secs(1), true))
    }
}

fn update_fps(
    mut state: Local<FPSState>,
    time: Res<Time>,
    mut fps_query: Query<&mut atari_text::TextArea, With<FPS>>,
    diagnostics: Res<Diagnostics>,
) {
    if state.0.tick(time.delta()).finished() {
        for mut fps in fps_query.iter_mut() {
            if let Some(ft) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
                if let Some(t) = diagnostics.get(TimeUsedPlugin::TIME_USED) {
                    if let (Some(ft), Some(t)) = (ft.average(), t.average()) {
                        fps.set_text(&format!("{:.1} {:.2}", 1.0 / ft, t / ft));
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct DisplayState {
    pub last: DisplayConfig,
}

fn update_display_config(
    mut state: Local<DisplayState>,
    config: ResMut<DisplayConfig>,
    mut q: QuerySet<(
        Query<&mut Visible, With<FPS>>,
        Query<&mut Visible, With<DebugComponent>>,
        Query<&mut Transform, With<MainCamera>>,
    )>,
) {
    for mut camera_transform in q.q2_mut().iter_mut() {
        *camera_transform = if !config.debug {
            Transform {
                scale: Vec3::new(0.5, 0.5, 1.0),
                ..Default::default()
            }
        } else {
            Transform {
                scale: Vec3::new(1.0, 1.0, 1.0),
                translation: Vec3::new(384.0 / 2.0, -240.0 / 2.0, 0.0),
                ..Default::default()
            }
        }
    }
    if *config != state.last {
        for mut v in q.q0_mut().iter_mut() {
            v.is_visible = config.fps;
        }
        for mut v in q.q1_mut().iter_mut() {
            v.is_visible = config.debug;
        }
        state.last = *config;
    }
}

fn debug_overlay_system(
    display_config: ResMut<DisplayConfig>,
    mut atari_system: ResMut<AtariSystem>,
    mut cpu_debug: Query<&mut atari_text::TextArea, With<CPUDebug>>,
    mut antic_debug: Query<&mut atari_text::TextArea, With<AnticDebug>>,
    mut gtia_debug: Query<&mut atari_text::TextArea, With<GtiaDebug>>,
    mut scan_line: Query<(&ScanLine, &mut GlobalTransform)>,
    cpu: ResMut<MOS6502>,
) {
    if !display_config.debug {
        return;
    }
    for mut text in cpu_debug.iter_mut() {
        let mut data = vec![];
        let f = cpu.get_status_register();
        data.extend(atari_text::atascii_to_screen(
            &format!(
                " A: {:02x}   X: {:02x}     Y: {:02x}   S: {:02x}     F: {}{}-{}{}{}{}{}       {:3} / {:<3}        ",
                cpu.get_accumulator(), cpu.get_x_register(), cpu.get_y_register(), cpu.get_stack_pointer(),
                if f & 0x80 > 0 {'N'} else {'-'},
                if f & 0x40 > 0 {'V'} else {'-'},
                if f & 0x10 > 0 {'B'} else {'-'},
                if f & 0x08 > 0 {'D'} else {'-'},
                if f & 0x04 > 0 {'I'} else {'-'},
                if f & 0x02 > 0 {'Z'} else {'-'},
                if f & 0x01 > 0 {'C'} else {'-'},
                atari_system.antic.scan_line, atari_system.antic.cycle,
            ),
            false,
        ));
        data.extend(&[0; 18]);
        let pc = cpu.get_program_counter();
        let mut bytes: [u8; 48] = [0; 48];
        atari_system.copy_to_slice(pc, &mut bytes);
        if let Ok(instructions) = disasm6502::from_addr_array(&bytes, pc) {
            for i in instructions.iter().take(16) {
                let line = format!(" {:04x} {:11} ", i.address, i.as_str());
                data.extend(atari_text::atascii_to_screen(&line, i.address == pc));
            }
        }
        &text.data.data[..data.len()].copy_from_slice(&data);
    }

    for mut text in antic_debug.iter_mut() {
        let status_text = format!(
            " IR: {:02x}      DMACTL: {:02x}  CHBASE: {:02x}  HSCROL: {:02x}  VSCROL: {:02x}  PMBASE: {:02x}  VCOUNT: {:02x}  NMIST:  {:02x}  NMIEN:  {:02x} ",
            atari_system.antic.ir(),
            atari_system.antic.dmactl.bits(),
            atari_system.antic.chbase,
            atari_system.antic.hscrol,
            atari_system.antic.vscrol,
            atari_system.antic.pmbase,
            atari_system.antic.vcount,
            atari_system.antic.nmist,
            atari_system.antic.nmien,
        );
        let data = atari_text::atascii_to_screen(&status_text, false);
        &text.data.data[..data.len()].copy_from_slice(&data);
    }
    for mut text in gtia_debug.iter_mut() {
        let status_text = format!(
            " COLBK:  {:02x}  COLPF0: {:02x}  COLPF1: {:02x}  COLPF2: {:02x}  COLPF3: {:02x}  PRIOR:  {:02x}  CONSOL: {:02x} ",
            atari_system.gtia.regs.colors[0] as u8,
            atari_system.gtia.regs.colors[1] as u8,
            atari_system.gtia.regs.colors[2] as u8,
            atari_system.gtia.regs.colors[3] as u8,
            atari_system.gtia.regs.colors[4] as u8,
            atari_system.gtia.regs.prior as u8,
            atari_system.gtia.consol,
        );
        let data = atari_text::atascii_to_screen(&status_text, false);
        &text.data.data[..data.len()].copy_from_slice(&data);
    }
    for (_, mut transform) in scan_line.iter_mut() {
        *transform = GlobalTransform::from_translation(Vec3::new(
            0.0,
            128.0 - atari_system.antic.scan_line as f32,
            0.1,
        ))
        .mul_transform(Transform::from_scale(Vec3::new(384.0, 1.0, 1.0)));
    }
}

pub struct AnticFrame;

fn post_running(
    mut atari_system: ResMut<AtariSystem>,
    mut atari_data_assets: ResMut<Assets<AnticData>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let antic_data = atari_data_assets.get_mut(ANTIC_DATA_HANDLE).unwrap();
    let mesh = antic_data.create_mesh();
    meshes.set(ANTIC_MESH_HANDLE, mesh);

    // let x_offs = if atari_system.readw(0x230) == 0x1348 {
    //     let a0 = atari_system.read(0xa0);
    //     let a3 = atari_system.read(0xa3);
    //     let a6 = atari_system.read(0xa6);
    //     a0 as f32 * 16.0 * 16.0 + a3 as f32 * 16.0 + 32.0 - 2.0 * a6 as f32
    // } else {
    //     -1.0
    // };
    // let y_offs = atari_system.read(0xaa) as f32;
    // let material = StandardMaterial {
    //     albedo: Color::rgb_linear(x_offs, y_offs, 0.0),
    //     ..Default::default()
    // };
    // materials.set(TEST_MATERIAL_HANDLE, material);
}

fn atari_system(
    mut display_config: ResMut<DisplayConfig>,
    mut frame: ResMut<FrameState>,
    mut cpu: ResMut<MOS6502>,
    mut atari_system: ResMut<AtariSystem>,
    mut atari_data_assets: ResMut<Assets<AnticData>>,
    keyboard: Res<Input<KeyCode>>,
) {
    if frame.paused {
        return;
    }
    let mut prev_pc = 0;
    let antic_data = atari_data_assets.get_mut(ANTIC_DATA_HANDLE).unwrap();
    // let data_texture = textures.get_mut(DATA_TEXTURE_HANDLE).unwrap();

    // materials.set_untracked(
    //     TEST_MATERIAL_HANDLE,
    //     StandardMaterial {
    //         albedo: Color::rgba(0.2, 0.2, 0.2, 0.5),
    //         albedo_texture: Some(DATA_TEXTURE_HANDLE.typed()),
    //         shaded: false,
    //         ..Default::default()
    //     },
    // );

    loop {
        if atari_system.antic.scan_line == 8 && atari_system.antic.cycle == 0 {
            antic_data.clear();
        } else if (atari_system.antic.scan_line, atari_system.antic.cycle) == (0, 0) {
            atari_system.gtia.collision_update_scanline = 0;
            if atari_system.handle_keyboard(&keyboard, &mut *cpu) {
                cpu.interrupt_request();
            }
        };

        match cpu.get_program_counter() {
            0xe459 => sio::sioint_hook(&mut *atari_system, &mut *cpu),
            _ => (),
        }

        antic::tick(
            &mut *atari_system,
            &mut *cpu,
            &mut *antic_data,
            // &mut *data_texture,
        );

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

pub const SCANLINE_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6039053558161382807);

fn events(
    mut state: ResMut<State<EmulatorState>>,
    mut atari_system: ResMut<AtariSystem>,
    mut frame: ResMut<FrameState>,
    mut cpu: ResMut<MOS6502>,
) {
    let mut guard = js_api::ARRAY.write();
    for event in guard.drain(..) {
        match event {
            js_api::Message::Reset {
                cold,
                disable_basic,
            } => {
                atari_system.reset(&mut *cpu, cold, disable_basic);
                state.set(EmulatorState::Running).ok();
            }
            js_api::Message::SetState(new_state) => {
                match new_state.as_ref() {
                    "running" => info!("{:?}", state.set(EmulatorState::Running)),
                    "idle" => {
                        state.set(EmulatorState::Idle).ok();
                    }
                    _ => panic!("invalid state requested"),
                };
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
                    atari_system.reset(&mut *cpu, true, true);
                }
                "osrom" => {
                    atari_system.set_osrom(data);
                    atari_system.reset(&mut *cpu, true, true);
                    atari_system.antic = antic::Antic::default();
                    // *frame = FrameState::default();
                    info!("RESET! {:04x}", cpu.get_program_counter());
                }
                "disk_1" => {
                    atari_system.disk_1 = data.map(|data| atr::ATR::new(&data));
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

#[allow(dead_code)]
fn animation(mut query: Query<&mut GlobalTransform, With<MainPass>>) {
    for mut transform in query.iter_mut() {
        transform.rotate(Quat::from_rotation_ypr(0.01, 0.002, 0.015));
    }
}

const ANTIC_TEXTURE_SIZE: (f32, f32) = (384.0, 240.0);

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut materials: ResMut<Assets<SimpleMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut textures: ResMut<Assets<CustomTexture>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut tex: ResMut<Assets<Texture>>,
) {
    debug!("here!");
    // let texture_handle = asset_server.load("bevy_logo_dark_big.png");
    standard_materials.set_untracked(
        RED_MATERIAL_HANDLE,
        Color::rgba(1.0, 0.0, 0.0, 1.0).into(),
    );

    // 30 * 1024 - max charset memory
    // 48 * 240 - max video memory
    // total: 42240
    // 42240 / (256 * 4 * 4) = 10.3125

    let texture = Texture::new_fill(
        Extent3d::new(256, 11, 1),
        TextureDimension::D2,
        &[0; 16],
        TextureFormat::Rgba32Uint,
    );

    tex.set_untracked(DATA_TEXTURE_HANDLE, texture);

    materials.set_untracked(
        TEST_MATERIAL_HANDLE,
        SimpleMaterial {
            base_color: Color::rgba(0.2, 0.2, 0.2, 0.5),
            base_color_texture: Some(DATA_TEXTURE_HANDLE.typed()),
        },
    );

    standard_materials.set_untracked(
        ATARI_MATERIAL_HANDLE,
        StandardMaterial {
            base_color_texture: Some(render::ANTIC_TEXTURE_HANDLE.typed()),
            unlit: true,
            ..Default::default()
        },
    );

    textures.set_untracked(
        COLLISIONS_MATERIAL_HANDLE,
        CustomTexture {
            color: Color::rgba(0.0, 1.0, 0.0, 1.0),
            texture: Some(render::COLLISIONS_TEXTURE_HANDLE.typed()),
        },
    );

    commands.spawn_bundle(entities::create_antic_camera(Vec2::new(
        ANTIC_TEXTURE_SIZE.0,
        ANTIC_TEXTURE_SIZE.1,
    )));
    if let Some((width, height)) = COLLISION_AGG_SIZE {
        commands.spawn_bundle(entities::create_collisions_camera(Vec2::new(
            width as f32,
            height as f32,
        )));

        let mesh = Mesh::from(shape::Quad::new(Vec2::new(width as f32, height as f32)));
        let mesh_handle = meshes.add(mesh);
        let bundle = entities::CollisionsAggBundle {
            mesh: mesh_handle,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                COLLISIONS_PIPELINE_HANDLE.typed(),
            )]),
            texture: COLLISIONS_MATERIAL_HANDLE.typed(),
            ..Default::default()
        };

        info!("bundle: {:?}", bundle.render_pipelines);
        commands.spawn_bundle(bundle);
    }

    let mesh_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        ANTIC_TEXTURE_SIZE.0,
        ANTIC_TEXTURE_SIZE.1,
    ))));

    commands.spawn_bundle(PbrBundle {
        mesh: mesh_handle,
        material: ATARI_MATERIAL_HANDLE.typed(),
        ..Default::default()
    });

    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
    //         50.0,
    //         50.0,
    //     )))),
    //     material: RED_MATERIAL_HANDLE.typed(),
    //     transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    //     ..Default::default()
    // });


    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.transform.scale = Vec3::new(0.5, 0.5, 1.0);
    commands.spawn_bundle(camera_bundle).insert(MainCamera);

    // commands.spawn_bundle(PerspectiveCameraBundle {
    //     transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::default(), Vec3::Y),
    //     ..Default::default()
    // });


    commands
        .spawn()
        .insert(Parent {
            transform: Transform {
                translation: Vec3::new(-384.0 / 2.0, 240.0 / 2.0, 0.0),
                scale: Vec3::new(1.0, 1.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn_bundle(atari_text::TextAreaBundle::new(10, 1, 0, 0))
                .insert(FPS);
        });

    commands
        .spawn()
        .insert(Parent {
            transform: Transform::from_translation(Vec3::new(384.0 / 2.0, 240.0 / 2.0, 0.0)),
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn_bundle(atari_text::TextAreaBundle::new(18, 20, 0, 0))
                .insert(CPUDebug)
                .insert(DebugComponent);
            commands
                .spawn_bundle(atari_text::TextAreaBundle::new(12, 20, 18 + 1, 0))
                .insert(AnticDebug)
                .insert(DebugComponent);
            commands
                .spawn_bundle(atari_text::TextAreaBundle::new(12, 20, 18 + 12 + 2, 0))
                .insert(GtiaDebug)
                .insert(DebugComponent);
        });

    commands
        .spawn()
        .insert(Parent {
            transform: Transform::from_translation(Vec3::new(0.0, -240.0, 0.0)),
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn_bundle(MeshBundle {
                    mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(384.0, 240.0)))),
                    render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                        DEBUG_COLLISIONS_PIPELINE_HANDLE.typed(),
                    )]),
                    ..Default::default()
                })
                .insert(materials.add(SimpleMaterial {
                    base_color: Color::rgba(0.0, 0.5, 0.0, 1.0),
                    base_color_texture: Some(render::COLLISIONS_TEXTURE_HANDLE.typed()),
                }))
                .insert(DebugComponent);
        });

    let bundle = MeshBundle {
        mesh: ANTIC_MESH_HANDLE.typed(),
        render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(pipelines.add(render::build_antic2_pipeline(&mut *shaders)))]),
        ..Default::default()
    };

    commands
        .spawn_bundle(bundle)
        .insert(AnticFrame)
        .insert(ATARI_PALETTE_HANDLE.typed::<AtariPalette>())
        .insert(ANTIC_DATA_HANDLE.typed::<AnticData>())
        .insert(TEST_MATERIAL_HANDLE.typed::<SimpleMaterial>())
        .remove::<MainPass>();

}
/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    let mut app = App::build();
    app.insert_resource(LogSettings {
        level: Level::DEBUG,
        ..Default::default()
    });
    app.insert_resource(Msaa { samples: 1 });
    app.insert_resource(WindowDescriptor {
        title: "GoodEnoughAtariEmulator".to_string(),
        width: ANTIC_TEXTURE_SIZE.0 * 2.0,
        height: ANTIC_TEXTURE_SIZE.1 * 2.0,
        resizable: false,
        mode: bevy::window::WindowMode::Windowed,
        #[cfg(target_arch = "wasm32")]
        canvas: Some("#bevy-canvas".to_string()),
        vsync: true,
        ..Default::default()
    });

    app.add_plugin(time_used_plugin::TimeUsedPlugin);
    // app.insert_resource(WinitConfig {
    //     force_fps: Some(50.0),
    //     return_from_run: false,
    // });
    app.add_plugins(DefaultPlugins);
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_plugin(atari_text::AtartTextPlugin::default());
    app.add_asset::<SimpleMaterial>();
    app.add_asset::<CustomTexture>();
    app.add_plugin(antic::AnticPlugin {
        texture_size: Vec2::new(ANTIC_TEXTURE_SIZE.0, ANTIC_TEXTURE_SIZE.1),
        enable_collisions: true,
        collision_agg_size: COLLISION_AGG_SIZE,
    });
    app.add_plugin(FrameTimeDiagnosticsPlugin::default());

    let mut system = AtariSystem::new();
    let mut cpu = MOS6502::default();
    system.reset(&mut cpu, true, true);

    let frame = FrameState::default();
    // frame.break_point = Some(BreakPoint::IndirectPC(0x2e0));
    // frame.break_point = Some(BreakPoint::PC(0x7100));

    app.insert_resource(ClearColor(gtia::atari_color(0)))
        .insert_resource(DisplayConfig {
            fps: false,
            debug: false,
        })
        .insert_resource(system)
        .insert_resource(cpu)
        .insert_resource(frame)
        .add_startup_system(setup.system())
        .add_state(EmulatorState::Idle)
        .add_system_to_stage(CoreStage::PreUpdate, keyboard_system.system())
        // .add_system_to_stage("pre_update", reload_system.system())
        // .add_system_to_stage(CoreStage::PostUpdate, debug_overlay_system.system())
        .add_system_set(
            SystemSet::on_update(EmulatorState::Running)
                .with_system(atari_system.system().label("run_atari"))
                .with_system(post_running.system().after("run_atari"))
                .with_system(update_fps.system())
        )
        .add_system(update_display_config.system())
        // .on_state_update("running", EmulatorState::Running, animation.system())
        .add_system(events.system())
        .run();
}
