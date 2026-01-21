use bevy::prelude::*;
use avian3d::prelude::*;

/// Configuration for ground sensing ray-cast
#[derive(Component, Clone)]
pub struct GroundSensorConfig {
    /// Maximum distance to cast ray downward
    pub ray_distance: f32,
    /// Distance threshold to consider grounded
    pub grounded_threshold: f32,
    /// Y offset from entity origin for ray start (usually half capsule height)
    pub ray_origin_offset: f32,
}

impl GroundSensorConfig {
    /// Configuration preset for player (capsule radius 0.4, half-length 0.4)
    pub fn player() -> Self {
        Self {
            ray_distance: 1.0,
            grounded_threshold: 0.9,
            ray_origin_offset: 0.0,
        }
    }

    /// Configuration preset for enemy (capsule radius 0.5, half-length 0.5)
    pub fn enemy() -> Self {
        Self {
            ray_distance: 1.8,
            grounded_threshold: 1.7,
            ray_origin_offset: 0.0,
        }
    }
}

impl Default for GroundSensorConfig {
    fn default() -> Self {
        Self::player()
    }
}

/// Current ground state computed by ground_sensing_system
#[derive(Component, Clone, Default)]
pub struct GroundState {
    /// Whether the entity is currently grounded
    pub grounded: bool,
    /// Normal vector of the ground surface (if grounded)
    pub ground_normal: Option<Vec3>,
    /// Distance to ground (if detected)
    pub ground_distance: Option<f32>,
}

impl GroundState {
    pub fn is_grounded(&self) -> bool {
        self.grounded
    }
}

/// Bundle for adding ground sensing to an entity
#[derive(Bundle, Clone, Default)]
pub struct GroundSensorBundle {
    pub config: GroundSensorConfig,
    pub state: GroundState,
}

impl GroundSensorBundle {
    pub fn player() -> Self {
        Self {
            config: GroundSensorConfig::player(),
            state: GroundState::default(),
        }
    }

    pub fn enemy() -> Self {
        Self {
            config: GroundSensorConfig::enemy(),
            state: GroundState::default(),
        }
    }
}

/// System that updates GroundState for all entities with GroundSensorConfig
pub fn ground_sensing_system(
    spatial_query: SpatialQuery,
    mut query: Query<(Entity, &Transform, &GroundSensorConfig, &mut GroundState)>,
) {
    for (entity, transform, config, mut ground_state) in query.iter_mut() {
        let ray_origin = transform.translation + Vec3::Y * config.ray_origin_offset;
        let ray_direction = Dir3::NEG_Y;

        if let Some(hit) = spatial_query.cast_ray(
            ray_origin,
            ray_direction,
            config.ray_distance,
            true,
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
        ) {
            ground_state.grounded = hit.distance < config.grounded_threshold;
            ground_state.ground_normal = Some(hit.normal);
            ground_state.ground_distance = Some(hit.distance);
        } else {
            ground_state.grounded = false;
            ground_state.ground_normal = None;
            ground_state.ground_distance = None;
        }
    }
}
