#[macro_use]
extern crate bitflags;
use std::io::prelude::*;
pub mod atr;
pub mod sio;

pub mod antic;
mod atari800_state;
pub mod atari_text;
pub mod gtia;
mod js_api;
mod palette;
pub mod pia;
pub mod pokey;
mod render_resources;
mod system;
use antic::{create_mode_line, ModeLineDescr, SCAN_LINE_CYCLES};

use bevy::reflect::TypeUuid;
use bevy::{
    prelude::*,
    render::{camera::Camera, entity::Camera2dBundle, pipeline::PipelineDescriptor},
};
use bevy::{
    render::{camera::CameraProjection, mesh::shape, render_graph::base::MainPass},
    window::WindowId,
    winit::WinitConfig,
};
use emulator_6502::{Interface6502, MOS6502};
use render_resources::AnticLine;
use system::AtariSystem;

pub const RED_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 11482402499638723727);

pub const ATARI_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 11482402499638723728);

pub struct DebugComponent;
pub struct ScanLine;
pub struct CPUDebug;
pub struct AnticDebug;
pub struct GtiaDebug;

#[derive(Clone, Debug)]
enum EmulatorState {
    Idle,
    Running,
}

#[derive(Debug, Default)]
struct PerfMetrics {
    frame_cnt: usize,
    cpu_cycle_cnt: usize,
}

#[derive(Debug)]
enum BreakPoint {
    PC(u16),
    NotPC(u16),
    ScanLine(usize),
}

