use std::borrow::Cow;

use crate::{
    entities,
    render_resources::{AnticData, AtariPalette, CustomTexture, SimpleMaterial},
    MainCamera,
};
use bevy::render::{
    pipeline::RenderPipeline,
    render_graph::{
        base::{node::MAIN_PASS, MainPass},
        AssetRenderResourcesNode,
    },
    renderer::RenderResourceContext,
};
use bevy::render::{
    render_graph::RenderGraph,
    texture::{Extent3d, TextureFormat},
};
use bevy::render::{
    render_graph::{Node, PassNode},
    renderer::{BufferId, BufferInfo, BufferUsage},
};
use bevy::{
    prelude::*,
    render::{
        pass::{LoadOp, Operations, PassDescriptor, RenderPassColorAttachment, TextureAttachment},
        pipeline::PipelineDescriptor,
        render_graph::ResourceSlotInfo,
        renderer::{RenderResourceId, RenderResourceType},
        shader::{ShaderStage, ShaderStages},
    },
};
use bevy::{
    reflect::TypeUuid,
    render::{camera::ActiveCameras, render_graph::CameraNode, texture::TextureDimension},
};

pub const RED_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 11482402499638723727);

pub const ATARI_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 11482402499638723728);

pub const COLLISIONS_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(CustomTexture::TYPE_UUID, 11482402411138723729);

pub const ANTIC_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 16056864393442354012);
pub const ANTIC_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 18422387557214033949);

pub const DATA_MATERIAL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(SimpleMaterial::TYPE_UUID, 18422387557214033950);

pub const DATA_TEXTURE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Texture::TYPE_UUID, 18422387557214033951);

pub const COLLISION_AGG_SIZE: Option<(u32, u32)> = Some((16, 240));

pub const ATARI_PALETTE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(AtariPalette::TYPE_UUID, 5197421896076365082);

pub const ANTIC_DATA_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(AnticData::TYPE_UUID, 11338886280454987747);

pub const COLLISIONS_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 6758940903835595297);

pub const DEBUG_COLLISIONS_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 12701505191960931865);

pub struct AnticFrame;

pub const ANTIC_PASS: &str = "antic_pass";
pub const ANTIC_CAMERA: &str = "antic_camera";
pub const ANTIC_TEXTURE: &str = "antic_texture";
pub const LOAD_COLLISIONS_PASS: &str = "load_collisions_pass";
pub const COLLISIONS_AGG_PASS: &str = "collisions_agg_pass";
pub const COLLISIONS_AGG_CAMERA: &str = "collisions_agg_camera";
pub const COLLISIONS_BUFFER: &str = "collisions_buffer";

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

pub fn build_antic2_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });

    pipeline_descr.color_target_states = Vec::new();
    pipeline_descr.primitive.cull_mode = None;
    pipeline_descr.name = Some("ANTIC2".to_string());
    pipeline_descr.depth_stencil = None;
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
    pipeline_descr.color_target_states = Vec::new();
    pipeline_descr.name = Some("COLLISIONS".to_string());
    pipeline_descr.primitive.cull_mode = None;
    pipeline_descr.depth_stencil = None;
    info!("created pipeline: {:?}", pipeline_descr);
    pipeline_descr
}

pub fn build_debug_collisions_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    let mut pipeline_descr = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(
            ShaderStage::Vertex,
            COLLISIONS_VERTEX_SHADER,
        )),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            include_str!("debug_collisions.frag"),
        ))),
    });
    pipeline_descr.color_target_states = Vec::new();
    pipeline_descr.name = Some("DEBUG_COLLISIONS".to_string());
    pipeline_descr.primitive.cull_mode = None;
    pipeline_descr.depth_stencil = None;
    info!("created pipeline: {:?}", pipeline_descr);
    pipeline_descr
}

