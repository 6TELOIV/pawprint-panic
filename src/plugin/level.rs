use avian3d::prelude::{Collider, ColliderConstructor, RigidBody, Sensor};
use bevy::{
    gltf::{GltfMesh, GltfNode},
    prelude::*,
};
use serde::Deserialize;

use crate::plugin::character_controller::{GravityField, GravityFieldType};

pub struct LevelPlugin;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum LoadingState {
    #[default]
    Loading,
    Loaded,
}

#[derive(Resource)]
struct LevelScene(Handle<Gltf>);

#[derive(Deserialize)]
struct NodeExtras {
    gravity_field: Option<u8>,
    gravity_field_priority: Option<u8>,
    gravity_field_radius: Option<f32>,
    gravity_field_height: Option<f32>,
    rigid_body: Option<u8>,
}

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<LoadingState>()
            .add_systems(Startup, startup)
            .add_systems(
                Update,
                wait_for_level_load.run_if(in_state(LoadingState::Loading)),
            )
            .add_systems(OnEnter(LoadingState::Loaded), setup_level);
    }
}

/// tell the asset server to go load the gltf
fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(LevelScene(asset_server.load("hole_planet.glb")));
}

/// once it's loaded, switch states
fn wait_for_level_load(
    asset_server: Res<AssetServer>,
    level_scene: Res<LevelScene>,
    mut next_loading_state: ResMut<NextState<LoadingState>>,
) {
    if asset_server.is_loaded_with_dependencies(&level_scene.0) {
        next_loading_state.set(LoadingState::Loaded);
    }
}

/// spawn all the stuff in from the loaded file
fn setup_level(
    mut commands: Commands,
    level_scene: Res<LevelScene>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_node_assets: Res<Assets<GltfNode>>,
    gltf_mesh_assets: Res<Assets<GltfMesh>>,
) -> Result {
    let gltf = gltf_assets
        .get(&level_scene.0)
        .ok_or_else(|| "level gltf not loaded")?;

    // PREF: clones just clones the handles
    for node_handle in &gltf.nodes {
        let node = gltf_node_assets
            .get(node_handle)
            .ok_or_else(|| "level gltf node not loaded")?;

        let extras = &node.extras.clone().unwrap_or(GltfExtras {
            value: "{}".to_string(),
        });
        let extras_value: NodeExtras = serde_json::from_str(&extras.value)?;
        match extras_value.gravity_field {
            // Sphere
            Some(0) => {
                let gravity_field_radius = extras_value
                    .gravity_field_radius
                    .ok_or_else(|| "Sphere gravity field requires a radius; none specified")?;
                let gravity_field_priority = extras_value
                    .gravity_field_priority
                    .ok_or_else(|| "Sphere gravity field requires a priority; none specified")?;
                commands.spawn((
                    GravityField {
                        field_type: GravityFieldType::Sphere,
                        priority: gravity_field_priority,
                    },
                    node.transform,
                    RigidBody::Static,
                    Collider::sphere(gravity_field_radius),
                    Sensor,
                ));
            }
            // Cylinder
            Some(1) => {
                let gravity_field_radius = extras_value
                    .gravity_field_radius
                    .ok_or_else(|| "Cylinder gravity field requires a radius; none specified")?;
                let gravity_field_height = extras_value
                    .gravity_field_height
                    .ok_or_else(|| "Cylinder gravity field requires a height; none specified")?;
                let gravity_field_priority = extras_value
                    .gravity_field_priority
                    .ok_or_else(|| "Cylinder gravity field requires a priority; none specified")?;
                commands.spawn((
                    GravityField {
                        field_type: GravityFieldType::HalfPlane,
                        priority: gravity_field_priority,
                    },
                    node.transform,
                    RigidBody::Static,
                    Collider::cylinder(gravity_field_radius, gravity_field_height),
                    Sensor,
                ));
            }
            Some(x) => {
                warn!("unknown gravity_field: {}", x);
            }
            None => {}
        }

        match extras_value.rigid_body {
            Some(0 | 1 | 2) => {
                let mesh_handle = node
                    .mesh
                    .clone()
                    .ok_or_else(|| "rigid_body must specify a mesh")?;
                let mesh = gltf_mesh_assets
                    .get(&mesh_handle)
                    .ok_or_else(|| "level gltf mesh not loaded")?;
                for primative in &mesh.primitives {
                    commands.spawn((
                        RigidBody::Static,
                        Mesh3d(primative.mesh.clone()),
                        MeshMaterial3d(
                            primative
                                .material
                                .clone()
                                .ok_or_else(|| "primative missing material")?,
                        ),
                        ColliderConstructor::ConvexDecompositionFromMesh,
                        node.transform,
                    ));
                }
            }
            Some(x) => {
                warn!("unknown rigid_body: {}", x);
            }
            None => {}
        }
    }
    return Ok(());
}
