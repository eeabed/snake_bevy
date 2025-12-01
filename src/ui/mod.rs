//! UI plugin - handles menus, game over screen, score display, and game flow.

use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Hdr;
use std::time::Duration;

use bevy_vector_shapes::prelude::*;

use crate::food::spawn_food;
use crate::game::{
    ARENA_BORDER_COLOR, ARENA_COLOR, ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, Food, GameOverUI,
    GamePhase, GameState, INITIAL_SNAKE_POSITION, InputBuffer, MenuUI, MoveTimer, ScoreText,
    SnakeHead, SnakeSegment,
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
                spawn_game_over_screen_system,
            )
                .chain(),
        );
    }
}

// Type alias for querying snake entities
type SnakeEntityQuery<'w, 's> = Query<'w, 's, Entity, Or<(With<SnakeSegment>, With<SnakeHead>)>>;

/// Initial setup system - camera, arena, score text.
fn setup_system(
    mut commands: Commands,
    game_state: ResMut<GameState>,
    asset_server: Res<AssetServer>,
) {
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
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    ));

    // Glowing arena border using hollow rectangle
    let arena_width = ARENA_WIDTH as f32 * CELL_SIZE;
    let arena_height = ARENA_HEIGHT as f32 * CELL_SIZE;
    commands.spawn(ShapeBundle::rect(
        &ShapeConfig {
            color: ARENA_BORDER_COLOR,
            alpha_mode: ShapeAlphaMode::Add,
            hollow: true,
            thickness: 4.0,
            corner_radii: Vec4::splat(0.02),
            transform: Transform::from_xyz(0.0, 0.0, 0.1),
            ..ShapeConfig::default_2d()
        },
        Vec2::new(arena_width + 4.0, arena_height + 4.0),
    ));

    // Score text (initially hidden until game starts)
    commands.spawn((
        Text::from("Score: 0"),
        TextFont {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 20.0,
            font_smoothing: Default::default(),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        ScoreText,
    ));

    // Show start menu if we're in the Menu phase
    if game_state.phase == GamePhase::Menu {
        spawn_start_menu(&mut commands, &asset_server);
    }
}

/// Spawns the start menu UI.
fn spawn_start_menu(commands: &mut Commands, asset_server: &Res<AssetServer>) {
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
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 80.0,
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
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 24.0,
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
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 18.0,
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
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 18.0,
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
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Start instructions
            parent.spawn((
                Text::from("Press SPACE to start"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 0.3, 1.0)),
            ));
        });
}

/// Spawns the game over screen UI.
fn spawn_game_over_screen(commands: &mut Commands, asset_server: &Res<AssetServer>, score: usize) {
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            GameOverUI,
        ))
        .with_children(|parent| {
            // "GAME OVER" text
            parent.spawn((
                Text::from("GAME OVER"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 60.0,
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
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
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
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 20.0,
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
    asset_server: Res<AssetServer>,
    game_over_ui: Query<Entity, With<GameOverUI>>,
) {
    // Only spawn if game just ended and no UI exists yet
    if game_state.is_changed() && game_state.phase == GamePhase::GameOver && game_over_ui.is_empty()
    {
        spawn_game_over_screen(&mut commands, &asset_server, game_state.score);
    }
}

/// System to start the game from the menu.
fn start_game_from_menu(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut move_timer: ResMut<MoveTimer>,
    menu_ui: Query<Entity, With<MenuUI>>,
) {
    if game_state.phase == GamePhase::Menu && keyboard_input.just_pressed(KeyCode::Space) {
        // Despawn menu UI
        for entity in menu_ui.iter() {
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
        }

        // Initialize game state
        game_state.snake_segments.clear();
        game_state.score = 0;
        game_state.game_over = false;
        game_state.phase = GamePhase::Playing;

        // Reset move timer
        move_timer.elapsed = Duration::ZERO;

        // Spawn initial snake
        let head_entity = spawn_snake_head(&mut commands);
        game_state.snake_segments.push(head_entity);

        // Spawn initial food
        spawn_food(&mut commands, &[INITIAL_SNAKE_POSITION]);
    }
}

/// System to restart the game from game over screen.
#[allow(clippy::too_many_arguments)]
fn restart_game(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut input_buffer: ResMut<InputBuffer>,
    mut move_timer: ResMut<MoveTimer>,
    segments: SnakeEntityQuery,
    food: Query<Entity, With<Food>>,
    game_over_ui: Query<Entity, With<GameOverUI>>,
) {
    if game_state.phase == GamePhase::GameOver && keyboard_input.just_pressed(KeyCode::Space) {
        // Despawn all existing snake segments and food
        for entity in segments.iter().chain(food.iter()) {
            commands.entity(entity).despawn();
        }

        // Despawn game over UI
        for entity in game_over_ui.iter() {
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
        }

        // Reset game state
        game_state.snake_segments.clear();
        game_state.score = 0;
        game_state.game_over = false;
        game_state.phase = GamePhase::Playing;

        // Clear input buffer and reset move timer
        input_buffer.clear();
        move_timer.elapsed = Duration::ZERO;

        // Spawn new snake head
        let head_entity = spawn_snake_head(&mut commands);
        game_state.snake_segments.push(head_entity);

        // Spawn new food
        spawn_food(&mut commands, &[INITIAL_SNAKE_POSITION]);
    }
}

/// System to update the score display.
fn update_score_text(game_state: Res<GameState>, mut query: Query<&mut Text, With<ScoreText>>) {
    if let Ok(mut text) = query.single_mut() {
        *text = Text::from(format!("Score: {}", game_state.score));
    }
}
