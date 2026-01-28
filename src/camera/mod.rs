mod outline;

use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::post_process::ChromaticAberration;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;

use crate::arena::ArenaConfig;
use crate::states::GameState;
use crate::player::Player;

pub use outline::{OutlinePostProcessPlugin, OutlineSettings};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OutlinePostProcessPlugin)
            .add_event::<ScreenShakeEvent>()
            .add_systems(OnEnter(GameState::Playing), (spawn_camera, grab_cursor))
            .add_systems(OnExit(GameState::Playing), release_cursor)
            .add_systems(OnEnter(GameState::Menu), release_cursor)
            .add_systems(
                Update,
                (mouse_look, handle_screen_shake, apply_screen_shake, camera_follow)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Event to trigger screen shake
#[derive(Event)]
pub struct ScreenShakeEvent {
    pub intensity: f32,  // Base intensity (0.0 to 1.0+)
    pub duration: f32,   // Duration in seconds
}

impl ScreenShakeEvent {
    pub fn new(intensity: f32, duration: f32) -> Self {
        Self { intensity, duration }
    }
}

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
pub struct CameraSensitivity(pub Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        // Horizontal sensitivity typically faster than vertical
        Self(Vec2::new(0.003, 0.002))
    }
}

#[derive(Component)]
pub struct CameraYaw(pub f32);

#[derive(Component)]
pub struct CameraPitch(pub f32);

/// Component to track active screen shake
#[derive(Component)]
pub struct ScreenShake {
    pub intensity: f32,
    pub timer: Timer,
    pub offset: Vec3,
}

fn spawn_camera(mut commands: Commands, arena_config: Res<ArenaConfig>) {
    // Create atmosphere-aware fog from config
    let fog = DistanceFog {
        color: Color::srgb(
            arena_config.fog_color[0],
            arena_config.fog_color[1],
            arena_config.fog_color[2],
        ),
        falloff: bevy::pbr::FogFalloff::Linear {
            start: arena_config.fog_start,
            end: arena_config.fog_end,
        },
        // Tint fog with sun color for atmospheric scattering effect
        directional_light_color: Color::srgb(
            arena_config.sun_color[0],
            arena_config.sun_color[1],
            arena_config.sun_color[2],
        ),
        directional_light_exponent: 8.0,
    };

    // First-person camera at eye level
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true, // Required for bloom
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 70.0_f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(0.0, 1.8, 0.0), // Eye height
        Tonemapping::AcesFitted, // Cinematic, saturated
        Bloom {
            intensity: 0.25,
            ..Bloom::OLD_SCHOOL // Additive, punchy
        },
        ChromaticAberration {
            intensity: 0.02, // Slight RGB fringing
            ..default()
        },
        fog, // Atmospheric fog
        OutlineSettings::subtle(), // Subtle depth-based outlines
        GameCamera,
        CameraSensitivity::default(),
        CameraYaw(0.0),
        CameraPitch(0.0),
        StateScoped(GameState::Playing),
        Name::new("FPS Camera"),
    ));
}

fn grab_cursor(mut windows: Query<&mut Window>) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

fn release_cursor(mut windows: Query<&mut Window>) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}

fn mouse_look(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera_query: Query<
        (&mut Transform, &CameraSensitivity, &mut CameraYaw, &mut CameraPitch),
        With<GameCamera>,
    >,
) {
    let Ok((mut transform, sensitivity, mut yaw, mut pitch)) = camera_query.get_single_mut() else {
        return;
    };

    let delta = accumulated_mouse_motion.delta;
    if delta == Vec2::ZERO {
        return;
    }

    // Apply sensitivity - note: no delta_time multiplication needed for mouse input
    // as AccumulatedMouseMotion already contains the full frame's movement
    let delta_yaw = -delta.x * sensitivity.0.x;
    let delta_pitch = -delta.y * sensitivity.0.y;

    // Update yaw (horizontal rotation)
    yaw.0 += delta_yaw;

    // Update pitch (vertical rotation) with clamping to prevent gimbal lock
    let pitch_limit = std::f32::consts::FRAC_PI_2 - 0.01;
    pitch.0 = (pitch.0 + delta_pitch).clamp(-pitch_limit, pitch_limit);

    // Apply rotation using Euler angles (YXZ order for FPS camera)
    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw.0, pitch.0, 0.0);
}

fn camera_follow(
    player_query: Query<&Transform, (With<Player>, Without<GameCamera>)>,
    mut camera_query: Query<(&mut Transform, Option<&ScreenShake>), With<GameCamera>>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let Ok((mut camera_transform, screen_shake)) = camera_query.get_single_mut() else {
        return;
    };

    // Position camera at player's eye level
    let eye_height = 1.8;
    let base_pos = player_transform.translation + Vec3::Y * eye_height;

    // Apply shake offset if active
    let shake_offset = screen_shake.map_or(Vec3::ZERO, |s| s.offset);
    camera_transform.translation = base_pos + shake_offset;
}

fn handle_screen_shake(
    mut commands: Commands,
    mut events: EventReader<ScreenShakeEvent>,
    mut camera_query: Query<(Entity, Option<&mut ScreenShake>), With<GameCamera>>,
) {
    for event in events.read() {
        let Ok((entity, existing_shake)) = camera_query.get_single_mut() else {
            continue;
        };

        // Add or update screen shake component
        if let Some(mut shake) = existing_shake {
            // Stack shakes - add intensity and reset timer if new shake is stronger
            if event.intensity > shake.intensity * shake.timer.fraction_remaining() {
                shake.intensity = event.intensity;
                shake.timer = Timer::from_seconds(event.duration, TimerMode::Once);
            }
        } else {
            commands.entity(entity).insert(ScreenShake {
                intensity: event.intensity,
                timer: Timer::from_seconds(event.duration, TimerMode::Once),
                offset: Vec3::ZERO,
            });
        }
    }
}

fn apply_screen_shake(
    mut commands: Commands,
    time: Res<Time>,
    mut camera_query: Query<(Entity, &mut ScreenShake), With<GameCamera>>,
) {
    for (entity, mut shake) in camera_query.iter_mut() {
        shake.timer.tick(time.delta());

        if shake.timer.finished() {
            // Remove shake component when done
            commands.entity(entity).remove::<ScreenShake>();
        } else {
            // Calculate shake intensity that decays over time
            let decay = shake.timer.fraction_remaining();
            let current_intensity = shake.intensity * decay;

            // Generate random offset using time-based noise
            let time_seed = time.elapsed_secs() * 50.0;
            let offset_x = (time_seed.sin() * 1.7 + (time_seed * 2.3).cos()) * current_intensity;
            let offset_y = ((time_seed * 1.3).cos() * 1.5 + (time_seed * 1.9).sin()) * current_intensity;
            let offset_z = ((time_seed * 0.9).sin() + (time_seed * 2.7).cos() * 0.8) * current_intensity * 0.5;

            shake.offset = Vec3::new(offset_x, offset_y, offset_z);
        }
    }
}
