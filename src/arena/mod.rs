mod terrain;

use bevy::prelude::*;
use bevy::pbr::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use avian3d::prelude::*;
use rand::Rng;

use crate::states::GameState;
use crate::physics::TerrainSampler;
use terrain::{TerrainConfig, generate_terrain_mesh, generate_heights_matrix};

// Gradient sky material for the sky dome
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct GradientSkyMaterial {
    #[uniform(0)]
    pub top_color: LinearRgba,
    #[uniform(0)]
    pub bottom_color: LinearRgba,
}

impl Material for GradientSkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/gradient_sky.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

pub struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GradientSkyMaterial>::default())
            .insert_resource(ArenaConfig {
                radius: 40.0,
            })
            .insert_resource(ObstacleConfig {
                obstacle_count: 20,
                min_spacing: 6.0,
                spawn_exclusion: 4.0,
                center_exclusion: 10.0,
            })
            // TerrainSampler with parameters matching terrain.rs (seed=42, height_scale=1.5, noise_scale=0.08)
            .insert_resource(TerrainSampler::new(42, 1.5, 0.08))
            .init_resource::<TerrainAssets>()
            .add_systems(Startup, load_terrain_assets)
            .add_systems(OnEnter(GameState::Playing), (spawn_arena, spawn_lighting));
    }
}

#[derive(Resource)]
pub struct ArenaConfig {
    pub radius: f32,
}

#[derive(Resource)]
pub struct ObstacleConfig {
    pub obstacle_count: u32,
    pub min_spacing: f32,
    pub spawn_exclusion: f32,
    pub center_exclusion: f32,
}

#[derive(Component)]
pub struct Arena;

#[derive(Component)]
pub struct Obstacle;

#[derive(Component)]
pub struct ObstacleBounds {
    pub avoidance_radius: f32,  // For enemy movement avoidance (trunk)
    pub spawn_radius: f32,      // For spawn exclusion (canopy)
}

#[derive(Resource, Default)]
pub struct TerrainAssets {
    pub collider_models: Vec<Handle<Scene>>,
    pub decorative_models: Vec<Handle<Scene>>,
}

fn load_terrain_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let collider_models = vec![
        asset_server.load("models/environment/colliders/Gradient_Boulder_01.glb#Scene0"),
        asset_server.load("models/environment/colliders/Gradient_Boulder_02.glb#Scene0"),
        asset_server.load("models/environment/colliders/Gradient_Boulder_03.glb#Scene0"),
        asset_server.load("models/environment/colliders/Gradient_Tree_Oak_large_green.glb#Scene0"),
        asset_server.load("models/environment/colliders/Gradient_Tree_Oak_large_orange.glb#Scene0"),
        asset_server.load("models/environment/colliders/Gradient_Tree_Oak_medium_green.glb#Scene0"),
        asset_server.load("models/environment/colliders/Gradient_Tree_Oak_medium_orange.glb#Scene0"),
    ];

    let decorative_models = vec![
        asset_server.load("models/environment/non-colliders/Gradient_Grass_large_summer.glb#Scene0"),
        asset_server.load("models/environment/non-colliders/Gradient_Grass_large_autumn.glb#Scene0"),
        asset_server.load("models/environment/non-colliders/Gradient_Grass_medium_summer.glb#Scene0"),
        asset_server.load("models/environment/non-colliders/Gradient_Grass_medium_autumn.glb#Scene0"),
        asset_server.load("models/environment/non-colliders/Gradient_Grass_small_summer.glb#Scene0"),
        asset_server.load("models/environment/non-colliders/Gradient_Grass_small_autumn.glb#Scene0"),
    ];

    commands.insert_resource(TerrainAssets {
        collider_models,
        decorative_models,
    });
}

