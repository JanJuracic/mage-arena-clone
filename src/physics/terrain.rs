use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

/// Resource for sampling terrain height at any XZ position
/// Uses the same Perlin noise algorithm as arena/terrain.rs
#[derive(Resource)]
pub struct TerrainSampler {
    perlin: Perlin,
    height_scale: f32,
    noise_scale: f32,
}

impl TerrainSampler {
    /// Create a new TerrainSampler with matching parameters to terrain generation
    pub fn new(seed: u32, height_scale: f32, noise_scale: f32) -> Self {
        Self {
            perlin: Perlin::new(seed),
            height_scale,
            noise_scale,
        }
    }

    /// Sample terrain height at a given XZ world position
    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        let wx = x * self.noise_scale;
        // Negate Z to match visual terrain rotation transform
        let wz = -z * self.noise_scale;

        // Multi-octave noise matching arena/terrain.rs
        let height = self.perlin.get([wx as f64, wz as f64]) as f32
            + 0.5 * self.perlin.get([wx as f64 * 2.0, wz as f64 * 2.0]) as f32
            + 0.25 * self.perlin.get([wx as f64 * 4.0, wz as f64 * 4.0]) as f32;

        height * self.height_scale
    }

    /// Get a spawn position at given XZ with a height offset above terrain
    pub fn get_spawn_position(&self, x: f32, z: f32, height_offset: f32) -> Vec3 {
        let terrain_height = self.sample_height(x, z);
        Vec3::new(x, terrain_height + height_offset, z)
    }
}

impl Default for TerrainSampler {
    fn default() -> Self {
        // Default parameters matching arena/terrain.rs
        Self::new(42, 1.5, 0.08)
    }
}
