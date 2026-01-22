use avian3d::prelude::*;
use bevy::prelude::*;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::TAU;

use crate::arena::assets::{ModelMetadata, TerrainAssets};
use crate::arena::config::{ArenaConfig, BiomeTheme, DecorationConfig, ObstacleConfig};
use crate::arena::shape::ArenaShape;
use crate::physics::TerrainSampler;
use crate::states::GameState;

/// Component to mark obstacles
#[derive(Component)]
pub struct Obstacle;

/// Component storing obstacle dimensions for gameplay systems
#[derive(Component)]
pub struct ObstacleBounds {
    pub avoidance_radius: f32,
    pub spawn_radius: f32,
}

/// Check if a position is valid for obstacle placement using shape-aware bounds
fn is_valid_position(
    pos: Vec3,
    obstacle_radius: f32,
    placed: &[(Vec3, f32)],
    min_spacing: f32,
    spawn_ring_radius: f32,
    spawn_exclusion: f32,
    center_exclusion: f32,
    shape: &ArenaShape,
) -> bool {
    let distance_from_center = (pos.x * pos.x + pos.z * pos.z).sqrt();

    // Check 1: Not too close to center (player spawn area)
    if distance_from_center < center_exclusion + obstacle_radius {
        return false;
    }

    // Check 2: Within arena bounds using shape-aware boundary check
    let wall_margin = 6.0; // Account for boulder boundary
    if !shape.contains_with_margin(pos.x, pos.z, wall_margin + obstacle_radius) {
        return false;
    }

    // Check 3: Not too close to spawn ring
    let dist_to_spawn_ring = (distance_from_center - spawn_ring_radius).abs();
    if dist_to_spawn_ring < spawn_exclusion + obstacle_radius {
        return false;
    }

    // Check 4: Not too close to other obstacles
    for (other_pos, other_radius) in placed {
        let dist = ((pos.x - other_pos.x).powi(2) + (pos.z - other_pos.z).powi(2)).sqrt();
        let required_dist = obstacle_radius + other_radius + min_spacing;
        if dist < required_dist {
            return false;
        }
    }

    true
}

/// Select a tree model based on theme with chance for dead trees
fn select_tree_model<'a>(
    terrain_assets: &'a TerrainAssets,
    theme: BiomeTheme,
    dead_tree_chance: f32,
    rng: &mut ChaCha8Rng,
) -> Option<&'a ModelMetadata> {
    let trees = terrain_assets.get_trees_for_theme(theme);
    let dead_trees = terrain_assets.get_dead_trees();

    if trees.is_empty() && dead_trees.is_empty() {
        return None;
    }

    // Check if we should spawn a dead tree
    if !dead_trees.is_empty() && rng.gen_bool(dead_tree_chance as f64) {
        // In summer, only use small dead trees
        let dead_tree_index = if theme == BiomeTheme::Summer {
            0 // Only small dead tree
        } else {
            rng.gen_range(0..dead_trees.len())
        };
        return Some(&dead_trees[dead_tree_index]);
    }

    if trees.is_empty() {
        return None;
    }

    let tree_index = rng.gen_range(0..trees.len());
    Some(&trees[tree_index])
}

/// Spawn themed obstacles (trees and rocks) within the arena
/// Uses density-based counts from config
pub fn spawn_obstacles(
    commands: &mut Commands,
    arena_config: &ArenaConfig,
    obstacle_config: &ObstacleConfig,
    decoration_config: &DecorationConfig,
    terrain_assets: &TerrainAssets,
    terrain_sampler: &TerrainSampler,
    rng: &mut ChaCha8Rng,
) {
    let mut placed_positions: Vec<(Vec3, f32)> = vec![];
    let spawn_ring_radius = arena_config.shape.base_radius * 0.7;
    let max_attempts = 100;

    info!(
        "Spawning {} obstacles with theme {:?} (density-based for area {:.0} sqm)",
        obstacle_config.count, arena_config.theme, arena_config.area
    );

    for _ in 0..obstacle_config.count {
        // Weighted selection between trees, rocks, and boulders
        let total_weight = obstacle_config.tree_weight
            + obstacle_config.rock_weight
            + obstacle_config.boulder_weight;

        let roll = rng.gen::<f32>() * total_weight;
        let tree_threshold = obstacle_config.tree_weight;
        let rock_threshold = tree_threshold + obstacle_config.rock_weight;

        let (model_meta, scale) = if roll < tree_threshold {
            // Spawn a tree (scale 1.0)
            if let Some(tree) = select_tree_model(
                terrain_assets,
                arena_config.theme,
                decoration_config.dead_tree_chance,
                rng,
            ) {
                (tree.clone(), 1.0)
            } else {
                continue;
            }
        } else if roll < rock_threshold {
            // Spawn a rock (scale 1.0)
            if terrain_assets.rocks.is_empty() {
                continue;
            }
            let rock_index = rng.gen_range(0..terrain_assets.rocks.len());
            (terrain_assets.rocks[rock_index].clone(), 1.0)
        } else {
            // Spawn an interior boulder (scale 0.5-1.0)
            if terrain_assets.interior_boulders.is_empty() {
                continue;
            }
            let boulder_index = rng.gen_range(0..terrain_assets.interior_boulders.len());
            let boulder_scale = rng.gen_range(0.5..1.0);
            (terrain_assets.interior_boulders[boulder_index].clone(), boulder_scale)
        };

        // Random rotation
        let rotation = Quat::from_rotation_y(rng.gen_range(0.0..TAU));

        // Try to find valid position within the irregular shape
        let scaled_spawn_radius = model_meta.spawn_radius * scale;
        for _ in 0..max_attempts {
            let angle = rng.gen_range(0.0..TAU);
            // Use the min_radius to ensure we stay within bounds
            let max_spawn_dist = arena_config.shape.min_radius() - 6.0 - scaled_spawn_radius;
            let min_spawn_dist = obstacle_config.center_exclusion + scaled_spawn_radius;

            if max_spawn_dist <= min_spawn_dist {
                continue;
            }

            let distance = rng.gen_range(min_spawn_dist..max_spawn_dist);
            let x = angle.cos() * distance;
            let z = angle.sin() * distance;
            let pos = terrain_sampler.get_spawn_position(x, z, 0.0);

            if is_valid_position(
                pos,
                scaled_spawn_radius,
                &placed_positions,
                obstacle_config.min_spacing,
                spawn_ring_radius,
                obstacle_config.spawn_exclusion,
                obstacle_config.center_exclusion,
                &arena_config.shape,
            ) {
                commands.spawn((
                    SceneRoot(model_meta.handle.clone()),
                    Transform::from_translation(pos)
                        .with_rotation(rotation)
                        .with_scale(Vec3::splat(scale)),
                    RigidBody::Static,
                    ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
                    Obstacle,
                    ObstacleBounds {
                        avoidance_radius: model_meta.avoidance_radius * scale,
                        spawn_radius: model_meta.spawn_radius * scale,
                    },
                    StateScoped(GameState::Playing),
                    Name::new("Terrain Obstacle"),
                ));

                placed_positions.push((pos, model_meta.spawn_radius * scale));
                break;
            }
        }
    }

    info!("Placed {} obstacles", placed_positions.len());
}

