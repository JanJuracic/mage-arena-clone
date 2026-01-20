use bevy::prelude::*;
use avian3d::prelude::*;
use std::collections::HashMap;

use crate::states::GameState;
use crate::player::{Player, AimDirection};
use crate::combat::{DamageEvent, Health, Mana, Team, Hittable, SlowEffect, Dead};
use crate::arena::ArenaConfig;
use crate::particles::{SpawnParticleEvent, ParticleSet, TrailConfig};
use bevy_hanabi::{EffectAsset, ParticleEffect};
use crate::camera::ScreenShakeEvent;

pub struct SpellPlugin;

/// System set for spell systems, used for ordering
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpellSet;

impl Plugin for SpellPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpellDefinitions::default())
            .add_event::<SpellCastEvent>()
            // Configure particle systems to run after spell systems
            .configure_sets(Update, ParticleSet.after(SpellSet))
            .add_systems(
                Update,
                (
                    spell_input,
                    handle_spell_cast,
                    update_missile_launchers,
                    update_homing_projectiles,
                    move_projectiles,
                    projectile_collision,
                    projectile_terrain_collision,
                    update_fireball_explosions,
                    tick_cooldowns,
                    despawn_expired_projectiles,
                )
                    .chain()
                    .in_set(SpellSet)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// Spell types
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SpellType {
    Fireball,
    Frostbolt,
    Wormhole,
    MagicMissile,
    EnemyBolt,
    EnemyFireball,  // Fire imp - orange, small AoE
    EnemyFrostbolt, // Frost mage - blue, applies slow
}

// Spell definitions
#[derive(Resource)]
pub struct SpellDefinitions {
    pub fireball: SpellStats,
    pub frostbolt: SpellStats,
    pub wormhole: SpellStats,
    pub magic_missile: SpellStats,
    pub enemy_bolt: SpellStats,
    pub enemy_fireball: SpellStats,
    pub enemy_frostbolt: SpellStats,
}

impl Default for SpellDefinitions {
    fn default() -> Self {
        Self {
            fireball: SpellStats {
                damage: 25.0,
                mana_cost: 15.0,
                cooldown: 0.5,
                speed: 25.0,
                radius: 0.0,
                range: 0.0,
            },
            frostbolt: SpellStats {
                damage: 15.0,
                mana_cost: 10.0,
                cooldown: 0.3,
                speed: 30.0,
                radius: 0.0,
                range: 0.0,
            },
            wormhole: SpellStats {
                damage: 0.0,
                mana_cost: 30.0,
                cooldown: 5.0,
                speed: 0.0,
                radius: 0.0,
                range: 20.0,
            },
            magic_missile: SpellStats {
                damage: 12.0,  // Per missile (3 missiles = 36 total)
                mana_cost: 20.0,
                cooldown: 1.5,
                speed: 18.0,   // Slower than other projectiles for better homing visuals
                radius: 0.0,
                range: 0.0,
            },
            enemy_bolt: SpellStats {
                damage: 10.0,
                mana_cost: 0.0,  // Enemies don't use mana
                cooldown: 2.0,
                speed: 20.0,
                radius: 0.0,
                range: 0.0,
            },
            enemy_fireball: SpellStats {
                damage: 15.0,
                mana_cost: 0.0,  // Enemies don't use mana
                cooldown: 1.5,
                speed: 22.0,
                radius: 2.0,  // Small AoE
                range: 0.0,
            },
            enemy_frostbolt: SpellStats {
                damage: 8.0,
                mana_cost: 0.0,  // Enemies don't use mana
                cooldown: 2.5,
                speed: 18.0,
                radius: 0.0,
                range: 0.0,
            },
        }
    }
}

pub struct SpellStats {
    pub damage: f32,
    pub mana_cost: f32,
    pub cooldown: f32,
    pub speed: f32,
    pub radius: f32,
    pub range: f32,
}

// Components
#[derive(Component)]
pub struct SpellCooldowns {
    pub cooldowns: HashMap<SpellType, Timer>,
}

impl SpellCooldowns {
    pub fn new() -> Self {
        Self {
            cooldowns: HashMap::new(),
        }
    }

    pub fn is_ready(&self, spell: SpellType) -> bool {
        self.cooldowns
            .get(&spell)
            .map_or(true, |timer| timer.finished())
    }

    pub fn trigger(&mut self, spell: SpellType, duration: f32) {
        self.cooldowns
            .insert(spell, Timer::from_seconds(duration, TimerMode::Once));
    }

    pub fn get_remaining(&self, spell: SpellType) -> f32 {
        self.cooldowns
            .get(&spell)
            .map_or(0.0, |timer| timer.remaining_secs())
    }

    pub fn get_fraction(&self, spell: SpellType, max_cooldown: f32) -> f32 {
        let remaining = self.get_remaining(spell);
        if max_cooldown > 0.0 {
            remaining / max_cooldown
        } else {
            0.0
        }
    }
}

#[derive(Component)]
pub struct Projectile {
    pub spell_type: SpellType,
    pub damage: f32,
    pub owner: Entity,
    pub owner_team: Team,
}

#[derive(Component)]
pub struct ProjectileSpeed(pub f32);

#[derive(Component)]
pub struct ProjectileRadius(pub f32);

#[derive(Component)]
pub struct ProjectileLifetime(pub Timer);

#[derive(Component)]
pub struct HomingProjectile {
    pub turn_rate: f32,  // Radians per second
}

#[derive(Component)]
pub struct MissileLauncher {
    pub missiles_remaining: u32,
    pub fire_timer: Timer,
    pub direction: Vec3,
    pub position: Vec3,
    pub owner: Entity,
    pub team: Team,
    pub damage: f32,
    pub speed: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ExplosionPhase {
    Expanding,
    Fading,
}

#[derive(Component)]
pub struct FireballExplosion {
    pub phase: ExplosionPhase,
    pub expand_timer: Timer,
    pub fade_timer: Timer,
    pub radius: f32,
    pub damage: f32,
    pub owner_team: Team,
    pub has_dealt_damage: bool,
}

// Events
#[derive(Event)]
pub struct SpellCastEvent {
    pub caster: Entity,
    pub spell_type: SpellType,
    pub position: Vec3,
    pub direction: Vec3,
    pub team: Team,
}

// Systems
fn spell_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut spell_events: EventWriter<SpellCastEvent>,
    player_query: Query<(Entity, &Transform, &AimDirection, &Mana, &SpellCooldowns, &Team), With<Player>>,
    spell_defs: Res<SpellDefinitions>,
) {
    let Ok((entity, transform, aim, mana, cooldowns, team)) = player_query.get_single() else {
        return;
    };

    // Helper to try casting a spell
    let mut try_cast = |spell_type: SpellType, stats: &SpellStats| {
        if cooldowns.is_ready(spell_type) && mana.can_afford(stats.mana_cost) {
            spell_events.send(SpellCastEvent {
                caster: entity,
                spell_type,
                position: transform.translation + Vec3::Y * 1.5, // Fire from eye level
                direction: aim.0,
                team: *team,
            });
        }
    };

    // Mouse buttons for quick casting (FPS style)
    if mouse.just_pressed(MouseButton::Left) {
        try_cast(SpellType::Fireball, &spell_defs.fireball);
    }
    if mouse.just_pressed(MouseButton::Right) {
        try_cast(SpellType::Frostbolt, &spell_defs.frostbolt);
    }

    // Number keys for all spells
    if keyboard.just_pressed(KeyCode::Digit1) {
        try_cast(SpellType::Fireball, &spell_defs.fireball);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        try_cast(SpellType::Frostbolt, &spell_defs.frostbolt);
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        try_cast(SpellType::Wormhole, &spell_defs.wormhole);
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        try_cast(SpellType::MagicMissile, &spell_defs.magic_missile);
    }

    // Q and E for utility spells
    if keyboard.just_pressed(KeyCode::KeyQ) {
        try_cast(SpellType::Wormhole, &spell_defs.wormhole);
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        try_cast(SpellType::MagicMissile, &spell_defs.magic_missile);
    }
}

fn handle_spell_cast(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut spell_events: EventReader<SpellCastEvent>,
    mut caster_query: Query<(Option<&mut Mana>, &mut SpellCooldowns)>,
    mut transform_query: Query<&mut Transform>,
    spell_defs: Res<SpellDefinitions>,
    arena_config: Res<ArenaConfig>,
    mut particle_events: EventWriter<SpawnParticleEvent>,
) {
    for event in spell_events.read() {
        let Ok((mana, mut cooldowns)) = caster_query.get_mut(event.caster) else {
            continue;
        };

        let stats = match event.spell_type {
            SpellType::Fireball => &spell_defs.fireball,
            SpellType::Frostbolt => &spell_defs.frostbolt,
            SpellType::Wormhole => &spell_defs.wormhole,
            SpellType::MagicMissile => &spell_defs.magic_missile,
            SpellType::EnemyBolt => &spell_defs.enemy_bolt,
            SpellType::EnemyFireball => &spell_defs.enemy_fireball,
            SpellType::EnemyFrostbolt => &spell_defs.enemy_frostbolt,
        };

        // Only check/spend mana if the caster has the Mana component
        if let Some(mut mana) = mana {
            if !mana.spend(stats.mana_cost) {
                continue;
            }
        }
        cooldowns.trigger(event.spell_type, stats.cooldown);

        // Spawn muzzle flash particle effect for projectile spells
        if matches!(event.spell_type, SpellType::Fireball | SpellType::Frostbolt | SpellType::MagicMissile | SpellType::EnemyBolt | SpellType::EnemyFireball | SpellType::EnemyFrostbolt) {
            particle_events.send(
                SpawnParticleEvent::muzzle_flash(event.position)
                    .with_direction(event.direction)
            );
        }

        match event.spell_type {
            SpellType::Fireball => {
                spawn_projectile(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut effects,
                    event,
                    stats,
                    Color::srgb(1.0, 0.4, 0.0),
                    Color::srgb(1.0, 0.6, 0.0),
                    TrailConfig::fire(),
                );
            }
            SpellType::Frostbolt => {
                spawn_projectile(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut effects,
                    event,
                    stats,
                    Color::srgb(0.4, 0.7, 1.0),
                    Color::srgb(0.6, 0.9, 1.0),
                    TrailConfig::frost(),
                );
            }
            SpellType::Wormhole => {
                // Teleport to aim direction, clamped to arena
                if let Ok(mut caster_transform) = transform_query.get_mut(event.caster) {
                    let target = event.position + event.direction * stats.range;
                    let clamped = clamp_to_arena(target, arena_config.radius - 1.0);
                    caster_transform.translation = Vec3::new(clamped.x, caster_transform.translation.y, clamped.z);
                }
            }
            SpellType::MagicMissile => {
                // Spawn a launcher that fires 3 missiles in quick succession
                commands.spawn((
                    MissileLauncher {
                        missiles_remaining: 3,
                        fire_timer: Timer::from_seconds(0.0, TimerMode::Once), // Fire first immediately
                        direction: event.direction,
                        position: event.position,
                        owner: event.caster,
                        team: event.team,
                        damage: stats.damage,
                        speed: stats.speed,
                    },
                    StateScoped(GameState::Playing),
                ));
            }
            SpellType::EnemyBolt => {
                // Yellow projectile for enemies
                spawn_projectile(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut effects,
                    event,
                    stats,
                    Color::srgb(1.0, 0.9, 0.2),   // Yellow
                    Color::srgb(1.0, 1.0, 0.4),   // Bright yellow emissive
                    TrailConfig::fire(), // Use fire trail for yellow glow
                );
            }
            SpellType::EnemyFireball => {
                // Orange fireball for Fire Imps
                spawn_projectile(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut effects,
                    event,
                    stats,
                    Color::srgb(1.0, 0.5, 0.1),   // Deep orange
                    Color::srgb(1.0, 0.6, 0.2),   // Orange emissive
                    TrailConfig::fire(),
                );
            }
            SpellType::EnemyFrostbolt => {
                // Blue frostbolt for Frost Mages
                spawn_projectile(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut effects,
                    event,
                    stats,
                    Color::srgb(0.3, 0.6, 1.0),   // Blue
                    Color::srgb(0.5, 0.8, 1.0),   // Light blue emissive
                    TrailConfig::frost(),
                );
            }
        }
    }
}

fn spawn_projectile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effects: &mut Assets<EffectAsset>,
    event: &SpellCastEvent,
    stats: &SpellStats,
    base_color: Color,
    emissive_color: Color,
    trail_config: TrailConfig,
) {
    use crate::particles::create_trail;

    // All projectiles spawn from the same point (slightly in front of player at eye level)
    let spawn_pos = event.position + Vec3::Y * 0.2;

    let projectile_radius = 0.06;
    let trail_effect = effects.add(create_trail(&trail_config));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(projectile_radius))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color,
            emissive: LinearRgba::from(emissive_color) * 2.0,
            ..default()
        })),
        Transform::from_translation(spawn_pos).looking_to(event.direction, Vec3::Y),
        Projectile {
            spell_type: event.spell_type,
            damage: stats.damage,
            owner: event.caster,
            owner_team: event.team,
        },
        ProjectileSpeed(stats.speed),
        ProjectileRadius(projectile_radius),
        ProjectileLifetime(Timer::from_seconds(3.0, TimerMode::Once)),
        StateScoped(GameState::Playing),
        Name::new(format!("{:?}", event.spell_type)),
    )).with_children(|parent| {
        // Spawn trail particle effect as child
        parent.spawn((
            ParticleEffect::new(trail_effect),
            Transform::default(),
        ));
    });
}

