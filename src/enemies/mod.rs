pub mod definitions;

use std::time::Duration;

use bevy::prelude::*;
use bevy::animation::prelude::AnimationTransitions;
use avian3d::prelude::*;
use rand::Rng;

use crate::states::GameState;
use crate::player::Player;
use crate::combat::{Health, Team, Hittable, SlowEffect};
use crate::spells::{SpellCastEvent, SpellType, SpellCooldowns};
use crate::arena::{ArenaConfig, Obstacle, ObstacleBounds};
use crate::particles::SpawnParticleEvent;
use crate::physics::TerrainSampler;

pub use definitions::{EnemyDefinitions, EnemyTypeId};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnemyDefinitions::default())
        .insert_resource(WaveState {
            wave_number: 1,
            enemies_remaining: 0,
            spawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
        })
        .add_systems(Startup, load_enemy_assets)
        .add_systems(OnEnter(GameState::Playing), reset_wave_state)
        .add_systems(
            Update,
            (
                wave_spawner,
                setup_enemy_animation_player,
                ai_detection,
                ai_movement,
                ai_attack,
                update_casting_spells,
                update_enemy_animations,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// Components
#[derive(Component)]
pub struct Enemy;

#[derive(Component, Default)]
pub enum AIState {
    #[default]
    Idle,
    Chasing(Entity),
    Attacking(Entity),
}

#[derive(Component)]
pub struct MovementSpeed(pub f32);

#[derive(Component)]
pub struct DetectionRange(pub f32);

#[derive(Component)]
pub struct AttackRange(pub f32);

#[derive(Component)]
pub struct AttackCooldown(pub Timer);

/// The spell type this enemy uses for their primary attack
#[derive(Component)]
pub struct PrimaryAttack(pub SpellType);

/// How long the enemy takes to cast their spell
#[derive(Component)]
pub struct CastDuration(pub f32);

#[derive(Component, Clone, Copy)]
pub enum EnemyType {
    FireImp,
    FrostMage,
}

/// Component to track which animation is currently playing
#[derive(Component)]
pub struct EnemyAnimator {
    pub current_state: EnemyAnimationState,
    pub animation_player_entity: Option<Entity>,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum EnemyAnimationState {
    #[default]
    Idle,
    Walking,
    Running,
    TakingDamage,
    CastingSpell,
}

/// Component for playing the take damage animation temporarily
#[derive(Component)]
pub struct PlayingDamageAnimation {
    pub timer: Timer,
}

/// Component for tracking spell casting animation
#[derive(Component)]
pub struct CastingSpellAnimation {
    pub timer: Timer,
    pub direction: Vec3,
    pub has_fired: bool,
}

/// Steering behavior configuration for obstacle avoidance
#[derive(Component)]
pub struct SteeringConfig {
    /// How far ahead to detect obstacles
    pub detection_radius: f32,
    /// Extra buffer distance from obstacle surface
    pub avoidance_margin: f32,
    /// How strongly to avoid obstacles (force multiplier)
    pub avoidance_weight: f32,
    /// Smoothing factor for steering changes (0-1, lower = smoother)
    pub steering_smoothing: f32,
}

impl Default for SteeringConfig {
    fn default() -> Self {
        Self {
            detection_radius: 8.0,
            avoidance_margin: 2.5,
            avoidance_weight: 2.5,
            steering_smoothing: 0.08,
        }
    }
}

/// Runtime state for steering behavior
#[derive(Component)]
pub struct SteeringState {
    /// Current smoothed steering velocity
    pub current_velocity: Vec3,
}

impl Default for SteeringState {
    fn default() -> Self {
        Self {
            current_velocity: Vec3::ZERO,
        }
    }
}

/// Distance threshold for switching between run and walk
const RUN_DISTANCE_THRESHOLD: f32 = 12.0;
/// Speed multiplier when running
const RUN_SPEED_MULTIPLIER: f32 = 1.5;

// Resources
#[derive(Resource)]
pub struct WaveState {
    pub wave_number: u32,
    pub enemies_remaining: u32,
    pub spawn_timer: Timer,
}

/// Resource to hold loaded enemy model and animation assets
#[derive(Resource)]
pub struct EnemyAssets {
    pub scene: Handle<Scene>,
    pub graph: Option<Handle<AnimationGraph>>,
    pub animations: Option<EnemyAnimationIndices>,
}

/// Animation node indices for each enemy type
#[derive(Component, Clone)]
pub struct EnemyAnimationIndices {
    pub idle: AnimationNodeIndex,
    pub walk: AnimationNodeIndex,
    pub run: AnimationNodeIndex,
    pub take_damage: AnimationNodeIndex,
    pub death: AnimationNodeIndex,
    pub cast_spell: AnimationNodeIndex,
}

// Systems

/// Load enemy model assets at startup
fn load_enemy_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Load scene file - enemy_wizard.glb for all enemies
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/enemy_wizard.glb"));

    // Try to load animation clips (may not exist in all models)
    // Animation indices in glTF files typically start at 0
    // Set HAS_ANIMATIONS to true if your model has idle (0) and walk (1) animations
    const HAS_ANIMATIONS: bool = true;

    let (graph, animations) = if HAS_ANIMATIONS {
        // Load all animation clips
        // Animation indices: 0=death, 2=idle, 7=take_damage, 9=run, 12=cast_spell, 14=walk
        let idle: Handle<AnimationClip> = asset_server.load(GltfAssetLabel::Animation(2).from_asset("models/enemy_wizard.glb"));
        let walk: Handle<AnimationClip> = asset_server.load(GltfAssetLabel::Animation(14).from_asset("models/enemy_wizard.glb"));
        let run: Handle<AnimationClip> = asset_server.load(GltfAssetLabel::Animation(9).from_asset("models/enemy_wizard.glb"));
        let take_damage: Handle<AnimationClip> = asset_server.load(GltfAssetLabel::Animation(7).from_asset("models/enemy_wizard.glb"));
        let death: Handle<AnimationClip> = asset_server.load(GltfAssetLabel::Animation(0).from_asset("models/enemy_wizard.glb"));
        let cast_spell: Handle<AnimationClip> = asset_server.load(GltfAssetLabel::Animation(12).from_asset("models/enemy_wizard.glb"));

        // Build animation graph with all clips
        let (graph, indices) = AnimationGraph::from_clips([
            idle,        // 0
            walk,        // 1
            run,         // 2
            take_damage, // 3
            death,       // 4
            cast_spell,  // 5
        ]);
        let graph_handle = graphs.add(graph);

        (
            Some(graph_handle),
            Some(EnemyAnimationIndices {
                idle: indices[0],
                walk: indices[1],
                run: indices[2],
                take_damage: indices[3],
                death: indices[4],
                cast_spell: indices[5],
            }),
        )
    } else {
        (None, None)
    };

    commands.insert_resource(EnemyAssets {
        scene,
        graph,
        animations,
    });
}

fn reset_wave_state(mut wave_state: ResMut<WaveState>) {
    wave_state.wave_number = 1;
    wave_state.enemies_remaining = 0;
    wave_state.spawn_timer = Timer::from_seconds(2.0, TimerMode::Once);
}

fn wave_spawner(
    mut commands: Commands,
    time: Res<Time>,
    mut wave_state: ResMut<WaveState>,
    enemies: Query<Entity, With<Enemy>>,
    arena_config: Res<ArenaConfig>,
    enemy_assets: Option<Res<EnemyAssets>>,
    enemy_defs: Res<EnemyDefinitions>,
    obstacles: Query<(&Transform, &ObstacleBounds), With<Obstacle>>,
    mut particle_events: EventWriter<SpawnParticleEvent>,
    terrain_sampler: Res<TerrainSampler>,
) {
    let Some(enemy_assets) = enemy_assets else {
        return; // Assets not loaded yet
    };

    wave_state.spawn_timer.tick(time.delta());

    // Count remaining enemies
    let enemy_count = enemies.iter().count() as u32;
    wave_state.enemies_remaining = enemy_count;

    // Spawn new wave if all enemies dead and timer finished
    if enemy_count == 0 && wave_state.spawn_timer.finished() {
        let num_enemies = ((wave_state.wave_number + 2) * 3).min(30);

        let mut rng = rand::thread_rng();
        let spawn_radius_min = arena_config.radius * 0.5;
        let spawn_radius_max = arena_config.radius * 0.85;
        let min_distance_from_obstacle = 4.5; // Must exceed largest obstacle radius (3.0) + enemy radius

        for i in 0..num_enemies {
            // Find a valid spawn position that doesn't overlap with obstacles
            let spawn_pos = find_valid_spawn_position(
                &mut rng,
                spawn_radius_min,
                spawn_radius_max,
                &obstacles,
                min_distance_from_obstacle,
                100, // max attempts
            );

            // Alternate enemy types
            let (enemy_type, type_id) = if i % 2 == 0 {
                (EnemyType::FireImp, EnemyTypeId::FIRE_IMP)
            } else {
                (EnemyType::FrostMage, EnemyTypeId::FROST_MAGE)
            };

            spawn_enemy(
                &mut commands,
                &enemy_assets,
                &enemy_defs,
                &terrain_sampler,
                spawn_pos,
                enemy_type,
                type_id,
            );

            // Spawn purple particle effect at enemy spawn location
            particle_events.send(SpawnParticleEvent::enemy_spawn(spawn_pos));
        }

        wave_state.wave_number += 1;
        wave_state.spawn_timer = Timer::from_seconds(3.0, TimerMode::Once);
    }
}

/// Find a valid spawn position in the spawn zone that doesn't overlap with obstacles
fn find_valid_spawn_position(
    rng: &mut impl Rng,
    spawn_radius_min: f32,
    spawn_radius_max: f32,
    obstacles: &Query<(&Transform, &ObstacleBounds), With<Obstacle>>,
    min_distance: f32,
    max_attempts: u32,
) -> Vec3 {
    let mut best_pos = None;
    let mut best_clearance = 0.0f32;

    for _ in 0..max_attempts {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let radius = rng.gen_range(spawn_radius_min..spawn_radius_max);
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        let pos = Vec3::new(x, 0.0, z);

        // Find minimum clearance from all obstacles (use spawn_radius to avoid spawning under canopies)
        let mut min_clearance = f32::MAX;
        for (obs_transform, obs_bounds) in obstacles.iter() {
            let obs_pos = obs_transform.translation;
            let dist = ((pos.x - obs_pos.x).powi(2) + (pos.z - obs_pos.z).powi(2)).sqrt();
            let clearance = dist - obs_bounds.spawn_radius;
            min_clearance = min_clearance.min(clearance);
        }

        // If we have enough clearance, use this position immediately
        if min_clearance >= min_distance {
            return pos;
        }

        // Track the best position found so far (largest clearance)
        if min_clearance > best_clearance {
            best_clearance = min_clearance;
            best_pos = Some(pos);
        }
    }

    // Fallback: use the best position found (largest clearance from obstacles)
    best_pos.unwrap_or_else(|| {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let radius = rng.gen_range(spawn_radius_min..spawn_radius_max);
        Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius)
    })
}

fn spawn_enemy(
    commands: &mut Commands,
    enemy_assets: &EnemyAssets,
    enemy_defs: &EnemyDefinitions,
    terrain_sampler: &TerrainSampler,
    position: Vec3,
    enemy_type: EnemyType,
    type_id: EnemyTypeId,
) {
    // Get the enemy definition
    let Some(def) = enemy_defs.get(type_id) else {
        warn!("No definition found for enemy type {:?}", type_id);
        return;
    };

    // Capsule dimensions: radius=0.5, half_length=0.5
    // Total height = 2*radius + 2*half_length = 2.0
    // Center to bottom = radius + half_length = 1.0
    // Use terrain sampler to get correct spawn height above terrain
    let spawn_position = terrain_sampler.get_spawn_position(position.x, position.z, 2.0);

    // Spawn the enemy entity with physics (root entity)
    commands.spawn((
        // Transform for the physics entity (capsule center)
        Transform::from_translation(spawn_position),
        Visibility::default(),
        // Physics components - capsule collider sized for the model
        (
            RigidBody::Dynamic,
            Collider::capsule(0.5, 1.0),
            CollisionMargin(0.05),
            LockedAxes::ROTATION_LOCKED,
            LinearDamping(5.0),
            Restitution::new(0.0),
            Friction::new(0.5),
        ),
        // Enemy identity
        (
            Enemy,
            enemy_type,
            Health::new(def.max_health),
            MovementSpeed(def.movement_speed),
            AIState::default(),
        ),
        // AI components - all from definition
        (
            DetectionRange(def.detection_range),
            AttackRange(def.attack_range),
            AttackCooldown(Timer::from_seconds(def.attack_cooldown, TimerMode::Once)),
            SpellCooldowns::new(),
        ),
        // Steering behavior for obstacle avoidance
        (
            SteeringConfig::default(),
            SteeringState::default(),
        ),
        // Attack components from definition
        (
            PrimaryAttack(def.primary_attack),
            CastDuration(def.cast_duration),
        ),
        // Animation tracking
        EnemyAnimator {
            current_state: EnemyAnimationState::Idle,
            animation_player_entity: None,
        },
        // Team and other
        (
            Team::ENEMY,
            Hittable,
            StateScoped(GameState::Playing),
            Name::new(def.name.clone()),
        ),
    )).with_children(|parent| {
        // Spawn the 3D model as a child with Y offset to anchor feet to capsule bottom
        parent.spawn((
            SceneRoot(enemy_assets.scene.clone()),
            Transform::from_translation(Vec3::new(0.0, def.model_y_offset, 0.0))
                .with_scale(Vec3::splat(def.model_scale)),
        ));
    });
}

/// Find and initialize the AnimationPlayer for newly spawned enemies
/// This runs every frame to catch AnimationPlayers that spawn asynchronously with the scene
fn setup_enemy_animation_player(
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut EnemyAnimator), Without<EnemyAnimationIndices>>,
    children_query: Query<&Children>,
    animation_player_query: Query<Entity, With<AnimationPlayer>>,
    enemy_assets: Option<Res<EnemyAssets>>,
) {
    let Some(enemy_assets) = enemy_assets else {
        return;
    };

    // Skip if no animations are available
    let (Some(graph), Some(animations)) = (&enemy_assets.graph, &enemy_assets.animations) else {
        return;
    };

    for (enemy_entity, mut animator) in enemies.iter_mut() {
        // Skip if already set up
        if animator.animation_player_entity.is_some() {
            continue;
        }

        // Find the AnimationPlayer in the entity hierarchy (glTF scenes add it to a child)
        if let Some(animation_player_entity) = find_animation_player(
            enemy_entity,
            &children_query,
            &animation_player_query,
        ) {
            animator.animation_player_entity = Some(animation_player_entity);

            // Add the animation graph and transitions to the animation player entity
            commands.entity(animation_player_entity).insert((
                AnimationGraphHandle(graph.clone()),
                AnimationTransitions::new(),
            ));

            // Store animation indices on the enemy for later use
            commands.entity(enemy_entity).insert(animations.clone());
        }
    }
}

