use bevy::core::Bytes;
use bevy::prelude::Color;
use bevy::render::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};
use bevy::render::{
    pipeline::{CullMode, PipelineDescriptor},
    render_graph::{base, RenderGraph, RenderResourcesNode},
    shader::{ShaderStage, ShaderStages},
};
use bevy::sprite::QUAD_HANDLE;
use bevy::{
    asset::Handle,
    render::{pipeline::RenderPipeline, render_graph::base::MainPass},
};
use bevy::{core::Byteable, reflect::TypeUuid};
use bevy::{prelude::*, render::renderer::RenderResources};

use crate::render_resources::Charset;

const VERTEX_SHADER: &str = include_str!("text.vert");
const FRAGMENT_SHADER: &str = include_str!("text.frag");
pub const CHARSET_DATA: &[u8] = include_bytes!("charset.dat");

pub const ATARI_TEXT_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 2785347777738765446);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextAreaData {
    pub data: [u8; 1024],
}

impl Default for TextAreaData {
    fn default() -> Self {
        Self { data: [0; 1024] }
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

pub fn atascii_to_screen(text: &str, inv: bool) -> Vec<u8> {
    text.as_bytes()
        .iter()
        .map(|c| match *c {
            0x00..=0x1f => *c + 0x40,
            0x20..=0x5f => *c - 0x20,
            _ => *c,
        } + (inv as u8) * 128)
        .collect()
}

impl TextArea {
    pub fn set_text(&mut self, text: &str) {
        let data = atascii_to_screen(text, false);
        self.data.data[..data.len()].copy_from_slice(&data);
    }
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
    pub fn new(width: i32, height: i32, x_offset: i32, y_offset: i32) -> TextAreaBundle {
        let width = width as f32;
        let height = height as f32;
        let mut charset = Charset::default();
        charset.data.extend_from_slice(CHARSET_DATA);
        TextAreaBundle {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                ATARI_TEXT_PIPELINE_HANDLE.typed(),
            )]),
            transform: Transform {
                translation: Vec3::new(
                    (x_offset as f32 + width / 2.0) * 8.0,
                    (y_offset as f32 - height / 2.0) * 8.0,
                    0.2,
                ),
                scale: Vec3::new(1.0 * (width as f32) * 8.0, 1.0 * (height as f32) * 8.0, 1.0),
                ..Default::default()
            },
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
                charset,
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

        let world = app.world_mut().cell();
        let mut render_graph = world.get_resource_mut::<RenderGraph>().unwrap();
        let mut pipelines = world.get_resource_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();

        render_graph.add_system_node("atari_text", RenderResourcesNode::<TextArea>::new(true));

        render_graph
            .add_node_edge("atari_text", base::node::MAIN_PASS)
            .unwrap();

        let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
            fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
        });
        pipeline_descr.primitive.cull_mode = CullMode::None;
        pipelines.set_untracked(ATARI_TEXT_PIPELINE_HANDLE, pipeline_descr);
    }
}
