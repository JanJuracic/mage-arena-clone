use bevy::prelude::*;
use avian3d::prelude::*;

use crate::states::GameState;
use crate::combat::{Health, Mana, Team, Hittable, SlowEffect};
use crate::spells::SpellCooldowns;
use crate::camera::{GameCamera, CameraYaw};
use crate::physics::{GroundSensorBundle, GroundState};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_player)
            .add_systems(
                Update,
                (
                    player_movement,
                    player_jump,
                    update_aim_from_camera,
                    regenerate_mana,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// Components
#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct MovementSpeed(pub f32);

#[derive(Component)]
pub struct AimDirection(pub Vec3);

#[derive(Component)]
pub struct JumpForce(pub f32);

// Systems
fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Player body - invisible in first person
    // We still need the collider for physics
    commands.spawn((
        // Visual components - make it invisible for first person
        (
            Mesh3d(meshes.add(Capsule3d::new(0.4, 0.8))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.4, 0.9, 0.0), // Transparent
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            // Spawn above terrain - higher to account for elevation
            Transform::from_xyz(0.0, 3.0, 0.0),
        ),
        // Physics components
        (
            RigidBody::Dynamic,
            // Capsule collider: radius 0.4, total height = 0.8 + 2*0.4 = 1.6
            Collider::capsule(0.4, 0.8),
            CollisionMargin(0.05),
            LockedAxes::ROTATION_LOCKED,
            // Reduce damping for more responsive movement
            LinearDamping(5.0),
            // Prevent sinking through floor
            Restitution::new(0.0),
            Friction::new(0.5),
        ),
        // Game components
        (
            Player,
            Health::new(100.0),
            Mana::new(100.0, 10.0),
            MovementSpeed(10.4),
            AimDirection(Vec3::NEG_Z),
            SpellCooldowns::new(),
            JumpForce(10.0),
        ),
        // Ground sensing
        GroundSensorBundle::player(),
        // Team and other
        (
            Team::PLAYER,
            Hittable,
            StateScoped(GameState::Playing),
            Name::new("Player"),
        ),
    ));
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&CameraYaw, With<GameCamera>>,
    mut player_query: Query<(&mut LinearVelocity, &MovementSpeed, Option<&SlowEffect>), With<Player>>,
) {
    let Ok((mut velocity, speed, slow_effect)) = player_query.get_single_mut() else {
        return;
    };
    let Ok(camera_yaw) = camera_query.get_single() else {
        return;
    };

    // Get input direction in local space
    let mut input = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        input.z -= 1.0; // Forward
    }
    if keyboard.pressed(KeyCode::KeyS) {
        input.z += 1.0; // Backward
    }
    if keyboard.pressed(KeyCode::KeyA) {
        input.x -= 1.0; // Left
    }
    if keyboard.pressed(KeyCode::KeyD) {
        input.x += 1.0; // Right
    }

    if input != Vec3::ZERO {
        input = input.normalize();
    }

    // Rotate input direction by camera yaw to get world-space movement
    let rotation = Quat::from_rotation_y(camera_yaw.0);
    let world_direction = rotation * input;

    let slow_factor = slow_effect.map_or(1.0, |s| s.factor);
    let target_velocity = world_direction * speed.0 * slow_factor;

    // Apply movement
    velocity.x = target_velocity.x;
    velocity.z = target_velocity.z;
}

fn update_aim_from_camera(
    camera_query: Query<&Transform, With<GameCamera>>,
    mut player_query: Query<&mut AimDirection, With<Player>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let Ok(mut aim_direction) = player_query.get_single_mut() else {
        return;
    };

    // Aim direction is the camera's forward direction
    aim_direction.0 = camera_transform.forward().as_vec3();
}

fn regenerate_mana(time: Res<Time>, mut query: Query<&mut Mana, With<Player>>) {
    for mut mana in query.iter_mut() {
        mana.regenerate(time.delta_secs());
    }
}

fn player_jump(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut LinearVelocity, &JumpForce, &GroundState), With<Player>>,
) {
    let Ok((mut velocity, jump_force, ground_state)) = player_query.get_single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Space) && ground_state.grounded {
        velocity.y = jump_force.0;
    }
}
