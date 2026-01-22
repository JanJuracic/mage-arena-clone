pub mod assets;
pub mod boundary;
pub mod config;
pub mod shape;
pub mod spawning;
mod terrain;

use bevy::pbr::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use avian3d::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::physics::TerrainSampler;
use crate::states::GameState;

use assets::{load_terrain_assets, TerrainAssets};
use boundary::spawn_boulder_boundary;
use config::{ArenaConfigFile, ArenaConfigFileRes, BoundaryConfig, DecorationConfig, ObstacleConfig};
use spawning::{spawn_grass, spawn_mushrooms, spawn_obstacles};
use terrain::{generate_heights_matrix, generate_terrain_mesh, TerrainMeshConfig};

// Re-export commonly used types for external modules
pub use config::{ArenaConfig, BiomeTheme};
pub use shape::ArenaShape;
pub use spawning::{Obstacle, ObstacleBounds};

/// Path to the arena configuration file
const ARENA_CONFIG_PATH: &str = "assets/config/arena.ron";

/// Gradient sky material for the sky dome
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
        // Load configuration from file (or use defaults)
        let config_file = ArenaConfigFile::load_or_default(ARENA_CONFIG_PATH);

        // Resolve seed first
        let seed = config_file.get_seed();
        let mut init_rng = ChaCha8Rng::seed_from_u64(seed);

        // Resolve presets and theme using the RNG
        let theme = config_file.get_resolved_theme(&mut init_rng);
        let size_preset = config_file.get_resolved_size_preset(&mut init_rng);
        let terrain_preset = config_file.get_resolved_terrain_preset(&mut init_rng);
        let atmosphere_preset = config_file.get_resolved_atmosphere_preset(&mut init_rng);

        info!(
            "Arena initialized with seed {}, theme {:?}, size '{}', terrain '{}', atmosphere '{}'",
            seed, theme, size_preset.name, terrain_preset.name, atmosphere_preset.name
        );

        // Create runtime config resources with resolved presets
        let arena_config = ArenaConfig::from_config_file(&config_file, seed, theme, &size_preset, &terrain_preset, &atmosphere_preset);

        // Calculate spawn counts based on arena area
        let area = arena_config.area;
        let obstacle_config = ObstacleConfig::from_config_file(&config_file, area);
        let boundary_config = BoundaryConfig::from_config_file(&config_file);
        let decoration_config = DecorationConfig::from_config_file(&config_file, theme, area);

        // Create TerrainSampler from config
        let terrain_sampler = TerrainSampler::from_config(&arena_config);

        app.add_plugins(MaterialPlugin::<GradientSkyMaterial>::default())
            // Store raw config file for reference
            .insert_resource(ArenaConfigFileRes(config_file))
            // Runtime configs
            .insert_resource(arena_config)
            .insert_resource(obstacle_config)
            .insert_resource(boundary_config)
            .insert_resource(decoration_config)
            .insert_resource(terrain_sampler)
            // Asset loading
            .init_resource::<TerrainAssets>()
            .add_systems(Startup, load_terrain_assets)
            // Arena spawning - regenerate config first, then spawn
            .add_systems(
                OnEnter(GameState::Playing),
                (regenerate_arena_config, spawn_arena, spawn_lighting).chain(),
            );
    }
}

/// Regenerate arena configuration with a new seed for each level
fn regenerate_arena_config(
    config_file_res: Res<ArenaConfigFileRes>,
    mut arena_config: ResMut<ArenaConfig>,
    mut obstacle_config: ResMut<ObstacleConfig>,
    mut decoration_config: ResMut<DecorationConfig>,
    mut terrain_sampler: ResMut<TerrainSampler>,
) {
    let config_file = &config_file_res.0;

    // Generate a new random seed based on current time
    let new_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);

    let mut rng = ChaCha8Rng::seed_from_u64(new_seed);

    // Resolve presets with new RNG
    let theme = config_file.get_resolved_theme(&mut rng);
    let size_preset = config_file.get_resolved_size_preset(&mut rng);
    let terrain_preset = config_file.get_resolved_terrain_preset(&mut rng);
    let atmosphere_preset = config_file.get_resolved_atmosphere_preset(&mut rng);

    // Update arena config
    *arena_config = ArenaConfig::from_config_file(
        config_file,
        new_seed,
        theme,
        &size_preset,
        &terrain_preset,
        &atmosphere_preset,
    );

    // Update dependent configs
    let area = arena_config.area;
    *obstacle_config = ObstacleConfig::from_config_file(config_file, area);
    *decoration_config = DecorationConfig::from_config_file(config_file, theme, area);

    // Update terrain sampler
    *terrain_sampler = TerrainSampler::from_config(&arena_config);

    info!(
        "Arena regenerated: seed={}, theme={:?}, size='{}', terrain='{}', atmosphere='{}'",
        new_seed, theme, size_preset.name, terrain_preset.name, atmosphere_preset.name
    );
}

