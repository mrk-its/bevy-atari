use bevy::{prelude::*, render::render_graph::base::MainPass};

use crate::render_resources::CustomTexture;



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