/// Recursively search for an AnimationPlayer in the entity hierarchy
fn find_animation_player(
    entity: Entity,
    children_query: &Query<&Children>,
    animation_player_query: &Query<Entity, With<AnimationPlayer>>,
) -> Option<Entity> {
    // Check if this entity has an AnimationPlayer
    if animation_player_query.get(entity).is_ok() {
        return Some(entity);
    }

    // Search children recursively
    if let Ok(children) = children_query.get(entity) {
        for &child in children.iter() {
            if let Some(found) = find_animation_player(child, children_query, animation_player_query) {
                return Some(found);
            }
        }
    }

    None
}

/// Duration for blending between animations
const ANIMATION_BLEND_DURATION: Duration = Duration::from_millis(200);
/// Faster blend for priority animations like damage
const ANIMATION_BLEND_FAST: Duration = Duration::from_millis(100);

/// Sync enemy animation state with AI state
fn update_enemy_animations(
    mut commands: Commands,
    time: Res<Time>,
    mut enemies: Query<(Entity, &EnemyAnimator, &EnemyAnimationIndices, Option<&mut PlayingDamageAnimation>), With<Enemy>>,
    mut animation_data: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (entity, animator, indices, damage_anim) in enemies.iter_mut() {
        // Get the animation player
        let Some(player_entity) = animator.animation_player_entity else {
            continue;
        };

        let Ok((mut player, mut transitions)) = animation_data.get_mut(player_entity) else {
            continue;
        };

        // Handle damage animation (takes priority)
        if let Some(mut damage_anim) = damage_anim {
            damage_anim.timer.tick(time.delta());

            if damage_anim.timer.finished() {
                // Remove the damage animation component
                commands.entity(entity).remove::<PlayingDamageAnimation>();
            } else {
                // Keep playing damage animation with fast blend
                if !player.is_playing_animation(indices.take_damage) {
                    transitions
                        .play(&mut player, indices.take_damage, ANIMATION_BLEND_FAST);
                }
                continue;
            }
        }

        // Get the appropriate animation index based on current state
        let animation_index = match animator.current_state {
            EnemyAnimationState::Idle => indices.idle,
            EnemyAnimationState::Walking => indices.walk,
            EnemyAnimationState::Running => indices.run,
            EnemyAnimationState::TakingDamage => indices.take_damage,
            EnemyAnimationState::CastingSpell => indices.cast_spell,
        };

        // Only change animation if not already playing
        if !player.is_playing_animation(animation_index) {
            // Use transitions for smooth blending
            if animator.current_state == EnemyAnimationState::CastingSpell {
                // Cast spell plays once (no repeat)
                transitions
                    .play(&mut player, animation_index, ANIMATION_BLEND_DURATION);
            } else {
                // Looping animations
                transitions
                    .play(&mut player, animation_index, ANIMATION_BLEND_DURATION)
                    .repeat();
            }
        }
    }
}

