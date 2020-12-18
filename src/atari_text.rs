use bevy::asset::Handle;
use bevy::core::Bytes;
use bevy::prelude::Color;
use bevy::render::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};
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

pub const ATARI_TEXT_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 2785347777738765446);

pub fn create_atari_text_pipeline(
    render_graph: &mut RenderGraph,
    shaders: &mut Assets<Shader>,
    pipelines: &mut Assets<PipelineDescriptor>,
) {
    // Add an AssetRenderResourcesNode to our Render Graph. This will bind AnticLine resources to our shader
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
    info!("text_pipeline_descr: {:#?}", pipeline_descr);
    pipelines.set(ATARI_TEXT_PIPELINE_HANDLE, pipeline_descr);
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextAreaData {
    pub data: [u8; 1024],
}

unsafe impl Byteable for TextAreaData {}
impl_render_resource_bytes!(TextAreaData);

#[derive(RenderResources, TypeUuid)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b250000"]
pub struct TextArea {
    pub width: f32,
    pub height: f32,
    pub fg_color: Color,
    pub bg_color: Color,
    pub data: TextAreaData,
    pub charset: Charset,
}