pub fn add_antic_graph(
    graph: &mut RenderGraph,
    world: &bevy::ecs::world::WorldCell,
    texture_size: &Vec2,
    enable_collisions: bool,
    collision_agg_size: Option<(u32, u32)>,
) {
    let mut textures = world.get_resource_mut::<Assets<Texture>>().unwrap();
    let mut active_cameras = world.get_resource_mut::<ActiveCameras>().unwrap();
    let mut pass_order: Vec<&str> = Vec::new();

    pass_order.push(ANTIC_PASS);

    active_cameras.add(ANTIC_CAMERA);

    let mut color_attachments = vec![RenderPassColorAttachment {
        attachment: TextureAttachment::Input("color_attachment".to_string()),
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(Color::rgb(0.0, 0.0, 0.0)),
            store: true,
        },
    }];
    if enable_collisions {
        color_attachments.push(RenderPassColorAttachment {
            attachment: TextureAttachment::Input("collisions_attachment".to_string()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::rgba(0.0, 0.0, 0.0, 0.0)), // TODO - remove?
                store: true,
            },
        })
    }

    let mut pass_node = PassNode::<&AnticFrame>::new(PassDescriptor {
        color_attachments,
        depth_stencil_attachment: None,
        sample_count: 1,
    });

    let texture = Texture::new(
        Extent3d::new(texture_size.x as u32, texture_size.y as u32, 1),
        TextureDimension::D2,
        vec![0; (texture_size.x * texture_size.y * 4.0) as usize],
        TextureFormat::Rgba8Unorm,
    );
    textures.set_untracked(ANTIC_TEXTURE_HANDLE, texture);

    graph.add_system_node(ANTIC_CAMERA, CameraNode::new(ANTIC_CAMERA));

    graph.add_node(
        ANTIC_TEXTURE,
        TextureNode::new(ANTIC_TEXTURE_HANDLE.typed()),
    );

    pass_node.add_camera(ANTIC_CAMERA);

    graph.add_node(ANTIC_PASS, pass_node);

    graph.add_node_edge(ANTIC_CAMERA, ANTIC_PASS).unwrap();

    graph
        .add_slot_edge(
            ANTIC_TEXTURE,
            TextureNode::TEXTURE,
            ANTIC_PASS,
            "color_attachment",
        )
        .unwrap();
    graph.add_node_edge(ANTIC_TEXTURE, ANTIC_PASS).unwrap();
    graph.add_node("data_texture_update", UpdateDataTextureNode::default());
    graph
        .add_node_edge("data_texture_update", ANTIC_PASS)
        .unwrap();
    if enable_collisions {
        let texture_format = TextureFormat::Rg32Uint;

        let collisions_texture = Texture::new(
            Extent3d::new(texture_size.x as u32, texture_size.y as u32, 1),
            TextureDimension::D2,
            vec![0; (texture_size.x * texture_size.y * 8.0) as usize],
            texture_format,
        );
        textures.set_untracked(COLLISIONS_TEXTURE_HANDLE, collisions_texture);
        graph.add_node(
            COLLISIONS_TEXTURE,
            TextureNode::new(COLLISIONS_TEXTURE_HANDLE.typed()),
        );
        graph.add_node_edge(COLLISIONS_TEXTURE, ANTIC_PASS).unwrap();
        graph
            .add_slot_edge(
                COLLISIONS_TEXTURE,
                TextureNode::TEXTURE,
                ANTIC_PASS,
                "collisions_attachment",
            )
            .unwrap();

        let (index, width, height) = if let Some((width, height)) = collision_agg_size {
            let mut collisions_agg_pass_node =
                PassNode::<&super::entities::CollisionsAggPass>::new(PassDescriptor {
                    color_attachments: vec![RenderPassColorAttachment {
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
            active_cameras.add(COLLISIONS_AGG_CAMERA);
            pass_order.push(COLLISIONS_AGG_PASS);
            collisions_agg_pass_node.add_camera(COLLISIONS_AGG_CAMERA);
            graph.add_node(COLLISIONS_AGG_PASS, collisions_agg_pass_node);
            graph.add_system_node(
                COLLISIONS_AGG_CAMERA,
                CameraNode::new(COLLISIONS_AGG_CAMERA),
            );
            graph
                .add_node_edge(COLLISIONS_AGG_CAMERA, COLLISIONS_AGG_PASS)
                .unwrap();
            let collisions_agg_texture = Texture::new(
                Extent3d::new(width as u32, height, 1),
                TextureDimension::D2,
                vec![0; (width * height * 8) as usize],
                texture_format,
            );
            textures.set_untracked(COLLISIONS_AGG_TEXTURE_HANDLE, collisions_agg_texture);

            graph.add_node(
                COLLISIONS_AGG_TEXTURE,
                TextureNode::new(COLLISIONS_AGG_TEXTURE_HANDLE.typed()),
            );
            graph
                .add_slot_edge(
                    COLLISIONS_AGG_TEXTURE,
                    TextureNode::TEXTURE,
                    COLLISIONS_AGG_PASS,
                    "collisions_attachment",
                )
                .unwrap();
            graph
                .add_node_edge(COLLISIONS_AGG_TEXTURE, COLLISIONS_AGG_PASS)
                .unwrap();
            (0, width, height)
        } else {
            (1, texture_size.x as u32, texture_size.y as u32)
        };

        graph.add_node(
            COLLISIONS_BUFFER,
            CollisionsBufferNode {
                buffer_info: BufferInfo {
                    size: width as usize * height as usize * 16,
                    buffer_usage: BufferUsage::COPY_DST | BufferUsage::INDIRECT,
                    mapped_at_creation: false,
                },
                buffer_id: None,
            },
        );
        graph.add_node(
            LOAD_COLLISIONS_PASS,
            LoadCollisionsPass {
                index,
                width,
                height,
                texture_format,
                texture_handle: COLLISIONS_AGG_TEXTURE_HANDLE.typed(),
            },
        );
        graph
            .add_node_edge(COLLISIONS_BUFFER, LOAD_COLLISIONS_PASS)
            .unwrap();
        graph
            .add_slot_edge(COLLISIONS_BUFFER, "buffer", LOAD_COLLISIONS_PASS, "buffer")
            .unwrap();
        pass_order.push(LOAD_COLLISIONS_PASS);
    }
    pass_order.push(MAIN_PASS);

    info!("pass_order: {:?}", pass_order);

    for (i, &pass_name) in pass_order[..pass_order.len() - 1].iter().enumerate() {
        graph.add_node_edge(pass_name, pass_order[i + 1]).unwrap();
    }

    graph.add_node_edge("transform", ANTIC_PASS).unwrap();
    graph.add_node_edge("atari_palette", ANTIC_PASS).unwrap();
    graph.add_node_edge("antic_data", ANTIC_PASS).unwrap();
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
#[derive(Default)]
pub struct CollisionsBufferNode {
    pub buffer_info: BufferInfo,
    pub buffer_id: Option<BufferId>,
}

impl CollisionsBufferNode {
    pub const BUFFER: &'static str = "buffer";
}

impl Node for CollisionsBufferNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(CollisionsBufferNode::BUFFER),
            resource_type: RenderResourceType::Buffer,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn bevy::render::renderer::RenderContext,
        _input: &bevy::render::render_graph::ResourceSlots,
        output: &mut bevy::render::render_graph::ResourceSlots,
    ) {
        let render_resource_context = render_context.resources_mut();
        if self.buffer_id.is_none() {
            let buffer_id = render_resource_context.create_buffer(self.buffer_info.clone());
            self.buffer_id = Some(buffer_id);
            output.set(
                CollisionsBufferNode::BUFFER,
                RenderResourceId::Buffer(buffer_id),
            );
        }
    }
}

#[allow(dead_code)]
pub struct LoadCollisionsPass {
    index: u32,
    width: u32,
    height: u32,
    texture_format: TextureFormat,
    texture_handle: Handle<Texture>,
}

impl Node for LoadCollisionsPass {
    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn bevy::render::renderer::RenderContext,
        input: &bevy::render::render_graph::ResourceSlots,
        _output: &mut bevy::render::render_graph::ResourceSlots,
    ) {
        let render_resource_context = render_context.resources_mut();
        let buffer_id = input.get("buffer").unwrap().get_buffer().unwrap();
        if let Some(texture_id) = render_resource_context
            .get_asset_resource_untyped(self.texture_handle.clone_weak_untyped(), 0)
            .and_then(|x| x.get_texture())
        {
            render_context.copy_texture_to_buffer(
                texture_id,
                [0, 0, 0],
                0,
                buffer_id,
                0,
                0,
                Extent3d::new(self.width, self.height, 0),
            );
        }
    }

    fn input(&self) -> &[ResourceSlotInfo] {
        static INPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed("buffer"),
            resource_type: RenderResourceType::Buffer,
        }];
        INPUT
    }

    fn output(&self) -> &[ResourceSlotInfo] {
        &[]
    }
}

