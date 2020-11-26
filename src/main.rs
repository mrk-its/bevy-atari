use bevy::{
    prelude::*,
    render::{
        mesh::shape,
        pipeline::{PipelineDescriptor, RenderPipeline, CullMode},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
    type_registry::TypeUuid,
};

/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    let mut app = App::build();
    app.add_plugins(DefaultPlugins);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_asset::<MyMaterial>()
    .add_asset::<StandardMaterial>()
    // .add_resource(bevy::render::pass::ClearColor(Color::rgb_u8(0x03, 0x52, 0xa8)))
    .add_startup_system(setup)
    .run();
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b25127e"]
struct MyMaterial {
    pub fg_color: Color,
    pub bg_color: Color,
    pub line_width: u32,
    #[render_resources(buffer)]
    pub charset: Vec<u8>,
    #[render_resources(buffer)]
    pub data: Vec<u8>,
}

const VERTEX_SHADER: &str = r#"
#version 300 es

precision highp float;

in vec3 Vertex_Position;
in vec2 Vertex_Uv;

out vec2 v_Uv;
flat out int instance_id;

layout(std140) uniform Camera {
    mat4 ViewProj;
};
layout(std140) uniform Transform { // set = 1 binding = 0
    mat4 Model;
};
void main() {
    vec3 pos = vec3(Vertex_Position) + vec3(float(gl_InstanceID), 0, 0);
    gl_Position = ViewProj * Model * vec4(pos, 1.0);
    v_Uv = Vertex_Uv;
    instance_id = gl_InstanceID;
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 300 es

precision highp float;

in vec2 v_Uv;
flat in int instance_id;
out vec4 o_Target;

layout(std140) uniform MyMaterial_fg_color { // set = 1 binding = 1
    vec4 fg_color;
};

layout(std140) uniform MyMaterial_bg_color { // set = 1 binding = 2
    vec4 bg_color;
};

layout(std140) uniform MyMaterial_line_width { // set = 1 binding = 3
    int line_width;
};

layout(std140) uniform MyMaterial_charset { // set = 1 binding = 4
    uvec4 charset[64];
};

layout(std140) uniform MyMaterial_data { // set = 1 binding = 5
    uvec4 data[3];
};


#define get_byte(data, offset) (int(data[offset >> 4][(offset >> 2) & 3] >> ((offset & 3) << 3)) & 255)

void main() {
    float w = v_Uv[0] * float(line_width);
    int n = int(w);
    float frac = w - float(n);
    int x = 7 - int(frac * 8.0);
    int y = int(v_Uv[1] * 8.0);


    int char = get_byte(data, n);
    int inv = char >> 7;
    int offs = (char & 0x7f) * 8 + y;
    int byte = get_byte(charset, offs);

    if((((byte >> x) & 1) ^ inv) != 0) {
        o_Target = fg_color;
    } else {
        o_Target = bg_color;
    }
}
"#;

fn setup(
    commands: &mut Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    let atari_rom = include_bytes!("../altirra.rom");
    let rom_start = 0xc000;
    let charset_start = 0xE000;
    let charset_offset = charset_start - rom_start;
    let charset: Vec<_> = (&atari_rom[charset_offset..charset_offset + 1024]).iter().cloned().collect();
    info!("charset data: {:?}", charset);

    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
        descr.cull_mode = CullMode::None;
    }

    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(pipeline_descr);

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind MyMaterial resources to our shader
    render_graph.add_system_node(
        "my_material",
        AssetRenderResourcesNode::<MyMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "my_material" node to the main pass node. This ensures "my_material" runs before the main pass
    render_graph
        .add_node_edge("my_material", base::node::MAIN_PASS)
        .unwrap();
    let fg_color_linear = Color::rgb_linear(0x69 as f32 / 255.0, 0xb8 as f32 / 255.0, 0xff as f32 / 255.0);
    let bg_color_linear = Color::rgb_linear(0x03 as f32 / 255.0, 0x52 as f32 / 255.0, 0xa8 as f32 / 255.0);
    let mut data = vec![0; 48];
    &data.as_mut_slice()[2..7].copy_from_slice(&[50, 37, 33, 36, 57]);
   // &data.as_mut_slice()[0..5].copy_from_slice("READY".as_bytes());
    let mut data2 = vec![0; 48];
    &data2.as_mut_slice()[2..3].copy_from_slice(&[128]);

    // Create a new material
    let material = materials.add(MyMaterial {
        fg_color: fg_color_linear,
        bg_color: bg_color_linear,
        line_width: 40,
        charset: charset.clone(),
        data: data,
    });
    let material2 = materials.add(MyMaterial {
        fg_color: fg_color_linear,
        bg_color: bg_color_linear,
        line_width: 40,
        charset,
        data: data2,
    });

    let mesh = Mesh::from(shape::Quad { size: Vec2::new(40.0, 1.0), flip: false});
    let mesh_handle = meshes.add(mesh);

    // Setup our world
    commands
        .spawn(MeshBundle {
            mesh: mesh_handle.clone_weak(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle.clone_weak(),
            )]),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        .with(material)
        .spawn(MeshBundle {
            mesh: mesh_handle,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle,
            )]),
            transform: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
            ..Default::default()
        })
        .with(material2)
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(-0.0, 0.0, 30.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::unit_y()),
            ..Default::default()
        });
}
