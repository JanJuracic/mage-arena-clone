use bevy::prelude::*;

use crate::states::GameState;
use crate::player::Player;
use crate::enemies::PlayingDamageAnimation;
use crate::particles::SpawnParticleEvent;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DamageEvent>()
            .add_event::<DeathEvent>()
            .add_systems(
                Update,
                (
                    process_damage,
                    handle_deaths,
                    tick_slow_effects,
                    apply_hit_shake,
                    apply_damage_flash,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// Components
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct Team(pub u8);

impl Team {
    pub const PLAYER: Team = Team(0);
    pub const ENEMY: Team = Team(1);
}

#[derive(Component)]
pub struct Hittable;

/// Marker for entities that are dead but not yet despawned.
/// Used to prevent targeting systems from selecting dying entities.
#[derive(Component)]
pub struct Dead;

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn take_damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    pub fn fraction(&self) -> f32 {
        self.current / self.max
    }
}

#[derive(Component)]
pub struct Mana {
    pub current: f32,
    pub max: f32,
    pub regen_rate: f32,
}

impl Mana {
    pub fn new(max: f32, regen_rate: f32) -> Self {
        Self {
            current: max,
            max,
            regen_rate,
        }
    }

    pub fn can_afford(&self, cost: f32) -> bool {
        self.current >= cost
    }

    pub fn spend(&mut self, cost: f32) -> bool {
        if self.can_afford(cost) {
            self.current -= cost;
            true
        } else {
            false
        }
    }

    pub fn regenerate(&mut self, delta: f32) {
        self.current = (self.current + self.regen_rate * delta).min(self.max);
    }

    pub fn fraction(&self) -> f32 {
        self.current / self.max
    }
}

#[derive(Component)]
pub struct SlowEffect {
    pub factor: f32,
    pub timer: Timer,
}

impl SlowEffect {
    pub fn new(factor: f32, duration: f32) -> Self {
        Self {
            factor,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

/// Component for hit shake visual feedback when an entity takes damage
#[derive(Component)]
pub struct HitShake {
    pub intensity: f32,
    pub timer: Timer,
    pub base_position: Vec3,
}

impl HitShake {
    pub fn new(intensity: f32, duration: f32, base_position: Vec3) -> Self {
        Self {
            intensity,
            timer: Timer::from_seconds(duration, TimerMode::Once),
            base_position,
        }
    }
}

/// Component for damage flash visual feedback (white flash when hit)
#[derive(Component)]
pub struct DamageFlash {
    pub original_color: Color,
    pub timer: Timer,
}

impl DamageFlash {
    pub fn new(original_color: Color, duration: f32) -> Self {
        Self {
            original_color,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

// Events
#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub damage: f32,
    pub source: Entity,
}

#[derive(Event)]
pub struct DeathEvent {
    pub entity: Entity,
    pub was_player: bool,
}

// Systems
fn process_damage(
    mut commands: Commands,
    mut damage_events: EventReader<DamageEvent>,
    mut health_query: Query<(
        &mut Health,
        &Transform,
        Option<&HitShake>,
        Option<&DamageFlash>,
        Option<&MeshMaterial3d<StandardMaterial>>,
    )>,
    mut death_events: EventWriter<DeathEvent>,
    player_query: Query<Entity, With<Player>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in damage_events.read() {
        if let Ok((mut health, transform, existing_shake, existing_flash, material_handle)) =
            health_query.get_mut(event.target)
        {
            health.take_damage(event.damage);

            // Add hit shake effect (only for non-players)
            let is_player = player_query.get(event.target).is_ok();
            if !is_player {
                // Get the base position (either from existing shake or current transform)
                let base_pos = existing_shake
                    .map(|s| s.base_position)
                    .unwrap_or(transform.translation);

                // Scale intensity based on damage (normalized around 15 damage)
                let intensity = (event.damage / 15.0).clamp(0.1, 1.0) * 0.15;

                commands.entity(event.target).insert(HitShake::new(
                    intensity,
                    0.2, // Duration in seconds
                    base_pos,
                ));

                // Add damage animation
                commands.entity(event.target).insert(PlayingDamageAnimation {
                    timer: Timer::from_seconds(0.5, TimerMode::Once),
                });

                // Add damage flash effect
                if let Some(mat_handle) = material_handle {
                    if let Some(material) = materials.get_mut(mat_handle) {
                        // Get original color (from existing flash or current material)
                        let original_color = existing_flash
                            .map(|f| f.original_color)
                            .unwrap_or(material.base_color);

                        // Set to white immediately
                        material.base_color = Color::WHITE;
                        material.emissive = LinearRgba::WHITE * 2.0;

                        commands.entity(event.target).insert(DamageFlash::new(
                            original_color,
                            0.15, // Flash duration in seconds
                        ));
                    }
                }
            }

            if health.is_dead() {
                // Mark as dead immediately so targeting systems skip this entity
                commands.entity(event.target).insert(Dead);

                death_events.send(DeathEvent {
                    entity: event.target,
                    was_player: is_player,
                });
            }
        }
    }
}

fn handle_deaths(
    mut commands: Commands,
    mut death_events: EventReader<DeathEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    mut particle_events: EventWriter<SpawnParticleEvent>,
    transform_query: Query<&Transform>,
) {
    for event in death_events.read() {
        if event.was_player {
            next_state.set(GameState::GameOver);
        } else {
            // Spawn death particle effect at enemy position
            if let Ok(transform) = transform_query.get(event.entity) {
                particle_events.send(SpawnParticleEvent::enemy_death(transform.translation));
                particle_events.send(SpawnParticleEvent::fire_smoke(transform.translation));
            }
            commands.entity(event.entity).despawn_recursive();
        }
    }
}

fn tick_slow_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SlowEffect)>,
) {
    for (entity, mut slow) in query.iter_mut() {
        slow.timer.tick(time.delta());
        if slow.timer.finished() {
            commands.entity(entity).remove::<SlowEffect>();
        }
    }
}

fn apply_hit_shake(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut HitShake)>,
) {
    for (entity, mut transform, mut shake) in query.iter_mut() {
        shake.timer.tick(time.delta());

        if shake.timer.finished() {
            // Reset to base position and remove shake component
            transform.translation = shake.base_position;
            commands.entity(entity).remove::<HitShake>();
        } else {
            // Calculate decaying shake intensity
            let decay = shake.timer.fraction_remaining();
            let current_intensity = shake.intensity * decay;

            // Generate random offset using time-based noise
            let time_seed = time.elapsed_secs() * 60.0;
            let offset_x = (time_seed.sin() * 2.1 + (time_seed * 3.7).cos()) * current_intensity;
            let offset_z = ((time_seed * 1.9).cos() * 1.8 + (time_seed * 2.3).sin()) * current_intensity;

            // Apply shake offset (keep Y stable for grounded enemies)
            transform.translation = shake.base_position + Vec3::new(offset_x, 0.0, offset_z);
        }
    }
}

fn apply_damage_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DamageFlash, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut flash, material_handle) in query.iter_mut() {
        flash.timer.tick(time.delta());

        if let Some(material) = materials.get_mut(material_handle) {
            if flash.timer.finished() {
                // Reset to original color and remove flash component
                material.base_color = flash.original_color;
                material.emissive = LinearRgba::BLACK;
                commands.entity(entity).remove::<DamageFlash>();
            } else {
                // Interpolate from white back to original color
                let progress = flash.timer.fraction();
                let white = Color::WHITE;

                // Lerp between white and original color
                let r = lerp_color_component(white, flash.original_color, progress, 0);
                let g = lerp_color_component(white, flash.original_color, progress, 1);
                let b = lerp_color_component(white, flash.original_color, progress, 2);
                let a = lerp_color_component(white, flash.original_color, progress, 3);

                material.base_color = Color::srgba(r, g, b, a);

                // Fade out emissive
                let emissive_strength = 2.0 * (1.0 - progress);
                material.emissive = LinearRgba::WHITE * emissive_strength;
            }
        }
    }
}

/// Helper function to lerp a single color component
fn lerp_color_component(from: Color, to: Color, t: f32, index: usize) -> f32 {
    let from_srgba = from.to_srgba();
    let to_srgba = to.to_srgba();

    let from_val = match index {
        0 => from_srgba.red,
        1 => from_srgba.green,
        2 => from_srgba.blue,
        _ => from_srgba.alpha,
    };

    let to_val = match index {
        0 => to_srgba.red,
        1 => to_srgba.green,
        2 => to_srgba.blue,
        _ => to_srgba.alpha,
    };

    from_val + (to_val - from_val) * t
}