/// Spawn themed decorative grass throughout the arena
/// Uses density-based counts from config
pub fn spawn_grass(
    commands: &mut Commands,
    arena_config: &ArenaConfig,
    obstacle_config: &ObstacleConfig,
    decoration_config: &DecorationConfig,
    terrain_assets: &TerrainAssets,
    terrain_sampler: &TerrainSampler,
    rng: &mut ChaCha8Rng,
) {
    let grass_models = terrain_assets.get_grass_for_theme(arena_config.theme);

    if grass_models.is_empty() {
        warn!("No grass models loaded for theme {:?}", arena_config.theme);
        return;
    }

    info!(
        "Spawning {} grass decorations with theme {:?} (density-based)",
        decoration_config.grass_count, arena_config.theme
    );

    let wall_margin = 5.0;
    let max_attempts = 20;

    for _ in 0..decoration_config.grass_count {
        let model_index = rng.gen_range(0..grass_models.len());
        let model = grass_models[model_index].clone();

        // Try to find a valid position within the irregular shape
        for _ in 0..max_attempts {
            let angle = rng.gen_range(0.0..TAU);
            let max_dist = arena_config.shape.min_radius() - wall_margin;
            let distance = rng.gen_range(obstacle_config.center_exclusion..max_dist.max(obstacle_config.center_exclusion + 1.0));
            let x = angle.cos() * distance;
            let z = angle.sin() * distance;

            // Verify position is within the irregular shape
            if arena_config.shape.contains_with_margin(x, z, wall_margin) {
                let pos = terrain_sampler.get_spawn_position(x, z, 0.0);

                let rotation = Quat::from_rotation_y(rng.gen_range(0.0..TAU));
                let scale = decoration_config.grass_scale * rng.gen_range(0.8..1.2);

                commands.spawn((
                    SceneRoot(model),
                    Transform::from_translation(pos)
                        .with_rotation(rotation)
                        .with_scale(Vec3::splat(scale)),
                    StateScoped(GameState::Playing),
                    Name::new("Decorative Grass"),
                ));
                break;
            }
        }
    }
}

/// Spawn themed mushrooms throughout the arena
/// Uses density-based counts from config
pub fn spawn_mushrooms(
    commands: &mut Commands,
    arena_config: &ArenaConfig,
    obstacle_config: &ObstacleConfig,
    decoration_config: &DecorationConfig,
    terrain_assets: &TerrainAssets,
    terrain_sampler: &TerrainSampler,
    rng: &mut ChaCha8Rng,
) {
    let mushroom_models = terrain_assets.get_mushrooms_for_theme(arena_config.theme);

    if mushroom_models.is_empty() {
        warn!(
            "No mushroom models loaded for theme {:?}",
            arena_config.theme
        );
        return;
    }

    info!(
        "Spawning {} mushrooms with theme {:?} (density-based)",
        decoration_config.mushroom_count, arena_config.theme
    );

    let wall_margin = 5.0;
    let max_attempts = 20;

    for _ in 0..decoration_config.mushroom_count {
        let model_index = rng.gen_range(0..mushroom_models.len());
        let model = mushroom_models[model_index].clone();

        // Try to find a valid position within the irregular shape
        for _ in 0..max_attempts {
            let angle = rng.gen_range(0.0..TAU);
            let max_dist = arena_config.shape.min_radius() - wall_margin;
            let distance = rng.gen_range(obstacle_config.center_exclusion..max_dist.max(obstacle_config.center_exclusion + 1.0));
            let x = angle.cos() * distance;
            let z = angle.sin() * distance;

            // Verify position is within the irregular shape
            if arena_config.shape.contains_with_margin(x, z, wall_margin) {
                let pos = terrain_sampler.get_spawn_position(x, z, 0.0);

                let rotation = Quat::from_rotation_y(rng.gen_range(0.0..TAU));
                let scale = rng.gen_range(0.6..1.0);

                commands.spawn((
                    SceneRoot(model),
                    Transform::from_translation(pos)
                        .with_rotation(rotation)
                        .with_scale(Vec3::splat(scale)),
                    StateScoped(GameState::Playing),
                    Name::new("Decorative Mushroom"),
                ));
                break;
            }
        }
    }
}
