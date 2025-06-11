mod debug;
mod plugin;

use avian3d::{math::*, prelude::*};
use bevy::prelude::*;
use plugin::*;

use crate::debug::uv_debug_texture;

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Player
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(1.0, 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(uv_debug_texture())),
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 0.0),
        CharacterControllerBundle::new(Collider::sphere(2.0), 9.81 * 2.0).with_movement(
            20.0,
            10.0,
            (30.0 as Scalar).to_radians(),
        ),
        LinearDamping(0.4),
        children![
            // Camera
            (
                Camera3d::default(),
                Transform::from_xyz(0.0, 5.0, 15.0).looking_at(
                    Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Vec3::Y,
                ),
            )
        ],
    ));
    // Surface
    commands.spawn((
        RigidBody::Static,
        Mesh3d(meshes.add(Sphere::new(30.0).mesh().uv(64, 48))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(uv_debug_texture())),
            ..default()
        })),
        Collider::sphere(30.0),
        Transform::from_xyz(0.0, -30.0, 0.0),
        GravityField::Sphere(Vec3 {
            x: 0.0,
            y: -30.0,
            z: 0.0,
        }),
    ));
    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            PhysicsPlugins::default(),
            CharacterControllerPlugin,
        ))
        .add_systems(Startup, startup)
        .run();
}
