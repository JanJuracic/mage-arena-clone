use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Explosion light data loaded from RON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplosionLightData {
    /// RGB color (0.0-1.0)
    pub color: (f32, f32, f32),
    /// Light intensity in lumens
    pub intensity: f32,
    /// Light range in meters
    pub range: f32,
    /// Duration in seconds before fade
    pub duration: f32,
}

impl Default for ExplosionLightData {
    fn default() -> Self {
        Self {
            color: (1.0, 0.5, 0.1),
            intensity: 3500.0,
            range: 8.0,
            duration: 0.2,
        }
    }
}

/// Projectile light data loaded from RON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileLightData {
    /// RGB color (0.0-1.0)
    pub color: (f32, f32, f32),
    /// Light intensity in lumens
    pub intensity: f32,
    /// Light range in meters
    pub range: f32,
}

impl Default for ProjectileLightData {
    fn default() -> Self {
        Self {
            color: (1.0, 0.6, 0.0),
            intensity: 1200.0,
            range: 4.0,
        }
    }
}

/// Root configuration structure for lighting.ron
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingConfigFile {
    pub explosion_lights: HashMap<String, ExplosionLightData>,
    pub projectile_lights: HashMap<String, ProjectileLightData>,
    pub emissive_multipliers: HashMap<String, f32>,
}

impl Default for LightingConfigFile {
    fn default() -> Self {
        let mut explosion_lights = HashMap::new();
        explosion_lights.insert(
            "fireball".to_string(),
            ExplosionLightData {
                color: (1.0, 0.5, 0.1),
                intensity: 3500.0,
                range: 8.0,
                duration: 0.2,
            },
        );
        explosion_lights.insert(
            "frost".to_string(),
            ExplosionLightData {
                color: (0.6, 0.9, 1.0),
                intensity: 3000.0,
                range: 7.0,
                duration: 0.2,
            },
        );
        explosion_lights.insert(
            "enemy_spawn".to_string(),
            ExplosionLightData {
                color: (0.9, 0.5, 1.0),
                intensity: 3500.0,
                range: 6.0,
                duration: 0.25,
            },
        );

        let mut projectile_lights = HashMap::new();
        projectile_lights.insert(
            "Fireball".to_string(),
            ProjectileLightData {
                color: (1.0, 0.6, 0.0),
                intensity: 1200.0,
                range: 4.0,
            },
        );
        projectile_lights.insert(
            "Frostbolt".to_string(),
            ProjectileLightData {
                color: (0.6, 0.9, 1.0),
                intensity: 1200.0,
                range: 4.0,
            },
        );
        projectile_lights.insert(
            "MagicMissile".to_string(),
            ProjectileLightData {
                color: (0.9, 0.4, 1.0),
                intensity: 900.0,
                range: 3.5,
            },
        );
        projectile_lights.insert(
            "EnemyFireball".to_string(),
            ProjectileLightData {
                color: (1.0, 0.6, 0.2),
                intensity: 1200.0,
                range: 4.0,
            },
        );
        projectile_lights.insert(
            "EnemyFrostbolt".to_string(),
            ProjectileLightData {
                color: (0.5, 0.8, 1.0),
                intensity: 1200.0,
                range: 4.0,
            },
        );
        projectile_lights.insert(
            "EnemyBolt".to_string(),
            ProjectileLightData {
                color: (1.0, 1.0, 0.4),
                intensity: 1200.0,
                range: 4.0,
            },
        );

        let mut emissive_multipliers = HashMap::new();
        emissive_multipliers.insert("Fireball".to_string(), 2.0);
        emissive_multipliers.insert("Frostbolt".to_string(), 2.0);
        emissive_multipliers.insert("MagicMissile".to_string(), 3.0);
        emissive_multipliers.insert("EnemyFireball".to_string(), 2.0);
        emissive_multipliers.insert("EnemyFrostbolt".to_string(), 2.0);
        emissive_multipliers.insert("EnemyBolt".to_string(), 2.0);

        Self {
            explosion_lights,
            projectile_lights,
            emissive_multipliers,
        }
    }
}

/// Resource containing loaded lighting configuration
#[derive(Resource)]
pub struct LightingConfig {
    data: LightingConfigFile,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            data: LightingConfigFile::default(),
        }
    }
}

impl LightingConfig {
    /// Load configuration from RON file, falling back to defaults if missing
    pub fn load() -> Self {
        let config_path = "assets/config/lighting.ron";

        match std::fs::read_to_string(config_path) {
            Ok(contents) => match ron::from_str::<LightingConfigFile>(&contents) {
                Ok(data) => {
                    info!("Loaded lighting config from {}", config_path);
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

    /// Get explosion light data by key
    pub fn get_explosion_light(&self, key: &str) -> Option<&ExplosionLightData> {
        self.data.explosion_lights.get(key)
    }

    /// Get explosion light data with fallback to default
    pub fn explosion_light_or_default(&self, key: &str) -> ExplosionLightData {
        self.data
            .explosion_lights
            .get(key)
            .cloned()
            .unwrap_or_default()
    }

    /// Get projectile light data by key
    pub fn get_projectile_light(&self, key: &str) -> Option<&ProjectileLightData> {
        self.data.projectile_lights.get(key)
    }

    /// Get projectile light data with fallback to default
    pub fn projectile_light_or_default(&self, key: &str) -> ProjectileLightData {
        self.data
            .projectile_lights
            .get(key)
            .cloned()
            .unwrap_or_default()
    }

    /// Get emissive multiplier by key
    pub fn get_emissive_multiplier(&self, key: &str) -> f32 {
        self.data.emissive_multipliers.get(key).copied().unwrap_or(2.0)
    }

    /// Convert ExplosionLightData to Bevy Color
    pub fn explosion_color(&self, key: &str) -> Color {
        let data = self.explosion_light_or_default(key);
        Color::srgb(data.color.0, data.color.1, data.color.2)
    }

    /// Convert ProjectileLightData to Bevy Color
    pub fn projectile_color(&self, key: &str) -> Color {
        let data = self.projectile_light_or_default(key);
        Color::srgb(data.color.0, data.color.1, data.color.2)
    }
}