/// Calculate obstacle avoidance force using proximity-based repulsion
/// Based on Craig Reynolds' steering behaviors
fn calculate_avoidance_force(
    enemy_pos: Vec3,
    velocity_dir: Vec3,
    config: &SteeringConfig,
    obstacles: &Query<(&Transform, &ObstacleBounds), With<Obstacle>>,
) -> Vec3 {
    let mut total_avoidance = Vec3::ZERO;

    // Enemy collision radius (approximation)
    const ENEMY_RADIUS: f32 = 0.5;

    for (obs_transform, obs_bounds) in obstacles.iter() {
        let obs_pos = obs_transform.translation;

        // Vector from obstacle to enemy (on XZ plane)
        let to_enemy = Vec3::new(
            enemy_pos.x - obs_pos.x,
            0.0,
            enemy_pos.z - obs_pos.z,
        );

        let distance = to_enemy.length();

        // Combined radius: obstacle avoidance + enemy + margin
        let combined_radius = obs_bounds.avoidance_radius + ENEMY_RADIUS + config.avoidance_margin;

        // Only consider obstacles within detection range
        if distance < config.detection_radius + obs_bounds.avoidance_radius {
            // Check if we're heading toward this obstacle
            let to_obstacle = -to_enemy.normalize_or_zero();
            let heading_toward = velocity_dir.dot(to_obstacle);

            // Only avoid if we're somewhat heading toward the obstacle
            // or if we're very close (emergency avoidance)
            if heading_toward > -0.3 || distance < combined_radius * 1.2 {
                // Calculate repulsion strength based on proximity
                // Closer = stronger repulsion, using inverse square falloff
                let penetration = combined_radius - distance;

                if penetration > 0.0 {
                    // We're inside the avoidance zone - very strong repulsion
                    let strength = (penetration / combined_radius).min(1.0);
                    // Exponential falloff for stronger close-range repulsion
                    let repulsion = to_enemy.normalize_or_zero() * strength * strength * 5.0;
                    total_avoidance += repulsion;
                } else if distance < combined_radius * 1.5 {
                    // Close proximity - moderate emergency avoidance regardless of heading
                    let proximity = 1.0 - (distance / (combined_radius * 1.5));
                    let repulsion = to_enemy.normalize_or_zero() * proximity * 2.0;
                    total_avoidance += repulsion;
                } else {
                    // Approaching obstacle - gradual avoidance
                    let proximity = 1.0 - (distance / (config.detection_radius + obs_bounds.avoidance_radius));
                    if proximity > 0.0 {
                        // Scale by how directly we're heading toward it
                        let directness = heading_toward.max(0.0);
                        let strength = proximity * proximity * directness;
                        let repulsion = to_enemy.normalize_or_zero() * strength;
                        total_avoidance += repulsion;
                    }
                }
            }
        }
    }

    total_avoidance * config.avoidance_weight
}

