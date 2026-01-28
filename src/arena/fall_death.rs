use bevy::prelude::*;
use avian3d::prelude::*;

use crate::arena::config::FallDeathConfig;
use crate::combat::Health;
use crate::enemies::Enemy;
use crate::physics::TerrainSampler;
use crate::player::Player;
use crate::states::GameState;

/// Component marking an entity as susceptible to fall death
#[derive(Component)]
pub struct FallDeath;

/// Event fired when an entity dies from falling
#[derive(Event)]
pub struct FallDeathEvent {
    pub entity: Entity,
    pub is_player: bool,
}

/// Component for tracking respawn invulnerability
#[derive(Component)]
pub struct RespawnInvulnerability {
    pub timer: Timer,
}

impl RespawnInvulnerability {
    pub fn new(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

/// Runtime fall death configuration resource (water-based)
#[derive(Resource, Clone)]
pub struct FallDeathSettings {
    pub water_level: f32,
    pub submersion_depth: f32,
    pub respawn_height: f32,
    pub respawn_invulnerability: f32,
}

impl Default for FallDeathSettings {
    fn default() -> Self {
        Self {
            water_level: 0.0,
            submersion_depth: 0.5,
            respawn_height: 2.0,
            respawn_invulnerability: 1.5,
        }
    }
}

impl FallDeathSettings {
    pub fn from_config(config: &FallDeathConfig, water_level: f32) -> Self {
        Self {
            water_level,
            submersion_depth: config.submersion_depth,
            respawn_height: config.respawn_height,
            respawn_invulnerability: config.respawn_invulnerability,
        }
    }

    /// Get the death threshold Y position
    pub fn death_threshold(&self) -> f32 {
        self.water_level - self.submersion_depth
    }
}

/// System to detect entities that have fallen below the water death threshold
pub fn detect_fall_death(
    mut commands: Commands,
    settings: Res<FallDeathSettings>,
    query: Query<(Entity, &Transform, Option<&Player>, Option<&Enemy>), With<FallDeath>>,
    mut fall_death_events: EventWriter<FallDeathEvent>,
) {
    let death_threshold = settings.death_threshold();

    for (entity, transform, player, _enemy) in query.iter() {
        if transform.translation.y < death_threshold {
            let is_player = player.is_some();

            fall_death_events.send(FallDeathEvent { entity, is_player });

            // Remove FallDeath component temporarily to prevent multiple death events
            commands.entity(entity).remove::<FallDeath>();
        }
    }
}

/// System to handle fall death for players - respawn at center
pub fn handle_player_fall_death(
    mut commands: Commands,
    settings: Res<FallDeathSettings>,
    terrain_sampler: Res<TerrainSampler>,
    mut fall_death_events: EventReader<FallDeathEvent>,
    mut player_query: Query<(&mut Transform, &mut LinearVelocity, &mut Health), With<Player>>,
) {
    for event in fall_death_events.read() {
        if !event.is_player {
            continue;
        }

        if let Ok((mut transform, mut velocity, mut health)) = player_query.get_mut(event.entity) {
            // Take fall damage (50% of max health)
            let fall_damage = health.max * 0.5;
            health.take_damage(fall_damage);

            // Respawn at arena center - use terrain height + offset
            let terrain_height = terrain_sampler.sample_height(0.0, 0.0);
            let respawn_y = terrain_height + settings.respawn_height;
            transform.translation = Vec3::new(0.0, respawn_y, 0.0);

            // Reset velocity
            velocity.0 = Vec3::ZERO;

            // Add respawn invulnerability and re-add FallDeath component
            commands.entity(event.entity).insert((
                RespawnInvulnerability::new(settings.respawn_invulnerability),
                FallDeath,
            ));

            info!("Player fell into water - respawned at center with {} HP remaining", health.current);
        }
    }
}

/// System to handle fall death for enemies - instant death
pub fn handle_enemy_fall_death(
    mut fall_death_events: EventReader<FallDeathEvent>,
    mut enemy_query: Query<&mut Health, With<Enemy>>,
) {
    for event in fall_death_events.read() {
        if event.is_player {
            continue;
        }

        if let Ok(mut health) = enemy_query.get_mut(event.entity) {
            // Instant death for enemies - use a large fixed value
            let max_hp = health.max;
            health.take_damage(max_hp * 2.0);

            info!("Enemy fell into water and died");
        }
    }
}

/// System to tick respawn invulnerability timers
pub fn tick_respawn_invulnerability(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut RespawnInvulnerability)>,
) {
    for (entity, mut invuln) in query.iter_mut() {
        invuln.timer.tick(time.delta());

        if invuln.timer.finished() {
            commands.entity(entity).remove::<RespawnInvulnerability>();
        }
    }
}

/// Plugin to add fall death systems
pub struct FallDeathPlugin;

impl Plugin for FallDeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FallDeathEvent>()
            .init_resource::<FallDeathSettings>()
            .add_systems(
                Update,
                (
                    detect_fall_death,
                    handle_player_fall_death,
                    handle_enemy_fall_death,
                    tick_respawn_invulnerability,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
