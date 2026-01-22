use bevy::prelude::*;
use bevy_heightmap::HeightMap;
use noise::{NoiseFn, Perlin};

use crate::arena::config::ArenaConfig;
use crate::arena::shape::ArenaShape;

/// Terrain generation configuration (used internally for mesh generation)
pub struct TerrainMeshConfig {
    pub shape: ArenaShape,
    pub subdivisions: u32,
    pub height_scale: f32,
    pub noise_scale: f32,
    pub octaves: u32,
    pub edge_falloff: f32,
    pub seed: u32,
}

impl TerrainMeshConfig {
    /// Create from ArenaConfig resource
    pub fn from_arena_config(config: &ArenaConfig) -> Self {
        Self {
            shape: config.shape.clone(),
            subdivisions: config.subdivisions,
            height_scale: config.height_scale,
            noise_scale: config.noise_scale,
            octaves: config.octaves,
            edge_falloff: config.edge_falloff,
            seed: config.seed as u32,
        }
    }
}

impl Default for TerrainMeshConfig {
    fn default() -> Self {
        Self {
            shape: ArenaShape::circle(40.0),
            subdivisions: 64,
            height_scale: 1.5,
            noise_scale: 0.08,
            octaves: 3,
            edge_falloff: 0.5,
            seed: 42,
        }
    }
}

/// Calculate multi-octave noise value at a position
fn sample_multi_octave_noise(perlin: &Perlin, x: f64, z: f64, octaves: u32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for _ in 0..octaves {
        total += perlin.get([x * frequency, z * frequency]) as f32 * amplitude;
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    // Normalize to roughly -1 to 1 range
    total / max_value
}

/// Calculate edge falloff multiplier based on distance from center
/// Returns 1.0 at center, smoothly transitions to 0.0 at edge
fn calculate_edge_falloff(x: f32, z: f32, shape: &ArenaShape, falloff: f32) -> f32 {
    if falloff <= 0.0 {
        return 1.0;
    }

    let edge_proximity = shape.edge_proximity(x, z);
    let falloff_start = 1.0 - falloff;

    if edge_proximity > falloff_start {
        let t = (edge_proximity - falloff_start) / falloff;
        // Smooth falloff using smoothstep
        let t_clamped = t.clamp(0.0, 1.0);
        1.0 - (t_clamped * t_clamped * (3.0 - 2.0 * t_clamped))
    } else {
        1.0
    }
}

/// Generate terrain mesh using Perlin noise with configurable octaves and edge falloff
pub fn generate_terrain_mesh(config: &TerrainMeshConfig) -> Mesh {
    let perlin = Perlin::new(config.seed);
    let radius = config.shape.base_radius;
    let noise_scale = config.noise_scale;
    let height_scale = config.height_scale;
    let octaves = config.octaves;
    let edge_falloff = config.edge_falloff;
    let shape = config.shape.clone();

    let heightmap = HeightMap {
        size: UVec2::splat(config.subdivisions),
        h: move |p: Vec2| {
            // Convert from normalized [-0.5, 0.5] to world coords
            let world_x = p.x * radius * 2.0;
            let world_z = p.y * radius * 2.0;

            // Apply noise scale for sampling
            let wx = world_x * noise_scale;
            let wz = world_z * noise_scale;

            // Multi-octave noise
            let height = sample_multi_octave_noise(&perlin, wx as f64, wz as f64, octaves);

            // Apply edge falloff
            let falloff_mult = calculate_edge_falloff(world_x, world_z, &shape, edge_falloff);

            height * height_scale * falloff_mult
        },
    };

    heightmap.into()
}

/// Generate height data for physics collider with configurable octaves and edge falloff
pub fn generate_heights_matrix(config: &TerrainMeshConfig) -> Vec<Vec<f32>> {
    let perlin = Perlin::new(config.seed);
    let size = config.subdivisions as usize + 1;
    let step = (config.shape.base_radius * 2.0) / config.subdivisions as f32;

    (0..size)
        .map(|x| {
            // Outer loop = rows = X axis (per Avian3D heightfield convention)
            (0..size)
                .map(|z| {
                    // Inner loop = columns = Z axis
                    let world_x = x as f32 * step - config.shape.base_radius;
                    // Negate Z to match visual mesh rotation transform
                    let world_z = -(z as f32 * step - config.shape.base_radius);

                    let wx = world_x * config.noise_scale;
                    let wz = world_z * config.noise_scale;

                    // Multi-octave noise
                    let height =
                        sample_multi_octave_noise(&perlin, wx as f64, wz as f64, config.octaves);

                    // Apply edge falloff
                    let falloff_mult =
                        calculate_edge_falloff(world_x, world_z, &config.shape, config.edge_falloff);

                    height * config.height_scale * falloff_mult
                })
                .collect()
        })
        .collect()
}
