use std::borrow::Cow;

use bevy::render::render_graph::Node;
use bevy::render::{
    render_graph::RenderGraph,
    texture::{Extent3d, TextureFormat},
};
use bevy::{
    prelude::*,
    render::{
        pass::{
            LoadOp, Operations, PassDescriptor, RenderPassColorAttachmentDescriptor,
            TextureAttachment,
        },
        pipeline::{CullMode, PipelineDescriptor},
        render_graph::{
            base::node::MAIN_PASS, AssetRenderResourcesNode, PassNode, RenderResourcesNode,
            ResourceSlotInfo,
        },
        renderer::{RenderResourceId, RenderResourceType},
        shader::{ShaderStage, ShaderStages},
    },
};
use bevy::{
    reflect::TypeUuid,
    render::{camera::ActiveCameras, render_graph::CameraNode, texture::TextureDimension},
};

use crate::render_resources::{AnticLine, AtariPalette};

pub const ANTIC_PASS: &str = "antic_pass";
pub const ANTIC_CAMERA: &str = "antic_camera";

pub const ANTIC_TEXTURE: &str = "antic_texture";
pub const ANTIC_TEXTURE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Texture::TYPE_UUID, 13378939762009864026);

const VERTEX_SHADER: &str = include_str!("antic.vert");
const FRAGMENT_SHADER: &str = include_str!("antic.frag");

pub fn build_antic_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
        descr.cull_mode = CullMode::None;
    }
    pipeline_descr.depth_stencil_state = None;
    pipeline_descr
}

pub trait AnticRendererGraphBuilder {
    fn add_antic_graph(&mut self, resources: &Resources, texture_size: &Vec2) -> &mut Self;
}

impl AnticRendererGraphBuilder for RenderGraph {
    fn add_antic_graph(&mut self, resources: &Resources, texture_size: &Vec2) -> &mut Self {
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        let mut palettes = resources.get_mut::<Assets<AtariPalette>>().unwrap();
        let mut textures = resources.get_mut::<Assets<Texture>>().unwrap();
        let mut active_cameras = resources.get_mut::<ActiveCameras>().unwrap();
        active_cameras.add(ANTIC_CAMERA);
        let mut pass_node = PassNode::<&AnticLine>::new(PassDescriptor {
            color_attachments: vec![RenderPassColorAttachmentDescriptor {
                attachment: TextureAttachment::Input("color_attachment".to_string()),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::rgb(0.1, 0.2, 0.1)),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            sample_count: 1,
        });

        let texture = Texture::new(
            Extent3d::new(texture_size.x as u32, texture_size.y as u32, 1),
            TextureDimension::D2,
            vec![],
            TextureFormat::Rgba8Uint,
        );
        textures.set_untracked(ANTIC_TEXTURE_HANDLE, texture);

        self.add_system_node(ANTIC_CAMERA, CameraNode::new(ANTIC_CAMERA));

        self.add_node(
            ANTIC_TEXTURE,
            TextureNode::new(ANTIC_TEXTURE_HANDLE.typed()),
        );

        pass_node.add_camera(ANTIC_CAMERA);
        self.add_node(ANTIC_PASS, pass_node);

        self.add_node_edge(ANTIC_CAMERA, ANTIC_PASS).unwrap();

        self.add_slot_edge(
            ANTIC_TEXTURE,
            TextureNode::TEXTURE,
            ANTIC_PASS,
            "color_attachment",
        )
        .unwrap();
        self.add_node_edge(ANTIC_TEXTURE, ANTIC_PASS).unwrap();
        self.add_node_edge(ANTIC_PASS, MAIN_PASS).unwrap();
        self.add_node_edge("transform", ANTIC_PASS).unwrap();
        // Create a new shader pipeline
        pipelines.set_untracked(
            super::ANTIC_PIPELINE_HANDLE,
            build_antic_pipeline(&mut shaders),
        );
        // Add an AssetRenderResourcesNode to our Render Graph. This will bind AnticCharset resources to our shader
        self.add_system_node(
            "atari_palette",
            AssetRenderResourcesNode::<AtariPalette>::new(false),
        );
        self.add_node_edge("atari_palette", ANTIC_PASS).unwrap();

        // Add an AssetRenderResourcesNode to our Render Graph. This will bind AnticLine resources to our shader
        self.add_system_node("antic_line", RenderResourcesNode::<AnticLine>::new(true));

        // Add a Render Graph edge connecting our new "antic_line" node to the main pass node. This ensures "antic_line" runs before the main pass
        self.add_node_edge("antic_line", ANTIC_PASS).unwrap();

        palettes.set_untracked(super::ATARI_PALETTE_HANDLE, AtariPalette::default());
        self
    }
}

pub struct TextureNode {
    texture_handle: Handle<Texture>,
}

impl TextureNode {
    pub const TEXTURE: &'static str = "texture";
    pub fn new(texture_handle: Handle<Texture>) -> Self {
        Self { texture_handle }
    }
}

impl Node for TextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(TextureNode::TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        render_context: &mut dyn bevy::render::renderer::RenderContext,
        _input: &bevy::render::render_graph::ResourceSlots,
        output: &mut bevy::render::render_graph::ResourceSlots,
    ) {
        let render_resource_context = render_context.resources_mut();
        if let Some(texture_id) = render_resource_context
            .get_asset_resource_untyped(self.texture_handle.clone_weak_untyped(), 0)
            .and_then(|x| x.get_texture())
        {
            output.set(0, RenderResourceId::Texture(texture_id));
        }
    }
}