fn spawn_magic_missile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effects: &mut Assets<EffectAsset>,
    position: Vec3,
    direction: Vec3,
    owner: Entity,
    team: Team,
    damage: f32,
    speed: f32,
) {
    use crate::particles::create_trail;

    // All projectiles spawn from the same point (no directional offset)
    let spawn_pos = position + Vec3::Y * 0.2;

    let projectile_radius = 0.04; // Smaller than regular projectiles
    let trail_effect = effects.add(create_trail(&TrailConfig::magic()));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(projectile_radius))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.3, 1.0),  // Purple
            emissive: LinearRgba::new(0.9, 0.4, 1.0, 1.0) * 3.0,
            ..default()
        })),
        Transform::from_translation(spawn_pos).looking_to(direction, Vec3::Y),
        Projectile {
            spell_type: SpellType::MagicMissile,
            damage,
            owner,
            owner_team: team,
        },
        ProjectileSpeed(speed),
        ProjectileRadius(projectile_radius),
        ProjectileLifetime(Timer::from_seconds(4.0, TimerMode::Once)), // Longer lifetime for homing
        HomingProjectile {
            turn_rate: 8.0, // Radians per second - very aggressive homing (2x strength)
        },
        StateScoped(GameState::Playing),
        Name::new("MagicMissile"),
    )).with_children(|parent| {
        // Spawn trail particle effect as child
        parent.spawn((
            ParticleEffect::new(trail_effect),
            Transform::default(),
        ));
    });
}

