use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use crate::arena::config::WaterSettings;
use crate::arena::shape::ArenaShape;
use crate::states::GameState;

/// Component marking the water plane entity
#[derive(Component)]
pub struct WaterPlane;

/// Spawn a water plane at Y=0 that extends beyond the arena boundary
pub fn spawn_water_plane(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    shape: &ArenaShape,
    water_settings: &WaterSettings,
) {
    // Create water plane extending well beyond island for dramatic water area
    let water_extent = shape.base_radius * 2.0;

    // Create a plane mesh for the water
    let water_mesh = Plane3d::default()
        .mesh()
        .size(water_extent * 2.0, water_extent * 2.0)
        .build();

    // Semi-transparent blue water material
    let water_material = materials.add(StandardMaterial {
        base_color: Color::srgba(
            water_settings.water_color[0],
            water_settings.water_color[1],
            water_settings.water_color[2],
            water_settings.water_color[3],
        ),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.3,
        reflectance: 0.8,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(water_mesh)),
        MeshMaterial3d(water_material),
        Transform::from_xyz(0.0, water_settings.water_level, 0.0),
        WaterPlane,
        NotShadowCaster,
        NotShadowReceiver,
        StateScoped(GameState::Playing),
        Name::new("Water Plane"),
    ));

    info!(
        "Spawned water plane at Y={} with extent {}",
        water_settings.water_level, water_extent
    );
}
