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
use antic::{create_mode_line, ModeLineDescr, MAX_SCAN_LINES, SCAN_LINE_CYCLES};
use atari800_state::{Atari800StateLoader, StateFile};
use bevy::reflect::TypeUuid;
use bevy::winit::WinitConfig;
use bevy::{
    prelude::*,
    render::{
        camera::{Camera, OrthographicProjection, WindowOrigin},
        entity::Camera2dBundle,
        pass::ClearColor,
        pipeline::{CullMode, PipelineDescriptor},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph, RenderResourcesNode},
        shader::{ShaderStage, ShaderStages},
    },
    sprite::QUAD_HANDLE,
};
use emulator_6502::MOS6502;
use render_resources::{AnticLine, AtariPalette, Charset};
use system::AtariSystem;
use wasm_bindgen::prelude::*;

const VERTEX_SHADER: &str = include_str!("shaders/antic.vert");
const FRAGMENT_SHADER: &str = include_str!("shaders/antic.frag");
pub const RED_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 11482402499638723727);

pub struct DebugComponent;
pub struct ScanLine;
pub struct CPUDebug;
pub struct AnticDebug;
pub struct GtiaDebug;

#[derive(Clone, Debug)]
enum EmulatorState {
    Loading,
    Running,
    Debugging,
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
    timer: Timer,
}

fn keyboard_system(
    mut debug_components: Query<&mut Visible, With<DebugComponent>>,
    time: Res<Time>,
    keyboard: Res<Input<KeyCode>>,
    mut autorepeat_disabled: Local<AutoRepeatTimer>,
    mut frame: ResMut<FrameState>,
    mut atari_system: ResMut<AtariSystem>,
    mut cpu: ResMut<MOS6502>,
    mut camera_query: Query<&mut GlobalTransform, With<Camera>>,
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
                frame.break_point = None;
                frame.paused = false;
            }
        } else if keyboard.pressed(KeyCode::F10) {
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
        &text.data.data[..data.len()].copy_from_slice(&data);
    }

    for mut text in antic_debug.iter_mut() {
        let status_text = format!(
            " IR: {:02x}      DMACTL: {:02x}  CHBASE: {:02x}  HSCROL: {:02x}  VSCROL: {:02x}  PMBASE: {:02x}  VCOUNT: {:02x} ",
            atari_system.antic.ir(),
            atari_system.antic.dmactl.bits(),
            atari_system.antic.chbase,
            atari_system.antic.hscrol,
            atari_system.antic.vscrol,
            atari_system.antic.pmbase,
            atari_system.antic.vcount,
        );
        let data = atascii_to_screen(&status_text, false);
        &text.data.data[..data.len()].copy_from_slice(&data);
    }
    for mut text in gtia_debug.iter_mut() {
        let status_text = format!(
            " COLBK:  {:02x}  COLPF0: {:02x}  COLPF1: {:02x}  COLPF2: {:02x}  COLPF3: {:02x}  PRIOR:  {:02x} ",
            atari_system.gtia.colbk(),
            atari_system.gtia.colpf0(),
            atari_system.gtia.colpf1(),
            atari_system.gtia.colpf2(),
            atari_system.gtia.colpf3(),
            atari_system.gtia.prior,
        );
        let data = atascii_to_screen(&status_text, false);
        &text.data.data[..data.len()].copy_from_slice(&data);
    }
    for (_, mut transform) in scan_line.iter_mut() {
        *transform =
            GlobalTransform::from_translation(Vec3::new(0.0, 128.0 - frame.scan_line as f32, 0.1))
                .mul_transform(Transform::from_scale(Vec3::new(384.0, 1.0, 1.0)));
    }
}