fn update_missile_launchers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effects: ResMut<Assets<EffectAsset>>,
    time: Res<Time>,
    mut launchers: Query<(Entity, &mut MissileLauncher)>,
) {
    for (entity, mut launcher) in launchers.iter_mut() {
        launcher.fire_timer.tick(time.delta());

        if launcher.fire_timer.finished() && launcher.missiles_remaining > 0 {
            spawn_magic_missile(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut effects,
                launcher.position,
                launcher.direction,
                launcher.owner,
                launcher.team,
                launcher.damage,
                launcher.speed,
            );

            launcher.missiles_remaining -= 1;

            // Reset timer for next missile (0.1 second delay between missiles)
            launcher.fire_timer = Timer::from_seconds(0.1, TimerMode::Once);
        }

        // Despawn launcher when done
        if launcher.missiles_remaining == 0 {
            commands.entity(entity).despawn();
        }
    }
}

fn update_homing_projectiles(
    time: Res<Time>,
    mut projectiles: Query<(&mut Transform, &Projectile, &HomingProjectile)>,
    targets: Query<(Entity, &GlobalTransform, &Team), (With<Hittable>, Without<Dead>)>,
) {
    for (mut proj_transform, projectile, homing) in projectiles.iter_mut() {
        let proj_pos = proj_transform.translation;

        // Find closest enemy (different team, not the owner)
        let mut closest_target: Option<Vec3> = None;
        let mut closest_distance = f32::MAX;

        for (target_entity, target_transform, target_team) in targets.iter() {
            // Skip same team and owner
            if target_team.0 == projectile.owner_team.0 || target_entity == projectile.owner {
                continue;
            }

            let target_pos = target_transform.translation();
            let distance = (target_pos - proj_pos).length();

            if distance < closest_distance {
                closest_distance = distance;
                closest_target = Some(target_pos);
            }
        }

        // If we found a target, steer toward it using pursuit steering
        if let Some(target_pos) = closest_target {
            let desired_direction = (target_pos - proj_pos).normalize_or_zero();

            if desired_direction.length_squared() > 0.0 {
                // Calculate the maximum rotation this frame
                let max_rotation = homing.turn_rate * time.delta_secs();

                // Smoothly interpolate toward the target direction using slerp on quaternions
                let current_rotation = proj_transform.rotation;
                // Use NEG_Z because Bevy's forward() is -Z axis
                let target_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, desired_direction);

                // Slerp with a factor based on turn rate
                let slerp_factor = (max_rotation / std::f32::consts::PI).min(1.0);
                proj_transform.rotation = current_rotation.slerp(target_rotation, slerp_factor);
            }
        }
    }
}

