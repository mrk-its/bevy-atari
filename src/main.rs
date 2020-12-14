#[macro_use]
extern crate bitflags;
use std::io::prelude::*;

pub mod antic;
mod atari800_state;
pub mod gtia;
mod js_api;
mod palette;
pub mod pia;
pub mod pokey;
mod render_resources;
mod system;
use antic::{ModeLineDescr, DMACTL, MODE_OPTS, NMIEN};
use atari800_state::{Atari800StateLoader, StateFile};
use bevy::reflect::TypeUuid;
use bevy::winit::WinitConfig;
use bevy::{
    prelude::*,
    render::{
        camera::{OrthographicProjection, WindowOrigin},
        entity::Camera2dBundle,
        mesh::shape,
        pass::ClearColor,
        pipeline::{CullMode, PipelineDescriptor, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph, RenderResourcesNode},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};
use emulator_6502::MOS6502;
use render_resources::{Charset, GTIARegs, GTIARegsArray, LineData, Palette};
use system::AtariSystem;
use wasm_bindgen::prelude::*;

const SCAN_LINE_CYCLES: usize = 114;
const PAL_SCAN_LINES: usize = 312;
#[allow(dead_code)]
const NTSC_SCAN_LINES: usize = 262;

const MAX_SCAN_LINES: usize = PAL_SCAN_LINES;

const VERTEX_SHADER: &str = include_str!("shaders/antic.vert");
const FRAGMENT_SHADER: &str = include_str!("shaders/antic.frag");

#[derive(RenderResources, TypeUuid)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b25127e"]
struct AnticLine {
    pub line_width: f32,
    pub mode: u32,
    pub hscrol: f32,
    pub data: LineData,
    pub gtia_regs_array: GTIARegsArray,
    pub charset: Charset,
}
#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "f145d910-99c5-4df5-b673-e822b1389222"]
struct AtariPalette {
    pub palette: Palette,
}

#[derive(Debug, Default)]
struct PerfMetrics {
    frame_cnt: usize,
    cpu_cycle_cnt: usize,
}

#[derive(Default)]
struct AnticResources {
    pipeline_handle: Handle<PipelineDescriptor>,
    palette_handle: Handle<AtariPalette>,
    mesh_handle: Handle<Mesh>,
}

#[derive(Default)]
struct State {
    requested_file: String,
    handle: Handle<StateFile>,
    initialized: bool,
}

fn gunzip(data: &[u8]) -> Vec<u8> {
    let mut decoder = flate2::read::GzDecoder::new(&data[..]);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result).unwrap();
    result
}

fn create_mode_line(
    commands: &mut Commands,
    resources: &AnticResources,
    mode_line: &ModeLineDescr,
    system: &AtariSystem,
    y_extra_offset: f32,
    enable_log: bool,
) {
    // if mode_line.n_bytes == 0 || mode_line.width == 0 || mode_line.height == 0 {
    //     // TODO - this way PM is not displayed on empty lines
    //     return;
    // }

    // TODO - check if PM DMA is working, page 114 of AHRM
    // if DMA is disabled display data from Graphics Data registers, p. 114
    // TODO - add suppor for low-res sprites
    let pm_hires = system.antic.dmactl.contains(antic::DMACTL::PM_HIRES);

    let pl_mem = |n: usize| {
        if system.antic.dmactl.contains(antic::DMACTL::PLAYER_DMA) {
            let beg = if pm_hires {
                0x400
                    + n * 0x100
                    + mode_line.scan_line
                    + (mode_line.pmbase & 0b11111000) as usize * 256
            } else {
                0x200
                    + n * 0x80
                    + mode_line.scan_line / 2
                    + (mode_line.pmbase & 0b11111100) as usize * 256
            };
            system.ram[beg..beg + 16].to_owned()
        } else {
            let v = system.gtia.player_graphics[n];
            vec![v, v, v, v, v, v, v, v, v, v, v, v, v, v, v, v]
        }
    };

    let line_data = LineData::new(
        &system.ram[mode_line.data_offset..mode_line.data_offset + 48],
        &pl_mem(0),
        &pl_mem(1),
        &pl_mem(2),
        &pl_mem(3),
    );

    let charset_offset = (mode_line.chbase as usize) * 256;

    // TODO suport 512 byte charsets?
    let charset = Charset::new(&system.ram[charset_offset..charset_offset + 1024]);

    commands
        .spawn(MeshBundle {
            mesh: resources.mesh_handle.clone(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                resources.pipeline_handle.clone_weak(),
            )]),
            transform: Transform::from_translation(Vec3::new(
                0.0,
                120.0
                    - (mode_line.scan_line as f32)
                    - y_extra_offset
                    - mode_line.height as f32 / 2.0
                    + 8.0,
                0.0,
            ))
            .mul_transform(Transform::from_scale(Vec3::new(
                mode_line.width as f32,
                mode_line.height as f32,
                1.0,
            ))),
            ..Default::default()
        })
        .with(AnticLine {
            // chbase: mode_line.chbase as u32,
            mode: mode_line.mode as u32,
            gtia_regs_array: mode_line.gtia_regs_array,
            line_width: mode_line.width as f32,
            hscrol: mode_line.hscrol as f32,
            data: line_data,
            charset: charset,
        })
        .with(resources.palette_handle.clone_weak());
}

