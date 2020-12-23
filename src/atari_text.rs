use bevy::{asset::Handle, render::{pipeline::RenderPipeline, render_graph::base::MainPass}};
use bevy::core::Bytes;
use bevy::prelude::Color;
use bevy::render::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};
use bevy::sprite::QUAD_HANDLE;
use bevy::{core::Byteable, reflect::TypeUuid};
use bevy::{prelude::*, render::renderer::RenderResources};
use bevy::{
    render::{
        pipeline::{CullMode, PipelineDescriptor},
        render_graph::{base, RenderGraph, RenderResourcesNode},
        shader::{ShaderStage, ShaderStages},
    },
};

use crate::render_resources::Charset;

const VERTEX_SHADER: &str = include_str!("shaders/antic.vert");
const FRAGMENT_SHADER: &str = include_str!("shaders/text.frag");
pub const CHARSET_DATA: &[u8] = include_bytes!("../assets/charset.dat");

pub const ATARI_TEXT_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 2785347777738765446);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextAreaData {
    pub data: [u8; 1024],
}

impl Default for TextAreaData {
    fn default() -> Self {
        Self {
            data: [0; 1024]
        }
    }
}

unsafe impl Byteable for TextAreaData {}
impl_render_resource_bytes!(TextAreaData);

#[derive(RenderResources, TypeUuid, Default)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b250000"]
pub struct TextArea {
    pub width: f32,
    pub height: f32,
    pub fg_color: Color,
    pub bg_color: Color,
    pub data: TextAreaData,
    pub charset: Charset,
}

#[derive(Bundle, Default)]
pub struct TextAreaBundle {
    pub mesh: Handle<Mesh>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub text_area: TextArea,
}

impl TextAreaBundle {
    pub fn new(width: f32, height: f32, x_offset: f32, y_offset: f32) -> TextAreaBundle {
        TextAreaBundle {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                ATARI_TEXT_PIPELINE_HANDLE.typed(),
            )]),
            transform: Transform::from_translation(Vec3::new(
                x_offset,
                y_offset,
                0.2,
            ))
            .mul_transform(Transform::from_scale(Vec3::new(
                1.0 * width * 8.0,
                1.0 * height * 8.0,
                1.0,
            ))),
            visible: Visible {
                is_visible: false,
                is_transparent: true,
            },
            text_area: TextArea {
                width,
                height,
                fg_color: Color::rgba_u8(0x00, 0xff, 0, 0xff),
                bg_color: Color::rgba_u8(0x00, 0x40, 0, 0xe0),
                data: TextAreaData { data: [0; 1024] },
                charset: Charset::new(CHARSET_DATA),
            },
            ..Default::default()
        }
    }
}


#[derive(Default)]
pub struct AtartTextPlugin;

// pub const QUAD_HANDLE: HandleUntyped =
//     HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 16824195407667777934);

impl Plugin for AtartTextPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // app.add_asset::<ColorMaterial>()
        //     .add_asset::<TextureAtlas>()
        //     .register_type::<Sprite>();

        let resources = app.resources_mut();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();

        render_graph.add_system_node("atari_text", RenderResourcesNode::<TextArea>::new(true));

        // Add a Render Graph edge connecting our new "antic_line" node to the main pass node. This ensures "antic_line" runs before the main pass
        render_graph
            .add_node_edge("atari_text", base::node::MAIN_PASS)
            .unwrap();

        let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
            fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
        });

        if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
            descr.cull_mode = CullMode::None;
        }
        pipelines.set_untracked(ATARI_TEXT_PIPELINE_HANDLE, pipeline_descr);
    }
}