#[derive(Component)]
pub struct Arena;

fn spawn_arena(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    arena_config: Res<ArenaConfig>,
    obstacle_config: Res<ObstacleConfig>,
    boundary_config: Res<BoundaryConfig>,
    decoration_config: Res<DecorationConfig>,
    terrain_assets: Res<TerrainAssets>,
    terrain_sampler: Res<TerrainSampler>,
) {
    // Create seeded RNG for reproducible generation
    let mut rng = ChaCha8Rng::seed_from_u64(arena_config.seed);

    let arena_radius = arena_config.radius();

    // Generate terrain mesh configuration
    let terrain_mesh_config = TerrainMeshConfig::from_arena_config(&arena_config);
    let terrain_mesh = generate_terrain_mesh(&terrain_mesh_config);
    let heights = generate_heights_matrix(&terrain_mesh_config);

    // Ground color based on atmosphere preset (hue/saturation/brightness shifts)
    // Base green color with atmosphere adjustments
    let base_green = 0.3;
    let ground_r = (base_green - arena_config.ground_hue_shift * 0.5) * arena_config.ground_brightness * 2.0;
    let ground_g = base_green * arena_config.ground_saturation * arena_config.ground_brightness * 2.0;
    let ground_b = (base_green * 0.6 - arena_config.ground_hue_shift * 0.3) * arena_config.ground_brightness * 2.0;

    // Visual terrain mesh
    // bevy_heightmap generates mesh on XY plane with Z as height, but Bevy uses XZ plane with Y as height
    // Rotation converts Z-up to Y-up; after rotation, scale (X, Y, Z) maps to (width, width, height)
    commands.spawn((
        Mesh3d(meshes.add(terrain_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(ground_r, ground_g, ground_b),
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

    // Spawn boulder boundary wall following the irregular contour
    spawn_boulder_boundary(
        &mut commands,
        &arena_config.shape,
        &boundary_config,
        &terrain_assets,
        &terrain_sampler,
        &mut rng,
    );

    // Spawn terrain obstacles (trees and rocks) with density-based counts
    spawn_obstacles(
        &mut commands,
        &arena_config,
        &obstacle_config,
        &decoration_config,
        &terrain_assets,
        &terrain_sampler,
        &mut rng,
    );

    // Spawn decorative grass with density-based counts
    spawn_grass(
        &mut commands,
        &arena_config,
        &obstacle_config,
        &decoration_config,
        &terrain_assets,
        &terrain_sampler,
        &mut rng,
    );

    // Spawn mushrooms with density-based counts
    spawn_mushrooms(
        &mut commands,
        &arena_config,
        &obstacle_config,
        &decoration_config,
        &terrain_assets,
        &terrain_sampler,
        &mut rng,
    );

    info!(
        "Arena spawned: size='{}' (radius={:.1}), terrain='{}', atmosphere='{}', area={:.0} sqm, seed={}, theme={:?}",
        arena_config.size_preset_name,
        arena_radius,
        arena_config.terrain_preset_name,
        arena_config.atmosphere_preset_name,
        arena_config.area,
        arena_config.seed,
        arena_config.theme
    );
}

fn spawn_lighting(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<GradientSkyMaterial>>,
    arena_config: Res<ArenaConfig>,
) {
    // Directional light (sun) - direction and color from atmosphere preset
    commands.spawn((
        DirectionalLight {
            illuminance: arena_config.sun_illuminance,
            color: Color::srgb(
                arena_config.sun_color[0],
                arena_config.sun_color[1],
                arena_config.sun_color[2],
            ),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            arena_config.sun_yaw,
            arena_config.sun_pitch,
            0.0,
        )),
        // Configure shadow cascades for arena size
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

    // Ambient light from atmosphere preset
    commands.insert_resource(AmbientLight {
        color: Color::srgb(
            arena_config.ambient_color[0],
            arena_config.ambient_color[1],
            arena_config.ambient_color[2],
        ),
        brightness: arena_config.ambient_brightness,
    });

    // Gradient sky dome - colors from atmosphere preset
    let top_color = LinearRgba::new(
        arena_config.sky_top_color[0],
        arena_config.sky_top_color[1],
        arena_config.sky_top_color[2],
        1.0,
    );
    let bottom_color = LinearRgba::new(
        arena_config.sky_bottom_color[0],
        arena_config.sky_bottom_color[1],
        arena_config.sky_bottom_color[2],
        1.0,
    );

    let sky_material = sky_materials.add(GradientSkyMaterial {
        top_color,
        bottom_color,
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
