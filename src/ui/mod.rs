//! UI plugin - handles menus, game over screen, score display, and game flow.

use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Hdr;
use bevy::text::FontWeight;

use bevy_vector_shapes::prelude::*;

use crate::food::spawn_food;
use crate::game::{
    ARENA_BORDER_COLOR, ARENA_COLOR, ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, CameraShake, Food,
    GameOverUI, GamePhase, GameSet, GameState, INITIAL_SNAKE_POSITION, InputBuffer, MenuUI,
    PulseEffect, ScoreText, SnakeHead, SnakeSegment, WinUI, Z_BACKGROUND,
};
use crate::snake::spawn_snake_head;

/// Plugin for UI and game flow systems.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_system).add_systems(
            Update,
            (
                start_game_from_menu,
                restart_game,
                update_score_text,
                update_score_visibility,
                spawn_game_over_screen_system,
                spawn_win_screen_system,
            )
                .chain()
                .in_set(GameSet::Ui),
        );
    }
}

// Type alias for querying snake entities
type SnakeEntityQuery<'w, 's> = Query<'w, 's, Entity, Or<(With<SnakeSegment>, With<SnakeHead>)>>;

/// Initial setup system - camera, arena, score text, start menu.
///
/// Runs once at app boot when `GameState::default()` is in `Menu` phase, so
/// the start menu can be spawned unconditionally.
fn setup_system(mut commands: Commands) {
    // Setup camera with HDR and bloom for glowing effects
    commands.spawn((
        Camera2d,
        Hdr,
        Bloom {
            intensity: 0.3,
            low_frequency_boost: 0.6,
            low_frequency_boost_curvature: 0.5,
            high_pass_frequency: 0.8,
            ..default()
        },
    ));

    // Arena background
    commands.spawn((
        Sprite {
            color: ARENA_COLOR,
            custom_size: Some(Vec2::new(
                ARENA_WIDTH as f32 * CELL_SIZE,
                ARENA_HEIGHT as f32 * CELL_SIZE,
            )),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, Z_BACKGROUND)),
    ));

    // Glowing arena border using hollow rectangle
    let arena_width = ARENA_WIDTH as f32 * CELL_SIZE;
    let arena_height = ARENA_HEIGHT as f32 * CELL_SIZE;
    commands.spawn(ShapeBundle::rect(
        &ShapeConfig {
            color: ARENA_BORDER_COLOR,
            alpha_mode: ShapeAlphaMode::Add,
            hollow: true,
            thickness: 2.0,
            corner_radii: Vec4::splat(0.02),
            transform: Transform::from_xyz(0.0, 0.0, 0.1),
            ..ShapeConfig::default_2d()
        },
        Vec2::new(arena_width + 4.0, arena_height + 4.0),
    ));

    // Score text — hidden at boot (Menu phase) and toggled by
    // `update_score_visibility` based on the current `GamePhase`.
    commands.spawn((
        Text::from("Score: 0"),
        TextFont {
            font_size: 20.0,
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        Visibility::Hidden,
        ScoreText,
    ));

    // Show start menu (we're always in `Menu` phase at Startup).
    spawn_start_menu(&mut commands);
}

