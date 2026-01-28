use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Serializable gradient key for RGBA colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientKeyRGBA(pub f32, pub (f32, f32, f32, f32));

/// Serializable gradient key for f32 sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientKeyF32(pub f32, pub f32);

/// Explosion effect data loaded from RON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplosionEffectData {
    pub color_gradient: Vec<GradientKeyRGBA>,
    pub size_gradient: Vec<GradientKeyF32>,
    pub particle_count: f32,
    pub speed: f32,
    pub lifetime: f32,
    pub spawn_radius: f32,
    pub drag: f32,
    pub gravity: (f32, f32, f32),
    pub circle_spawn: bool,
    pub spawn_offset: (f32, f32, f32),
}

impl ExplosionEffectData {
    /// Convert to bevy_hanabi Gradient<Vec4> for colors
    pub fn to_color_gradient(&self) -> Gradient<Vec4> {
        let mut gradient = Gradient::new();
        for key in &self.color_gradient {
            gradient.add_key(key.0, Vec4::new(key.1.0, key.1.1, key.1.2, key.1.3));
        }
        gradient
    }

    /// Convert to bevy_hanabi Gradient<Vec3> for sizes
    pub fn to_size_gradient(&self) -> Gradient<Vec3> {
        let mut gradient = Gradient::new();
        for key in &self.size_gradient {
            gradient.add_key(key.0, Vec3::splat(key.1));
        }
        gradient
    }

    /// Convert gravity tuple to Vec3
    pub fn gravity_vec(&self) -> Vec3 {
        Vec3::new(self.gravity.0, self.gravity.1, self.gravity.2)
    }

    /// Convert spawn_offset tuple to Vec3
    pub fn spawn_offset_vec(&self) -> Vec3 {
        Vec3::new(self.spawn_offset.0, self.spawn_offset.1, self.spawn_offset.2)
    }
}

/// Trail effect data loaded from RON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailEffectData {
    pub color_gradient: Vec<GradientKeyRGBA>,
    pub size_gradient: Vec<GradientKeyF32>,
    pub spawn_rate: f32,
    pub speed: f32,
    pub lifetime: f32,
    pub spawn_radius: f32,
    pub gravity: (f32, f32, f32),
}

impl TrailEffectData {
    /// Convert to bevy_hanabi Gradient<Vec4> for colors
    pub fn to_color_gradient(&self) -> Gradient<Vec4> {
        let mut gradient = Gradient::new();
        for key in &self.color_gradient {
            gradient.add_key(key.0, Vec4::new(key.1.0, key.1.1, key.1.2, key.1.3));
        }
        gradient
    }

    /// Convert to bevy_hanabi Gradient<Vec3> for sizes
    pub fn to_size_gradient(&self) -> Gradient<Vec3> {
        let mut gradient = Gradient::new();
        for key in &self.size_gradient {
            gradient.add_key(key.0, Vec3::splat(key.1));
        }
        gradient
    }

    /// Convert gravity tuple to Vec3
    pub fn gravity_vec(&self) -> Vec3 {
        Vec3::new(self.gravity.0, self.gravity.1, self.gravity.2)
    }
}

/// Root configuration structure for particles_explosions.ron
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleExplosionsFile {
    pub effects: HashMap<String, ExplosionEffectData>,
}