fn spawn_arena(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    arena_config: Res<ArenaConfig>,
    obstacle_config: Res<ObstacleConfig>,
    terrain_assets: Res<TerrainAssets>,
    terrain_sampler: Res<TerrainSampler>,
) {
    let arena_radius = arena_config.radius;
    let mut rng = rand::thread_rng();

    // Terrain ground with varying elevation
    let terrain_config = TerrainConfig {
        radius: arena_radius,
        subdivisions: 64,
        height_scale: 1.5,
        noise_scale: 0.08,
    };

    let terrain_mesh = generate_terrain_mesh(&terrain_config);
    let heights = generate_heights_matrix(&terrain_config);

    // Random ground color
    let ground_green = rng.gen_range(0.25..0.35);
    let ground_teal_shift = rng.gen_range(0.0..0.1);

    // Visual terrain mesh
    // bevy_heightmap generates mesh on XY plane with Z as height, but Bevy uses XZ plane with Y as height
    // Rotation converts Z-up to Y-up; after rotation, scale (X, Y, Z) maps to (width, width, height)
    commands.spawn((
        Mesh3d(meshes.add(terrain_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15 + ground_teal_shift, ground_green, 0.2 + ground_teal_shift),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::new(arena_radius * 2.0, arena_radius * 2.0, 1.0)),
        Arena,
        StateScoped(GameState::Playing),
        Name::new("Terrain"),
    ));

    // Physics collider (heightfield)
    commands.spawn((
        Collider::heightfield(heights, Vec3::new(arena_radius * 2.0, 1.0, arena_radius * 2.0)),
        RigidBody::Static,
        Transform::from_xyz(0.0, 0.0, 0.0),
        StateScoped(GameState::Playing),
        Name::new("Terrain Collider"),
    ));

    // Arena boundary walls (invisible but collidable)
    // Increase segments for larger arena
    let wall_height = 5.0;
    let wall_thickness = 1.0;
    let num_segments = 48; // More segments for larger, smoother boundary

    for i in 0..num_segments {
        let angle = (i as f32 / num_segments as f32) * std::f32::consts::TAU;
        let x = angle.cos() * (arena_radius + wall_thickness / 2.0);
        let z = angle.sin() * (arena_radius + wall_thickness / 2.0);

        commands.spawn((
            Transform::from_xyz(x, wall_height / 2.0, z)
                .with_rotation(Quat::from_rotation_y(-angle)),
            RigidBody::Static,
            Collider::cuboid(
                std::f32::consts::TAU * arena_radius / num_segments as f32,
                wall_height,
                wall_thickness,
            ),
            StateScoped(GameState::Playing),
        ));
    }

    // Spawn terrain obstacles
    spawn_obstacles_procedural(
        &mut commands,
        &arena_config,
        &obstacle_config,
        &terrain_assets,
        &terrain_sampler,
    );

    // Spawn decorative grass
    spawn_decorative_grass(
        &mut commands,
        &arena_config,
        &obstacle_config,
        &terrain_assets,
        &terrain_sampler,
    );
}