fn move_projectiles(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &ProjectileSpeed), With<Projectile>>,
) {
    for (mut transform, speed) in query.iter_mut() {
        let forward = transform.forward();
        transform.translation += *forward * speed.0 * time.delta_secs();
    }
}

fn projectile_collision(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut damage_events: EventWriter<DamageEvent>,
    mut particle_events: EventWriter<SpawnParticleEvent>,
    mut shake_events: EventWriter<ScreenShakeEvent>,
    projectiles: Query<(Entity, &Transform, &Projectile, &ProjectileRadius)>,
    mut hittables: Query<(Entity, &Transform, &Team, Option<&mut Health>), With<Hittable>>,
    player_query: Query<&Transform, With<Player>>,
) {
    const TARGET_RADIUS: f32 = 0.6; // Approximate radius for enemies/player
    const FIREBALL_EXPLOSION_RADIUS: f32 = 3.0;
    const MAX_SHAKE_DISTANCE: f32 = 20.0;

    let player_pos = player_query.get_single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    for (proj_entity, proj_transform, projectile, proj_radius) in projectiles.iter() {
        for (target_entity, target_transform, target_team, _) in hittables.iter_mut() {
            // Don't hit self or same team
            if target_entity == projectile.owner || target_team.0 == projectile.owner_team.0 {
                continue;
            }

            let distance = (target_transform.translation - proj_transform.translation).length();
            let hit_radius = proj_radius.0 + TARGET_RADIUS; // Combined radius

            if distance < hit_radius {
                let hit_pos = proj_transform.translation;

                // Calculate screen shake based on distance to player and damage
                let distance_to_player = (hit_pos - player_pos).length();
                let proximity_factor = (1.0 - (distance_to_player / MAX_SHAKE_DISTANCE)).max(0.0);
                let damage_factor = projectile.damage / 25.0; // Normalize around fireball damage
                let shake_intensity = proximity_factor * damage_factor * 0.15;

                if shake_intensity > 0.01 {
                    shake_events.send(ScreenShakeEvent::new(shake_intensity, 0.3));
                }

                // Spawn impact particle effect based on spell type
                match projectile.spell_type {
                    SpellType::Fireball | SpellType::EnemyFireball => {
                        particle_events.send(SpawnParticleEvent::fire_explosion(hit_pos));
                        particle_events.send(SpawnParticleEvent::fire_smoke(hit_pos));
                    }
                    SpellType::Frostbolt | SpellType::EnemyFrostbolt => {
                        particle_events.send(SpawnParticleEvent::frost_impact(hit_pos));
                    }
                    _ => {
                        particle_events.send(SpawnParticleEvent::hit_spark(hit_pos));
                    }
                }

                // Handle spell-specific effects
                match projectile.spell_type {
                    SpellType::Fireball => {
                        // Spawn explosion sphere for AoE damage
                        spawn_fireball_explosion(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            hit_pos,
                            FIREBALL_EXPLOSION_RADIUS,
                            projectile.damage,
                            projectile.owner_team,
                        );
                    }
                    SpellType::EnemyFireball => {
                        // Smaller AoE explosion for enemy fireballs
                        const ENEMY_FIREBALL_RADIUS: f32 = 2.0;
                        spawn_fireball_explosion(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            hit_pos,
                            ENEMY_FIREBALL_RADIUS,
                            projectile.damage,
                            projectile.owner_team,
                        );
                    }
                    SpellType::Frostbolt => {
                        // Direct damage + slow
                        damage_events.send(DamageEvent {
                            target: target_entity,
                            damage: projectile.damage,
                            source: projectile.owner,
                        });
                        commands.entity(target_entity).insert(SlowEffect::new(0.5, 2.0));
                    }
                    SpellType::EnemyFrostbolt => {
                        // Direct damage + slow (same as player frostbolt)
                        damage_events.send(DamageEvent {
                            target: target_entity,
                            damage: projectile.damage,
                            source: projectile.owner,
                        });
                        commands.entity(target_entity).insert(SlowEffect::new(0.5, 2.0));
                    }
                    _ => {
                        // Direct damage for other spells
                        damage_events.send(DamageEvent {
                            target: target_entity,
                            damage: projectile.damage,
                            source: projectile.owner,
                        });
                    }
                }

                // Despawn projectile
                commands.entity(proj_entity).despawn_recursive();
                break;
            }
        }
    }
}