fn atari_system(
    commands: &mut Commands,
    antic_lines: Query<(Entity, &AnticLine)>,
    mut frame: ResMut<FrameState>,
    mut cpu: ResMut<MOS6502>,
    mut atari_system: ResMut<AtariSystem>,
) {
    if frame.paused {
        return;
    }
    if frame.scan_line == 0 {
        frame.vblank = false;
    }

    let debug_mode = frame.paused || frame.break_point.is_some();

    if !debug_mode && frame.scan_line == 0 && frame.cycle == 0 {
        for (entity, _) in antic_lines.iter() {
            commands.despawn(entity);
        }
    }

    loop {
        atari_system
            .antic
            .set_scan_line(frame.scan_line, frame.cycle);
        if frame.cycle == 0 {
            // if frame.scan_line == 8 {
            //     let offs = atari_system.antic.dlist_offset(0) as usize;
            //     info!("dlist: offs: {:04x} {:x?}", offs, &atari_system.ram[offs..offs+128]);
            // }

            if atari_system
                .antic
                .dmactl
                .contains(antic::DMACTL::PLAYER_DMA)
            {
                if atari_system.gtia.gractl.contains(gtia::GRACTL::MISSILE_DMA) {
                    atari_system.gtia.reg[gtia::GRAFM] =
                        antic::get_pm_data(&*atari_system, frame.scan_line, 0);
                }
                if atari_system.gtia.gractl.contains(gtia::GRACTL::PLAYER_DMA) {
                    atari_system.gtia.reg[gtia::GRAFP0] =
                        antic::get_pm_data(&*atari_system, frame.scan_line, 1);
                    atari_system.gtia.reg[gtia::GRAFP1] =
                        antic::get_pm_data(&*atari_system, frame.scan_line, 2);
                    atari_system.gtia.reg[gtia::GRAFP2] =
                        antic::get_pm_data(&*atari_system, frame.scan_line, 3);
                    atari_system.gtia.reg[gtia::GRAFP3] =
                        antic::get_pm_data(&*atari_system, frame.scan_line, 4);
                }
            }

            frame.is_visible = false;
            if let Some(dlist_data) = atari_system.antic.prefetch_dlist(&atari_system.ram) {
                atari_system.antic.set_dlist_data(dlist_data);
            }

            let cycles = atari_system.antic.get_dma_cycles();
            frame.cycle = cycles.0;
            frame.visible_cycle = cycles.1;
            frame.dma_cycles = cycles.2;

            // if frame.scan_line == 0 {
            //     *clear_color = ClearColor(gtia::atari_color(atari_system.gtia.colbk()));
            // }

            if atari_system.antic.is_vbi() {
                frame.vblank = true;
                cpu.non_maskable_interrupt_request();
            } else if atari_system.antic.is_dli() {
                cpu.non_maskable_interrupt_request();
            }
            if frame.wsync {
                frame.wsync = false;
                frame.cycle = 104;
            }
        }

        if frame.cycle >= frame.visible_cycle && !frame.is_visible {
            // info!("here: {} {}", frame.cycle, frame.visible_cycle);
            if frame.scan_line >= 8 && frame.scan_line == atari_system.antic.start_scan_line {
                // info!("creating mode line, cycle: {:?}", frame.cycle);
                let mode_line = atari_system.antic.create_next_mode_line();
                let prev_mode_line = frame.current_mode.take();
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
                    create_mode_line(commands, &prev_mode_line, 0.0);
                }
                if mode_line.is_some() {
                    // info!("created mode_line {:?}", mode_line.as_ref().unwrap());
                    frame.current_mode = mode_line;
                } else {
                    frame.vblank = true;
                }
            }

            let current_scan_line = frame.scan_line;
            if let Some(current_line) = &mut frame.current_mode {
                let k = (current_scan_line - current_line.scan_line).min(7);
                current_line.gtia_regs_array.regs[k] = atari_system.gtia.get_colors();
                if k == 0 {
                    let charset_offset = (current_line.chbase as usize) * 256;
                    // TODO suport 512 byte charsets?
                    current_line.line_data =
                        antic::create_line_data(&atari_system, current_line.data_offset);
                    current_line.charset =
                        Charset::new(&atari_system.ram[charset_offset..charset_offset + 1024]);
                }
                current_line.gtia_regs_array.regs[k].grafm =
                    atari_system.gtia.reg[gtia::GRAFM] as u32;
                current_line.gtia_regs_array.regs[k].grafp[0] =
                    atari_system.gtia.reg[gtia::GRAFP0] as u32;
                current_line.gtia_regs_array.regs[k].grafp[1] =
                    atari_system.gtia.reg[gtia::GRAFP1] as u32;
                current_line.gtia_regs_array.regs[k].grafp[2] =
                    atari_system.gtia.reg[gtia::GRAFP2] as u32;
                current_line.gtia_regs_array.regs[k].grafp[3] =
                    atari_system.gtia.reg[gtia::GRAFP3] as u32;
            }
            frame.is_visible = true;
        }

        if frame.cycle == frame.visible_cycle {
            frame.cycle += frame.dma_cycles;
        }

        cpu.cycle(&mut *atari_system);
        atari_system.tick();
        if atari_system.antic.wsync() {
            if frame.cycle < 104 {
                frame.cycle = 104;
            } else {
                frame.wsync = true;
                frame.cycle = SCAN_LINE_CYCLES - 1;
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
            frame.scan_line = (frame.scan_line + 1) % MAX_SCAN_LINES;
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
        if frame.paused {
            break;
        }
    }
}

pub const SCANLINE_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6039053558161382807);

