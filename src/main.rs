pub mod antic;
mod color_set;
pub mod gtia;
mod palette;
pub mod pia;
pub mod pokey;
mod system;

use antic::ModeLineDescr;
use bevy::reflect::TypeUuid;
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
use color_set::ColorSet;
use system::{AtariSystem, W65C02S};

const SCAN_LINE_CYCLES: usize = 114;
const PAL_SCAN_LINES: usize = 312;

const VERTEX_SHADER: &str = include_str!("shaders/antic.vert");
const FRAGMENT_SHADER: &str = include_str!("shaders/antic.frag");

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b25127e"]
struct AnticLine {
    pub line_width: u32,
    pub mode: u32,
    #[render_resources(buffer)]
    pub data: Vec<u8>,
    pub color_set: ColorSet,
    // pub chbase: u32,
    #[render_resources(buffer)]
    pub charset: Vec<u8>,
}
// #[derive(RenderResources, Default, TypeUuid)]
// #[uuid = "f145d910-99c5-4df5-b673-e822b1389222"]
// struct AnticCharset {
//     #[render_resources(buffer)]
//     pub charset: Vec<u8>,
// }

#[derive(Debug, Default)]
struct PerfMetrics {
    frame_cnt: usize,
    cpu_cycle_cnt: usize,
}

#[derive(Default)]
struct AnticResources {
    pipeline_handle: Handle<PipelineDescriptor>,
    // charset_handle: Handle<AnticCharset>,
    mesh_handle: Handle<Mesh>,
}
fn create_mode_line(
    commands: &mut Commands,
    resources: &AnticResources,
    mode_line: ModeLineDescr,
    system: &AtariSystem,
) {
    if mode_line.n_bytes == 0 || mode_line.width == 0 || mode_line.height == 0 {
        return;
    }
    let line_data = &system.ram[mode_line.data_offset..mode_line.data_offset + mode_line.n_bytes];
    let color_set = system.gtia.get_color_set();

    let charset_offset = (mode_line.chbase as usize) * 256;
    let charset = &system.ram[charset_offset..charset_offset + 1024]; // TODO - 512 byte charsets?

    commands
        .spawn(MeshBundle {
            mesh: resources.mesh_handle.clone(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                resources.pipeline_handle.clone_weak(),
            )]),
            transform: Transform::from_translation(Vec3::new(
                0.0,
                120.0 - (mode_line.scan_line as f32) - mode_line.height as f32 / 2.0,
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
            color_set: color_set,
            line_width: mode_line.width as u32,
            data: line_data.to_vec(),
            charset: charset.to_vec(),
        })
        // .with(resources.charset_handle.clone_weak())
        ;
}

#[derive(Default)]
struct Debugger {
    enabled: bool,
    instr_cnt: usize,
}

fn atari_system(
    commands: &mut Commands,
    mut antic_resources: ResMut<AnticResources>,
    antic_lines: Query<(Entity, &AnticLine)>,
    // mut charsets: ResMut<Assets<AnticCharset>>,
    mut debug: Local<Debugger>,
    mut cpu: ResMut<W65C02S>,
    mut atari_system: ResMut<AtariSystem>,
    mut perf_metrics: Local<PerfMetrics>,
) {
    // if perf_metrics.frame_cnt > 0 {
    //     return;
    // }
    for (entity, _) in antic_lines.iter() {
        commands.despawn(entity);
    }
    let mut vblank = false;
    let mut next_scan_line: usize = 8;
    let mut dli_scan_line: usize = 0xffff;


    // let charset: Vec<_> = atari_system.ram
    //     .iter()
    //     .cloned()
    //     .collect();
    // charsets.set(&antic_resources.charset_handle, AnticCharset { charset });

    for scan_line in 0..PAL_SCAN_LINES {
        // info!("scan_line: {}", scan_line);
        atari_system.antic.scan_line = scan_line;

        vblank = vblank || scan_line >= 248;
        if !vblank && next_scan_line == scan_line {
            let dlist = atari_system.antic.dlist();
            let mut dlist_data: [u8; 3] = [0; 3];
            dlist_data.copy_from_slice(&atari_system.ram[dlist..dlist + 3]);
            if let Some(mode_line) = atari_system.antic.create_next_mode_line(&dlist_data) {
                next_scan_line = scan_line + mode_line.height;
                if mode_line.dli {
                    dli_scan_line = next_scan_line - 1;
                }
                // info!("antic line: {:?}, next_scan_line: {:?}", mode_line, next_scan_line);
                create_mode_line(commands, &antic_resources, mode_line, &atari_system);
            } else {
                vblank = true;
            }
        }
        for n in 0..SCAN_LINE_CYCLES {
            if n < 2 {
                if scan_line == dli_scan_line {
                    // bevy::log::info!("DLI, scanline: {}", scan_line);
                    atari_system.antic.set_dli();
                    cpu.set_nmi(n == 0);
                } else if scan_line == 248 {
                    // bevy::log::info!("VBI, scanline: {}", scan_line);
                    atari_system.antic.set_vbi();
                    cpu.set_nmi(n == 0);
                }
            }
            let pc = cpu.get_pc() as usize;
            // if pc == 0xc2b3 {
            //     debug.enabled = true;
            // }
            if false || debug.enabled {
                if debug.instr_cnt < 200 {
                    if let Ok(inst) = disasm6502::from_array(&atari_system.ram[pc..pc + 16]) {
                        if let Some(i) = inst.get(0) {
                            // info!("{:04x?}: {} {:?}", pc, i, *cpu);
                        }
                    }
                } else {
                    panic!("STOP");
                }
                debug.instr_cnt += 1;
            }
            cpu.step(&mut *atari_system);
            perf_metrics.cpu_cycle_cnt += 1;
        }

    }
    // if perf_metrics.frame_cnt % 60 == 0 {
    //     info!("{:?}", *perf_metrics);
    // }
    perf_metrics.frame_cnt += 1;
}

