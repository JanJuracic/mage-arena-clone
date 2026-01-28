pub mod ambient;
pub mod config;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_hanabi::ScalarType;

use crate::states::GameState;

pub use config::{ParticleExplosionsConfig, ParticleTrailsConfig};
pub use ambient::AmbientParticlePlugin;

pub struct ParticlePlugin;

/// System set for particle systems, used for ordering
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParticleSet;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        // Load particle configs (fall back to defaults if files missing)
        let explosions_config = ParticleExplosionsConfig::load();
        let trails_config = ParticleTrailsConfig::load();

        app.add_plugins(HanabiPlugin)
            .add_plugins(AmbientParticlePlugin) // Add ambient floating particles
            .insert_resource(explosions_config)
            .insert_resource(trails_config)
            .add_event::<SpawnParticleEvent>()
            // Warmup system runs once when entering Playing state to initialize GPU resources
            // This works around bevy_hanabi issue #319 where first spawn doesn't emit particles
            .add_systems(OnEnter(GameState::Playing), warmup_particle_effects)
            .add_systems(
                Update,
                (
                    handle_spawn_particle_events,
                    despawn_finished_effects,
                )
                    .chain()
                    .in_set(ParticleSet)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ============================================================================
// COMPONENTS
// ============================================================================

/// Component to track lifetime of particle effect entities.
/// When the timer finishes, the effect entity is despawned.
#[derive(Component)]
pub struct ParticleEffectLifetime(pub Timer);

// ============================================================================
// CONFIGURATION STRUCTS
// ============================================================================

/// Configuration for explosion-style effects (single burst)
#[derive(Clone)]
pub struct ExplosionConfig {
    pub color_gradient: Gradient<Vec4>,
    pub size_gradient: Gradient<Vec3>,
    pub particle_count: f32,
    pub speed: f32,
    pub lifetime: f32,
    pub spawn_radius: f32,
    pub drag: f32,
    pub gravity: Vec3,
    /// Spawn shape: true = circle on XZ plane, false = sphere
    pub circle_spawn: bool,
    /// Spawn center offset
    pub spawn_offset: Vec3,
}

impl ExplosionConfig {
    pub fn fire() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(1.0, 1.0, 0.6, 1.0));
        color_gradient.add_key(0.15, Vec4::new(1.0, 0.8, 0.2, 1.0));
        color_gradient.add_key(0.4, Vec4::new(1.0, 0.4, 0.1, 1.0));
        color_gradient.add_key(0.7, Vec4::new(0.9, 0.2, 0.05, 0.8));
        color_gradient.add_key(1.0, Vec4::new(0.3, 0.1, 0.05, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.08));
        size_gradient.add_key(0.3, Vec3::splat(0.15));
        size_gradient.add_key(0.7, Vec3::splat(0.1));
        size_gradient.add_key(1.0, Vec3::splat(0.0));

        Self {
            color_gradient,
            size_gradient,
            particle_count: 300.0,
            speed: 12.0,
            lifetime: 0.9,
            spawn_radius: 0.5,
            drag: 3.0,
            gravity: Vec3::new(0.0, -8.0, 0.0),
            circle_spawn: false,
            spawn_offset: Vec3::ZERO,
        }
    }

    pub fn smoke() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(0.3, 0.25, 0.2, 0.6));
        color_gradient.add_key(0.3, Vec4::new(0.25, 0.22, 0.18, 0.5));
        color_gradient.add_key(0.6, Vec4::new(0.2, 0.18, 0.15, 0.3));
        color_gradient.add_key(1.0, Vec4::new(0.15, 0.13, 0.1, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.1));
        size_gradient.add_key(0.3, Vec3::splat(0.25));
        size_gradient.add_key(0.7, Vec3::splat(0.35));
        size_gradient.add_key(1.0, Vec3::splat(0.4));

        Self {
            color_gradient,
            size_gradient,
            particle_count: 80.0,
            speed: 4.0,
            lifetime: 1.5,
            spawn_radius: 0.6,
            drag: 2.5,
            gravity: Vec3::new(0.0, 3.0, 0.0),
            circle_spawn: false,
            spawn_offset: Vec3::ZERO,
        }
    }

    pub fn frost() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(1.0, 1.0, 1.0, 1.0));
        color_gradient.add_key(0.2, Vec4::new(0.7, 0.9, 1.0, 1.0));
        color_gradient.add_key(0.5, Vec4::new(0.4, 0.7, 1.0, 0.9));
        color_gradient.add_key(0.8, Vec4::new(0.2, 0.5, 0.9, 0.6));
        color_gradient.add_key(1.0, Vec4::new(0.1, 0.3, 0.7, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.06));
        size_gradient.add_key(0.2, Vec3::splat(0.12));
        size_gradient.add_key(0.6, Vec3::splat(0.08));
        size_gradient.add_key(1.0, Vec3::splat(0.0));

        Self {
            color_gradient,
            size_gradient,
            particle_count: 250.0,
            speed: 8.0,
            lifetime: 1.0,
            spawn_radius: 0.4,
            drag: 4.0,
            gravity: Vec3::new(0.0, 2.0, 0.0),
            circle_spawn: false,
            spawn_offset: Vec3::ZERO,
        }
    }

    pub fn arcane() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(0.8, 0.4, 1.0, 1.0));
        color_gradient.add_key(0.5, Vec4::new(0.6, 0.2, 0.9, 0.7));
        color_gradient.add_key(1.0, Vec4::new(0.3, 0.1, 0.5, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.06));
        size_gradient.add_key(0.5, Vec3::splat(0.1));
        size_gradient.add_key(1.0, Vec3::splat(0.03));

        Self {
            color_gradient,
            size_gradient,
            particle_count: 200.0,
            speed: 8.0,
            lifetime: 0.5,
            spawn_radius: 0.5,
            drag: 2.0,
            gravity: Vec3::ZERO,
            circle_spawn: true,
            spawn_offset: Vec3::ZERO,
        }
    }

    pub fn hit_spark() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(1.0, 1.0, 1.0, 1.0));
        color_gradient.add_key(0.2, Vec4::new(0.9, 0.7, 1.0, 1.0));
        color_gradient.add_key(0.5, Vec4::new(0.7, 0.3, 1.0, 0.9));
        color_gradient.add_key(1.0, Vec4::new(0.4, 0.1, 0.6, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.05));
        size_gradient.add_key(0.3, Vec3::splat(0.1));
        size_gradient.add_key(1.0, Vec3::splat(0.0));

        Self {
            color_gradient,
            size_gradient,
            particle_count: 150.0,
            speed: 8.0,
            lifetime: 0.5,
            spawn_radius: 0.2,
            drag: 4.0,
            gravity: Vec3::ZERO,
            circle_spawn: false,
            spawn_offset: Vec3::ZERO,
        }
    }

    pub fn muzzle_flash() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(1.0, 0.9, 0.7, 1.0));
        color_gradient.add_key(0.3, Vec4::new(1.0, 0.6, 0.3, 0.6));
        color_gradient.add_key(1.0, Vec4::new(0.8, 0.3, 0.1, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.08));
        size_gradient.add_key(0.2, Vec3::splat(0.12));
        size_gradient.add_key(1.0, Vec3::splat(0.0));

        Self {
            color_gradient,
            size_gradient,
            particle_count: 30.0,
            speed: 2.0,
            lifetime: 0.15,
            spawn_radius: 0.05,
            drag: 0.0,
            gravity: Vec3::ZERO,
            circle_spawn: false,
            spawn_offset: Vec3::ZERO,
        }
    }

    pub fn enemy_death() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(1.0, 0.2, 0.1, 1.0));
        color_gradient.add_key(0.2, Vec4::new(0.8, 0.1, 0.05, 0.9));
        color_gradient.add_key(0.5, Vec4::new(0.4, 0.05, 0.02, 0.7));
        color_gradient.add_key(0.7, Vec4::new(0.15, 0.1, 0.1, 0.5));
        color_gradient.add_key(1.0, Vec4::new(0.05, 0.03, 0.03, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.05));
        size_gradient.add_key(0.3, Vec3::splat(0.1));
        size_gradient.add_key(0.7, Vec3::splat(0.125));
        size_gradient.add_key(1.0, Vec3::splat(0.075));

        Self {
            color_gradient,
            size_gradient,
            particle_count: 200.0,
            speed: 3.0,
            lifetime: 1.2,
            spawn_radius: 0.5,
            drag: 2.0,
            gravity: Vec3::new(0.0, -4.0, 0.0),
            circle_spawn: false,
            spawn_offset: Vec3::new(0.0, 0.5, 0.0),
        }
    }

    pub fn enemy_spawn() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(0.9, 0.5, 1.0, 1.0));
        color_gradient.add_key(0.2, Vec4::new(0.7, 0.3, 0.9, 0.9));
        color_gradient.add_key(0.5, Vec4::new(0.5, 0.1, 0.8, 0.7));
        color_gradient.add_key(0.8, Vec4::new(0.3, 0.05, 0.5, 0.4));
        color_gradient.add_key(1.0, Vec4::new(0.1, 0.0, 0.2, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.3));
        size_gradient.add_key(0.3, Vec3::splat(0.5));
        size_gradient.add_key(0.7, Vec3::splat(0.35));
        size_gradient.add_key(1.0, Vec3::splat(0.0));

        Self {
            color_gradient,
            size_gradient,
            particle_count: 300.0,
            speed: 8.0,
            lifetime: 1.0,
            spawn_radius: 2.0,
            drag: 2.0,
            gravity: Vec3::new(0.0, 8.0, 0.0),
            circle_spawn: true,
            spawn_offset: Vec3::ZERO,
        }
    }

    /// Returns the duration for entity lifetime (particle lifetime + buffer)
    pub fn duration(&self) -> f32 {
        self.lifetime + 0.2
    }

    /// Create an ExplosionConfig from loaded config data
    pub fn from_config(data: &config::ExplosionEffectData) -> Self {
        Self {
            color_gradient: data.to_color_gradient(),
            size_gradient: data.to_size_gradient(),
            particle_count: data.particle_count,
            speed: data.speed,
            lifetime: data.lifetime,
            spawn_radius: data.spawn_radius,
            drag: data.drag,
            gravity: data.gravity_vec(),
            circle_spawn: data.circle_spawn,
            spawn_offset: data.spawn_offset_vec(),
        }
    }
}

