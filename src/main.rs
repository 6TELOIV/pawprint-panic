mod debug;
mod plugin;

use avian3d::{math::*, prelude::*};
use bevy::prelude::*;
use plugin::character_controller::*;

use crate::{
    debug::uv_debug_texture,
    plugin::camera_controller::{CameraController, CameraControllerPlugin},
};

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let hole_planet_mesh = Mesh3d(
        asset_server.load(
            GltfAssetLabel::Primitive {
                mesh: 0,
                primitive: 0,
            }
            .from_asset("hole_planet.glb"),
        ),
    );

    // Player
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(1.0, 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(uv_debug_texture())),
            ..default()
        })),
        Transform::from_xyz(0.0, -14.0, 16.0),
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
                CameraController
            )
        ],
    ));
    // Surface
    commands.spawn((
        RigidBody::Static,
        hole_planet_mesh.clone(),
        ColliderConstructor::ConvexDecompositionFromMesh,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(uv_debug_texture())),
            ..default()
        })),
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
        Transform::from_xyz(12.0, 16.0, 8.0),
    ));
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            PhysicsPlugins::default(),
            CharacterControllerPlugin,
            CameraControllerPlugin,
        ))
        .add_systems(Startup, startup)
        .run();
}
