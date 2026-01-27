use bevy::prelude::*;
use bevy_heightmap::HeightMap;
use noise::{NoiseFn, Perlin};
use std::f32::consts::PI;

use crate::arena::config::{ArenaConfig, PondConfig, WaterSettings};
use crate::arena::ponds::TerrainPonds;
use crate::arena::shape::ArenaShape;

/// Water level constant
const WATER_LEVEL: f32 = 0.0;
/// How far below water the shore extends
const BELOW_WATER_DEPTH: f32 = -2.0;
/// Maximum slope angle (in radians) where grass can grow (30 degrees)
pub const MAX_GRASS_SLOPE_RAD: f32 = 0.5236;

/// Terrain generation configuration (used internally for mesh generation)
pub struct TerrainMeshConfig {
    pub shape: ArenaShape,
    pub subdivisions: u32,
    pub height_scale: f32,
    pub noise_scale: f32,
    pub octaves: u32,
    pub seed: u32,
    pub ponds: TerrainPonds,
    pub pond_slope_degrees: f32,
    pub pond_max_depth: f32,
    pub pond_edge_smooth_width: f32,
    pub water_level: f32,
    pub base_elevation: f32,
    pub shore_width_min: f32,
    pub shore_width_max: f32,
    pub shore_noise_scale: f32,
    pub shore_noise_seed: u32,
    pub playable_min_height: f32,
}

impl TerrainMeshConfig {
    /// Create from ArenaConfig resource with ponds and water settings
    pub fn from_arena_config(
        config: &ArenaConfig,
        ponds: TerrainPonds,
        pond_config: &PondConfig,
        water_settings: &WaterSettings,
    ) -> Self {
        Self {
            shape: config.shape.clone(),
            subdivisions: config.subdivisions,
            height_scale: config.height_scale,
            noise_scale: config.noise_scale,
            octaves: config.octaves,
            seed: config.seed as u32,
            ponds,
            pond_slope_degrees: pond_config.slope_degrees,
            pond_max_depth: pond_config.max_depth,
            pond_edge_smooth_width: pond_config.edge_smooth_width,
            water_level: water_settings.water_level,
            base_elevation: water_settings.base_elevation,
            shore_width_min: water_settings.shore_width_min,
            shore_width_max: water_settings.shore_width_max,
            shore_noise_scale: water_settings.shore_slope_noise_scale,
            shore_noise_seed: config.seed as u32 + 1000,
            playable_min_height: water_settings.playable_min_height,
        }
    }
}