fn projectile_terrain_collision(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut particle_events: EventWriter<SpawnParticleEvent>,
    mut shake_events: EventWriter<ScreenShakeEvent>,
    spatial_query: SpatialQuery,
    projectiles: Query<(Entity, &Transform, &Projectile, &ProjectileSpeed, &ProjectileRadius)>,
    hittables: Query<Entity, With<Hittable>>,
    player_query: Query<&Transform, With<Player>>,
    time: Res<Time>,
) {
    const FIREBALL_EXPLOSION_RADIUS: f32 = 3.0;
    const MAX_SHAKE_DISTANCE: f32 = 20.0;

    let player_pos = player_query.get_single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    // Create sets for quick lookup
    let hittable_entities: std::collections::HashSet<Entity> = hittables.iter().collect();

    for (proj_entity, proj_transform, projectile, speed, proj_radius) in projectiles.iter() {
        let forward = proj_transform.forward();
        let ray_origin = proj_transform.translation;
        let ray_direction = Dir3::new(forward.as_vec3()).unwrap_or(Dir3::Z);

        // Cast a ray ahead of the projectile to detect collisions
        // Distance includes projectile radius so we detect when the surface touches terrain
        let ray_distance = speed.0 * time.delta_secs() + proj_radius.0 + 0.1;

        if let Some(hit) = spatial_query.cast_ray(
            ray_origin,
            ray_direction,
            ray_distance,
            true,
            &SpatialQueryFilter::default().with_excluded_entities([proj_entity]),
        ) {
            // Skip if we hit a hittable entity (player/enemy) - handled by projectile_collision
            if hittable_entities.contains(&hit.entity) {
                continue;
            }

            // We hit terrain/obstacle - calculate hit position at the surface
            // Account for projectile radius to get the impact point
            let impact_distance = (hit.distance - proj_radius.0).max(0.0);
            let hit_position = ray_origin + forward.as_vec3() * impact_distance;

            // Calculate screen shake based on distance to player and damage
            let distance_to_player = (hit_position - player_pos).length();
            let proximity_factor = (1.0 - (distance_to_player / MAX_SHAKE_DISTANCE)).max(0.0);
            let damage_factor = projectile.damage / 25.0;
            let shake_intensity = proximity_factor * damage_factor * 0.15;

            if shake_intensity > 0.01 {
                shake_events.send(ScreenShakeEvent::new(shake_intensity, 0.3));
            }

            // Spawn impact particle effect based on spell type
            match projectile.spell_type {
                SpellType::Fireball | SpellType::EnemyFireball => {
                    particle_events.send(SpawnParticleEvent::fire_explosion(hit_position));
                    particle_events.send(SpawnParticleEvent::fire_smoke(hit_position));
                }
                SpellType::Frostbolt | SpellType::EnemyFrostbolt => {
                    particle_events.send(SpawnParticleEvent::frost_impact(hit_position));
                }
                _ => {
                    particle_events.send(SpawnParticleEvent::hit_spark(hit_position));
                }
            }

            // Spawn fireball explosion for AoE damage
            match projectile.spell_type {
                SpellType::Fireball => {
                    spawn_fireball_explosion(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        hit_position,
                        FIREBALL_EXPLOSION_RADIUS,
                        projectile.damage,
                        projectile.owner_team,
                    );
                }
                SpellType::EnemyFireball => {
                    const ENEMY_FIREBALL_RADIUS: f32 = 2.0;
                    spawn_fireball_explosion(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        hit_position,
                        ENEMY_FIREBALL_RADIUS,
                        projectile.damage,
                        projectile.owner_team,
                    );
                }
                _ => {}
            }

            // Despawn projectile
            commands.entity(proj_entity).despawn_recursive();
        }
    }
}

