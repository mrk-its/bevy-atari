use bevy::{prelude::*, render2::camera::OrthographicCameraBundle};

pub use bevy_atari_antic::{
    AnticData, AtariAnticPlugin, GTIARegs, ModeLineDescr
};

#[derive(Default)]
pub struct AnticRenderPlugin;

impl Plugin for AnticRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AtariAnticPlugin {collisions: false}).add_startup_system(setup);
    }
}

fn setup(mut commands: Commands) {
    // commands
    //     .spawn()
    //     .insert_bundle((ANTIC_DATA_HANDLE.typed::<AnticData>(),));



    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.transform.scale = Vec3::new(1.0, 1.0, 1.0);
    camera_bundle.transform.translation = Vec3::new(0.0, 0.0, 0.0);

    // camera
    commands.spawn_bundle(camera_bundle);
}
