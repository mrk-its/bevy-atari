mod color_set;
mod system;
mod palette;

use bevy::{
    prelude::*,
    render::{
        mesh::shape,
        pass::ClearColor,
        pipeline::{CullMode, PipelineDescriptor, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
    type_registry::TypeUuid,
};

use color_set::ColorSet;
use system::{AtariSystem, W65C02S};
use palette::jakub::PALETTE;
/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    let mut app = App::build();
    app.add_plugins(DefaultPlugins);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_asset::<AnticLine>()
        .add_asset::<StandardMaterial>()
        .add_resource(ClearColor(Color::rgb_u8(0x0, 0x0, 0x0)))
        .add_resource(AtariSystem::new())
        .add_resource(W65C02S::new())
        .add_startup_system(setup)
        .add_system(atari_system)
        .run();
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b25127e"]
struct AnticLine {
    pub line_width: u32,
    pub mode: u32,
    #[render_resources(buffer)]
    pub charset: Vec<u8>,
    #[render_resources(buffer)]
    pub data: Vec<u8>,
    pub color_set: ColorSet,
}

const VERTEX_SHADER: &str = include_str!("shaders/antic.vert");
const FRAGMENT_SHADER: &str = include_str!("shaders/antic.frag");
const MEMORY: &[u8] = include_bytes!("../robbo_memory.dat");
const ANTIC_DLIST: usize = 44239;
const ANTIC_CHBASE: usize = 32;

const COLBK: usize = 0;
const COLPF0: usize = 114;
const COLPF1: usize = 100;
const COLPF2: usize = 104;
const COLPF3: usize = 160;

#[derive(Debug, Default)]
struct PerfMetrics {
    frame_cnt: usize,
    cpu_cycle_cnt: usize,
}

fn atari_system(mut cpu: ResMut<W65C02S>, mut atari_system: ResMut<AtariSystem>, mut perf_metrics: Local<PerfMetrics>) {
    for _ in 0..35568 {
        cpu.step(&mut *atari_system);
        perf_metrics.cpu_cycle_cnt += 1;
    }
    perf_metrics.frame_cnt += 1;
    if perf_metrics.frame_cnt % 60 == 0 {
        info!("{:?}", *perf_metrics);
    }
}

fn setup(
    commands: &mut Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<AnticLine>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    let charset_start = ANTIC_CHBASE * 256;
    let charset: Vec<_> = (&MEMORY[charset_start..charset_start + 1024])
        .iter()
        .cloned()
        .collect();
    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
        descr.cull_mode = CullMode::None;
    }

    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(pipeline_descr);

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind AnticLine resources to our shader
    render_graph.add_system_node(
        "antic_line",
        AssetRenderResourcesNode::<AnticLine>::new(true),
    );

    // Add a Render Graph edge connecting our new "antic_line" node to the main pass node. This ensures "antic_line" runs before the main pass
    render_graph
        .add_node_edge("antic_line", base::node::MAIN_PASS)
        .unwrap();

    fn atari_color(index: usize) -> Color {
        Color::rgb_u8(PALETTE[index][0], PALETTE[index][1], PALETTE[index][2])
    }

    let color_set = ColorSet {
        c0: atari_color(COLPF2),
        c1: atari_color(COLPF2 & 0xf0 | COLPF1 & 0x0f),
        c0_0: atari_color(COLBK),
        c1_0: atari_color(COLPF0),
        c2_0: atari_color(COLPF1),
        c3_0: atari_color(COLPF2),
        c0_1: atari_color(COLBK),
        c1_1: atari_color(COLPF0),
        c2_1: atari_color(COLPF1),
        c3_1: atari_color(COLPF3),
    };

    info!("dlist: {:?}", &MEMORY[ANTIC_DLIST..ANTIC_DLIST + 256]);
    let mut dlist = ANTIC_DLIST;
    let mut video_memory = 0;
    let mut line_cnt = 0;

    let mesh = Mesh::from(shape::Quad {
        size: Vec2::new(32.0, 1.0),
        flip: false,
    });
    let mesh_handle = meshes.add(mesh);
    loop {
        let op = MEMORY[dlist];
        dlist += 1;
        info!("op: {:02x}", op);

        let mods = op & 0xf0;
        let mode = op & 0x0f;
        if mode == 0x0 {
            // empty lines
            let n_lines = ((mods >> 4) & 7) as usize + 1;
            info!("{} empty line(s)", n_lines);
            line_cnt += n_lines;
            continue;
        }
        if (mods & 0x40 > 0) || (mode == 1) {
            let addr = MEMORY[dlist] as usize + (MEMORY[dlist + 1] as usize * 256);
            dlist += 2;
            if mode == 1 {
                // jmp / jbl
                // generate 1 empty line
                dlist = addr;
                if mods & 0x40 > 0 {
                    // jbl
                    break;
                }
                line_cnt += 1;
            } else {
                video_memory = addr;
                info!("video memory: {:04x}", video_memory);
            }
        }
        info!("video mode: {:x}", mode);
        match mode {
            0x2 => {
                let line_data = &MEMORY[video_memory..video_memory + 32];
                info!("{:?}", line_data);
                // Create a new material
                let material = materials.add(AnticLine {
                    mode: mode as u32,
                    color_set,
                    line_width: 32,
                    charset: charset.clone(),
                    data: line_data.to_vec(),
                });
                commands
                    .spawn(MeshBundle {
                        mesh: mesh_handle.clone(),
                        render_pipelines: RenderPipelines::from_pipelines(vec![
                            RenderPipeline::new(pipeline_handle.clone()),
                        ]),
                        transform: Transform::from_translation(Vec3::new(
                            0.0,
                            -(line_cnt as f32 / 8.0),
                            0.0,
                        )),
                        ..Default::default()
                    })
                    .with(material);

                video_memory += 32;
                line_cnt += 8;
            }
            0xa => {
                let line_data = &MEMORY[video_memory..video_memory + 16];
                // Create a new material
                let material = materials.add(AnticLine {
                    mode: mode as u32,
                    color_set,
                    line_width: 32,
                    charset: charset.clone(),
                    data: line_data.to_vec(),
                });
                commands
                    .spawn(MeshBundle {
                        mesh: mesh_handle.clone(),
                        render_pipelines: RenderPipelines::from_pipelines(vec![
                            RenderPipeline::new(pipeline_handle.clone()),
                        ]),
                        transform: Transform::from_translation(Vec3::new(
                            0.0,
                            -(line_cnt as f32 / 8.0),
                            0.0,
                        ))
                        .mul_transform(Transform::from_scale(Vec3::new(1.0, 0.5, 1.0))),
                        ..Default::default()
                    })
                    .with(material);

                video_memory += 16;
                line_cnt += 4;
            }
            _ => (),
        }
    }

    // Setup our world
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(Vec3::new(-10.0, -14.0, 30.0))
            .looking_at(Vec3::new(-2.0, -14.0, 0.0), Vec3::unit_y()),
        ..Default::default()
    });
}
