use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

use crate::arena::config::ArenaConfig;
use crate::arena::shape::ArenaShape;

/// Resource for sampling terrain height at any XZ position
/// Uses the same Perlin noise algorithm as arena/terrain.rs
#[derive(Resource)]
pub struct TerrainSampler {
    perlin: Perlin,
    shape: ArenaShape,
    height_scale: f32,
    noise_scale: f32,
    octaves: u32,
    edge_falloff: f32,
}

impl TerrainSampler {
    /// Create a new TerrainSampler with matching parameters to terrain generation
    pub fn new(
        seed: u32,
        shape: ArenaShape,
        height_scale: f32,
        noise_scale: f32,
        octaves: u32,
        edge_falloff: f32,
    ) -> Self {
        Self {
            perlin: Perlin::new(seed),
            shape,
            height_scale,
            noise_scale,
            octaves,
            edge_falloff,
        }
    }

    /// Create a TerrainSampler from ArenaConfig
    pub fn from_config(config: &ArenaConfig) -> Self {
        Self::new(
            config.seed as u32,
            config.shape.clone(),
            config.height_scale,
            config.noise_scale,
            config.octaves,
            config.edge_falloff,
        )
    }

    /// Calculate multi-octave noise value at a position
    fn sample_multi_octave_noise(&self, x: f64, z: f64) -> f32 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        for _ in 0..self.octaves {
            total += self.perlin.get([x * frequency, z * frequency]) as f32 * amplitude;
            max_value += amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }

        // Normalize to roughly -1 to 1 range
        total / max_value
    }

    /// Calculate edge falloff multiplier based on distance from center
    fn calculate_edge_falloff(&self, x: f32, z: f32) -> f32 {
        if self.edge_falloff <= 0.0 {
            return 1.0;
        }

        let edge_proximity = self.shape.edge_proximity(x, z);
        let falloff_start = 1.0 - self.edge_falloff;

        if edge_proximity > falloff_start {
            let t = (edge_proximity - falloff_start) / self.edge_falloff;
            // Smooth falloff using smoothstep
            let t_clamped = t.clamp(0.0, 1.0);
            1.0 - (t_clamped * t_clamped * (3.0 - 2.0 * t_clamped))
        } else {
            1.0
        }
    }

    /// Sample terrain height at a given XZ world position
    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        let wx = x * self.noise_scale;
        // Negate Z to match visual terrain rotation transform
        let wz = -z * self.noise_scale;

        // Multi-octave noise
        let height = self.sample_multi_octave_noise(wx as f64, wz as f64);

        // Apply edge falloff
        let falloff_mult = self.calculate_edge_falloff(x, z);

        height * self.height_scale * falloff_mult
    }

    /// Get a spawn position at given XZ with a height offset above terrain
    pub fn get_spawn_position(&self, x: f32, z: f32, height_offset: f32) -> Vec3 {
        let terrain_height = self.sample_height(x, z);
        Vec3::new(x, terrain_height + height_offset, z)
    }

    /// Get the arena shape for boundary checks
    pub fn shape(&self) -> &ArenaShape {
        &self.shape
    }
}

impl Default for TerrainSampler {
    fn default() -> Self {
        // Default parameters matching arena/terrain.rs defaults
        Self::new(42, ArenaShape::circle(40.0), 1.5, 0.08, 3, 0.5)
    }
}
