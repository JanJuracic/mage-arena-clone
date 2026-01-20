use bevy::prelude::*;

use crate::states::GameState;
use crate::player::Player;
use crate::combat::{Health, Mana};
use crate::spells::{SpellType, SpellCooldowns, SpellDefinitions};
use crate::enemies::WaveState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_game_ui)
            .add_systems(
                Update,
                (
                    update_health_bar,
                    update_mana_bar,
                    update_cooldown_overlays,
                    update_wave_text,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// Marker components
#[derive(Component)]
struct GameUi;

#[derive(Component)]
struct HealthBar;

#[derive(Component)]
struct ManaBar;

#[derive(Component)]
struct CooldownOverlay(SpellType);

#[derive(Component)]
struct WaveText;

fn spawn_game_ui(mut commands: Commands, _spell_defs: Res<SpellDefinitions>) {
    // Root container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            GameUi,
            StateScoped(GameState::Playing),
        ))
        .with_children(|parent| {
            // Top bar (wave info)
            spawn_top_bar(parent);

            // Bottom bar (health, spells, mana)
            spawn_bottom_bar(parent);
        });

    // Crosshair (separate from main UI to be centered)
    spawn_crosshair(&mut commands);
}

fn spawn_crosshair(commands: &mut Commands) {
    // Crosshair container - centered on screen
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            StateScoped(GameState::Playing),
        ))
        .with_children(|parent| {
            // Crosshair dot
            parent.spawn((
                Node {
                    width: Val::Px(4.0),
                    height: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
                BorderRadius::all(Val::Px(2.0)),
            ));

            // Horizontal line (left)
            parent.spawn((
                Node {
                    width: Val::Px(12.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-20.0),
                        top: Val::Px(-1.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ));

            // Horizontal line (right)
            parent.spawn((
                Node {
                    width: Val::Px(12.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(8.0),
                        top: Val::Px(-1.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ));

            // Vertical line (top)
            parent.spawn((
                Node {
                    width: Val::Px(2.0),
                    height: Val::Px(12.0),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-1.0),
                        top: Val::Px(-20.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ));

            // Vertical line (bottom)
            parent.spawn((
                Node {
                    width: Val::Px(2.0),
                    height: Val::Px(12.0),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-1.0),
                        top: Val::Px(8.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ));
        });
}

fn spawn_top_bar(parent: &mut ChildBuilder) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(50.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new("Wave 1"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                WaveText,
            ));
        });
}

fn spawn_bottom_bar(parent: &mut ChildBuilder) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(80.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(10.0)),
            column_gap: Val::Px(20.0),
            ..default()
        })
        .with_children(|parent| {
            // Health bar
            spawn_resource_bar(parent, Color::srgb(0.8, 0.2, 0.2), "HP", HealthBar);

            // Spell icons
            spawn_spell_icons(parent);

            // Mana bar
            spawn_resource_bar(parent, Color::srgb(0.2, 0.4, 0.9), "MP", ManaBar);
        });
}

fn spawn_resource_bar<M: Component>(parent: &mut ChildBuilder, color: Color, label: &str, marker: M) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(5.0),
            ..default()
        })
        .with_children(|parent| {
            // Label
            parent.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));

            // Bar background
            parent
                .spawn((
                    Node {
                        width: Val::Px(150.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|parent| {
                    // Bar fill
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(color),
                        BorderRadius::all(Val::Px(3.0)),
                        marker,
                    ));
                });
        });
}

fn spawn_spell_icons(parent: &mut ChildBuilder) {
    let spells = [
        (SpellType::Fireball, "1", Color::srgb(1.0, 0.4, 0.0)),
        (SpellType::Frostbolt, "2", Color::srgb(0.4, 0.7, 1.0)),
        (SpellType::Wormhole, "3", Color::srgb(0.7, 0.3, 1.0)),
        (SpellType::MagicMissile, "4", Color::srgb(0.8, 0.3, 1.0)),
    ];

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|parent| {
            for (spell_type, key, color) in spells {
                // Icon container
                parent
                    .spawn((
                        Node {
                            width: Val::Px(50.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(color),
                        BorderRadius::all(Val::Px(5.0)),
                    ))
                    .with_children(|parent| {
                        // Cooldown overlay (darkens from bottom up)
                        parent.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(0.0),
                                position_type: PositionType::Absolute,
                                top: Val::Px(0.0),
                                left: Val::Px(0.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                            CooldownOverlay(spell_type),
                        ));

                        // Key label
                        parent.spawn((
                            Text::new(key),
                            TextFont {
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            }
        });
}

fn update_health_bar(
    player_query: Query<&Health, With<Player>>,
    mut bar_query: Query<&mut Node, With<HealthBar>>,
) {
    let Ok(health) = player_query.get_single() else {
        return;
    };
    let Ok(mut node) = bar_query.get_single_mut() else {
        return;
    };

    node.width = Val::Percent(health.fraction() * 100.0);
}

fn update_mana_bar(
    player_query: Query<&Mana, With<Player>>,
    mut bar_query: Query<&mut Node, With<ManaBar>>,
) {
    let Ok(mana) = player_query.get_single() else {
        return;
    };
    let Ok(mut node) = bar_query.get_single_mut() else {
        return;
    };

    node.width = Val::Percent(mana.fraction() * 100.0);
}

fn update_cooldown_overlays(
    player_query: Query<&SpellCooldowns, With<Player>>,
    mut overlay_query: Query<(&mut Node, &CooldownOverlay)>,
    spell_defs: Res<SpellDefinitions>,
) {
    let Ok(cooldowns) = player_query.get_single() else {
        return;
    };

    for (mut node, overlay) in overlay_query.iter_mut() {
        let max_cooldown = match overlay.0 {
            SpellType::Fireball => spell_defs.fireball.cooldown,
            SpellType::Frostbolt => spell_defs.frostbolt.cooldown,
            SpellType::Wormhole => spell_defs.wormhole.cooldown,
            SpellType::MagicMissile => spell_defs.magic_missile.cooldown,
            SpellType::EnemyBolt | SpellType::EnemyFireball | SpellType::EnemyFrostbolt => 0.0, // Not displayed in player UI
        };

        let fraction = cooldowns.get_fraction(overlay.0, max_cooldown);
        node.height = Val::Percent(fraction * 100.0);
    }
}

fn update_wave_text(
    wave_state: Res<WaveState>,
    mut text_query: Query<&mut Text, With<WaveText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    **text = format!(
        "Wave {} - Enemies: {}",
        wave_state.wave_number.saturating_sub(1).max(1),
        wave_state.enemies_remaining
    );
}
