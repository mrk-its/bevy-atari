use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render2::{camera::OrthographicCameraBundle},
};

pub use bevy_atari_antic::{atari_data::AnticData, AtariAnticPlugin, GTIARegs, ModeLineDescr, AnticMesh};

#[derive(Default)]
pub struct AnticRenderPlugin;

impl Plugin for AnticRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AtariAnticPlugin);
        app.init_resource::<AnticData>().add_startup_system(setup);
    }
}

pub const ANTIC_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(AnticMesh::TYPE_UUID, 16056864393442354012);

pub const ANTIC_DATA_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(AnticData::TYPE_UUID, 16056864393442354013);

fn setup(
    mut commands: Commands,
    mut antic_data_assets: ResMut<Assets<AnticData>>,
) {
    let mut antic_data = AnticData::default();

    antic_data.insert_mode_line(&ModeLineDescr{
        mode: 0,
        scan_line: 116,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: 0,
        charset_memory_offset: 0,
    });

    antic_data_assets.set_untracked(ANTIC_DATA_HANDLE, antic_data);

    commands.spawn().insert_bundle((
        Transform::from_xyz(-1.0, 0.0, 0.0),
        GlobalTransform::default(),
        ANTIC_DATA_HANDLE.typed::<AnticData>(),
    ));

    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.camera.name = Some("camera_3d".to_string());
    camera_bundle.transform.scale = Vec3::new(0.5, 0.5, 1.0);
    camera_bundle.transform.translation = Vec3::new(0.0, 0.0, 0.0);

    // camera
    commands.spawn_bundle(camera_bundle);
}
