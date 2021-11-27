use crate::system::AtariSystem;
use bevy::prelude::MouseButton;
use bevy::prelude::*;
use bevy::render2::camera::Camera;
use bevy::render2::texture::Image;
use bevy::sprite2::{PipelinedSpriteBundle, Sprite};
use bevy_atari_antic::wgpu::Extent3d;

#[derive(Component, Default)]
pub struct Focused;

pub fn update(
    windows: Res<Windows>,
    mut commands: Commands,
    mouse_buttons: Res<Input<MouseButton>>,
    mut query: Query<
        (Entity, &mut AtariSystem, &Transform, Option<&Focused>),
        (Without<Focus>, Without<Camera>),
    >,
    mut focus_query: Query<&mut Transform, (With<Focus>, Without<Camera>)>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    let (mut focus_transform, camera_transform) =
        if let (Some(a), Some(b)) = (focus_query.iter_mut().next(), camera_query.iter().next()) {
            (a, b)
        } else {
            return;
        };
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let window = if let Some(window) = windows.get_primary() {
        window
    } else {
        return;
    };
    let cursor_position = if let Some(pos) = window.cursor_position() {
        pos
    } else {
        return;
    };
    let phy_width = window.physical_width() as f32;
    let phy_height = window.physical_height() as f32;

    let cursor_position = Vec3::new(
        cursor_position.x - phy_width / 2.0,
        cursor_position.y - phy_height / 2.0,
        0.0,
    );

    bevy::log::info!("corrected cursor position: {:?}", cursor_position);

    let wp = (*camera_transform * cursor_position).truncate();

    let slot_size = Vec2::new(400.0, 256.0);

    for (entity, mut atari_system, transform, was_focused) in query.iter_mut() {
        let was_focused = was_focused.is_some();
        let translation = transform.translation.truncate();
        let sw = translation - slot_size / 2.0;
        let ne = translation + slot_size / 2.0;

        let is_focused = wp.x >= sw.x && wp.y >= sw.y && wp.x < ne.x && wp.y < ne.y;
        atari_system.pokey.mute(!is_focused);
        if is_focused {
            focus_transform.translation = transform.translation;
            focus_transform.translation.z = -1.0;
        }

        if is_focused ^ was_focused {
            let mut cmd = commands.entity(entity);
            if is_focused {
                cmd.insert(Focused);
            } else {
                cmd.remove::<Focused>();
            }
        }
    }
}

#[derive(Component)]
pub struct Focus;

pub fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy_atari_antic::wgpu::TextureDimension::D2,
        vec![0, 0, 255, 255],
        bevy_atari_antic::wgpu::TextureFormat::Rgba8UnormSrgb,
    );
    let texture = images.add(image);
    commands
        .spawn()
        .insert(Focus {})
        .insert_bundle(PipelinedSpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(384.0 + 4.0, 240.0 + 4.0)),
                ..Default::default()
            },
            texture,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 100.0),
                ..Default::default()
            },
            ..Default::default()
        });
}