fn tick_cooldowns(time: Res<Time>, mut query: Query<&mut SpellCooldowns>) {
    for mut cooldowns in query.iter_mut() {
        for timer in cooldowns.cooldowns.values_mut() {
            timer.tick(time.delta());
        }
    }
}

fn despawn_expired_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ProjectileLifetime)>,
) {
    for (entity, mut lifetime) in query.iter_mut() {
        lifetime.0.tick(time.delta());
        if lifetime.0.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_fireball_explosion(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    radius: f32,
    damage: f32,
    owner_team: Team,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))), // Unit sphere, scaled by transform
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.4, 0.0, 0.4),
            emissive: LinearRgba::new(1.0, 0.3, 0.0, 1.0) * 3.0,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(position).with_scale(Vec3::ZERO), // Start at scale 0
        FireballExplosion {
            phase: ExplosionPhase::Expanding,
            expand_timer: Timer::from_seconds(0.15, TimerMode::Once), // Quick expand
            fade_timer: Timer::from_seconds(0.3, TimerMode::Once),    // Quick fade
            radius,
            damage,
            owner_team,
            has_dealt_damage: false,
        },
        StateScoped(GameState::Playing),
        Name::new("FireballExplosion"),
    ));
}

fn update_fireball_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut explosions: Query<(Entity, &mut Transform, &mut FireballExplosion, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut damage_events: EventWriter<DamageEvent>,
    hittables: Query<(Entity, &GlobalTransform, &Team), With<Hittable>>,
) {
    for (entity, mut transform, mut explosion, material_handle) in explosions.iter_mut() {
        match explosion.phase {
            ExplosionPhase::Expanding => {
                explosion.expand_timer.tick(time.delta());

                // Expand from 0 to full radius
                let progress = explosion.expand_timer.fraction();
                let current_scale = explosion.radius * progress;
                transform.scale = Vec3::splat(current_scale);

                // Deal damage when fully expanded
                if explosion.expand_timer.finished() {
                    explosion.phase = ExplosionPhase::Fading;

                    // Deal AoE damage once at full size
                    if !explosion.has_dealt_damage {
                        explosion.has_dealt_damage = true;

                        for (target_entity, target_transform, target_team) in hittables.iter() {
                            // Don't damage same team
                            if target_team.0 == explosion.owner_team.0 {
                                continue;
                            }

                            let distance = (target_transform.translation() - transform.translation).length();
                            if distance <= explosion.radius {
                                damage_events.send(DamageEvent {
                                    target: target_entity,
                                    damage: explosion.damage,
                                    source: entity,
                                });
                            }
                        }
                    }
                }
            }
            ExplosionPhase::Fading => {
                explosion.fade_timer.tick(time.delta());

                // Fade out the material
                let fade_progress = explosion.fade_timer.fraction();
                if let Some(material) = materials.get_mut(material_handle) {
                    let alpha = 0.4 * (1.0 - fade_progress);
                    material.base_color = Color::srgba(1.0, 0.4, 0.0, alpha);
                    material.emissive = LinearRgba::new(1.0, 0.3, 0.0, 1.0) * (3.0 * (1.0 - fade_progress));
                }

                // Despawn when fade finished
                if explosion.fade_timer.finished() {
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }
}

fn clamp_to_arena(pos: Vec3, radius: f32) -> Vec3 {
    let horizontal = Vec2::new(pos.x, pos.z);
    let clamped = if horizontal.length() > radius {
        horizontal.normalize() * radius
    } else {
        horizontal
    };
    Vec3::new(clamped.x, pos.y, clamped.y)
}