#[derive(Default)]
pub struct UpdateDataTextureNode {
    pub buffer_id: Option<BufferId>,
}

impl UpdateDataTextureNode {
    pub const BUFFER: &'static str = "buffer";
}

impl Node for UpdateDataTextureNode {
    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn bevy::render::renderer::RenderContext,
        _input: &bevy::render::render_graph::ResourceSlots,
        _output: &mut bevy::render::render_graph::ResourceSlots,
    ) {
        let render_resource_context = render_context.resources_mut();

        if self.buffer_id.is_none() {
            let buffer_info = BufferInfo {
                size: 11 * 256 * 4 * 4,
                buffer_usage: BufferUsage::MAP_WRITE | BufferUsage::COPY_SRC,
                mapped_at_creation: false,
            };
            let buffer_id = render_resource_context.create_buffer(buffer_info);
            self.buffer_id = Some(buffer_id);
            info!("created texture buffer!");
        }
        let antic_data_assets = world.get_resource::<Assets<AnticData>>().unwrap();
        let antic_data = antic_data_assets.get(ANTIC_DATA_HANDLE).unwrap();
        let len = antic_data.texture_data.len() as u64;
        render_resource_context.write_mapped_buffer(
            self.buffer_id.unwrap(),
            0..len,
            &mut |data, _renderer| data.copy_from_slice(&antic_data.texture_data),
        );
        let texture = render_resource_context
            .get_asset_resource_untyped(DATA_TEXTURE_HANDLE, 0)
            .unwrap();
        render_context.copy_buffer_to_texture(
            self.buffer_id.unwrap(),
            0,
            0,
            texture.get_texture().unwrap(),
            [0, 0, 0],
            0,
            Extent3d::new(256, 11, 1),
        )
    }
}

