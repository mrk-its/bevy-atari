#[macro_use]
extern crate bitflags;
use std::io::prelude::*;

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
use antic::{create_mode_line, AnticResources, ModeLineDescr, DMACTL, MODE_OPTS, NMIEN};
use atari800_state::{Atari800StateLoader, StateFile};
use bevy::reflect::TypeUuid;
use bevy::{
    prelude::*,
    render::{
        camera::{OrthographicProjection, WindowOrigin},
        entity::Camera2dBundle,
        mesh::shape,
        pass::ClearColor,
        pipeline::{CullMode, PipelineDescriptor},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph, RenderResourcesNode},
        shader::{ShaderStage, ShaderStages},
    },
};

use bevy::{render::pipeline::RenderPipeline, winit::WinitConfig};
use emulator_6502::MOS6502;
use render_resources::{AnticLine, AtariPalette, Charset};
use system::AtariSystem;
use wasm_bindgen::prelude::*;

const SCAN_LINE_CYCLES: usize = 114;
const PAL_SCAN_LINES: usize = 312;
#[allow(dead_code)]
const NTSC_SCAN_LINES: usize = 262;

const MAX_SCAN_LINES: usize = PAL_SCAN_LINES;

const VERTEX_SHADER: &str = include_str!("shaders/antic.vert");
const FRAGMENT_SHADER: &str = include_str!("shaders/antic.frag");

#[derive(Debug, Default)]
struct PerfMetrics {
    frame_cnt: usize,
    cpu_cycle_cnt: usize,
}

#[derive(Default)]
struct State {
    requested_file: String,
    handle: Handle<StateFile>,
    initialized: bool,
}
#[derive(Debug)]
enum BreakPoint {
    PC(u16),
    NotPC(u16),
    ScanLine(usize),
}

#[derive(Default, Debug)]
struct FrameState {
    scan_line: usize,
    cycle: usize,
    vblank: bool,
    is_visible: bool,
    wsync: bool,
    visible_cycle: usize,
    dma_cycles: usize,
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
    timer: Timer
}

