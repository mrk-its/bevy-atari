use bevy::{prelude::*, render2::camera::OrthographicCameraBundle};

use bevy::sprite2::{PipelinedSpriteBundle, Sprite};
pub use bevy_atari_antic::{
    AnticData, AtariAnticPlugin, GTIARegs, ModeLineDescr, ANTIC_DATA_HANDLE, ANTIC_IMAGE_HANDLE,
};

#[derive(Default)]
pub struct AnticRenderPlugin;

impl Plugin for AnticRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AtariAnticPlugin).add_startup_system(setup);
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn()
        .insert_bundle((ANTIC_DATA_HANDLE.typed::<AnticData>(),));

    commands.spawn_bundle(PipelinedSpriteBundle {
        sprite: Sprite::default(),
        texture: ANTIC_IMAGE_HANDLE.typed(),
        ..Default::default()
    });

    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.transform.scale = Vec3::new(0.5, 0.5, 1.0);
    camera_bundle.transform.translation = Vec3::new(0.0, 0.0, 0.0);

    // camera
    commands.spawn_bundle(camera_bundle);
}
