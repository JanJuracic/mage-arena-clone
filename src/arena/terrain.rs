use bevy::prelude::*;
use bevy_heightmap::HeightMap;
use noise::{NoiseFn, Perlin};

pub struct TerrainConfig {
    pub radius: f32,
    pub subdivisions: u32,
    pub height_scale: f32,
    pub noise_scale: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            radius: 40.0,
            subdivisions: 64,
            height_scale: 1.5,
            noise_scale: 0.08,
        }
    }
}

/// Generate terrain mesh using Perlin noise
pub fn generate_terrain_mesh(config: &TerrainConfig) -> Mesh {
    let perlin = Perlin::new(42);
    let radius = config.radius;
    let noise_scale = config.noise_scale;
    let height_scale = config.height_scale;

    let heightmap = HeightMap {
        size: UVec2::splat(config.subdivisions),
        h: move |p: Vec2| {
            // Convert from normalized [-0.5, 0.5] to world coords, then apply noise_scale
            let wx = p.x * radius * 2.0 * noise_scale;
            let wy = p.y * radius * 2.0 * noise_scale;

            // Multi-octave noise for natural look
            let height = perlin.get([wx as f64, wy as f64]) as f32
                + 0.5 * perlin.get([wx as f64 * 2.0, wy as f64 * 2.0]) as f32
                + 0.25 * perlin.get([wx as f64 * 4.0, wy as f64 * 4.0]) as f32;

            height * height_scale
        },
    };

    heightmap.into()
}

/// Generate height data for physics collider
pub fn generate_heights_matrix(config: &TerrainConfig) -> Vec<Vec<f32>> {
    let perlin = Perlin::new(42);
    let size = config.subdivisions as usize + 1;
    let step = (config.radius * 2.0) / config.subdivisions as f32;

    (0..size)
        .map(|x| {
            // Outer loop = rows = X axis (per Avian3D heightfield convention)
            (0..size)
                .map(|z| {
                    // Inner loop = columns = Z axis
                    let wx = (x as f32 * step - config.radius) * config.noise_scale;
                    // Negate Z to match visual mesh rotation transform
                    let wz = -(z as f32 * step - config.radius) * config.noise_scale;

                    let height = perlin.get([wx as f64, wz as f64]) as f32
                        + 0.5 * perlin.get([wx as f64 * 2.0, wz as f64 * 2.0]) as f32
                        + 0.25 * perlin.get([wx as f64 * 4.0, wz as f64 * 4.0]) as f32;

                    height * config.height_scale
                })
                .collect()
        })
        .collect()
}