fn keyboard_system(
    time: Res<Time>,
    keyboard: Res<Input<KeyCode>>,
    mut autorepeat_disabled: Local<AutoRepeatTimer>,
    mut frame: ResMut<FrameState>,
    mut atari_system: ResMut<AtariSystem>,
    mut cpu: ResMut<MOS6502>,
) {
    let handled = if autorepeat_disabled.timer.finished() {
        let mut handled = true;
        if keyboard.pressed(KeyCode::F9) {
            if !frame.paused {
                frame.set_breakpoint(BreakPoint::ScanLine(248))
            } else {
                frame.break_point = None;
                frame.paused = false;
            }
        } else if keyboard.pressed(KeyCode::F10) {
            info!("timer: {:?}",autorepeat_disabled.timer);
            let next_scan_line = (frame.scan_line + 1) % MAX_SCAN_LINES;
            frame.set_breakpoint(BreakPoint::ScanLine(next_scan_line));
        } else if keyboard.pressed(KeyCode::F11) {
            if atari_system.ram[cpu.program_counter as usize] == 0x20 {
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
    if !handled && atari_system.handle_keyboard(&keyboard) {
        cpu.interrupt_request();
    }
}

fn reload_system(
    mut state: ResMut<State>,
    state_files: ResMut<Assets<StateFile>>,
    asset_server: Res<AssetServer>,
    mut frame: ResMut<FrameState>,
    mut atari_system: ResMut<AtariSystem>,
    mut cpu: ResMut<MOS6502>,
) {
    let requested_file = get_fragment().unwrap_or("laserdemo".to_string());
    if requested_file != state.requested_file {
        state.requested_file = requested_file;
        state.initialized = false;
        let file_name = format!("{}.state", state.requested_file);
        state.handle = asset_server.load(file_name.as_str());
    }
    if !state.initialized {
        if let Some(state_file) = state_files.get(&state.handle) {
            let data = gunzip(&state_file.data);
            let a800_state = atari800_state::Atari800State::new(&data);
            a800_state.reload(&mut *atari_system, &mut *cpu);
            let data = &atari_system.ram[0xe000..0xe400];
            info!("charset data: {:x?}", data);

            *frame = FrameState::default();
            frame.scan_line = 248;
            state.initialized = true;
        }
    }
    {
        let mut guard = js_api::ARRAY.write();
        for event in guard.drain(..) {
            match event {
                js_api::Message::JoyState {
                    port,
                    up,
                    down,
                    left,
                    right,
                    fire,
                } => atari_system.set_joystick(port, up, down, left, right, fire),
                js_api::Message::DraggedFileData { data } => {
                    let data = gunzip(&data);
                    let state = atari800_state::Atari800State::new(&data);
                    state.reload(&mut *atari_system, &mut *cpu);
                    *frame = FrameState::default();
                    frame.scan_line = 248;
                }
            }
        }
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
    atari_system: Res<AtariSystem>,
    mut text_areas: Query<&mut atari_text::TextArea>,
    mut scan_line: Query<(&ScanLine, &mut GlobalTransform)>,
    mut frame: ResMut<FrameState>,
    cpu: ResMut<MOS6502>,
) {
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
            frame.scan_line, frame.cycle,
        ),
        false,
    ));
    data.extend(&[0; 18]);
    let pc = cpu.program_counter;
    let bytes = &atari_system.ram[(pc as usize)..(pc + 48) as usize];
    if let Ok(instructions) = disasm6502::from_addr_array(bytes, pc) {
        for i in instructions.iter().take(16) {
            let line = format!(" {:04x} {:11} ", i.address, i.as_str());
            data.extend(atascii_to_screen(&line, i.address == pc));
        }
    }
    for mut text in text_areas.iter_mut() {
        &text.data.data[..data.len()].copy_from_slice(&data);
        text.charset = Charset::new(&atari_system.ram[0xe000..0xe400])
    }
    for (_, mut transform) in scan_line.iter_mut() {
        *transform =
            GlobalTransform::from_translation(Vec3::new(0.0, 128.0 - frame.scan_line as f32, 0.1));
    }
}

fn atari_system(
    commands: &mut Commands,
    state: ResMut<State>,
    antic_resources: ResMut<AnticResources>,
    antic_lines: Query<(Entity, &AnticLine)>,
    mut frame: ResMut<FrameState>,
    mut cpu: ResMut<MOS6502>,
    mut atari_system: ResMut<AtariSystem>,
    mut perf_metrics: Local<PerfMetrics>,
) {
    if !state.initialized {
        return;
    }
    if frame.paused {
        return;
    }
    let enable_log = false;
    atari_system.enable_log(enable_log);

    if frame.scan_line == 0 {
        frame.vblank = false;
    }

    if frame.scan_line == 0 && frame.cycle == 0 {
        for (entity, antic_line) in antic_lines.iter() {
            commands.despawn(entity);
        }
    }
    // for (entity, antic_line) in antic_lines.iter() {
    //     if frame.scan_line >= antic_line.start_scan_line && frame.scan_line < antic_line.end_scan_line {
    //         commands.despawn(entity);
    //     }
    // }

    'outer: loop {
        atari_system.antic.scan_line = frame.scan_line;

        if frame.cycle == 0 {
            if !frame.vblank {
                let next_scan_line = frame
                    .current_mode
                    .as_ref()
                    .map(|m| m.next_mode_line())
                    .unwrap_or(8);
                if atari_system.antic.dmactl.contains(DMACTL::DLIST_DMA)
                    && frame.scan_line == next_scan_line
                {
                    let dlist_data = atari_system.antic.prefetch_dlist(&atari_system.ram);
                    let mode = atari_system
                        .antic
                        .create_next_mode_line(&dlist_data, next_scan_line);
                    if let Some(mode_line) = mode {
                        let prev_mode_line = frame.current_mode.replace(mode_line);
                        if let Some(prev_mode_line) = prev_mode_line {
                            create_mode_line(
                                commands,
                                &antic_resources,
                                &prev_mode_line,
                                &atari_system,
                                0.0,
                                enable_log,
                            );
                        }
                    // y_extra_offset += 1.0;
                    } else {
                        frame.vblank = true;
                        let prev_mode_line = frame.current_mode.take();
                        if let Some(prev_mode_line) = prev_mode_line {
                            create_mode_line(
                                commands,
                                &antic_resources,
                                &prev_mode_line,
                                &atari_system,
                                0.0,
                                enable_log,
                            );
                        }
                    }
                }
            }

            if frame.scan_line == 248 {
                frame.vblank = true;
                if atari_system.antic.nmien.contains(NMIEN::VBI) {
                    if enable_log {
                        info!("VBI, scanline: {}", frame.scan_line);
                    }
                    atari_system.antic.set_vbi();
                    cpu.non_maskable_interrupt_request();
                }
            } else if atari_system.antic.nmien.contains(NMIEN::DLI) {
                if let Some(mode_line) = &frame.current_mode {
                    if mode_line.opts.contains(MODE_OPTS::DLI)
                        && frame.scan_line == (mode_line.next_mode_line() - 1)
                    {
                        if enable_log {
                            info!("DLI, scanline: {}", frame.scan_line);
                        }
                        atari_system.antic.set_dli();
                        cpu.non_maskable_interrupt_request();
                    }
                }
            }
        }
        if frame.wsync {
            frame.wsync = false;
            frame.cycle = 104;
        } else if frame.cycle == 0 {
            let (start_dma_cycles, line_start_cycle, dma_cycles) =
                if let Some(current_mode) = &frame.current_mode {
                    atari_system.antic.get_dma_cycles(current_mode)
                } else {
                    (0, 0, 0)
                };
            frame.cycle = start_dma_cycles;
            frame.visible_cycle = line_start_cycle;
            frame.dma_cycles = dma_cycles;
        }

        // TODO
        // if vblank {
        //     assert!(current_mode.is_none());
        //     assert!(dma_cycles == 0);
        // }

        loop {
            // if n == 110 {
            //     atari_system.antic.scan_line = scan_line + 1;
            // }

            if frame.cycle >= frame.visible_cycle && !frame.is_visible {
                let current_scan_line = frame.scan_line;
                if let Some(current_line) = &mut frame.current_mode {
                    let k = (current_scan_line - current_line.scan_line).min(7);
                    current_line.gtia_regs_array.regs[k] = atari_system.gtia.get_colors();
                    frame.is_visible = true;
                }
            }

            if frame.cycle == frame.visible_cycle {
                frame.cycle += frame.dma_cycles;
            }

            cpu.cycle(&mut *atari_system);
            perf_metrics.cpu_cycle_cnt += 1;
            atari_system.tick();
            if atari_system.antic.wsync() {
                if enable_log {
                    warn!("WSYNC, cycle: {}", frame.cycle);
                }
                if frame.cycle < 104 {
                    frame.cycle = 104;
                } else {
                    frame.wsync = true;
                    frame.cycle = 0;
                    break;
                }
            }
            if let Some(BreakPoint::PC(pc)) = frame.break_point {
                if cpu.program_counter == pc {
                    frame.clear_break_point();
                }
            }
            if let Some(BreakPoint::NotPC(pc)) = frame.break_point {
                if cpu.program_counter != pc {
                    frame.clear_break_point();
                }
            }

            frame.cycle = (frame.cycle + 1) % SCAN_LINE_CYCLES;
            if frame.cycle == 0 {
                frame.is_visible = false;
                frame.scan_line = (frame.scan_line + 1) % MAX_SCAN_LINES;
            } else if frame.cycle >= 110 {
                atari_system.antic.scan_line = frame.scan_line + 1;
            }
            if frame.paused {
                break 'outer;
            } else if frame.cycle == 0 {
                break;
            }
        }

        if let Some(BreakPoint::ScanLine(scan_line)) = &frame.break_point {
            if *scan_line == frame.scan_line {
                frame.paused = true;
                frame.break_point = None;
                break;
            }
        }
        if frame.scan_line == 0 {
            break;
        }
    }
    perf_metrics.frame_cnt += 1;
}