/// Configuration for trail-style effects (continuous stream)
#[derive(Clone)]
pub struct TrailConfig {
    pub color_gradient: Gradient<Vec4>,
    pub size_gradient: Gradient<Vec3>,
    pub spawn_rate: f32,
    pub speed: f32,
    pub lifetime: f32,
    pub spawn_radius: f32,
    pub gravity: Vec3,
}

impl TrailConfig {
    pub fn fire() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(1.0, 0.8, 0.3, 0.8));
        color_gradient.add_key(0.3, Vec4::new(1.0, 0.5, 0.1, 0.6));
        color_gradient.add_key(0.7, Vec4::new(0.8, 0.2, 0.0, 0.3));
        color_gradient.add_key(1.0, Vec4::new(0.3, 0.1, 0.0, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.04));
        size_gradient.add_key(0.3, Vec3::splat(0.06));
        size_gradient.add_key(1.0, Vec3::splat(0.0));

        Self {
            color_gradient,
            size_gradient,
            spawn_rate: 80.0,
            speed: 0.5,
            lifetime: 0.25,
            spawn_radius: 0.02,
            gravity: Vec3::new(0.0, 1.0, 0.0),
        }
    }

    pub fn frost() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(0.9, 0.95, 1.0, 0.8));
        color_gradient.add_key(0.3, Vec4::new(0.6, 0.8, 1.0, 0.6));
        color_gradient.add_key(0.7, Vec4::new(0.3, 0.5, 0.9, 0.3));
        color_gradient.add_key(1.0, Vec4::new(0.1, 0.3, 0.6, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.03));
        size_gradient.add_key(0.3, Vec3::splat(0.05));
        size_gradient.add_key(1.0, Vec3::splat(0.0));

        Self {
            color_gradient,
            size_gradient,
            spawn_rate: 100.0,
            speed: 0.3,
            lifetime: 0.2,
            spawn_radius: 0.02,
            gravity: Vec3::new(0.0, -0.5, 0.0),
        }
    }

    pub fn magic() -> Self {
        let mut color_gradient = Gradient::new();
        color_gradient.add_key(0.0, Vec4::new(1.0, 0.7, 1.0, 0.9));
        color_gradient.add_key(0.3, Vec4::new(0.9, 0.4, 1.0, 0.7));
        color_gradient.add_key(0.7, Vec4::new(0.6, 0.2, 0.9, 0.4));
        color_gradient.add_key(1.0, Vec4::new(0.3, 0.1, 0.5, 0.0));

        let mut size_gradient = Gradient::new();
        size_gradient.add_key(0.0, Vec3::splat(0.025));
        size_gradient.add_key(0.2, Vec3::splat(0.04));
        size_gradient.add_key(1.0, Vec3::splat(0.0));

        Self {
            color_gradient,
            size_gradient,
            spawn_rate: 60.0,
            speed: 0.4,
            lifetime: 0.18,
            spawn_radius: 0.015,
            gravity: Vec3::ZERO,
        }
    }

    /// Returns the duration for entity lifetime (particle lifetime + buffer)
    pub fn duration(&self) -> f32 {
        self.lifetime + 0.1
    }

    /// Create a TrailConfig from loaded config data
    pub fn from_config(data: &config::TrailEffectData) -> Self {
        Self {
            color_gradient: data.to_color_gradient(),
            size_gradient: data.to_size_gradient(),
            spawn_rate: data.spawn_rate,
            speed: data.speed,
            lifetime: data.lifetime,
            spawn_radius: data.spawn_radius,
            gravity: data.gravity_vec(),
        }
    }
}