#[derive(Default)]
struct CollistionsReadState {
    buffer: Vec<u8>,
}

fn collisions_read(world: &mut World) {
    let world = world.cell();
    let mut state = world.get_resource_mut::<CollistionsReadState>().unwrap();
    let render_graph = world.get_resource_mut::<RenderGraph>().unwrap();
    let render_resource_context = world.get_resource_mut::<Box<dyn RenderResourceContext>>();
    if let Some(render_resource_context) = render_resource_context {
        let collisions_buffer_node: &CollisionsBufferNode =
            render_graph.get_node(COLLISIONS_BUFFER).unwrap();
        if state.buffer.len() != collisions_buffer_node.buffer_info.size {
            state.buffer = Vec::with_capacity(collisions_buffer_node.buffer_info.size);
            unsafe {
                state
                    .buffer
                    .set_len(collisions_buffer_node.buffer_info.size);
            }
        }
        if let Some(buffer_id) = collisions_buffer_node.buffer_id {
            let atari_system = world.get_resource::<crate::AtariSystem>().unwrap();
            render_resource_context.read_mapped_buffer(
                buffer_id,
                0..(state.buffer.len() as u64),
                &|data, _| {
                    let data = unsafe { std::mem::transmute::<&[u8], &[u64]>(&data) };
                    // collision texture is RG texture, but we read it in RGBA format (4 * u32)
                    // where only RG components are set. That's why we skip every second u64
                    let len = data.len() / 8;
                    let data = &data[..len];
                    let collision_array = &mut *atari_system.gtia.collision_array.write();
                    let width = len / 240 / 2;
                    let mut index = 0;
                    for i in 0..240 {
                        let mut agg = 0;
                        for _ in 0..width {
                            agg |= data[index];
                            index += 2;
                        }
                        collision_array[i] = agg;
                    }
                },
            );
        }
    }
}

pub const ANTIC_TEXTURE_SIZE: (f32, f32) = (384.0, 240.0);

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut materials: ResMut<Assets<SimpleMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut textures: ResMut<Assets<CustomTexture>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut tex: ResMut<Assets<Texture>>,
) {
    // standard_materials.set_untracked(RED_MATERIAL_HANDLE, Color::rgba(1.0, 0.0, 0.0, 1.0).into());

    // 30 * 1024 - max charset memory
    // 48 * 240 - max video memory
    // total: 42240
    // 42240 / (256 * 4 * 4) = 10.3125

    let texture = Texture::new_fill(
        Extent3d::new(256, 11, 1),
        TextureDimension::D2,
        &[0; 16],
        TextureFormat::Rgba32Uint,
    );

    tex.set_untracked(DATA_TEXTURE_HANDLE, texture);

    materials.set_untracked(
        DATA_MATERIAL_HANDLE,
        SimpleMaterial {
            base_color_texture: Some(DATA_TEXTURE_HANDLE.typed()),
            ..Default::default()
        },
    );

    standard_materials.set_untracked(
        ATARI_MATERIAL_HANDLE,
        StandardMaterial {
            base_color_texture: Some(ANTIC_TEXTURE_HANDLE.typed()),
            unlit: true,
            ..Default::default()
        },
    );

    textures.set_untracked(
        COLLISIONS_MATERIAL_HANDLE,
        CustomTexture {
            color: Color::rgba(0.0, 1.0, 0.0, 1.0),
            texture: Some(COLLISIONS_TEXTURE_HANDLE.typed()),
        },
    );

    commands.spawn_bundle(entities::create_antic_camera(Vec2::new(
        ANTIC_TEXTURE_SIZE.0,
        ANTIC_TEXTURE_SIZE.1,
    )));
    if let Some((width, height)) = COLLISION_AGG_SIZE {
        commands.spawn_bundle(entities::create_collisions_camera(Vec2::new(
            width as f32,
            height as f32,
        )));

        let mesh = Mesh::from(shape::Quad::new(Vec2::new(width as f32, height as f32)));
        let mesh_handle = meshes.add(mesh);
        let bundle = entities::CollisionsAggBundle {
            mesh: mesh_handle,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                COLLISIONS_PIPELINE_HANDLE.typed(),
            )]),
            texture: COLLISIONS_MATERIAL_HANDLE.typed(),
            ..Default::default()
        };

        info!("bundle: {:?}", bundle.render_pipelines);
        commands.spawn_bundle(bundle);
    }

    let mesh_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        ANTIC_TEXTURE_SIZE.0,
        ANTIC_TEXTURE_SIZE.1,
    ))));

    commands.spawn_bundle(PbrBundle {
        mesh: mesh_handle,
        material: ATARI_MATERIAL_HANDLE.typed(),
        ..Default::default()
    });

    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
    //         50.0,
    //         50.0,
    //     )))),
    //     material: RED_MATERIAL_HANDLE.typed(),
    //     transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    //     ..Default::default()
    // });

    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.transform.scale = Vec3::new(0.5, 0.5, 1.0);
    commands.spawn_bundle(camera_bundle).insert(MainCamera);

    // commands.spawn_bundle(PerspectiveCameraBundle {
    //     transform: Transform::from_xyz(-500.0, 1.5, 500.0).looking_at(Vec3::default(), Vec3::Y),
    //     ..Default::default()
    // });

    let bundle = MeshBundle {
        mesh: ANTIC_MESH_HANDLE.typed(),
        render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
            pipelines.add(build_antic2_pipeline(&mut *shaders)),
        )]),
        ..Default::default()
    };

    commands
        .spawn_bundle(bundle)
        .insert(AnticFrame)
        .insert(ATARI_PALETTE_HANDLE.typed::<AtariPalette>())
        .insert(ANTIC_DATA_HANDLE.typed::<AnticData>())
        .insert(DATA_MATERIAL_HANDLE.typed::<SimpleMaterial>())
        .remove::<MainPass>();
}

