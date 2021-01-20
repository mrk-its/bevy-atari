use std::borrow::Cow;

use bevy::render::{
    pipeline::PrimitiveTopology,
    render_graph::{Node, PassNode},
};
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
        render_graph::{base::node::MAIN_PASS, ResourceSlotInfo},
        renderer::{RenderResourceId, RenderResourceType},
        shader::{ShaderStage, ShaderStages},
    },
};
use bevy::{
    reflect::TypeUuid,
    render::{camera::ActiveCameras, render_graph::CameraNode, texture::TextureDimension},
};

use crate::render_resources::AnticLine;

pub const ANTIC_PASS: &str = "antic_pass";
pub const ANTIC_CAMERA: &str = "antic_camera";
pub const ANTIC_TEXTURE: &str = "antic_texture";
pub const LOAD_COLLISIONS_PASS: &str = "load_collisions_pass";
pub const COLLISIONS_AGG_PASS: &str = "collisions_agg_pass";
pub const COLLISIONS_AGG_CAMERA: &str = "collisions_agg_camera";

pub const COLLISIONS_TEXTURE: &str = "collisions_texture";
pub const COLLISIONS_AGG_TEXTURE: &str = "collisions_agg_texture";

pub const ANTIC_TEXTURE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Texture::TYPE_UUID, 13378939762009864026);

pub const COLLISIONS_TEXTURE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Texture::TYPE_UUID, 13378939762009864077);

pub const COLLISIONS_AGG_TEXTURE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Texture::TYPE_UUID, 13378939762609864078);

const VERTEX_SHADER: &str = include_str!("antic.vert");
const FRAGMENT_SHADER: &str = include_str!("antic.frag");
const COLLISIONS_FRAGMENT_SHADER: &str = include_str!("collisions.frag");
const COLLISIONS_VERTEX_SHADER: &str = include_str!("collisions.vert");

pub fn build_antic_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });

    if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
        descr.cull_mode = CullMode::None;
    }
    pipeline_descr.name = Some("ANTIC".to_string());
    pipeline_descr.depth_stencil_state = None;
    pipeline_descr
}

pub fn build_collisions_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(
            ShaderStage::Vertex,
            COLLISIONS_VERTEX_SHADER,
        )),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            COLLISIONS_FRAGMENT_SHADER,
        ))),
    });
    pipeline_descr.name = Some("COLLISIONS".to_string());

    if let Some(descr) = pipeline_descr.rasterization_state.as_mut() {
        descr.cull_mode = CullMode::None;
    }

    pipeline_descr.depth_stencil_state = None;
    pipeline_descr.primitive_topology = PrimitiveTopology::PointList;
    info!("created pipeline: {:?}", pipeline_descr);
    pipeline_descr
}

pub trait AnticRendererGraphBuilder {
    fn add_antic_graph(&mut self, resources: &Resources, texture_size: &Vec2, enable_collisions: bool) -> &mut Self;
}