/// Type of particle effect to spawn
#[derive(Clone)]
pub enum ParticleSpawnType {
    Explosion(ExplosionConfig),
    Trail(TrailConfig),
}

// ============================================================================
// EVENTS
// ============================================================================

/// Event to spawn a particle effect at a specific location.
/// Send this event from any system to create particle effects.
#[derive(Event, Clone)]
pub struct SpawnParticleEvent {
    pub effect: ParticleSpawnType,
    pub position: Vec3,
    /// Optional: direction for directional effects (normalized)
    pub direction: Option<Vec3>,
    /// Optional: scale multiplier (default 1.0)
    pub scale: Option<f32>,
}

impl SpawnParticleEvent {
    pub fn new(effect: ParticleSpawnType, position: Vec3) -> Self {
        Self {
            effect,
            position,
            direction: None,
            scale: None,
        }
    }

    pub fn with_direction(mut self, direction: Vec3) -> Self {
        self.direction = Some(direction.normalize_or_zero());
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale);
        self
    }

    // Convenience constructors for explosion effects
    pub fn fire_explosion(position: Vec3) -> Self {
        Self::new(ParticleSpawnType::Explosion(ExplosionConfig::fire()), position)
    }

    pub fn fire_smoke(position: Vec3) -> Self {
        Self::new(ParticleSpawnType::Explosion(ExplosionConfig::smoke()), position)
    }

    pub fn frost_impact(position: Vec3) -> Self {
        Self::new(ParticleSpawnType::Explosion(ExplosionConfig::frost()), position)
    }

    pub fn arcane_nova(position: Vec3) -> Self {
        Self::new(ParticleSpawnType::Explosion(ExplosionConfig::arcane()), position)
    }

    pub fn hit_spark(position: Vec3) -> Self {
        Self::new(ParticleSpawnType::Explosion(ExplosionConfig::hit_spark()), position)
    }

    pub fn muzzle_flash(position: Vec3) -> Self {
        Self::new(ParticleSpawnType::Explosion(ExplosionConfig::muzzle_flash()), position)
    }

    pub fn enemy_death(position: Vec3) -> Self {
        Self::new(ParticleSpawnType::Explosion(ExplosionConfig::enemy_death()), position)
    }

    pub fn enemy_spawn(position: Vec3) -> Self {
        Self::new(ParticleSpawnType::Explosion(ExplosionConfig::enemy_spawn()), position)
    }
}