#[derive(Default, Debug)]
struct FrameState {
    is_debug: bool,
    current_mode: Option<ModeLineDescr>,
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
struct AutoRepeatTimer {
    timer: Timer,
}

fn keyboard_system(
    mut debug_components: Query<&mut Visible, With<DebugComponent>>,
    mut camera_query: Query<(&mut GlobalTransform, &Camera)>,
    time: Res<Time>,
    keyboard: Res<Input<KeyCode>>,
    mut autorepeat_disabled: Local<AutoRepeatTimer>,
    mut frame: ResMut<FrameState>,
    mut atari_system: ResMut<AtariSystem>,
    mut cpu: ResMut<MOS6502>,
) {
    let handled = if autorepeat_disabled.timer.finished() {
        let mut handled = true;
        if keyboard.just_pressed(KeyCode::F8) {
            frame.is_debug = !frame.is_debug;
            set_debug(frame.is_debug, &mut debug_components, &mut camera_query);
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
            if atari_system.read(cpu.program_counter) == 0x20 {
                // JSR
                frame.set_breakpoint(BreakPoint::PC(cpu.program_counter + 3));
            } else {
                frame.set_breakpoint(BreakPoint::NotPC(cpu.program_counter));
            }
        } else if keyboard.pressed(KeyCode::F12) {
            frame.set_breakpoint(BreakPoint::NotPC(cpu.program_counter));
        } else {
            handled = false;
        };
        handled
    } else {
        false
    };
    for _ in keyboard.get_just_pressed() {
        autorepeat_disabled.timer.set_duration(0.2);
        autorepeat_disabled.timer.set_repeating(false);
        autorepeat_disabled.timer.reset();
        break;
    }
    for _ in keyboard.get_just_released() {
        autorepeat_disabled.timer.set_duration(0.0);
        autorepeat_disabled.timer.reset();
        break;
    }
    autorepeat_disabled.timer.tick(time.delta_seconds());
    if !handled && atari_system.handle_keyboard(&keyboard, &mut *cpu) {
        cpu.interrupt_request();
    }
}

fn atascii_to_screen(text: &str, inv: bool) -> Vec<u8> {
    text.as_bytes()
        .iter()
        .map(|c| match *c {
            0x00..=0x1f => *c + 0x40,
            0x20..=0x5f => *c - 0x20,
            _ => *c,
        } + (inv as u8) * 128)
        .collect()
}

fn debug_overlay_system(
    mut atari_system: ResMut<AtariSystem>,
    mut cpu_debug: Query<&mut atari_text::TextArea, With<CPUDebug>>,
    mut antic_debug: Query<&mut atari_text::TextArea, With<AnticDebug>>,
    mut gtia_debug: Query<&mut atari_text::TextArea, With<GtiaDebug>>,
    mut scan_line: Query<(&ScanLine, &mut GlobalTransform)>,
    frame: ResMut<FrameState>,
    cpu: ResMut<MOS6502>,
) {
    if !frame.is_debug {
        return;
    }
    for mut text in cpu_debug.iter_mut() {
        let mut data = vec![];
        let f = cpu.status_register;
        data.extend(atascii_to_screen(
            &format!(
                " A: {:02x}   X: {:02x}     Y: {:02x}   S: {:02x}     F: {}{}-{}{}{}{}{}       {:3} / {:<3}        ",
                cpu.accumulator, cpu.x_register, cpu.y_register, cpu.stack_pointer,
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
        let pc = cpu.program_counter;
        let mut bytes: [u8; 48] = [0; 48];
        atari_system.copy_to_slice(pc, &mut bytes);
        if let Ok(instructions) = disasm6502::from_addr_array(&bytes, pc) {
            for i in instructions.iter().take(16) {
                let line = format!(" {:04x} {:11} ", i.address, i.as_str());
                data.extend(atascii_to_screen(&line, i.address == pc));
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
        let data = atascii_to_screen(&status_text, false);
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
        let data = atascii_to_screen(&status_text, false);
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

fn atari_system(
    commands: &mut Commands,
    antic_lines: Query<(Entity, &AnticLine)>,
    mut frame: ResMut<FrameState>,
    mut cpu: ResMut<MOS6502>,
    mut atari_system: ResMut<AtariSystem>,
    mut debug_components: Query<&mut Visible, With<DebugComponent>>,
    mut camera_query: Query<(&mut GlobalTransform, &Camera)>,
) {
    if frame.paused {
        return;
    }

    let debug_mode = frame.paused || frame.break_point.is_some();

    if !debug_mode && atari_system.antic.scan_line == 0 && atari_system.antic.cycle == 0 {
        for (entity, _) in antic_lines.iter() {
            commands.despawn(entity);
        }
    }

    loop {
        match cpu.program_counter {
            0xe459 => sio::sioint_hook(&mut *atari_system, &mut *cpu),
            _ => (),
        }
        if atari_system.antic.cycle == 0 {
            if atari_system.antic.scan_line == 0 {
                // antic reset
                atari_system.antic.next_scan_line = 8;
            }
            atari_system.scanline_tick();

            if atari_system
                .antic
                .dmactl
                .contains(antic::DMACTL::PLAYER_DMA)
            {
                if atari_system.gtia.gractl.contains(gtia::GRACTL::MISSILE_DMA) {
                    let b = antic::get_pm_data(&mut *atari_system, 0);
                    atari_system.gtia.write(gtia::GRAFM, b);
                }
                if atari_system.gtia.gractl.contains(gtia::GRACTL::PLAYER_DMA) {
                    let b = antic::get_pm_data(&mut *atari_system, 1);
                    atari_system.gtia.write(gtia::GRAFP0, b);
                    let b = antic::get_pm_data(&mut *atari_system, 2);
                    atari_system.gtia.write(gtia::GRAFP1, b);
                    let b = antic::get_pm_data(&mut *atari_system, 3);
                    atari_system.gtia.write(gtia::GRAFP2, b);
                    let b = antic::get_pm_data(&mut *atari_system, 4);
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
                if frame.paused {
                    return;
                }
            }
        }
        if atari_system.antic.fire_nmi() {
            cpu.non_maskable_interrupt_request();
        }
        if atari_system.antic.gets_visible() {
            // info!("here: {} {}", atari_system.antic.cycle, frame.visible_cycle);
            let prev_mode_line = if atari_system.antic.scan_line >= 8
                && atari_system.antic.scan_line == atari_system.antic.start_scan_line
            {
                // info!("creating mode line, cycle: {:?}", atari_system.antic.cycle);
                let mode_line = atari_system.antic.create_next_mode_line();
                let prev_mode_line = frame.current_mode.take();
                // info!("created mode_line {:?}", mode_line.as_ref().unwrap());
                frame.current_mode = Some(mode_line);
                prev_mode_line
            } else if atari_system.antic.scan_line == 248 {
                frame.current_mode.take()
            } else {
                None
            };
            if let Some(prev_mode_line) = prev_mode_line {
                if debug_mode {
                    for (entity, antic_line) in antic_lines.iter() {
                        let not_intersects = antic_line.start_scan_line
                            >= prev_mode_line.next_mode_line()
                            || prev_mode_line.scan_line >= antic_line.end_scan_line;
                        if !not_intersects {
                            commands.despawn(entity);
                        }
                    }
                }
                create_mode_line(commands, prev_mode_line, 0.0);
            }

            let current_scan_line = atari_system.antic.scan_line;
            if let Some(current_line) = &mut frame.current_mode {
                let k = (current_scan_line - current_line.scan_line).min(7);
                if current_line.gtia_regs_array.regs.len() < 8 {
                    current_line
                        .gtia_regs_array
                        .regs
                        .push(atari_system.gtia.regs);
                }
                if k == 0 {
                    let charset_offset = (current_line.chbase as usize) * 256;
                    current_line.line_data.set_data(
                        &mut atari_system,
                        current_line.data_offset,
                        current_line.n_bytes,
                    );
                    // TODO suport 512 byte charsets?
                    current_line.charset.set_data(
                        &mut atari_system,
                        charset_offset,
                        current_line.charset_size(),
                    );
                }
            }
        }

        atari_system.antic.steal_cycles();

        cpu.cycle(&mut *atari_system);

        if cpu.remaining_cycles == 0 {
            if atari_system.antic.wsync() {
                if atari_system.antic.cycle < 104 {
                    atari_system.antic.cycle = 104;
                    atari_system.antic.clear_wsync();
                } else {
                    atari_system.antic.cycle = SCAN_LINE_CYCLES - 1;
                }
            }
            match frame.break_point {
                Some(BreakPoint::PC(pc)) => {
                    if cpu.program_counter == pc {
                        frame.clear_break_point();
                        frame.is_debug = true;
                        set_debug(true, &mut debug_components, &mut camera_query);
                    }
                }
                Some(BreakPoint::NotPC(pc)) => {
                    if cpu.program_counter != pc {
                        frame.clear_break_point();
                        frame.is_debug = true;
                        set_debug(true, &mut debug_components, &mut camera_query);
                    }
                }
                _ => (),
            }
        }
        atari_system.antic.inc_cycle();
        atari_system.gtia.scan_line = atari_system.antic.scan_line;
        if atari_system.antic.cycle == 0 {
            if let Some(BreakPoint::ScanLine(scan_line)) = &frame.break_point {
                if *scan_line == atari_system.antic.scan_line {
                    frame.paused = true;
                    frame.break_point = None;
                    frame.is_debug = true;
                    set_debug(true, &mut debug_components, &mut camera_query);
                    break;
                }
            }
            if atari_system.antic.scan_line == 0 {
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
                *frame = FrameState::default();
                atari_system.antic.scan_line = 0;
                state.set_next(EmulatorState::Running).ok();
            }
            js_api::Message::SetState(new_state) => {
                match new_state.as_ref() {
                    "running" => info!("{:?}", state.set_next(EmulatorState::Running)),
                    "idle" => {
                        state.set_next(EmulatorState::Idle).ok();
                    }
                    _ => panic!("invalid state requested"),
                };
            }
            js_api::Message::JoyState {
                port,
                up,
                down,
                left,
                right,
                fire,
            } => atari_system.set_joystick(port, up, down, left, right, fire),
            js_api::Message::BinaryData { key, data, .. } => match key.as_str() {
                "basic" => {
                    atari_system.set_basic(data);
                    atari_system.reset(&mut *cpu, true, true);
                }
                "osrom" => {
                    atari_system.set_osrom(data);
                    atari_system.reset(&mut *cpu, true, true);
                    atari_system.antic = antic::Antic::default();
                    *frame = FrameState::default();
                    info!("RESET! {:04x}", cpu.program_counter);
                }
                "disk_1" => {
                    atari_system.disk_1 = data.map(|data| atr::ATR::new(&data));
                }
                "state" => {
                    if let Some(data) = data {
                        let data = gunzip(&data);
                        let a800_state = atari800_state::Atari800State::new(&data);
                        a800_state.reload(&mut *atari_system, &mut *cpu);
                        *frame = FrameState::default();
                        state.set_next(EmulatorState::Running).ok();
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
                            cpu.program_counter = pc;
                        }
                    }
                    "brk" => {
                        if let Ok(pc) = u16::from_str_radix(parts[1], 16) {
                            frame.break_point = Some(BreakPoint::PC(pc));
                            info!("breakpoint set on pc={:04x}", pc);
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}

fn set_debug(
    is_visible: bool,
    debug_components: &mut Query<&mut Visible, With<DebugComponent>>,
    camera_query: &mut Query<(&mut GlobalTransform, &Camera)>,
) {
    for mut visible in debug_components.iter_mut() {
        visible.is_visible = is_visible;
    }
    for (mut transform, camera) in camera_query.iter_mut() {
        if camera.window.is_primary() {
            if is_visible {
                *transform = GlobalTransform::from_translation(Vec3::new(384.0, -240.0, 0.0))
                    .mul_transform(Transform::from_scale(Vec3::new(2.0, 2.0, 1.0)))
            } else {
                *transform = GlobalTransform::default()
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

const ANTIC_TEXTURE_SIZE: Vec2 = Vec2 { x: 384.0, y: 240.0 };

fn setup(
    commands: &mut Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // let texture_handle = asset_server.load("bevy_logo_dark_big.png");
    materials.set(
        RED_MATERIAL_HANDLE,
        StandardMaterial {
            albedo: Color::rgba(1.0, 0.0, 0.0, 1.0),
            albedo_texture: None,
            shaded: false,
        },
    );
    // load a texture and retrieve its aspect ratio
    // let texture_handle = asset_server.load("bevy_logo_dark_big.png");

    materials.set_untracked(
        ATARI_MATERIAL_HANDLE,
        StandardMaterial {
            // albedo: Color::rgba(0.2, 0.2, 0.2, 0.5),
            albedo_texture: Some(antic::render::ANTIC_TEXTURE_HANDLE.typed()),
            shaded: false,
            ..Default::default()
        },
    );

    let mut antic_camera_bundle = Camera2dBundle {
        camera: Camera {
            name: Some(antic::render::ANTIC_CAMERA.to_string()),
            ..Default::default()
        },
        transform: Transform::from_scale(Vec3::new(1.0, -1.0, 1.0)),
        ..Default::default()
    };

    antic_camera_bundle.camera.window = WindowId::new();
    let camera_projection = &mut antic_camera_bundle.orthographic_projection;
    camera_projection.update(ANTIC_TEXTURE_SIZE.x, ANTIC_TEXTURE_SIZE.y);
    antic_camera_bundle.camera.projection_matrix = camera_projection.get_projection_matrix();
    antic_camera_bundle.camera.depth_calculation = camera_projection.depth_calculation();
    commands.spawn(antic_camera_bundle);

    let mesh_handle = meshes.add(Mesh::from(shape::Quad::new(ANTIC_TEXTURE_SIZE)));
    // let mesh_handle = meshes.add(Mesh::from(shape::Box::new(5.0, 5.0, 5.0)));

    commands.spawn(PbrBundle {
        mesh: mesh_handle,
        material: ATARI_MATERIAL_HANDLE.typed(),
        visible: Visible {
            is_visible: true,
            is_transparent: false,
        },
        transform: Transform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(2.0, 2.0, 1.0),
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn(Camera2dBundle::default());
    // commands.spawn(LightBundle {
    //     transform: Transform::from_translation(Vec3::new(-20.0, 0.0, 30.0)),
    //     ..Default::default()
    // });
    // commands.spawn(Camera3dBundle {
    //     transform: Transform::from_translation(Vec3::new(0.0 , 0.0, 10.0))
    //         .looking_at(Vec3::new(-0.0, -0.0, 0.0), Vec3::unit_y()),
    //     ..Default::default()
    // });

    // commands
    //     .spawn(PbrBundle {
    //         mesh: QUAD_HANDLE.typed(),
    //         material: RED_MATERIAL_HANDLE.typed(),
    //         visible: Visible {
    //             is_visible: false,
    //             is_transparent: false,
    //         },
    //         ..Default::default()
    //     })
    //     .with(DebugComponent)
    //     .with(ScanLine);
    // commands
    //     .spawn(atari_text::TextAreaBundle::new(
    //         18.0,
    //         20.0,
    //         (384.0 + 18.0 * 8.0) / 2.0,
    //         (256.0 - 20.0 * 8.0) / 2.0,
    //     ))
    //     .with(DebugComponent)
    //     .with(CPUDebug);
    // commands
    //     .spawn(atari_text::TextAreaBundle::new(
    //         12.0,
    //         20.0,
    //         (384.0 + 12.0 * 8.0) / 2.0 + 19.0 * 8.0,
    //         (256.0 - 20.0 * 8.0) / 2.0,
    //     ))
    //     .with(DebugComponent)
    //     .with(AnticDebug);
    // commands
    //     .spawn(atari_text::TextAreaBundle::new(
    //         12.0,
    //         20.0,
    //         (384.0 + 12.0 * 8.0) / 2.0 + (20.0 + 12.0) * 8.0,
    //         (256.0 - 20.0 * 8.0) / 2.0,
    //     ))
    //     .with(DebugComponent)
    //     .with(GtiaDebug);

    // Setup our world
}

/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    let mut app = App::build();
    app.add_resource(WindowDescriptor {
        title: "GoodEnoughAtariEmulator".to_string(),
        width: 384.0 * 2.0,
        height: 240.0 * 2.0,
        resizable: false,
        mode: bevy::window::WindowMode::Windowed,
        #[cfg(target_arch = "wasm32")]
        canvas: Some("#bevy-canvas".to_string()),
        vsync: true,
        ..Default::default()
    });
    app.add_resource(WinitConfig {
        force_fps: Some(50.0),
        return_from_run: false,
    });
    app.add_plugins(DefaultPlugins);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_plugin(atari_text::AtartTextPlugin::default());
    app.add_plugin(antic::AnticPlugin {
        texture_size: ANTIC_TEXTURE_SIZE,
    });

    let mut system = AtariSystem::new();
    let mut cpu = MOS6502::default();
    system.reset(&mut cpu, true, true);

    let frame = FrameState::default();

    app.add_resource(State::new(EmulatorState::Idle))
        // .add_resource(ClearColor(gtia::atari_color(0)))
        .add_resource(system)
        .add_resource(cpu)
        .add_resource(frame)
        .add_startup_system(setup.system())
        .add_stage_after(
            stage::UPDATE,
            "running",
            StateStage::<EmulatorState>::default(),
        )
        .add_stage_after(
            stage::UPDATE,
            "idle_update",
            StateStage::<EmulatorState>::default(),
        )
        .add_system_to_stage("pre_update", keyboard_system.system())
        // .add_system_to_stage("pre_update", reload_system.system())
        .add_system_to_stage("post_update", debug_overlay_system.system())
        .on_state_update("running", EmulatorState::Running, atari_system.system())
        // .on_state_update("running", EmulatorState::Running, animation.system())
        .add_system(events.system())
        .run();
}