impl Default for ParticleExplosionsFile {
    fn default() -> Self {
        let mut effects = HashMap::new();

        // Fire explosion
        effects.insert(
            "fire".to_string(),
            ExplosionEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (1.0, 1.0, 0.6, 1.0)),
                    GradientKeyRGBA(0.15, (1.0, 0.8, 0.2, 1.0)),
                    GradientKeyRGBA(0.4, (1.0, 0.4, 0.1, 1.0)),
                    GradientKeyRGBA(0.7, (0.9, 0.2, 0.05, 0.8)),
                    GradientKeyRGBA(1.0, (0.3, 0.1, 0.05, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.08),
                    GradientKeyF32(0.3, 0.15),
                    GradientKeyF32(0.7, 0.1),
                    GradientKeyF32(1.0, 0.0),
                ],
                particle_count: 300.0,
                speed: 12.0,
                lifetime: 0.9,
                spawn_radius: 0.5,
                drag: 3.0,
                gravity: (0.0, -8.0, 0.0),
                circle_spawn: false,
                spawn_offset: (0.0, 0.0, 0.0),
            },
        );

        // Smoke
        effects.insert(
            "smoke".to_string(),
            ExplosionEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (0.3, 0.25, 0.2, 0.6)),
                    GradientKeyRGBA(0.3, (0.25, 0.22, 0.18, 0.5)),
                    GradientKeyRGBA(0.6, (0.2, 0.18, 0.15, 0.3)),
                    GradientKeyRGBA(1.0, (0.15, 0.13, 0.1, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.1),
                    GradientKeyF32(0.3, 0.25),
                    GradientKeyF32(0.7, 0.35),
                    GradientKeyF32(1.0, 0.4),
                ],
                particle_count: 80.0,
                speed: 4.0,
                lifetime: 1.5,
                spawn_radius: 0.6,
                drag: 2.5,
                gravity: (0.0, 3.0, 0.0),
                circle_spawn: false,
                spawn_offset: (0.0, 0.0, 0.0),
            },
        );

        // Frost
        effects.insert(
            "frost".to_string(),
            ExplosionEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (1.0, 1.0, 1.0, 1.0)),
                    GradientKeyRGBA(0.2, (0.7, 0.9, 1.0, 1.0)),
                    GradientKeyRGBA(0.5, (0.4, 0.7, 1.0, 0.9)),
                    GradientKeyRGBA(0.8, (0.2, 0.5, 0.9, 0.6)),
                    GradientKeyRGBA(1.0, (0.1, 0.3, 0.7, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.06),
                    GradientKeyF32(0.2, 0.12),
                    GradientKeyF32(0.6, 0.08),
                    GradientKeyF32(1.0, 0.0),
                ],
                particle_count: 250.0,
                speed: 8.0,
                lifetime: 1.0,
                spawn_radius: 0.4,
                drag: 4.0,
                gravity: (0.0, 2.0, 0.0),
                circle_spawn: false,
                spawn_offset: (0.0, 0.0, 0.0),
            },
        );

        // Arcane
        effects.insert(
            "arcane".to_string(),
            ExplosionEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (0.8, 0.4, 1.0, 1.0)),
                    GradientKeyRGBA(0.5, (0.6, 0.2, 0.9, 0.7)),
                    GradientKeyRGBA(1.0, (0.3, 0.1, 0.5, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.06),
                    GradientKeyF32(0.5, 0.1),
                    GradientKeyF32(1.0, 0.03),
                ],
                particle_count: 200.0,
                speed: 8.0,
                lifetime: 0.5,
                spawn_radius: 0.5,
                drag: 2.0,
                gravity: (0.0, 0.0, 0.0),
                circle_spawn: true,
                spawn_offset: (0.0, 0.0, 0.0),
            },
        );

        // Hit spark
        effects.insert(
            "hit_spark".to_string(),
            ExplosionEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (1.0, 1.0, 1.0, 1.0)),
                    GradientKeyRGBA(0.2, (0.9, 0.7, 1.0, 1.0)),
                    GradientKeyRGBA(0.5, (0.7, 0.3, 1.0, 0.9)),
                    GradientKeyRGBA(1.0, (0.4, 0.1, 0.6, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.05),
                    GradientKeyF32(0.3, 0.1),
                    GradientKeyF32(1.0, 0.0),
                ],
                particle_count: 150.0,
                speed: 8.0,
                lifetime: 0.5,
                spawn_radius: 0.2,
                drag: 4.0,
                gravity: (0.0, 0.0, 0.0),
                circle_spawn: false,
                spawn_offset: (0.0, 0.0, 0.0),
            },
        );

        // Muzzle flash
        effects.insert(
            "muzzle_flash".to_string(),
            ExplosionEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (1.0, 0.9, 0.7, 1.0)),
                    GradientKeyRGBA(0.3, (1.0, 0.6, 0.3, 0.6)),
                    GradientKeyRGBA(1.0, (0.8, 0.3, 0.1, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.08),
                    GradientKeyF32(0.2, 0.12),
                    GradientKeyF32(1.0, 0.0),
                ],
                particle_count: 30.0,
                speed: 2.0,
                lifetime: 0.15,
                spawn_radius: 0.05,
                drag: 0.0,
                gravity: (0.0, 0.0, 0.0),
                circle_spawn: false,
                spawn_offset: (0.0, 0.0, 0.0),
            },
        );

        // Enemy death
        effects.insert(
            "enemy_death".to_string(),
            ExplosionEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (1.0, 0.2, 0.1, 1.0)),
                    GradientKeyRGBA(0.2, (0.8, 0.1, 0.05, 0.9)),
                    GradientKeyRGBA(0.5, (0.4, 0.05, 0.02, 0.7)),
                    GradientKeyRGBA(0.7, (0.15, 0.1, 0.1, 0.5)),
                    GradientKeyRGBA(1.0, (0.05, 0.03, 0.03, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.05),
                    GradientKeyF32(0.3, 0.1),
                    GradientKeyF32(0.7, 0.125),
                    GradientKeyF32(1.0, 0.075),
                ],
                particle_count: 200.0,
                speed: 3.0,
                lifetime: 1.2,
                spawn_radius: 0.5,
                drag: 2.0,
                gravity: (0.0, -4.0, 0.0),
                circle_spawn: false,
                spawn_offset: (0.0, 0.5, 0.0),
            },
        );

        // Enemy spawn
        effects.insert(
            "enemy_spawn".to_string(),
            ExplosionEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (0.9, 0.5, 1.0, 1.0)),
                    GradientKeyRGBA(0.2, (0.7, 0.3, 0.9, 0.9)),
                    GradientKeyRGBA(0.5, (0.5, 0.1, 0.8, 0.7)),
                    GradientKeyRGBA(0.8, (0.3, 0.05, 0.5, 0.4)),
                    GradientKeyRGBA(1.0, (0.1, 0.0, 0.2, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.3),
                    GradientKeyF32(0.3, 0.5),
                    GradientKeyF32(0.7, 0.35),
                    GradientKeyF32(1.0, 0.0),
                ],
                particle_count: 300.0,
                speed: 8.0,
                lifetime: 1.0,
                spawn_radius: 2.0,
                drag: 2.0,
                gravity: (0.0, 8.0, 0.0),
                circle_spawn: true,
                spawn_offset: (0.0, 0.0, 0.0),
            },
        );

        Self { effects }
    }
}