// ============================================================================
// EFFECT BUILDERS
// ============================================================================

/// Creates an explosion effect from config
fn create_explosion(config: &ExplosionConfig) -> EffectAsset {
    let writer = ExprWriter::new();

    // Random lifetime: base_lifetime * (0.75 + rand * 0.5) for ±25% variation
    let lifetime_rand = writer.rand(ScalarType::Float) * writer.lit(0.5) + writer.lit(0.75);
    let lifetime_varied = writer.lit(config.lifetime) * lifetime_rand;
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime_varied.expr());

    // Random speed: base_speed * (0.75 + rand * 0.5) for ±25% variation
    // Pre-compute the expression handle so it can be used in both branches
    let speed_rand = writer.rand(ScalarType::Float) * writer.lit(0.5) + writer.lit(0.75);
    let speed_varied = (writer.lit(config.speed) * speed_rand).expr();

    // Random size scale: (0.75 + rand * 0.5) for ±25% variation
    // SizeOverLifetimeModifier multiplies gradient values by the SIZE attribute
    let size_rand = writer.rand(ScalarType::Float) * writer.lit(0.5) + writer.lit(0.75);
    let init_size = SetAttributeModifier::new(Attribute::SIZE, size_rand.expr());

    // Create drag and gravity modifiers
    let drag = LinearDragModifier {
        drag: writer.lit(config.drag).expr(),
    };
    let gravity = AccelModifier::new(writer.lit(config.gravity).expr());

    // Build effect based on spawn shape - all expressions must use the same writer/module
    if config.circle_spawn {
        let init_pos = SetPositionCircleModifier {
            center: writer.lit(config.spawn_offset).expr(),
            axis: writer.lit(Vec3::Y).expr(),
            radius: writer.lit(config.spawn_radius).expr(),
            dimension: ShapeDimension::Volume,
        };
        // Velocity spreads outward from center - each particle gets its own random direction
        let init_vel = SetVelocitySphereModifier {
            center: writer.lit(config.spawn_offset).expr(),
            speed: speed_varied,
        };

        let module = writer.finish();

        EffectAsset::new(
            4096,
            SpawnerSettings::once(config.particle_count.into()),
            module,
        )
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .init(init_size)
        .update(drag)
        .update(gravity)
        .render(OrientModifier::new(OrientMode::FaceCameraPosition))
        .render(ColorOverLifetimeModifier::new(config.color_gradient.clone()))
        .render(SizeOverLifetimeModifier {
            gradient: config.size_gradient.clone(),
            screen_space_size: false,
        })
    } else {
        let init_pos = SetPositionSphereModifier {
            center: writer.lit(config.spawn_offset).expr(),
            radius: writer.lit(config.spawn_radius).expr(),
            dimension: ShapeDimension::Volume,
        };
        // Velocity spreads outward from center - each particle gets its own random direction
        let init_vel = SetVelocitySphereModifier {
            center: writer.lit(config.spawn_offset).expr(),
            speed: speed_varied,
        };

        let module = writer.finish();

        EffectAsset::new(
            4096,
            SpawnerSettings::once(config.particle_count.into()),
            module,
        )
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .init(init_size)
        .update(drag)
        .update(gravity)
        .render(OrientModifier::new(OrientMode::FaceCameraPosition))
        .render(ColorOverLifetimeModifier::new(config.color_gradient.clone()))
        .render(SizeOverLifetimeModifier {
            gradient: config.size_gradient.clone(),
            screen_space_size: false,
        })
    }
}

