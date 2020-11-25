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
    .add_resource(bevy::render::pass::ClearColor(Color::rgb_u8(0x03, 0x52, 0xa8)))
    .add_startup_system(setup)
    .run();
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b25127e"]
struct MyMaterial {
    pub color: Color,
    #[render_resources(buffer)]
    pub charset: Vec<u8>,
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

layout(std140) uniform MyMaterial_color { // set = 1 binding = 1
    vec4 color;
};

layout(std140) uniform MyMaterial_charset { // set = 1 binding = 2
    uvec4 charset[64];
};

int text[5] = int[](50,37,33,36,57);

vec4 encodeSRGB(vec4 linearRGB_in) {
    vec3 linearRGB = linearRGB_in.rgb;
    vec3 a = 12.92 * linearRGB;
    vec3 b = 1.055 * pow(linearRGB, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), linearRGB);
    return vec4(mix(a, b, c), linearRGB_in.a);
}

#define get_byte(data, offset) (data[offset >> 4][(offset >> 2) & 3] >> ((offset & 3) << 3))

void main() {
    int x = 7 - int(v_Uv[0] * 8.0);
    int y = int(v_Uv[1] * 8.0);

    int offs = text[instance_id] * 8 + y; // char byte offset
    uint byte = get_byte(charset, offs);

    if(((byte >> x) & uint(1)) != uint(0)) {
        o_Target = encodeSRGB(color);
    } else {
        discard;
    }
}
"#;

fn setup(
    commands: &mut Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
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

    // Create a new material
    let material = materials.add(MyMaterial {
        color: Color::rgb_u8(0x69, 0xb8, 0xff),
        charset,
    });

    let material2 = standard_materials.add(StandardMaterial {
        albedo: Color::rgb_u8(0x69, 0xb8, 0xff),
        albedo_texture: None,
        shaded: false,
    });

    let mut mesh = Mesh::from(shape::Quad { size: Vec2::new(1.0, 1.0), flip: false});
    mesh.set_instances(5);

    let mut mesh2 = Mesh::from(shape::Quad { size: Vec2::new(1.0, 1.0), flip: false});
    mesh2.set_instances(1);

    // Setup our world
    commands
        .spawn(MeshBundle {
            mesh: meshes.add(mesh),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle
            )]),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        .with(material)
        // .spawn(MeshBundle {
        //     mesh: meshes.add(mesh2),
        //     render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
        //         pipeline_handle,
        //     )]),
        //     transform: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        //     ..Default::default()
        // })
        // .with(material)
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Quad { size: Vec2::new(1.0, 1.0), flip: false})),
            material: material2,
            transform: Transform {
                translation: Vec3::new(0.0, -1.0, 0.0),
                ..Default::default()
            },
            draw: Draw {
                is_transparent: false,
                ..Default::default()
            },
            ..Default::default()
        })
        // camera
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(-3.0, 0.0, 3.0))
                .looking_at(Vec3::new(2.0, 0.0, 0.0), Vec3::unit_y()),
            ..Default::default()
        });
}
