use bevy::prelude::*;
use std::collections::HashMap;

use crate::spells::SpellType;

/// Unique identifier for enemy types
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EnemyTypeId(pub u32);

impl EnemyTypeId {
    pub const FIRE_IMP: EnemyTypeId = EnemyTypeId(0);
    pub const FROST_MAGE: EnemyTypeId = EnemyTypeId(1);
}

/// Data-driven definition for an enemy type
#[derive(Clone, Debug)]
pub struct EnemyDefinition {
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

/// Resource containing all enemy definitions
#[derive(Resource)]
pub struct EnemyDefinitions {
    definitions: HashMap<EnemyTypeId, EnemyDefinition>,
}

impl Default for EnemyDefinitions {
    fn default() -> Self {
        let mut definitions = HashMap::new();

        // Fire Imp - aggressive, fast, uses fireballs
        definitions.insert(
            EnemyTypeId::FIRE_IMP,
            EnemyDefinition {
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

        // Frost Mage - tankier, slower, uses frostbolts that slow
        definitions.insert(
            EnemyTypeId::FROST_MAGE,
            EnemyDefinition {
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

        Self { definitions }
    }
}

impl EnemyDefinitions {
    pub fn get(&self, id: EnemyTypeId) -> Option<&EnemyDefinition> {
        self.definitions.get(&id)
    }
}
