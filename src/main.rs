use bevy::prelude::*;
use avian3d::prelude::*;

mod states;
mod player;
mod arena;
mod camera;
mod spells;
mod combat;
mod enemies;
mod ui;
mod particles;
mod physics;

use states::StatesPlugin;
use player::PlayerPlugin;
use arena::ArenaPlugin;
use camera::CameraPlugin;
use spells::SpellPlugin;
use combat::CombatPlugin;
use enemies::EnemyPlugin;
use ui::UiPlugin;
use particles::ParticlePlugin;
use physics::PhysicsUtilPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Mage Arena".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        // Configure gravity (default is -9.81 on Y axis)
        .insert_resource(Gravity(Vec3::new(0.0, -19.6, 0.0))) // Slightly stronger for snappier feel
        .add_plugins((
            StatesPlugin,
            PlayerPlugin,
            ArenaPlugin,
            CameraPlugin,
            SpellPlugin,
            CombatPlugin,
            EnemyPlugin,
            UiPlugin,
            ParticlePlugin,
            PhysicsUtilPlugin,
        ))
        .run();
}