#[derive(Default)]
struct Debugger {
    enabled: bool,
    instr_cnt: usize,
}

fn atari_system(
    commands: &mut Commands,
    mut state: ResMut<State>,
    state_files: ResMut<Assets<StateFile>>,
    asset_server: Res<AssetServer>,
    keyboard: Res<Input<KeyCode>>,
    antic_resources: ResMut<AnticResources>,
    antic_lines: Query<(Entity, &AnticLine)>,
    // mut charsets: ResMut<Assets<AnticCharset>>,
    mut debug: Local<Debugger>,
    mut cpu: ResMut<MOS6502>,
    mut atari_system: ResMut<AtariSystem>,
    mut perf_metrics: Local<PerfMetrics>,
) {
    // if perf_metrics.frame_cnt > 120 {
    //     return;
    // }
    // let enable_log = perf_metrics.frame_cnt < 2;
    let enable_log = false;
    atari_system.enable_log(enable_log);
    // let enable_log = false;
    // assert!(perf_metrics.frame_cnt < 35);
    // if perf_metrics.frame_cnt % 60 == 0 || enable_log {
    //     info!("frame_cnt: {:?}", *perf_metrics);

    // }
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
            state.initialized = true;
        }
    }
    if !state.initialized {
        return;
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
                }
            }
        }
    }
    let mut irq = atari_system.handle_keyboard(&keyboard);
    if irq {
        cpu.interrupt_request();
    }
    for (entity, _) in antic_lines.iter() {
        commands.despawn(entity);
    }
    let mut vblank = false;
    let mut next_scan_line: usize = 8;
    let mut dli_scan_line: usize = 0xffff;

    let mut y_extra_offset = 0.0;

    // debug.enabled = true;
    // let charset: Vec<_> = atari_system.ram
    //     .iter()
    //     .cloned()
    //     .collect();
    // charsets.set(&antic_resources.charset_handle, AnticCharset { charset });
    let mut wsync = false;
    let mut current_mode = None;
    'outer: for scan_line in 0..MAX_SCAN_LINES {
        if enable_log {
            info!("scan_line: {}", scan_line);
        }
        atari_system.antic.scan_line = scan_line;

        if scan_line == 248 {
            vblank = true;
            if atari_system.antic.nmien.contains(NMIEN::VBI) {
                if enable_log {
                    info!("VBI, scanline: {}", scan_line);
                }
                atari_system.antic.set_vbi();
                cpu.non_maskable_interrupt_request();
            }
        } else if scan_line >= dli_scan_line {
            if atari_system.antic.nmien.contains(NMIEN::DLI) {
                dli_scan_line = 0xffff;
                if enable_log {
                    info!("DLI, scanline: {}", scan_line);
                }
                atari_system.antic.set_dli();
                cpu.non_maskable_interrupt_request();
                debug.enabled = true;
            }
        }

        if !vblank {
            if next_scan_line == scan_line {
                let dlist_data = atari_system.antic.prefetch_dlist(&atari_system.ram);
                let mode = atari_system
                    .antic
                    .create_next_mode_line(&dlist_data, next_scan_line);
                if let Some(mode_line) = mode {
                    next_scan_line = mode_line.scan_line + mode_line.height;
                    if mode_line.opts.contains(MODE_OPTS::DLI) {
                        dli_scan_line = next_scan_line - 1;
                    }
                    if enable_log {
                        info!(
                            "antic line: {:?}, next_scan_line: {:?}, dli_scan_line: {:?}",
                            mode_line, next_scan_line, dli_scan_line
                        );
                    }
                    let prev_mode_line = current_mode.replace(mode_line);
                    if let Some(prev_mode_line) = prev_mode_line {
                        create_mode_line(
                            commands,
                            &antic_resources,
                            &prev_mode_line,
                            &atari_system,
                            y_extra_offset,
                            enable_log,
                        );
                    }
                    // y_extra_offset += 1.0;
                } else {
                    vblank = true;
                    let prev_mode_line = current_mode.take();
                    if let Some(prev_mode_line) = prev_mode_line {
                        create_mode_line(
                            commands,
                            &antic_resources,
                            &prev_mode_line,
                            &atari_system,
                            y_extra_offset,
                            enable_log,
                        );
                    }
                }
            }
        }

        let (start_dma_cycles, line_start_cycle, dma_cycles) = if let Some(current_mode) = &current_mode {
            atari_system.antic.get_dma_cycles(current_mode)
        } else {
            (0, 0, 0)
        };
        if enable_log {
            info!("start_dma_cycles: {} line_start: {}, dma_cycles: {}", start_dma_cycles, line_start_cycle, dma_cycles);
        }

        let mut n = if wsync {
            wsync = false;
            104
        } else {
            start_dma_cycles
        };
        let mut last_pc = 0;

        let mut is_visible = false;

        while n < SCAN_LINE_CYCLES {
            // if n == 110 {
            //     atari_system.antic.scan_line = scan_line + 1;
            // }

            if let Some(current_line) = &mut current_mode {
                if n >= line_start_cycle && !is_visible {
                    let k = (scan_line - current_line.scan_line).min(7);
                    current_line.gtia_regs_array.regs[k] = atari_system.gtia.get_colors();
                    is_visible = true;
                }
            }

            if n == line_start_cycle {
                n += dma_cycles;
            }

            if enable_log && debug.enabled {
                if debug.instr_cnt < 30 {
                    if last_pc != cpu.program_counter {
                        last_pc = cpu.program_counter;
                        let pc = cpu.program_counter as usize;
                        if let Ok(inst) = disasm6502::from_array(&atari_system.ram[pc..pc + 16]) {
                            if let Some(i) = inst.get(0) {
                                info!("{:04x?}: {} {:?}, cycle: {}", pc, i, *cpu, n);
                            }
                            debug.instr_cnt += 1;
                        }
                    }
                } else {
                    debug.enabled = false;
                    debug.instr_cnt = 0;
                    //panic!("STOP");
                }
            }

            cpu.cycle(&mut *atari_system);
            perf_metrics.cpu_cycle_cnt += 1;
            atari_system.tick();
            if atari_system.antic.wsync() {
                if enable_log {
                    warn!("WSYNC, cycle: {}", n);
                }
                if n < 104 {
                    n = 104;
                } else {
                    wsync = true;
                    continue;
                }
            }
            n += 1;
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

fn setup(
    commands: &mut Commands,
    mut state: ResMut<State>,
    mut antic_resources: ResMut<AnticResources>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut palettes: ResMut<Assets<AtariPalette>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // let state_data = include_bytes!("../fred.state.dat");
    // let state_data = include_bytes!("../ls.state.dat");
    // let state_data = include_bytes!("../lvl2.state.dat");
    // let state_data = include_bytes!("../acid800.state.dat");
    // let state_data = include_bytes!("../robbo.state.dat");
    // let state_data = include_bytes!("../laserdemo.state.dat");
    // let state_data = include_bytes!("../lasermania.state.dat");
    // let state_data = include_bytes!("../basic.state.dat");

    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
        descr.cull_mode = CullMode::None;
    }

    // Create a new shader pipeline
    antic_resources.pipeline_handle = pipelines.add(pipeline_descr);

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
        width: 320*2,
        height: 240*2,
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
        .add_startup_system(setup)
        .add_system(atari_system)
        .run();
}