/// Root configuration structure for particles_trails.ron
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleTrailsFile {
    pub trails: HashMap<String, TrailEffectData>,
}

impl Default for ParticleTrailsFile {
    fn default() -> Self {
        let mut trails = HashMap::new();

        // Fire trail
        trails.insert(
            "fire_trail".to_string(),
            TrailEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (1.0, 0.8, 0.3, 0.8)),
                    GradientKeyRGBA(0.3, (1.0, 0.5, 0.1, 0.6)),
                    GradientKeyRGBA(0.7, (0.8, 0.2, 0.0, 0.3)),
                    GradientKeyRGBA(1.0, (0.3, 0.1, 0.0, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.04),
                    GradientKeyF32(0.3, 0.06),
                    GradientKeyF32(1.0, 0.0),
                ],
                spawn_rate: 80.0,
                speed: 0.5,
                lifetime: 0.25,
                spawn_radius: 0.02,
                gravity: (0.0, 1.0, 0.0),
            },
        );

        // Frost trail
        trails.insert(
            "frost_trail".to_string(),
            TrailEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (0.9, 0.95, 1.0, 0.8)),
                    GradientKeyRGBA(0.3, (0.6, 0.8, 1.0, 0.6)),
                    GradientKeyRGBA(0.7, (0.3, 0.5, 0.9, 0.3)),
                    GradientKeyRGBA(1.0, (0.1, 0.3, 0.6, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.03),
                    GradientKeyF32(0.3, 0.05),
                    GradientKeyF32(1.0, 0.0),
                ],
                spawn_rate: 100.0,
                speed: 0.3,
                lifetime: 0.2,
                spawn_radius: 0.02,
                gravity: (0.0, -0.5, 0.0),
            },
        );

        // Magic trail
        trails.insert(
            "magic_trail".to_string(),
            TrailEffectData {
                color_gradient: vec![
                    GradientKeyRGBA(0.0, (1.0, 0.7, 1.0, 0.9)),
                    GradientKeyRGBA(0.3, (0.9, 0.4, 1.0, 0.7)),
                    GradientKeyRGBA(0.7, (0.6, 0.2, 0.9, 0.4)),
                    GradientKeyRGBA(1.0, (0.3, 0.1, 0.5, 0.0)),
                ],
                size_gradient: vec![
                    GradientKeyF32(0.0, 0.025),
                    GradientKeyF32(0.2, 0.04),
                    GradientKeyF32(1.0, 0.0),
                ],
                spawn_rate: 60.0,
                speed: 0.4,
                lifetime: 0.18,
                spawn_radius: 0.015,
                gravity: (0.0, 0.0, 0.0),
            },
        );

        Self { trails }
    }
}