fn setup(
    commands: &mut Commands,
    mut atari_system: ResMut<AtariSystem>,
    mut cpu: ResMut<W65C02S>,
    mut antic_resources: ResMut<AnticResources>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut charsets: ResMut<Assets<AnticCharset>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    if true {
        let memory = include_bytes!("../robbo_memory.dat");
        atari_system.ram.copy_from_slice(memory);

        atari_system.gtia.write(gtia::COLBK, 0);
        atari_system.gtia.write(gtia::COLPF0, 114);
        atari_system.gtia.write(gtia::COLPF1, 100);
        atari_system.gtia.write(gtia::COLPF2, 104);
        atari_system.gtia.write(gtia::COLPF3, 160);

        atari_system.antic.write(antic::DMACTL, 33);
        atari_system.antic.write(antic::CHACTL, 2);
        atari_system.antic.write(antic::CHBASE, 32);
        atari_system.antic.write(antic::DLIST, (44239 & 0xff) as u8);
        atari_system
            .antic
            .write(antic::DLIST + 1, (44239 >> 8) as u8);
        atari_system.antic.write(antic::NMIEN, 64);
        atari_system.antic.write(antic::NMIST, 31);
        atari_system.antic.write(antic::PMBASE, 0);

        cpu.step(&mut *atari_system); // changes state into Running

        cpu.set_pc(44196);
        cpu.set_a(14);
        cpu.set_x(36);
        cpu.set_y(2);
        cpu.set_p(240);
        cpu.set_s(253);


    } else {
        let memory = include_bytes!("../robbo_memory_play.dat");
        atari_system.ram.copy_from_slice(memory);

        atari_system.gtia.write(gtia::COLBK, 0);
        atari_system.gtia.write(gtia::COLPF0, 66);
        atari_system.gtia.write(gtia::COLPF1, 212);
        atari_system.gtia.write(gtia::COLPF2, 24);
        atari_system.gtia.write(gtia::COLPF3, 112);

        atari_system.antic.write(antic::DMACTL, 33);
        atari_system.antic.write(antic::CHACTL, 2);
        atari_system.antic.write(antic::CHBASE, 32);
        atari_system.antic.write(antic::DLIST, (44239 & 0xff) as u8);
        atari_system
            .antic
            .write(antic::DLIST + 1, (44239 >> 8) as u8);
        atari_system.antic.write(antic::NMIEN, 64);
        atari_system.antic.write(antic::NMIST, 31);
        atari_system.antic.write(antic::PMBASE, 0);

        cpu.step(&mut *atari_system); // changes state into Running

        cpu.set_pc(44199);
        cpu.set_a(14);
        cpu.set_x(36);
        cpu.set_y(2);
        cpu.set_p(113);
        cpu.set_s(253);

    }



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
    // render_graph.add_system_node(
    //     "antic_charset",
    //     AssetRenderResourcesNode::<AnticCharset>::new(false),
    // );

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind AnticLine resources to our shader
    render_graph.add_system_node("antic_line", RenderResourcesNode::<AnticLine>::new(true));

    // Add a Render Graph edge connecting our new "antic_line" node to the main pass node. This ensures "antic_line" runs before the main pass
    render_graph
        .add_node_edge("antic_line", base::node::MAIN_PASS)
        .unwrap();
    // render_graph
    //     .add_node_edge("antic_charset", base::node::MAIN_PASS)
    //     .unwrap();

    // let charset: Vec<_> = atari_system.ram
    //     .iter()
    //     .cloned()
    //     .collect();

    // antic_resources.charset_handle = charsets.add(AnticCharset { charset });

    antic_resources.mesh_handle = meshes.add(Mesh::from(shape::Quad {
        size: Vec2::new(1.0, 1.0),
        flip: false,
    }));

    // Setup our world
    // commands.spawn(Camera3dBundle {
    //     transform: Transform::from_translation(Vec3::new(-10.0 * 8.0, -14.0 * 8.0, 30.0 * 8.0))
    //         .looking_at(Vec3::new(-2.0 * 8.0, -14.0 * 8.0, 0.0), Vec3::unit_y()),
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
            .mul_transform(Transform::from_scale(Vec3::new(1.0 / 3.0, 1.0 / 3.0, 1.0))),
        ..Default::default()
    });

    // antic.dlist = ANTIC_DLIST;
    // antic.scan_line = 0;
    // antic.dmactl = 1;

    // while let Some(mode_line) = antic.create_next_mode_line(&atari_system) {
    //     info!("modeline: {:?}", mode_line);
    //     create_mode_line(commands, &antic_resources, mode_line, &atari_system);
    // }
}

/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    let mut app = App::build();
    app.add_plugins(DefaultPlugins);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_asset::<AnticLine>()
        // .add_asset::<AnticCharset>()
        .add_asset::<StandardMaterial>()
        .add_resource(ClearColor(color_set::atari_color(0)))
        .add_resource(AtariSystem::new())
        .add_resource(W65C02S::new())
        .add_resource(AnticResources::default())
        .add_startup_system(setup)
        .add_system(atari_system)
        .run();
}