/// Check if there's a clear line of sight from origin to target (on XZ plane)
/// Returns true if no obstacles block the path
fn has_line_of_sight(
    origin: Vec3,
    target: Vec3,
    obstacles: &Query<(&Transform, &ObstacleBounds), With<Obstacle>>,
) -> bool {
    // Spell origin height offset
    let ray_origin = Vec3::new(origin.x, origin.y + 1.0, origin.z);
    let ray_target = Vec3::new(target.x, target.y + 1.0, target.z);

    let ray_dir = ray_target - ray_origin;
    let ray_length = ray_dir.length();

    if ray_length < 0.01 {
        return true;
    }

    let ray_dir_norm = ray_dir / ray_length;

    // Check intersection with each obstacle cylinder (2D circle on XZ plane)
    for (obs_transform, obs_bounds) in obstacles.iter() {
        let obs_pos = obs_transform.translation;

        // Vector from ray origin to obstacle center (XZ plane)
        let to_obstacle = Vec3::new(
            obs_pos.x - ray_origin.x,
            0.0,
            obs_pos.z - ray_origin.z,
        );

        // Project obstacle center onto ray direction (XZ plane)
        let ray_dir_xz = Vec3::new(ray_dir_norm.x, 0.0, ray_dir_norm.z).normalize_or_zero();
        let projection_length = to_obstacle.dot(ray_dir_xz);

        // Skip if obstacle is behind us or beyond target
        if projection_length < 0.0 || projection_length > ray_length {
            continue;
        }

        // Find closest point on ray to obstacle center
        let closest_point = Vec3::new(
            ray_origin.x + ray_dir_xz.x * projection_length,
            0.0,
            ray_origin.z + ray_dir_xz.z * projection_length,
        );

        // Distance from obstacle center to closest point on ray
        let distance_to_ray = ((obs_pos.x - closest_point.x).powi(2)
            + (obs_pos.z - closest_point.z).powi(2))
        .sqrt();

        // If ray passes through obstacle trunk (with small buffer), line of sight is blocked
        if distance_to_ray < obs_bounds.avoidance_radius + 0.3 {
            return false;
        }
    }

    true
}