/// Resource containing loaded particle explosion configuration
#[derive(Resource)]
pub struct ParticleExplosionsConfig {
    data: ParticleExplosionsFile,
}

impl Default for ParticleExplosionsConfig {
    fn default() -> Self {
        Self {
            data: ParticleExplosionsFile::default(),
        }
    }
}

impl ParticleExplosionsConfig {
    /// Load configuration from RON file, falling back to defaults if missing
    pub fn load() -> Self {
        let config_path = "assets/config/particles_explosions.ron";

        match std::fs::read_to_string(config_path) {
            Ok(contents) => match ron::from_str::<ParticleExplosionsFile>(&contents) {
                Ok(data) => {
                    info!("Loaded particle explosions config from {}", config_path);
                    Self { data }
                }
                Err(e) => {
                    warn!(
                        "Failed to parse {}: {}. Using defaults.",
                        config_path, e
                    );
                    Self::default()
                }
            },
            Err(_) => {
                info!(
                    "No config file at {}. Using defaults.",
                    config_path
                );
                Self::default()
            }
        }
    }

    /// Get explosion effect data by key
    pub fn get(&self, key: &str) -> Option<&ExplosionEffectData> {
        self.data.effects.get(key)
    }

    /// Get all effect keys for warmup
    pub fn effect_keys(&self) -> impl Iterator<Item = &String> {
        self.data.effects.keys()
    }
}

/// Resource containing loaded particle trail configuration
#[derive(Resource)]
pub struct ParticleTrailsConfig {
    data: ParticleTrailsFile,
}

impl Default for ParticleTrailsConfig {
    fn default() -> Self {
        Self {
            data: ParticleTrailsFile::default(),
        }
    }
}

impl ParticleTrailsConfig {
    /// Load configuration from RON file, falling back to defaults if missing
    pub fn load() -> Self {
        let config_path = "assets/config/particles_trails.ron";

        match std::fs::read_to_string(config_path) {
            Ok(contents) => match ron::from_str::<ParticleTrailsFile>(&contents) {
                Ok(data) => {
                    info!("Loaded particle trails config from {}", config_path);
                    Self { data }
                }
                Err(e) => {
                    warn!(
                        "Failed to parse {}: {}. Using defaults.",
                        config_path, e
                    );
                    Self::default()
                }
            },
            Err(_) => {
                info!(
                    "No config file at {}. Using defaults.",
                    config_path
                );
                Self::default()
            }
        }
    }

    /// Get trail effect data by key
    pub fn get(&self, key: &str) -> Option<&TrailEffectData> {
        self.data.trails.get(key)
    }

    /// Get all trail keys for warmup
    pub fn trail_keys(&self) -> impl Iterator<Item = &String> {
        self.data.trails.keys()
    }
}
