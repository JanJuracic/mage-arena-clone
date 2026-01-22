use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::spells::SpellType;

/// Steering behavior configuration loaded from RON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringConfigData {
    /// How far ahead to detect obstacles
    pub detection_radius: f32,
    /// Extra buffer distance from obstacle surface
    pub avoidance_margin: f32,
    /// How strongly to avoid obstacles (force multiplier)
    pub avoidance_weight: f32,
    /// Smoothing factor for steering changes (0-1, lower = smoother)
    pub steering_smoothing: f32,
    /// Distance to detect nearby enemies for separation
    pub separation_radius: f32,
    /// Strength of separation force
    pub separation_weight: f32,
}

impl Default for SteeringConfigData {
    fn default() -> Self {
        Self {
            detection_radius: 8.0,
            avoidance_margin: 2.5,
            avoidance_weight: 2.5,
            steering_smoothing: 0.08,
            separation_radius: 3.5,
            separation_weight: 0.8,
        }
    }
}

/// Enemy type definition loaded from RON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyTypeData {
    pub name: String,
    pub max_health: f32,
    pub movement_speed: f32,
    pub detection_range: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub primary_attack: SpellType,
    pub cast_duration: f32,
    pub model_scale: f32,
    pub model_y_offset: f32,
}

/// Root configuration structure for enemies.ron
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyConfigFile {
    pub steering: SteeringConfigData,
    pub enemy_types: HashMap<String, EnemyTypeData>,
}

impl Default for EnemyConfigFile {
    fn default() -> Self {
        let mut enemy_types = HashMap::new();

        enemy_types.insert(
            "fire_imp".to_string(),
            EnemyTypeData {
                name: "Fire Imp".to_string(),
                max_health: 30.0,
                movement_speed: 5.46,
                detection_range: 60.0,
                attack_range: 8.0,
                attack_cooldown: 1.5,
                primary_attack: SpellType::EnemyFireball,
                cast_duration: 0.8,
                model_scale: 0.7,
                model_y_offset: -1.0,
            },
        );

        enemy_types.insert(
            "frost_mage".to_string(),
            EnemyTypeData {
                name: "Frost Mage".to_string(),
                max_health: 50.0,
                movement_speed: 3.64,
                detection_range: 66.0,
                attack_range: 10.0,
                attack_cooldown: 2.5,
                primary_attack: SpellType::EnemyFrostbolt,
                cast_duration: 1.0,
                model_scale: 0.7,
                model_y_offset: -1.0,
            },
        );

        Self {
            steering: SteeringConfigData::default(),
            enemy_types,
        }
    }
}

/// Resource containing loaded enemy configuration
#[derive(Resource)]
pub struct EnemyConfig {
    pub data: EnemyConfigFile,
}

impl Default for EnemyConfig {
    fn default() -> Self {
        Self {
            data: EnemyConfigFile::default(),
        }
    }
}

impl EnemyConfig {
    /// Load configuration from RON file, falling back to defaults if missing
    pub fn load() -> Self {
        let config_path = "assets/config/enemies.ron";

        match std::fs::read_to_string(config_path) {
            Ok(contents) => match ron::from_str::<EnemyConfigFile>(&contents) {
                Ok(data) => {
                    info!("Loaded enemy config from {}", config_path);
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

    /// Get steering configuration data
    pub fn steering(&self) -> &SteeringConfigData {
        &self.data.steering
    }

    /// Get enemy type data by string key
    pub fn get_enemy_type(&self, key: &str) -> Option<&EnemyTypeData> {
        self.data.enemy_types.get(key)
    }
}