impl AnticRendererGraphBuilder for RenderGraph {
    fn add_antic_graph(&mut self, resources: &Resources, texture_size: &Vec2, enable_collisions: bool) -> &mut Self {
        let mut textures = resources.get_mut::<Assets<Texture>>().unwrap();
        let mut active_cameras = resources.get_mut::<ActiveCameras>().unwrap();
        let mut pass_order: Vec<&str> = Vec::new();

        pass_order.push(ANTIC_PASS);

        active_cameras.add(ANTIC_CAMERA);

        let mut color_attachments = vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color_attachment".to_string()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::rgb(0.0, 0.0, 0.0)),
                store: true,
            },
        }];
        if enable_collisions {
            active_cameras.add(COLLISIONS_AGG_CAMERA);
            pass_order.push(COLLISIONS_AGG_PASS);
            color_attachments.push(RenderPassColorAttachmentDescriptor {
                attachment: TextureAttachment::Input("collisions_attachment".to_string()),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::rgba(0.0, 0.0, 0.0, 0.0)), // TODO - remove?
                    store: true,
                },
            })
        }

        let mut pass_node = PassNode::<&AnticLine>::new(PassDescriptor {
            color_attachments,
            depth_stencil_attachment: None,
            sample_count: 1,
        });

        let texture = Texture::new(
            Extent3d::new(texture_size.x as u32, texture_size.y as u32, 1),
            TextureDimension::D2,
            vec![],
            TextureFormat::Rgba8Unorm,
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

        if enable_collisions {
            let mut collisions_agg_pass_node =
                PassNode::<&super::entities::CollisionsAggPass>::new(PassDescriptor {
                    color_attachments: vec![RenderPassColorAttachmentDescriptor {
                        attachment: TextureAttachment::Input("collisions_attachment".to_string()),
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load, // TODO - remove?
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                    sample_count: 1,
                });
            collisions_agg_pass_node.add_camera(COLLISIONS_AGG_CAMERA);
            self.add_node(COLLISIONS_AGG_PASS, collisions_agg_pass_node);
            self.add_system_node(
                COLLISIONS_AGG_CAMERA,
                CameraNode::new(COLLISIONS_AGG_CAMERA),
                );
            self.add_node_edge(COLLISIONS_AGG_CAMERA, COLLISIONS_AGG_PASS)
                .unwrap();

            let collisions_texture = Texture::new(
                Extent3d::new(texture_size.x as u32, texture_size.y as u32, 1),
                TextureDimension::D2,
                vec![],
                TextureFormat::Rgba16Uint,
            );
            textures.set_untracked(COLLISIONS_TEXTURE_HANDLE, collisions_texture);

            let collisions_agg_texture = Texture::new(
                Extent3d::new(texture_size.x as u32, 1, 1),
                TextureDimension::D2,
                vec![],
                TextureFormat::Rgba16Uint,
            );
            textures.set_untracked(COLLISIONS_AGG_TEXTURE_HANDLE, collisions_agg_texture);

            self.add_node(
                COLLISIONS_TEXTURE,
                TextureNode::new(COLLISIONS_TEXTURE_HANDLE.typed()),
            );
            self.add_node(
                COLLISIONS_AGG_TEXTURE,
                TextureNode::new(COLLISIONS_AGG_TEXTURE_HANDLE.typed()),
            );
            self.add_node(LOAD_COLLISIONS_PASS, LoadCollisionsPass);
            pass_order.push(LOAD_COLLISIONS_PASS);
            self.add_slot_edge(
                COLLISIONS_TEXTURE,
                TextureNode::TEXTURE,
                ANTIC_PASS,
                "collisions_attachment",
            )
            .unwrap();
            self.add_slot_edge(
                COLLISIONS_AGG_TEXTURE,
                TextureNode::TEXTURE,
                COLLISIONS_AGG_PASS,
                "collisions_attachment",
            )
            .unwrap();
            self.add_node_edge(COLLISIONS_TEXTURE, ANTIC_PASS).unwrap();
            self.add_node_edge(COLLISIONS_AGG_TEXTURE, COLLISIONS_AGG_PASS)
                .unwrap();
        }

        pass_order.push(MAIN_PASS);

        for (i, &pass_name) in pass_order[..pass_order.len() - 1].iter().enumerate() {
            self.add_node_edge(pass_name, pass_order[i + 1]).unwrap();
        }

        self.add_node_edge("transform", ANTIC_PASS).unwrap();
        self.add_node_edge("atari_palette", ANTIC_PASS).unwrap();
        self.add_node_edge("antic_line", ANTIC_PASS).unwrap();

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

pub struct LoadCollisionsPass;

impl Node for LoadCollisionsPass {
    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn bevy::render::renderer::RenderContext,
        _input: &bevy::render::render_graph::ResourceSlots,
        _output: &mut bevy::render::render_graph::ResourceSlots,
    ) {
        let mut buffer: Vec<u32> = Vec::with_capacity(384 * 1 * 4);
        unsafe {
            buffer.set_len(buffer.capacity());
        }
        render_context.read_pixels_u32(0, 0, 0, 384, 1, &mut buffer);
        let mut dst: [u32; 4] = [0; 4];

        for (i, b) in buffer.iter().enumerate() {
            dst[i & 3] |= *b;
        }
        if dst[0] > 0 || dst[1] > 0 || dst[2] > 0 || dst[3] > 0 {
            let mut atari_system = resources.get_mut::<crate::AtariSystem>().unwrap();
            atari_system.gtia.update_collisions(dst);
        }
    }
}