fn ai_detection(
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<(&Transform, &DetectionRange, &mut AIState), (With<Enemy>, Without<Player>)>,
) {
    let Ok((player_entity, player_transform)) = player_query.get_single() else {
        return;
    };

    for (enemy_transform, detection_range, mut ai_state) in enemy_query.iter_mut() {
        let distance = (player_transform.translation - enemy_transform.translation).length();

        match *ai_state {
            AIState::Idle => {
                if distance < detection_range.0 {
                    *ai_state = AIState::Chasing(player_entity);
                }
            }
            AIState::Chasing(_) | AIState::Attacking(_) => {
                if distance > detection_range.0 * 1.5 {
                    *ai_state = AIState::Idle;
                }
            }
        }
    }
}

fn ai_movement(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    obstacles: Query<(&Transform, &ObstacleBounds), With<Obstacle>>,
    mut enemy_query: Query<
        (
            &mut Transform,
            &mut LinearVelocity,
            &MovementSpeed,
            &AttackRange,
            &mut AIState,
            &mut EnemyAnimator,
            &SteeringConfig,
            &mut SteeringState,
            Option<&SlowEffect>,
            Option<&CastingSpellAnimation>,
        ),
        (With<Enemy>, Without<Player>, Without<Obstacle>),
    >,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    let dt = time.delta_secs();

    for (mut transform, mut velocity, speed, attack_range, mut ai_state, mut animator, steering_config, mut steering_state, slow_effect, casting) in
        enemy_query.iter_mut()
    {
        let is_casting = casting.is_some();
        let to_player = player_transform.translation - transform.translation;
        let distance = to_player.length();
        let seek_direction = to_player.normalize_or_zero();

        let slow_factor = slow_effect.map_or(1.0, |s| s.factor);

        // While casting: stop moving and face the player (regardless of AI state)
        if is_casting {
            velocity.x = 0.0;
            velocity.z = 0.0;
            steering_state.current_velocity = Vec3::ZERO;

            // Face player during spell cast
            if seek_direction != Vec3::ZERO {
                let target_rotation = Quat::from_rotation_y(seek_direction.x.atan2(seek_direction.z));
                transform.rotation = transform.rotation.slerp(target_rotation, 0.15);
            }
            continue;
        }

        match *ai_state {
            AIState::Chasing(target) => {
                // Only attack if in range AND have line of sight
                let can_attack = distance < attack_range.0
                    && has_line_of_sight(transform.translation, player_transform.translation, &obstacles);

                if can_attack {
                    *ai_state = AIState::Attacking(target);
                } else {
                    // Determine run vs walk based on distance
                    let (anim_state, speed_mult) = if distance > RUN_DISTANCE_THRESHOLD {
                        (EnemyAnimationState::Running, RUN_SPEED_MULTIPLIER)
                    } else {
                        (EnemyAnimationState::Walking, 1.0)
                    };

                    animator.current_state = anim_state;

                    // Calculate target speed
                    let max_speed = speed.0 * speed_mult * slow_factor;

                    // Calculate desired velocity: seek player
                    let desired_velocity = seek_direction * max_speed;

                    // Calculate avoidance force based on current movement direction
                    let current_dir = if steering_state.current_velocity.length_squared() > 0.01 {
                        steering_state.current_velocity.normalize()
                    } else {
                        seek_direction
                    };

                    let avoidance_force = calculate_avoidance_force(
                        transform.translation,
                        current_dir,
                        steering_config,
                        &obstacles,
                    );

                    // Combine seek and avoidance using weighted sum
                    // Avoidance force is already scaled by weight in the function
                    let combined_velocity = desired_velocity + avoidance_force * max_speed;

                    // Clamp to max speed
                    let target_velocity = if combined_velocity.length() > max_speed {
                        combined_velocity.normalize() * max_speed
                    } else {
                        combined_velocity
                    };

                    // Smooth velocity changes using exponential smoothing
                    // This prevents jerky movement
                    let smoothing = 1.0 - (-steering_config.steering_smoothing * 60.0 * dt).exp();
                    steering_state.current_velocity = steering_state.current_velocity
                        .lerp(target_velocity, smoothing.clamp(0.0, 1.0));

                    // Apply velocity
                    velocity.x = steering_state.current_velocity.x;
                    velocity.z = steering_state.current_velocity.z;
                    // Clamp upward velocity to prevent climbing on obstacles
                    if velocity.y > 0.5 {
                        velocity.y = 0.5;
                    }

                    // Face movement direction for natural navigation while walking/running
                    let move_dir = steering_state.current_velocity.normalize_or_zero();
                    if move_dir.length_squared() > 0.01 {
                        let target_rotation = Quat::from_rotation_y(move_dir.x.atan2(move_dir.z));
                        transform.rotation = transform.rotation.slerp(target_rotation, 0.1);
                    }
                }
            }
            AIState::Attacking(target) => {
                // Go back to chasing if out of range OR line of sight is blocked
                let los_blocked = !has_line_of_sight(transform.translation, player_transform.translation, &obstacles);

                if distance > attack_range.0 * 1.2 || los_blocked {
                    *ai_state = AIState::Chasing(target);
                } else {
                    // Stop moving
                    velocity.x = 0.0;
                    velocity.z = 0.0;
                    steering_state.current_velocity = Vec3::ZERO;

                    animator.current_state = EnemyAnimationState::Idle;

                    // Face player when in attack range (preparing to cast)
                    if seek_direction != Vec3::ZERO {
                        let target_rotation = Quat::from_rotation_y(seek_direction.x.atan2(seek_direction.z));
                        transform.rotation = transform.rotation.slerp(target_rotation, 0.1);
                    }
                }
            }
            AIState::Idle => {
                velocity.x = 0.0;
                velocity.z = 0.0;
                steering_state.current_velocity = Vec3::ZERO;
                animator.current_state = EnemyAnimationState::Idle;
            }
        }
    }
}

