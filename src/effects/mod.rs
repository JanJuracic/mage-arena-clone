pub mod config;

use bevy::prelude::*;

pub use config::LightingConfig;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        // Load lighting config (falls back to defaults if file missing)
        let lighting_config = LightingConfig::load();

        app.insert_resource(lighting_config)
            .add_systems(Update, update_temporary_lights);
    }
}

#[derive(Component)]
pub struct TemporaryLight {
    pub fade_timer: Timer,
    pub initial_intensity: f32,
}

fn update_temporary_lights(
    mut commands: Commands,
    time: Res<Time>,
    mut lights: Query<(Entity, &mut TemporaryLight, &mut PointLight)>,
) {
    for (entity, mut temp_light, mut point_light) in lights.iter_mut() {
        temp_light.fade_timer.tick(time.delta());
        let progress = temp_light.fade_timer.fraction();
        point_light.intensity = temp_light.initial_intensity * (1.0 - progress);

        if temp_light.fade_timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn spawn_temporary_light(
    commands: &mut Commands,
    position: Vec3,
    color: Color,
    intensity: f32,
    range: f32,
    duration: f32,
) -> Entity {
    commands.spawn((
        PointLight {
            color,
            intensity,
            range,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(position),
        TemporaryLight {
            fade_timer: Timer::from_seconds(duration, TimerMode::Once),
            initial_intensity: intensity,
        },
    )).id()
}

// Preset spawners - use config values when available
pub fn spawn_fireball_explosion_light(commands: &mut Commands, position: Vec3, config: &LightingConfig) -> Entity {
    let data = config.explosion_light_or_default("fireball");
    spawn_temporary_light(
        commands,
        position,
        Color::srgb(data.color.0, data.color.1, data.color.2),
        data.intensity,
        data.range,
        data.duration,
    )
}

pub fn spawn_frost_explosion_light(commands: &mut Commands, position: Vec3, config: &LightingConfig) -> Entity {
    let data = config.explosion_light_or_default("frost");
    spawn_temporary_light(
        commands,
        position,
        Color::srgb(data.color.0, data.color.1, data.color.2),
        data.intensity,
        data.range,
        data.duration,
    )
}

pub fn spawn_enemy_spawn_light(commands: &mut Commands, position: Vec3, config: &LightingConfig) -> Entity {
    let data = config.explosion_light_or_default("enemy_spawn");
    spawn_temporary_light(
        commands,
        position,
        Color::srgb(data.color.0, data.color.1, data.color.2),
        data.intensity,
        data.range,
        data.duration,
    )
}
