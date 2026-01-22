use bevy::prelude::*;

pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .enable_state_scoped_entities::<GameState>()
            .init_resource::<RunState>()
            .add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(Update, menu_button_system.run_if(in_state(GameState::Menu)))
            .add_systems(OnEnter(GameState::GameOver), setup_game_over)
            .add_systems(Update, game_over_input.run_if(in_state(GameState::GameOver)))
            .add_systems(OnEnter(GameState::Shop), setup_shop_ui)
            .add_systems(Update, shop_button_system.run_if(in_state(GameState::Shop)));
    }
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
    Shop,      // Between combat phases
    Paused,
    GameOver,
}

/// Tracks the current run progress (resets on death)
#[derive(Resource, Default)]
pub struct RunState {
    pub current_level: u32,
}

impl RunState {
    pub fn reset(&mut self) {
        self.current_level = 1;
    }

    pub fn advance_level(&mut self) {
        self.current_level += 1;
    }
}

#[derive(Component)]
struct MenuUi;

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct GameOverUi;

fn setup_menu(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        StateScoped(GameState::Menu),
    ));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
            StateScoped(GameState::Menu),
            MenuUi,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("MAGE ARENA"),
                TextFont {
                    font_size: 72.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.6, 1.0)),
            ));

            // Subtitle
            parent.spawn((
                Text::new("Cast spells. Defeat enemies. Survive."),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
            ));

            // Play button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(60.0),
                        margin: UiRect::top(Val::Px(50.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.5, 0.3)),
                    BorderColor(Color::srgb(0.4, 0.7, 0.4)),
                    PlayButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("PLAY"),
                        TextFont {
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Controls info
            parent.spawn((
                Text::new("Controls: WASD to move, Mouse to aim, 1-4 to cast spells"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                Node {
                    margin: UiRect::top(Val::Px(100.0)),
                    ..default()
                },
            ));
        });
}

fn menu_button_system(
    mut next_state: ResMut<NextState<GameState>>,
    mut run_state: ResMut<RunState>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<PlayButton>),
    >,
) {
    for (interaction, mut bg_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                // Reset run state for a fresh start
                run_state.reset();
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.4, 0.6, 0.4));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.3, 0.5, 0.3));
            }
        }
    }
}

fn setup_game_over(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        StateScoped(GameState::GameOver),
    ));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.0, 0.0, 0.9)),
            StateScoped(GameState::GameOver),
            GameOverUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("GAME OVER"),
                TextFont {
                    font_size: 72.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.2, 0.2)),
            ));

            parent.spawn((
                Text::new("Press SPACE to return to menu"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(30.0)),
                    ..default()
                },
            ));
        });
}

fn game_over_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Menu);
    }
}

// Shop UI components
#[derive(Component)]
struct ShopUi;

#[derive(Component)]
struct ContinueButton;

fn setup_shop_ui(mut commands: Commands, run_state: Res<RunState>) {
    commands.spawn((
        Camera2d,
        StateScoped(GameState::Shop),
    ));

    let current_level = run_state.current_level;
    let next_level = current_level + 1;

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.95)),
            StateScoped(GameState::Shop),
            ShopUi,
        ))
        .with_children(|parent| {
            // Level complete title
            parent.spawn((
                Text::new(format!("Level {} Complete!", current_level)),
                TextFont {
                    font_size: 56.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.8, 0.4)),
            ));

            // Subtitle
            parent.spawn((
                Text::new("Prepare for the next challenge"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
            ));

            // Continue button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Px(60.0),
                        margin: UiRect::top(Val::Px(50.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.5, 0.3)),
                    BorderColor(Color::srgb(0.4, 0.7, 0.4)),
                    ContinueButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("Continue to Level {}", next_level)),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Info text
            parent.spawn((
                Text::new("Health and mana will be fully restored"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                Node {
                    margin: UiRect::top(Val::Px(30.0)),
                    ..default()
                },
            ));
        });
}

fn shop_button_system(
    mut next_state: ResMut<NextState<GameState>>,
    mut run_state: ResMut<RunState>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ContinueButton>),
    >,
) {
    for (interaction, mut bg_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                run_state.advance_level();
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.4, 0.6, 0.4));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.3, 0.5, 0.3));
            }
        }
    }
}
