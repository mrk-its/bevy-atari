use bevy::{
    prelude::*,
    render::{
        camera::{Camera, CameraProjection},
        render_graph::base::MainPass,
    },
    window::WindowId,
};

use crate::render_resources::CustomTexture;

use super::render;

#[derive(Default)]
pub struct CollisionsAggPass;

#[derive(Bundle, Default)]
pub struct CollisionTextureDebugBundle {
    pub mesh: Handle<Mesh>,
    pub draw: Draw,
    pub texture: Handle<CustomTexture>,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(Bundle, Default)]
pub struct CollisionsAggBundle {
    pub mesh: Handle<Mesh>,
    pub draw: Draw,
    pub texture: Handle<CustomTexture>,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub collision_agg_pass: CollisionsAggPass,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(Bundle, Default)]
pub struct AnticLineBundle {
    pub mesh: Handle<Mesh>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    // pub main_pass: MainPass,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

pub fn create_2d_camera(name: &str, width: f32, height: f32) -> Camera2dBundle {
    let mut camera_bundle = Camera2dBundle {
        camera: Camera {
            name: Some(name.to_string()),
            window: WindowId::new(),
            ..Default::default()
        },
        transform: Transform {
            scale: Vec3::new(1.0, -1.0, 1.0),
            ..Default::default()
        },
        ..Default::default()
    };

    let camera_projection = &mut camera_bundle.orthographic_projection;
    camera_projection.update(width, height);
    camera_bundle.camera.projection_matrix = camera_projection.get_projection_matrix();
    camera_bundle.camera.depth_calculation = camera_projection.depth_calculation();
    camera_bundle
}

pub fn create_antic_camera(size: Vec2) -> Camera2dBundle {
    create_2d_camera(render::ANTIC_CAMERA, size.x, size.y)
}

pub fn create_collisions_camera(size: Vec2) -> Camera2dBundle {
    create_2d_camera(render::COLLISIONS_AGG_CAMERA, size.x, size.y)
}