/// Spawns the start menu UI.
fn spawn_start_menu(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            MenuUI,
        ))
        .with_children(|parent| {
            // "SNAKE" title
            parent.spawn((
                Text::from("SNAKE"),
                TextFont {
                    font_size: 80.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(0.3, 1.0, 0.3, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Controls section
            parent.spawn((
                Text::from("CONTROLS"),
                TextFont {
                    font_size: 24.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(15.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::from("Arrow Keys or WASD to move"),
                TextFont {
                    font_size: 18.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::from("Eat the red apples to grow"),
                TextFont {
                    font_size: 18.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::from("Don't run into yourself!"),
                TextFont {
                    font_size: 18.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Start instructions — same hue as the title, slightly dimmer so
            // the title stays the dominant element on the menu.
            parent.spawn((
                Text::from("Press SPACE to start"),
                TextFont {
                    font_size: 24.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(0.4, 0.85, 0.4, 1.0)),
            ));
        });
}

/// Spawns the game over screen UI.
fn spawn_game_over_screen(commands: &mut Commands, score: usize) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // Dim scrim — high enough alpha to make the overlay text dominant,
            // but still translucent so the player can see where they died.
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.82)),
            GameOverUI,
        ))
        .with_children(|parent| {
            // "GAME OVER" text
            parent.spawn((
                Text::from("GAME OVER"),
                TextFont {
                    font_size: 60.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 0.3, 0.3, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Final score text
            parent.spawn((
                Text::from(format!("Final Score: {}", score)),
                TextFont {
                    font_size: 30.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ));

            // Restart instructions
            parent.spawn((
                Text::from("Press SPACE to restart"),
                TextFont {
                    font_size: 20.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            ));
        });
}

/// System to spawn game over screen when game ends.
fn spawn_game_over_screen_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    game_over_ui: Query<Entity, With<GameOverUI>>,
) {
    // Only spawn if game just ended and no UI exists yet
    if game_state.is_changed() && game_state.phase == GamePhase::GameOver && game_over_ui.is_empty()
    {
        spawn_game_over_screen(&mut commands, game_state.score);
    }
}

/// Spawns the win-screen UI.
fn spawn_win_screen(commands: &mut Commands, score: usize) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.82)),
            WinUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::from("YOU WIN!"),
                TextFont {
                    font_size: 60.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(0.3, 1.0, 0.3, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::from(format!("Final Score: {}", score)),
                TextFont {
                    font_size: 30.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::from("Press SPACE to play again"),
                TextFont {
                    font_size: 20.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            ));
        });
}

/// System to spawn the win screen when the player fills the arena.
fn spawn_win_screen_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    win_ui: Query<Entity, With<WinUI>>,
) {
    if game_state.is_changed() && game_state.phase == GamePhase::Won && win_ui.is_empty() {
        spawn_win_screen(&mut commands, game_state.score);
    }
}

/// Resets all shared game state, deterministically clears any leftover camera
/// shake, and spawns a fresh snake head and food.
///
/// Called by both `start_game_from_menu` and `restart_game`.
fn begin_new_game(
    commands: &mut Commands,
    game_state: &mut GameState,
    camera_shake: &mut CameraShake,
) {
    game_state.snake_segments.clear();
    game_state.score = 0;
    game_state.phase = GamePhase::Playing;

    // Cancel any leftover camera shake so the new game doesn't start mid-shake.
    camera_shake.timer = Timer::from_seconds(0.0, TimerMode::Once);
    camera_shake.intensity = 0.0;

    let head_entity = spawn_snake_head(commands);
    game_state.snake_segments.push(head_entity);
    spawn_food(commands, &[INITIAL_SNAKE_POSITION]);
}

/// System to start the game from the menu.
fn start_game_from_menu(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut camera_shake: ResMut<CameraShake>,
    menu_ui: Query<Entity, With<MenuUI>>,
) {
    if game_state.phase == GamePhase::Menu && keyboard_input.just_pressed(KeyCode::Space) {
        for entity in menu_ui.iter() {
            commands.entity(entity).despawn();
        }
        begin_new_game(&mut commands, &mut game_state, &mut camera_shake);
    }
}

/// System to restart the game from the game-over or win screen.
#[allow(clippy::too_many_arguments)]
fn restart_game(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut input_buffer: ResMut<InputBuffer>,
    mut camera_shake: ResMut<CameraShake>,
    segments: SnakeEntityQuery,
    food: Query<Entity, With<Food>>,
    pulse_effects: Query<Entity, With<PulseEffect>>,
    game_over_ui: Query<Entity, With<GameOverUI>>,
    win_ui: Query<Entity, With<WinUI>>,
) {
    let restartable = matches!(game_state.phase, GamePhase::GameOver | GamePhase::Won);
    if !(restartable && keyboard_input.just_pressed(KeyCode::Space)) {
        return;
    }

    // Despawn snake, food, and any in-flight food-eaten flash effects so the
    // new game starts with a visually clean arena.
    for entity in segments
        .iter()
        .chain(food.iter())
        .chain(pulse_effects.iter())
    {
        commands.entity(entity).despawn();
    }
    // Despawn whichever end-screen overlay is currently visible.
    for entity in game_over_ui.iter().chain(win_ui.iter()) {
        commands.entity(entity).despawn();
    }
    input_buffer.clear();
    begin_new_game(&mut commands, &mut game_state, &mut camera_shake);
}

/// System to update the score display.
///
/// Caches the last-rendered score in a `Local` and skips both the format and
/// the component write when the score hasn't changed. This avoids the spurious
/// re-render that `is_changed()` alone would trigger on every `GameState`
/// mutation (phase changes, segment-vec updates, etc.) regardless of whether
/// the score actually changed.
fn update_score_text(
    game_state: Res<GameState>,
    mut last_score: Local<Option<usize>>,
    mut query: Query<&mut Text, With<ScoreText>>,
) {
    if *last_score == Some(game_state.score) {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    *text = Text::from(format!("Score: {}", game_state.score));
    *last_score = Some(game_state.score);
}

/// Shows the score HUD only during `GamePhase::Playing`, hides it on the
/// menu, game-over, and win screens. Tracks the previous phase in a `Local`
/// so we only mutate `Visibility` on transitions.
fn update_score_visibility(
    game_state: Res<GameState>,
    mut last_phase: Local<Option<GamePhase>>,
    mut score: Query<&mut Visibility, With<ScoreText>>,
) {
    if *last_phase == Some(game_state.phase) {
        return;
    }
    let Ok(mut visibility) = score.single_mut() else {
        return;
    };
    *visibility = if game_state.phase == GamePhase::Playing {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    *last_phase = Some(game_state.phase);
}