impl Default for TerrainMeshConfig {
    fn default() -> Self {
        Self {
            shape: ArenaShape::circle(40.0),
            subdivisions: 64,
            height_scale: 2.4,
            noise_scale: 0.055,
            octaves: 3,
            seed: 42,
            ponds: TerrainPonds::default(),
            pond_slope_degrees: 25.0,
            pond_max_depth: 2.0,
            pond_edge_smooth_width: 1.5,
            water_level: 0.0,
            base_elevation: 4.0,
            shore_width_min: 8.0,
            shore_width_max: 14.0,
            shore_noise_scale: 3.0,
            shore_noise_seed: 1042,
            playable_min_height: 2.0,
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

/// Linear interpolation
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smoothstep interpolation
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Apply shore slope to terrain height
/// Creates organic shore that slopes down into water at island edges
/// Uses additive base elevation for terrain variation instead of clamping
fn apply_shore_slope(
    noise_height: f32,
    height_scale: f32,
    base_elevation: f32,
    x: f32,
    z: f32,
    shape: &ArenaShape,
    shore_perlin: &Perlin,
    shore_width_min: f32,
    shore_width_max: f32,
    shore_noise_scale: f32,
) -> f32 {
    let edge_proximity = shape.edge_proximity(x, z);
    let angle = x.atan2(z);

    // Noise-based shore width variation for organic look
    let noise_x = angle.cos() as f64 * shore_noise_scale as f64;
    let noise_z = angle.sin() as f64 * shore_noise_scale as f64;
    let shore_noise = shore_perlin.get([noise_x, noise_z]) as f32;
    let shore_width = lerp(shore_width_min, shore_width_max, (shore_noise + 1.0) / 2.0);

    // Calculate where shore zone starts
    let radius_at_angle = shape.radius_at_angle(angle);
    let shore_start = 1.0 - (shore_width / radius_at_angle).min(0.5);

    // Interior terrain: base elevation + noise variation (no clamping)
    let interior_height = base_elevation + (noise_height * height_scale);

    if edge_proximity < shore_start {
        // Interior zone - full terrain variation
        interior_height
    } else {
        // Shore zone - smooth slope down to below water level
        let t = (edge_proximity - shore_start) / (1.0 - shore_start);
        lerp(interior_height, BELOW_WATER_DEPTH, smoothstep(t))
    }
}

/// Apply pond depths to terrain height
/// Creates gentle slopes into interior pond areas with smooth edge transitions
fn apply_pond_depths(
    base_height: f32,
    x: f32,
    z: f32,
    ponds: &TerrainPonds,
    pond_slope_degrees: f32,
    pond_max_depth: f32,
    edge_smooth_width: f32,
) -> f32 {
    let (signed_dist, _) = ponds.signed_distance_to_pond(x, z);

    if signed_dist >= edge_smooth_width {
        // Outside transition zone - return base height
        base_height
    } else if signed_dist >= 0.0 {
        // Transition zone: smooth from base_height toward water level
        let t = signed_dist / edge_smooth_width;
        lerp(0.0, base_height, smoothstep(t))
    } else {
        // Inside pond - gentle slope with smooth entry
        let dist_inside = -signed_dist;
        let slope_tan = (pond_slope_degrees * PI / 180.0).tan();

        // Smooth entry transition at pond edge
        let entry_smooth = if dist_inside < edge_smooth_width {
            smoothstep(dist_inside / edge_smooth_width)
        } else {
            1.0
        };

        let pond_depth = -(dist_inside * slope_tan * entry_smooth);
        pond_depth.max(-pond_max_depth)
    }
}

/// Generate terrain mesh using Perlin noise with shore slopes and ponds
pub fn generate_terrain_mesh(config: &TerrainMeshConfig) -> Mesh {
    let perlin = Perlin::new(config.seed);
    let shore_perlin = Perlin::new(config.shore_noise_seed);
    let radius = config.shape.base_radius;
    let noise_scale = config.noise_scale;
    let height_scale = config.height_scale;
    let base_elevation = config.base_elevation;
    let octaves = config.octaves;
    let shape = config.shape.clone();
    let ponds = config.ponds.clone();
    let shore_width_min = config.shore_width_min;
    let shore_width_max = config.shore_width_max;
    let shore_noise_scale = config.shore_noise_scale;
    let pond_slope_degrees = config.pond_slope_degrees;
    let pond_max_depth = config.pond_max_depth;
    let pond_edge_smooth_width = config.pond_edge_smooth_width;

    let heightmap = HeightMap {
        size: UVec2::splat(config.subdivisions),
        h: move |p: Vec2| {
            // Convert from normalized [-0.5, 0.5] to world coords
            let world_x = p.x * radius * 2.0;
            let world_z = p.y * radius * 2.0;

            // Apply noise scale for sampling
            let wx = world_x * noise_scale;
            let wz = world_z * noise_scale;

            // Multi-octave noise for base terrain (normalized -1 to 1)
            let noise_height = sample_multi_octave_noise(&perlin, wx as f64, wz as f64, octaves);

            // Apply shore slope (island edges slope into water)
            let shore_adjusted = apply_shore_slope(
                noise_height,
                height_scale,
                base_elevation,
                world_x,
                world_z,
                &shape,
                &shore_perlin,
                shore_width_min,
                shore_width_max,
                shore_noise_scale,
            );

            // Apply pond depths (interior water features)
            apply_pond_depths(
                shore_adjusted,
                world_x,
                world_z,
                &ponds,
                pond_slope_degrees,
                pond_max_depth,
                pond_edge_smooth_width,
            )
        },
    };

    heightmap.into()
}

/// Generate height data for physics collider with shore slopes and ponds
pub fn generate_heights_matrix(config: &TerrainMeshConfig) -> Vec<Vec<f32>> {
    let perlin = Perlin::new(config.seed);
    let shore_perlin = Perlin::new(config.shore_noise_seed);
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

                    // Multi-octave noise for base terrain (normalized -1 to 1)
                    let noise_height =
                        sample_multi_octave_noise(&perlin, wx as f64, wz as f64, config.octaves);

                    // Apply shore slope
                    let shore_adjusted = apply_shore_slope(
                        noise_height,
                        config.height_scale,
                        config.base_elevation,
                        world_x,
                        world_z,
                        &config.shape,
                        &shore_perlin,
                        config.shore_width_min,
                        config.shore_width_max,
                        config.shore_noise_scale,
                    );

                    // Apply pond depths
                    apply_pond_depths(
                        shore_adjusted,
                        world_x,
                        world_z,
                        &config.ponds,
                        config.pond_slope_degrees,
                        config.pond_max_depth,
                        config.pond_edge_smooth_width,
                    )
                })
                .collect()
        })
        .collect()
}