pub fn get_fragment() -> Result<String, JsValue> {
    let win = web_sys::window().unwrap();
    let loc = win.location();
    let v = loc.hash()?;
    if v == "" {
        return Err(JsValue::NULL);
    }
    Ok(v[1..].to_string())
}

pub struct ScanLine;
pub const SCANLINE_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6039053558161382807);

fn setup(
    commands: &mut Commands,
    mut antic_resources: ResMut<AnticResources>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut palettes: ResMut<Assets<AtariPalette>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
        descr.cull_mode = CullMode::None;
    }

    // Create a new shader pipeline
    antic_resources.pipeline_handle = pipelines.add(pipeline_descr);

    atari_text::create_atari_text_pipeline(&mut *render_graph, &mut *shaders, &mut pipelines);

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind AnticCharset resources to our shader
    render_graph.add_system_node(
        "atari_palette",
        AssetRenderResourcesNode::<AtariPalette>::new(false),
    );
    render_graph
        .add_node_edge("atari_palette", base::node::MAIN_PASS)
        .unwrap();

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind AnticLine resources to our shader
    render_graph.add_system_node("antic_line", RenderResourcesNode::<AnticLine>::new(true));

    // Add a Render Graph edge connecting our new "antic_line" node to the main pass node. This ensures "antic_line" runs before the main pass
    render_graph
        .add_node_edge("antic_line", base::node::MAIN_PASS)
        .unwrap();



    antic_resources.palette_handle = palettes.add(AtariPalette::default());
    antic_resources.mesh_handle = meshes.add(Mesh::from(shape::Quad {
        size: Vec2::new(1.0, 1.0),
        flip: false,
    }));

    let scan_line_mesh_handle = meshes.add(Mesh::from(shape::Quad {
        size: Vec2::new(384.0, 1.0),
        flip: false,
    }));

    let red_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgba(1.0, 0.0, 0.0, 1.0),
        albedo_texture: None,
        shaded: false,
    });

    commands
        .spawn(PbrBundle {
            mesh: scan_line_mesh_handle.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            material: red_material_handle,
            ..Default::default()
        })
        .with(ScanLine);

    let width = 18.0;
    let height = 20.0;

    commands
        .spawn(MeshBundle {
            mesh: antic_resources.mesh_handle.clone(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                atari_text::ATARI_TEXT_PIPELINE_HANDLE.typed(),
            )]),
            transform: Transform::from_translation(Vec3::new(
                192.0 - width * 4.0 / 2.0,
                128.0 - height * 4.0 / 2.0,
                0.2,
            ))
            .mul_transform(Transform::from_scale(Vec3::new(
                1.0 * width * 4.0,
                1.0 * height * 4.0,
                1.0,
            ))),
            ..Default::default()
        })
        .with(atari_text::TextArea {
            width,
            height,
            fg_color: Color::rgba_u8(0x00, 0xff, 0, 0xff),
            bg_color: Color::rgba_u8(0x00, 0xff, 0, 0x3f),
            data: atari_text::TextAreaData { data: [0; 1024] },
            charset: Charset { data: [0x0; 1024] },
        });

    // Setup our world
    // commands.spawn(Camera3dBundle {
    //     transform: Transform::from_translation(Vec3::new(-10.0 * 8.0, 0.0 * 8.0, 40.0 * 8.0))
    //         .looking_at(Vec3::new(-2.0 * 8.0, -0.0 * 8.0, 0.0), Vec3::unit_y()),
    //     ..Default::default()
    // });

    commands.spawn(Camera2dBundle {
        orthographic_projection: OrthographicProjection {
            bottom: 0.0,
            top: 2.0 * 240.0,
            left: 0.0,
            right: 2.0 * 384.0,
            window_origin: WindowOrigin::Center,
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::default())
            .mul_transform(Transform::from_scale(Vec3::new(1.0 / 2.0, 1.0 / 2.0, 1.0))),
        ..Default::default()
    });
}

/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    let mut app = App::build();
    app.add_resource(WindowDescriptor {
        title: "GoodEnoughAtariEmulator".to_string(),
        width: 384.0 * 2.0,
        height: 256.0 * 2.0,
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

    // app.add_stage_before("UPDATE", "pre_update", SystemStage::parallel());

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_asset::<AnticLine>()
        .add_asset::<AtariPalette>()
        .add_asset::<StandardMaterial>()
        .add_asset::<StateFile>()
        .init_asset_loader::<Atari800StateLoader>()
        .add_resource(State::default())
        .add_resource(ClearColor(gtia::atari_color(0)))
        .add_resource(AtariSystem::new())
        .add_resource(MOS6502::default())
        .add_resource(AnticResources::default())
        .add_resource(FrameState::default())
        .add_startup_system(setup.system())
        .add_system_to_stage("pre_update", keyboard_system.system())
        .add_system_to_stage("pre_update", reload_system.system())
        .add_system_to_stage("post_update", debug_overlay_system.system())
        .add_system(atari_system.system())
        .run();
}