/// Check if a position is valid for obstacle placement
fn is_valid_position(
    pos: Vec3,
    obstacle_radius: f32,
    placed: &[(Vec3, f32)],
    min_spacing: f32,
    spawn_ring_radius: f32,
    spawn_exclusion: f32,
    center_exclusion: f32,
    arena_radius: f32,
) -> bool {
    let distance_from_center = (pos.x * pos.x + pos.z * pos.z).sqrt();

    // Check 1: Not too close to center (player spawn area)
    if distance_from_center < center_exclusion + obstacle_radius {
        return false;
    }

    // Check 2: Within arena bounds (with margin from walls)
    let wall_margin = 4.0;
    if distance_from_center > arena_radius - wall_margin - obstacle_radius {
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

fn spawn_obstacles_procedural(
    commands: &mut Commands,
    arena_config: &ArenaConfig,
    obstacle_config: &ObstacleConfig,
    terrain_assets: &TerrainAssets,
    terrain_sampler: &TerrainSampler,
) {
    let mut rng = rand::thread_rng();
    let mut placed_positions: Vec<(Vec3, f32)> = vec![];
    let spawn_ring_radius = arena_config.radius * 0.7;
    let max_attempts = 100;

    if terrain_assets.collider_models.is_empty() {
        return;
    }

    // Spawn terrain obstacles (boulders and trees)
    // Boulders are indices 0-2, trees are indices 3+
    let num_boulders = 3;
    let num_trees = terrain_assets.collider_models.len() - num_boulders;

    for _ in 0..obstacle_config.obstacle_count {
        // Pick a random model with weighted selection (25% rocks, 75% trees)
        let model_index = if rng.gen_bool(0.25) {
            // Rock
            rng.gen_range(0..num_boulders)
        } else {
            // Tree
            num_boulders + rng.gen_range(0..num_trees)
        };
        let model = terrain_assets.collider_models[model_index].clone();

        // Set different radii based on model type (boulders are indices 0-2, trees are 3+)
        // avoidance_radius: for enemy movement (trunk/solid obstacle)
        // spawn_radius: for spawn exclusion and placement spacing (canopy)
        let (avoidance_radius, spawn_radius) = if model_index <= 2 {
            (1.5, 2.0) // Boulders: solid, similar sizes
        } else {
            (1.2, 3.0) // Trees: small trunk, large canopy
        };

        // Random rotation for variety
        let rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

        for _ in 0..max_attempts {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = rng.gen_range(
                obstacle_config.center_exclusion + spawn_radius
                    ..arena_config.radius - 4.0 - spawn_radius,
            );
            let x = angle.cos() * distance;
            let z = angle.sin() * distance;
            let pos = terrain_sampler.get_spawn_position(x, z, 0.0);

            if is_valid_position(
                pos,
                spawn_radius,
                &placed_positions,
                obstacle_config.min_spacing,
                spawn_ring_radius,
                obstacle_config.spawn_exclusion,
                obstacle_config.center_exclusion,
                arena_config.radius,
            ) {
                commands.spawn((
                    SceneRoot(model),
                    Transform::from_translation(pos).with_rotation(rotation),
                    RigidBody::Static,
                    ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
                    Obstacle,
                    ObstacleBounds {
                        avoidance_radius,
                        spawn_radius,
                    },
                    StateScoped(GameState::Playing),
                    Name::new("Terrain Obstacle"),
                ));

                placed_positions.push((pos, spawn_radius));
                break;
            }
        }
    }
}

fn spawn_decorative_grass(
    commands: &mut Commands,
    arena_config: &ArenaConfig,
    obstacle_config: &ObstacleConfig,
    terrain_assets: &TerrainAssets,
    terrain_sampler: &TerrainSampler,
) {
    let mut rng = rand::thread_rng();
    let grass_count = 500;

    if terrain_assets.decorative_models.is_empty() {
        return;
    }

    for _ in 0..grass_count {
        let model_index = rng.gen_range(0..terrain_assets.decorative_models.len());
        let model = terrain_assets.decorative_models[model_index].clone();

        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let distance = rng.gen_range(
            obstacle_config.center_exclusion..arena_config.radius - 3.0,
        );
        let x = angle.cos() * distance;
        let z = angle.sin() * distance;
        let pos = terrain_sampler.get_spawn_position(x, z, 0.0);

        // Random rotation for variety
        let rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

        commands.spawn((
            SceneRoot(model),
            Transform::from_translation(pos)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(0.7)),
            StateScoped(GameState::Playing),
            Name::new("Decorative Grass"),
        ));
    }
}

fn spawn_lighting(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<GradientSkyMaterial>>,
) {
    // Directional light (sun) - based on official Bevy skybox example
    // About 45 degrees down from horizontal, angled from front-right
    commands.spawn((
        DirectionalLight {
            illuminance: 2500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            std::f32::consts::FRAC_PI_4,   // 45 degrees yaw
            -std::f32::consts::FRAC_PI_4,  // 45 degrees pitch down (standard sun angle)
            0.0,
        )),
        // Configure shadow cascades for arena size (radius 40 units)
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            minimum_distance: 0.1,
            maximum_distance: 100.0,
            first_cascade_far_bound: 15.0,
            overlap_proportion: 0.3,
        }
        .build(),
        StateScoped(GameState::Playing),
    ));

    // Ambient light - higher for softer overall illumination
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.5, 0.5, 0.6),
        brightness: 1500.0,
    });

    // Gradient sky dome
    // Sky colors: deep blue at top, warm orange at horizon
    let sky_material = sky_materials.add(GradientSkyMaterial {
        top_color: LinearRgba::new(0.2, 0.4, 0.8, 1.0),    // Sky blue
        bottom_color: LinearRgba::new(0.95, 0.6, 0.3, 1.0), // Warm orange
    });

    // Create inverted sphere for sky dome (normals face inward)
    let mut sky_mesh = Sphere::new(500.0).mesh().build();
    // Flip normals by reversing indices for inside-out rendering
    if let Some(indices) = sky_mesh.indices_mut() {
        match indices {
            bevy::render::mesh::Indices::U16(ref mut v) => {
                for chunk in v.chunks_mut(3) {
                    chunk.swap(0, 2);
                }
            }
            bevy::render::mesh::Indices::U32(ref mut v) => {
                for chunk in v.chunks_mut(3) {
                    chunk.swap(0, 2);
                }
            }
        }
    }

    commands.spawn((
        Mesh3d(meshes.add(sky_mesh)),
        MeshMaterial3d(sky_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
        NotShadowCaster,
        NotShadowReceiver,
        StateScoped(GameState::Playing),
        Name::new("Sky Dome"),
    ));
}
