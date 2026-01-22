use avian3d::prelude::*;
use bevy::prelude::*;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::TAU;

use crate::arena::assets::TerrainAssets;
use crate::arena::config::BoundaryConfig;
use crate::arena::shape::ArenaShape;
use crate::physics::TerrainSampler;
use crate::states::GameState;

/// Component to mark boundary boulders
#[derive(Component)]
pub struct BoundaryBoulder;

/// Average boulder diameter for spacing calculations
const AVG_BOULDER_DIAMETER: f32 = 4.0;

/// Calculate the number of boulders needed to form a dense boundary
/// Uses actual perimeter length for irregular shapes
pub fn calculate_boundary_boulder_count(shape: &ArenaShape, spacing: f32, inward_offset: f32) -> u32 {
    // Create a shape offset inward for boulder placement
    let effective_shape = ArenaShape::new(
        shape.base_radius - inward_offset,
        shape.distortion_strength,
        shape.distortion_scale,
        0, // Seed doesn't matter for perimeter calc
    );

    let perimeter = effective_shape.approximate_perimeter();
    let slot_width = AVG_BOULDER_DIAMETER + spacing;
    (perimeter / slot_width).ceil() as u32
}

/// Spawn a dense boulder boundary wall following the arena's irregular contour
pub fn spawn_boulder_boundary(
    commands: &mut Commands,
    shape: &ArenaShape,
    boundary_config: &BoundaryConfig,
    terrain_assets: &TerrainAssets,
    terrain_sampler: &TerrainSampler,
    rng: &mut ChaCha8Rng,
) {
    if terrain_assets.boulders.is_empty() {
        warn!("No boulder models loaded, skipping boundary generation");
        return;
    }

    let boulder_count = calculate_boundary_boulder_count(
        shape,
        boundary_config.boulder_spacing,
        boundary_config.inward_offset,
    );

    info!(
        "Spawning {} boundary boulders following irregular contour",
        boulder_count
    );

    let angle_step = TAU / boulder_count as f32;

    for i in 0..boulder_count {
        // Base angle for this boulder
        let base_angle = i as f32 * angle_step;

        // Slight angular jitter for natural look (max 10% of angle step)
        let angle_jitter = rng.gen_range(-0.1..0.1) * angle_step;
        let angle = base_angle + angle_jitter;

        // Get radius at this angle from the irregular shape
        let base_radius = shape.radius_at_angle(angle);
        let effective_radius = base_radius - boundary_config.inward_offset;

        // Slight radial jitter for natural look
        let radial_jitter = rng.gen_range(-0.3..0.3);
        let radius = effective_radius + radial_jitter;

        // Calculate position
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        let pos = terrain_sampler.get_spawn_position(x, z, 0.0);

        // Random rotation (full rotation variation)
        let rotation = Quat::from_rotation_y(rng.gen_range(0.0..TAU));

        // Size variation for X/Z (horizontal)
        let size_variation = boundary_config.boulder_size_variation;
        let xz_scale = 1.0 + rng.gen_range(-size_variation..size_variation);

        // Height variation (Y) - independent, more dramatic
        let height_scale = rng.gen_range(
            boundary_config.height_scale_min..boundary_config.height_scale_max
        );

        // Select random boulder model
        let boulder_index = rng.gen_range(0..terrain_assets.boulders.len());
        let boulder = &terrain_assets.boulders[boulder_index];

        commands.spawn((
            SceneRoot(boulder.handle.clone()),
            Transform::from_translation(pos)
                .with_rotation(rotation)
                .with_scale(Vec3::new(xz_scale, height_scale, xz_scale)),
            RigidBody::Static,
            ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
            BoundaryBoulder,
            StateScoped(GameState::Playing),
            Name::new("Boundary Boulder"),
        ));
    }

    // Add invisible backup collider ring just outside boulders to prevent escape
    // Use the max radius to ensure it covers the irregular shape
    spawn_backup_collider_ring(commands, shape);
}

/// Spawn a thin invisible collider ring as backup to absolutely prevent escape
/// Follows the irregular shape contour
fn spawn_backup_collider_ring(commands: &mut Commands, shape: &ArenaShape) {
    let wall_height = 8.0;
    let wall_thickness = 0.5;
    let num_segments = 64;
    let outward_offset = 1.0; // Place slightly outside the boundary

    for i in 0..num_segments {
        let angle = (i as f32 / num_segments as f32) * TAU;

        // Get radius at this angle and add offset
        let radius = shape.radius_at_angle(angle) + outward_offset;

        let x = angle.cos() * radius;
        let z = angle.sin() * radius;

        // Calculate segment width based on arc length
        // For irregular shapes, calculate the actual distance to the next point
        let next_angle = ((i + 1) as f32 / num_segments as f32) * TAU;
        let next_radius = shape.radius_at_angle(next_angle) + outward_offset;
        let next_x = next_angle.cos() * next_radius;
        let next_z = next_angle.sin() * next_radius;

        let dx = next_x - x;
        let dz = next_z - z;
        let segment_width = (dx * dx + dz * dz).sqrt();

        commands.spawn((
            Transform::from_xyz(x, wall_height / 2.0, z).with_rotation(Quat::from_rotation_y(-angle)),
            RigidBody::Static,
            Collider::cuboid(segment_width, wall_height, wall_thickness),
            StateScoped(GameState::Playing),
            Name::new("Backup Boundary Collider"),
        ));
    }
}