fn loading(
    mut state: ResMut<State<EmulatorState>>,
    mut assets: ResMut<Assets<StateFile>>,
    mut atari_system: ResMut<AtariSystem>,
    mut frame: ResMut<FrameState>,
    mut cpu: ResMut<MOS6502>,
) {
    for (_, state_file) in assets.iter() {
        let data = gunzip(&state_file.data);
        let a800_state = atari800_state::Atari800State::new(&data);
        a800_state.reload(&mut *atari_system, &mut *cpu);
        *frame = FrameState::default();
        frame.scan_line = 248;
        state.set_next(EmulatorState::Running).ok();
        info!("LOADED! {:?}", *state);
    }
    assets.clear()

    // state.set_next(EmulatorState::Running).ok();
}

#[derive(Default)]
struct FragmentState {
    fragment: Option<String>,
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

fn events(
    mut state: ResMut<State<EmulatorState>>,
    mut current_fragment: Local<FragmentState>,
    asset_server: Res<AssetServer>,
    mut atari_system: ResMut<AtariSystem>,
    mut assets: ResMut<Assets<StateFile>>,
) {
    let f = get_fragment().ok();
    if f.is_some() && f != current_fragment.fragment {
        current_fragment.fragment = f;
        state.set_next(EmulatorState::Loading).ok();
        let _: Handle<StateFile> = asset_server
            .load(format!("{}.state", current_fragment.fragment.as_ref().unwrap()).as_str());
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
                    assets.add(StateFile { data });
                    state.set_next(EmulatorState::Loading).unwrap();
                }
            }
        }
    }
}

fn set_debug(
    is_visible: bool,
    debug_components: &mut Query<&mut Visible, With<DebugComponent>>,
    camera_query: &mut Query<&mut GlobalTransform, With<Camera>>,
) {
    for mut visible in debug_components.iter_mut() {
        visible.is_visible = is_visible;
    }
    for mut transform in camera_query.iter_mut() {
        if is_visible {
            *transform =
                GlobalTransform::from_translation(Vec3::new(384.0 / 2.0, -240.0 / 2.0, 0.0))
                    .mul_transform(Transform::from_scale(Vec3::new(1.0 / 1.0, 1.0 / 1.0, 1.0)))
        } else {
            *transform = GlobalTransform::from_scale(Vec3::new(1.0 / 2.0, 1.0 / 2.0, 1.0))
        }
    }
}