fn ai_attack(
    mut commands: Commands,
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<
        (
            Entity,
            &Transform,
            &AIState,
            &mut AttackCooldown,
            &mut EnemyAnimator,
            &CastDuration,
            Option<&CastingSpellAnimation>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    for (entity, transform, ai_state, mut cooldown, mut animator, cast_duration, casting) in enemy_query.iter_mut() {
        cooldown.0.tick(time.delta());

        // Don't start a new cast if already casting
        if casting.is_some() {
            continue;
        }

        if matches!(ai_state, AIState::Attacking(_)) && cooldown.0.finished() {
            let direction = (player_transform.translation - transform.translation).normalize_or_zero();

            // Start casting animation - use cast_duration from component
            animator.current_state = EnemyAnimationState::CastingSpell;
            commands.entity(entity).insert(CastingSpellAnimation {
                timer: Timer::from_seconds(cast_duration.0, TimerMode::Once),
                direction,
                has_fired: false,
            });

            cooldown.0.reset();
        }
    }
}

/// System to handle casting animation and fire spell when complete
fn update_casting_spells(
    mut commands: Commands,
    time: Res<Time>,
    mut spell_events: EventWriter<SpellCastEvent>,
    mut enemy_query: Query<
        (
            Entity,
            &Transform,
            &Team,
            &PrimaryAttack,
            &mut CastingSpellAnimation,
            &mut EnemyAnimator,
        ),
        With<Enemy>,
    >,
) {
    for (entity, transform, team, primary_attack, mut casting, mut animator) in enemy_query.iter_mut() {
        casting.timer.tick(time.delta());

        // Fire the spell at 70% through the animation (wind-up complete)
        if !casting.has_fired && casting.timer.fraction() >= 0.7 {
            casting.has_fired = true;

            spell_events.send(SpellCastEvent {
                caster: entity,
                spell_type: primary_attack.0,  // Use the enemy's primary attack spell type
                position: transform.translation + Vec3::Y * 1.0, // Fire from hand height
                direction: casting.direction,
                team: *team,
            });
        }

        // Remove casting component when animation finishes
        if casting.timer.finished() {
            commands.entity(entity).remove::<CastingSpellAnimation>();
            animator.current_state = EnemyAnimationState::Idle;
        }
    }
}