/// Creates a trail effect from config - public for direct attachment to entities
pub fn create_trail(config: &TrailConfig) -> EffectAsset {
    let writer = ExprWriter::new();

    // Spawn at origin with tiny spread
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(config.spawn_radius).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Minimal velocity
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(config.speed).expr(),
    };

    // Set lifetime
    let lifetime = writer.lit(config.lifetime).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Gravity modifier
    let gravity = AccelModifier::new(writer.lit(config.gravity).expr());

    let module = writer.finish();

    EffectAsset::new(
        512,
        SpawnerSettings::rate(config.spawn_rate.into()),
        module,
    )
    .init(init_pos)
    .init(init_vel)
    .init(init_lifetime)
    .update(gravity)
    .render(OrientModifier::new(OrientMode::FaceCameraPosition))
    .render(ColorOverLifetimeModifier::new(config.color_gradient.clone()))
    .render(SizeOverLifetimeModifier {
        gradient: config.size_gradient.clone(),
        screen_space_size: false,
    })
}

// ============================================================================
// SYSTEMS
// ============================================================================

/// System to handle particle spawn events - creates effects dynamically from configs
fn handle_spawn_particle_events(
    mut commands: Commands,
    mut events: EventReader<SpawnParticleEvent>,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    for event in events.read() {
        let (effect_asset, duration) = match &event.effect {
            ParticleSpawnType::Explosion(config) => {
                (create_explosion(config), config.duration())
            }
            ParticleSpawnType::Trail(config) => {
                (create_trail(config), config.duration())
            }
        };

        let effect_handle = effects.add(effect_asset);

        let scale = event.scale.unwrap_or(1.0);
        let mut transform = Transform::from_translation(event.position)
            .with_scale(Vec3::splat(scale));

        // Orient effect if direction is provided
        if let Some(dir) = event.direction {
            if dir != Vec3::ZERO {
                transform.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            }
        }

        // Create lifetime timer
        let lifetime = ParticleEffectLifetime(Timer::from_seconds(
            duration,
            TimerMode::Once,
        ));

        commands.spawn((
            ParticleEffect::new(effect_handle),
            transform,
            lifetime,
            StateScoped(GameState::Playing),
        ));
    }
}

