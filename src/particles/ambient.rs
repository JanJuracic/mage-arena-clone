use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::arena::ArenaConfig;
use crate::player::Player;
use crate::states::GameState;

pub struct AmbientParticlePlugin;

impl Plugin for AmbientParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_ambient_particles)
            .add_systems(
                Update,
                update_ambient_particle_position.run_if(in_state(GameState::Playing)),
            );
    }
}

/// Marker component for the ambient particle emitter
#[derive(Component)]
pub struct AmbientParticleEmitter;

/// Configuration for ambient particles based on atmosphere
#[derive(Clone)]
pub struct AmbientParticleConfig {
    pub color: Vec4,
    pub spawn_rate: f32,
    pub lifetime: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub spawn_radius: f32,
    pub spawn_height: f32,
    pub drift_speed: f32,
}

impl AmbientParticleConfig {
    /// Create config based on atmosphere name
    pub fn from_atmosphere(atmosphere_name: &str) -> Self {
        match atmosphere_name {
            "Dawn" => Self {
                color: Vec4::new(1.0, 0.95, 0.7, 0.4), // Golden pollen
                spawn_rate: 8.0,
                lifetime: 12.0,
                size_start: 0.02,
                size_end: 0.04,
                spawn_radius: 15.0,
                spawn_height: 8.0,
                drift_speed: 0.3,
            },
            "Noon" => Self {
                color: Vec4::new(1.0, 1.0, 1.0, 0.35), // White dust motes
                spawn_rate: 6.0,
                lifetime: 10.0,
                size_start: 0.015,
                size_end: 0.035,
                spawn_radius: 18.0,
                spawn_height: 10.0,
                drift_speed: 0.2,
            },
            "Dusk" => Self {
                color: Vec4::new(1.0, 0.85, 0.6, 0.45), // Warm amber particles
                spawn_rate: 10.0,
                lifetime: 14.0,
                size_start: 0.025,
                size_end: 0.045,
                spawn_radius: 14.0,
                spawn_height: 7.0,
                drift_speed: 0.25,
            },
            "Night" => Self {
                color: Vec4::new(0.7, 0.8, 1.0, 0.2), // Dim, sparse particles
                spawn_rate: 3.0,
                lifetime: 15.0,
                size_start: 0.01,
                size_end: 0.025,
                spawn_radius: 20.0,
                spawn_height: 12.0,
                drift_speed: 0.15,
            },
            _ => Self::default(),
        }
    }
}

impl Default for AmbientParticleConfig {
    fn default() -> Self {
        Self {
            color: Vec4::new(1.0, 1.0, 1.0, 0.3),
            spawn_rate: 6.0,
            lifetime: 10.0,
            size_start: 0.02,
            size_end: 0.04,
            spawn_radius: 15.0,
            spawn_height: 8.0,
            drift_speed: 0.2,
        }
    }
}

/// Create the ambient particle effect asset
fn create_ambient_effect(config: &AmbientParticleConfig) -> EffectAsset {
    let writer = ExprWriter::new();

    // Spawn in a large cylinder around the emitter
    let init_pos = SetPositionCone3dModifier {
        base_radius: writer.lit(config.spawn_radius).expr(),
        top_radius: writer.lit(config.spawn_radius * 0.8).expr(),
        height: writer.lit(config.spawn_height).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Slow random velocity with slight upward drift
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::new(0.0, config.drift_speed * 0.5, 0.0)).expr(),
        speed: writer.lit(config.drift_speed).expr(),
    };

    // Set lifetime
    let lifetime = writer.lit(config.lifetime).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Gentle gravity/buoyancy for floating effect
    let gravity = AccelModifier::new(writer.lit(Vec3::new(0.0, 0.05, 0.0)).expr());

    // Color gradient - fade in and out
    let mut color_gradient = Gradient::new();
    let base_color = config.color;
    color_gradient.add_key(0.0, Vec4::new(base_color.x, base_color.y, base_color.z, 0.0));
    color_gradient.add_key(0.15, base_color);
    color_gradient.add_key(0.85, base_color);
    color_gradient.add_key(1.0, Vec4::new(base_color.x, base_color.y, base_color.z, 0.0));

    // Size gradient - grow slightly over lifetime
    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(config.size_start));
    size_gradient.add_key(0.5, Vec3::splat(config.size_end));
    size_gradient.add_key(1.0, Vec3::splat(config.size_start * 0.5));

    let module = writer.finish();

    EffectAsset::new(
        512, // Max particles
        SpawnerSettings::rate(config.spawn_rate.into()),
        module,
    )
    .init(init_pos)
    .init(init_vel)
    .init(init_lifetime)
    .update(gravity)
    .render(OrientModifier::new(OrientMode::FaceCameraPosition))
    .render(ColorOverLifetimeModifier::new(color_gradient))
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient,
        screen_space_size: false,
    })
}

/// Spawn the ambient particle emitter
fn spawn_ambient_particles(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    arena_config: Res<ArenaConfig>,
) {
    let config = AmbientParticleConfig::from_atmosphere(&arena_config.atmosphere_preset_name);
    let effect = create_ambient_effect(&config);
    let effect_handle = effects.add(effect);

    // Spawn at origin - will be moved to follow player
    commands.spawn((
        ParticleEffect::new(effect_handle),
        Transform::from_xyz(0.0, 0.0, 0.0),
        AmbientParticleEmitter,
        StateScoped(GameState::Playing),
        Name::new("Ambient Particles"),
    ));

    info!(
        "Spawned ambient particles for '{}' atmosphere",
        arena_config.atmosphere_preset_name
    );
}

/// Update the ambient particle emitter position to follow the player
fn update_ambient_particle_position(
    player_query: Query<&Transform, (With<Player>, Without<AmbientParticleEmitter>)>,
    mut emitter_query: Query<&mut Transform, With<AmbientParticleEmitter>>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let Ok(mut emitter_transform) = emitter_query.get_single_mut() else {
        return;
    };

    // Center the particle volume around the player, slightly above ground
    emitter_transform.translation = player_transform.translation + Vec3::new(0.0, 2.0, 0.0);
}
