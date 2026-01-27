use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use std::f32::consts::PI;

use crate::arena::config::{ArenaConfig, PondConfig, WaterSettings};
use crate::arena::ponds::TerrainPonds;
use crate::arena::shape::ArenaShape;

/// How far below water the shore extends
const BELOW_WATER_DEPTH: f32 = -2.0;

/// Resource for sampling terrain height at any XZ position
/// Uses the same algorithm as arena/terrain.rs
#[derive(Resource)]
pub struct TerrainSampler {
    perlin: Perlin,
    shore_perlin: Perlin,
    shape: ArenaShape,
    height_scale: f32,
    noise_scale: f32,
    octaves: u32,
    ponds: TerrainPonds,
    pond_slope_degrees: f32,
    pond_max_depth: f32,
    pond_edge_smooth_width: f32,
    water_level: f32,
    base_elevation: f32,
    shore_width_min: f32,
    shore_width_max: f32,
    shore_noise_scale: f32,
}

impl TerrainSampler {
    /// Create a new TerrainSampler with matching parameters to terrain generation
    pub fn new(
        seed: u32,
        shape: ArenaShape,
        height_scale: f32,
        noise_scale: f32,
        octaves: u32,
        water_level: f32,
        base_elevation: f32,
        shore_width_min: f32,
        shore_width_max: f32,
        shore_noise_scale: f32,
    ) -> Self {
        Self {
            perlin: Perlin::new(seed),
            shore_perlin: Perlin::new(seed + 1000),
            shape,
            height_scale,
            noise_scale,
            octaves,
            ponds: TerrainPonds::default(),
            pond_slope_degrees: 25.0,
            pond_max_depth: 2.0,
            pond_edge_smooth_width: 1.5,
            water_level,
            base_elevation,
            shore_width_min,
            shore_width_max,
            shore_noise_scale,
        }
    }

    /// Create a TerrainSampler from ArenaConfig and WaterSettings
    pub fn from_config(config: &ArenaConfig, water_settings: &WaterSettings) -> Self {
        Self::new(
            config.seed as u32,
            config.shape.clone(),
            config.height_scale,
            config.noise_scale,
            config.octaves,
            water_settings.water_level,
            water_settings.base_elevation,
            water_settings.shore_width_min,
            water_settings.shore_width_max,
            water_settings.shore_slope_noise_scale,
        )
    }

    /// Set the terrain ponds and pond configuration
    pub fn set_ponds(&mut self, ponds: TerrainPonds, pond_config: &PondConfig) {
        self.ponds = ponds;
        self.pond_slope_degrees = pond_config.slope_degrees;
        self.pond_max_depth = pond_config.max_depth;
        self.pond_edge_smooth_width = pond_config.edge_smooth_width;
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
    /// Uses additive base elevation for terrain variation instead of clamping
    fn apply_shore_slope(&self, noise_height: f32, x: f32, z: f32) -> f32 {
        let edge_proximity = self.shape.edge_proximity(x, z);
        let angle = x.atan2(z);

        // Noise-based shore width variation
        let noise_x = angle.cos() as f64 * self.shore_noise_scale as f64;
        let noise_z = angle.sin() as f64 * self.shore_noise_scale as f64;
        let shore_noise = self.shore_perlin.get([noise_x, noise_z]) as f32;
        let shore_width = Self::lerp(
            self.shore_width_min,
            self.shore_width_max,
            (shore_noise + 1.0) / 2.0,
        );

        // Calculate where shore zone starts
        let radius_at_angle = self.shape.radius_at_angle(angle);
        let shore_start = 1.0 - (shore_width / radius_at_angle).min(0.5);

        // Interior terrain: base elevation + noise variation (no clamping)
        let interior_height = self.base_elevation + (noise_height * self.height_scale);

        if edge_proximity < shore_start {
            // Interior zone - full terrain variation
            interior_height
        } else {
            // Shore zone - smooth slope down to below water level
            let t = (edge_proximity - shore_start) / (1.0 - shore_start);
            Self::lerp(interior_height, BELOW_WATER_DEPTH, Self::smoothstep(t))
        }
    }

    /// Apply pond depths with smooth edge transitions
    fn apply_pond_depths(&self, base_height: f32, x: f32, z: f32) -> f32 {
        let (signed_dist, _) = self.ponds.signed_distance_to_pond(x, z);

        if signed_dist >= self.pond_edge_smooth_width {
            // Outside transition zone - return base height
            base_height
        } else if signed_dist >= 0.0 {
            // Transition zone: smooth from base_height toward water level
            let t = signed_dist / self.pond_edge_smooth_width;
            Self::lerp(0.0, base_height, Self::smoothstep(t))
        } else {
            // Inside pond - gentle slope with smooth entry
            let dist_inside = -signed_dist;
            let slope_tan = (self.pond_slope_degrees * PI / 180.0).tan();

            // Smooth entry transition at pond edge
            let entry_smooth = if dist_inside < self.pond_edge_smooth_width {
                Self::smoothstep(dist_inside / self.pond_edge_smooth_width)
            } else {
                1.0
            };

            let pond_depth = -(dist_inside * slope_tan * entry_smooth);
            pond_depth.max(-self.pond_max_depth)
        }
    }

    /// Sample terrain height at a given XZ world position
    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        let wx = x * self.noise_scale;
        // Negate Z to match visual terrain rotation transform
        let wz = -z * self.noise_scale;

        // Multi-octave noise for base terrain (normalized -1 to 1)
        let noise_height = self.sample_multi_octave_noise(wx as f64, wz as f64);

        // Apply shore slope (handles base_elevation + noise internally)
        let shore_adjusted = self.apply_shore_slope(noise_height, x, z);

        // Apply pond depths
        self.apply_pond_depths(shore_adjusted, x, z)
    }

    /// Calculate the terrain slope at a position (returns angle in radians)
    pub fn calculate_slope(&self, x: f32, z: f32) -> f32 {
        let delta = 0.5;
        let h_px = self.sample_height(x + delta, z);
        let h_nx = self.sample_height(x - delta, z);
        let h_pz = self.sample_height(x, z + delta);
        let h_nz = self.sample_height(x, z - delta);

        let dx = (h_px - h_nx) / (2.0 * delta);
        let dz = (h_pz - h_nz) / (2.0 * delta);

        (dx * dx + dz * dz).sqrt().atan()
    }

    /// Check if a position is underwater
    pub fn is_underwater(&self, x: f32, z: f32) -> bool {
        self.sample_height(x, z) < self.water_level
    }

    /// Get the water level
    pub fn water_level(&self) -> f32 {
        self.water_level
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

    /// Get the terrain ponds
    pub fn ponds(&self) -> &TerrainPonds {
        &self.ponds
    }
}

impl Default for TerrainSampler {
    fn default() -> Self {
        let mut sampler = Self::new(
            42,
            ArenaShape::circle(40.0),
            2.4,
            0.055,
            3,
            0.0,  // water_level
            4.0,  // base_elevation
            8.0,  // shore_width_min
            14.0, // shore_width_max
            3.0,  // shore_noise_scale
        );
        sampler.pond_edge_smooth_width = 1.5;
        sampler
    }
}