pub fn post_running(
    mut atari_data_assets: ResMut<Assets<AnticData>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let antic_data = atari_data_assets.get_mut(ANTIC_DATA_HANDLE).unwrap();
    let mesh = antic_data.create_mesh();
    meshes.set_untracked(ANTIC_MESH_HANDLE, mesh);
}

#[derive(Default)]
pub struct AnticRenderPlugin {
    pub texture_size: Vec2,
    pub enable_collisions: bool,
    pub collision_agg_size: Option<(u32, u32)>,
}

impl Plugin for AnticRenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<AnticData>().add_asset::<AtariPalette>();
        app.add_system_to_stage(CoreStage::PreUpdate, collisions_read.exclusive_system());

        app.init_resource::<CollistionsReadState>();
        app.add_startup_system(setup.system());

        let world = app.world_mut().cell();
        let mut pipelines = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();
        let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();
        let mut palettes = world.get_resource_mut::<Assets<AtariPalette>>().unwrap();
        let mut antic_data = world.get_resource_mut::<Assets<AnticData>>().unwrap();
        let mut render_graph = world.get_resource_mut::<RenderGraph>().unwrap();

        pipelines.set_untracked(
            COLLISIONS_PIPELINE_HANDLE,
            build_collisions_pipeline(&mut shaders),
        );
        pipelines.set_untracked(
            DEBUG_COLLISIONS_PIPELINE_HANDLE,
            build_debug_collisions_pipeline(&mut shaders),
        );
        palettes.set_untracked(ATARI_PALETTE_HANDLE, AtariPalette::default());
        antic_data.set_untracked(ANTIC_DATA_HANDLE, AnticData::default());

        render_graph.add_system_node(
            "atari_palette",
            AssetRenderResourcesNode::<AtariPalette>::new(false),
        );
        render_graph.add_system_node(
            "antic_data",
            AssetRenderResourcesNode::<AnticData>::new(false),
        );

        render_graph.add_system_node(
            "custom_texture",
            AssetRenderResourcesNode::<CustomTexture>::new(false),
        );
        render_graph
            .add_node_edge("custom_texture", MAIN_PASS)
            .unwrap();

        let size = Vec2::new(self.texture_size.x, self.texture_size.y);
        add_antic_graph(
            &mut *render_graph,
            &world,
            &size,
            self.enable_collisions,
            self.collision_agg_size,
        );
        render_graph.add_system_node(
            "simple_material",
            AssetRenderResourcesNode::<SimpleMaterial>::new(false),
        );
        render_graph
            .add_node_edge("simple_material", "antic_pass")
            .unwrap();
    }
}