/// System to despawn particle effects after their lifetime expires
fn despawn_finished_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ParticleEffectLifetime)>,
) {
    for (entity, mut lifetime) in query.iter_mut() {
        lifetime.0.tick(time.delta());
        if lifetime.0.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Workaround for bevy_hanabi issue #319: first spawn of each effect type doesn't emit particles.
/// This system spawns one of each preset config far below the visible area when the game starts.
/// This initializes GPU resources so subsequent spawns work correctly.
fn warmup_particle_effects(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    // Position far below the arena where particles won't be visible
    let warmup_position = Vec3::new(0.0, -1000.0, 0.0);

    // Warmup all explosion presets
    let explosion_configs = [
        ExplosionConfig::fire(),
        ExplosionConfig::smoke(),
        ExplosionConfig::frost(),
        ExplosionConfig::arcane(),
        ExplosionConfig::hit_spark(),
        ExplosionConfig::muzzle_flash(),
        ExplosionConfig::enemy_death(),
        ExplosionConfig::enemy_spawn(),
    ];

    for config in explosion_configs {
        let handle = effects.add(create_explosion(&config));
        commands.spawn((
            ParticleEffect::new(handle),
            Transform::from_translation(warmup_position),
            ParticleEffectLifetime(Timer::from_seconds(0.5, TimerMode::Once)),
            StateScoped(GameState::Playing),
        ));
    }

    // Warmup all trail presets
    let trail_configs = [
        TrailConfig::fire(),
        TrailConfig::frost(),
        TrailConfig::magic(),
    ];

    for config in trail_configs {
        let handle = effects.add(create_trail(&config));
        commands.spawn((
            ParticleEffect::new(handle),
            Transform::from_translation(warmup_position),
            ParticleEffectLifetime(Timer::from_seconds(0.5, TimerMode::Once)),
            StateScoped(GameState::Playing),
        ));
    }
}