fn setup(
    commands: &mut Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut palettes: ResMut<Assets<AtariPalette>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    //emulator_state.set_next(EmulatorState::Loading("laserdemo".to_string())).unwrap();
    // let _: Handle<StateFile> = asset_server.load("laserdemo.state");

    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
        descr.cull_mode = CullMode::None;
    }

    // Create a new shader pipeline
    pipelines.set_untracked(antic::ANTIC_PIPELINE_HANDLE, pipeline_descr);
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

    palettes.set_untracked(antic::ATARI_PALETTE_HANDLE, AtariPalette::default());
    materials.set(
        RED_MATERIAL_HANDLE,
        StandardMaterial {
            albedo: Color::rgba(1.0, 0.0, 0.0, 1.0),
            albedo_texture: None,
            shaded: false,
        },
    );

    commands.spawn(Camera2dBundle {
        orthographic_projection: OrthographicProjection {
            bottom: 0.0,
            top: 2.0 * 240.0,
            left: 0.0,
            right: 2.0 * 384.0,
            window_origin: WindowOrigin::Center,
            ..Default::default()
        },
        transform: Transform::from_scale(Vec3::new(1.0 / 2.0, 1.0 / 2.0, 1.0)),
        ..Default::default()
    });

    commands
        .spawn(PbrBundle {
            mesh: QUAD_HANDLE.typed(),
            material: RED_MATERIAL_HANDLE.typed(),
            visible: Visible {
                is_visible: false,
                is_transparent: false,
            },
            ..Default::default()
        })
        .with(DebugComponent)
        .with(ScanLine);
    commands
        .spawn(atari_text::TextAreaBundle::new(
            18.0,
            20.0,
            (384.0 + 18.0 * 8.0) / 2.0,
            (256.0 - 20.0 * 8.0) / 2.0,
        ))
        .with(DebugComponent)
        .with(CPUDebug);
    commands
        .spawn(atari_text::TextAreaBundle::new(
            12.0,
            20.0,
            (384.0 + 12.0 * 8.0) / 2.0 + 19.0 * 8.0,
            (256.0 - 20.0 * 8.0) / 2.0,
        ))
        .with(DebugComponent)
        .with(AnticDebug);
    commands
        .spawn(atari_text::TextAreaBundle::new(
            12.0,
            20.0,
            (384.0 + 12.0 * 8.0) / 2.0 + (20.0 + 12.0) * 8.0,
            (256.0 - 20.0 * 8.0) / 2.0,
        ))
        .with(DebugComponent)
        .with(GtiaDebug);

    // Setup our world
    // commands.spawn(Camera3dBundle {
    //     transform: Transform::from_translation(Vec3::new(-10.0 * 8.0, 0.0 * 8.0, 40.0 * 8.0))
    //         .looking_at(Vec3::new(-2.0 * 8.0, -0.0 * 8.0, 0.0), Vec3::unit_y()),
    //     ..Default::default()
    // });
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
    app.add_plugin(atari_text::AtartTextPlugin::default());

    // app.add_stage_before("UPDATE", "pre_update", SystemStage::parallel());

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_asset::<AnticLine>()
        .add_asset::<AtariPalette>()
        .add_asset::<StandardMaterial>()
        .add_asset::<StateFile>()
        .init_asset_loader::<Atari800StateLoader>()
        .add_resource(State::new(EmulatorState::Loading))
        .add_resource(ClearColor(gtia::atari_color(0)))
        .add_resource(AtariSystem::new())
        .add_resource(MOS6502::default())
        .add_resource(FrameState::default())
        .add_startup_system(setup.system())
        .add_stage_after(
            stage::UPDATE,
            "running",
            StateStage::<EmulatorState>::default(),
        )
        .add_stage_after(
            stage::UPDATE,
            "debugging",
            StateStage::<EmulatorState>::default(),
        )
        .add_stage_after(
            stage::UPDATE,
            "idle_update",
            StateStage::<EmulatorState>::default(),
        )
        .add_stage_after(
            stage::UPDATE,
            "loading",
            StateStage::<EmulatorState>::default(),
        )
        .add_system_to_stage("pre_update", keyboard_system.system())
        // .add_system_to_stage("pre_update", reload_system.system())
        .add_system_to_stage("post_update", debug_overlay_system.system())
        .on_state_update("running", EmulatorState::Running, atari_system.system())
        .on_state_update("debugging", EmulatorState::Debugging, atari_system.system())
        .on_state_update("loading", EmulatorState::Loading, loading.system())
        .add_system(events.system())
        .run();
}
