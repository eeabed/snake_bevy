//! UI plugin - handles menus, game over screen, score display, and game flow.
//!
//! Screens are declared with Bevy's BSN scene notation (`bsn!`): each screen
//! is a plain function returning `impl Scene`, composed from the `overlay`
//! and `label` building blocks below and spawned via `Commands::spawn_scene`.

use bevy::camera::Hdr;
use bevy::picking::hover::Hovered;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::settings::SaveSettings;
use bevy::text::FontWeight;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, Button as WidgetButton};

use bevy_vector_shapes::prelude::*;

use crate::food::spawn_food;
use crate::game::{
    ARENA_BORDER_COLOR, ARENA_COLOR, ARENA_COLOR_ALT, ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE,
    CameraShake, Food, GameOverUI, GamePhase, GameSet, GameState, HighScore,
    INITIAL_SNAKE_POSITION, InputBuffer, MenuUI, Particle, PulseEffect, ScorePopup, ScoreText,
    SnakeHead, SnakeSegment, StartRequested, WinUI, Z_BACKGROUND,
};
use crate::snake::spawn_snake_head;

// Shared UI palette.
const TITLE_GREEN: Color = Color::srgba(0.3, 1.0, 0.3, 1.0);
const HINT_GRAY: Color = Color::srgba(0.8, 0.8, 0.8, 1.0);
const GAME_OVER_RED: Color = Color::srgba(1.0, 0.3, 0.3, 1.0);
// Start instructions — same hue as the title, slightly dimmer so the title
// stays the dominant element on the menu.
const START_GREEN: Color = Color::srgba(0.4, 0.85, 0.4, 1.0);
const MENU_GOLD: Color = Color::srgba(0.9, 0.8, 0.3, 1.0);
const RECORD_GOLD: Color = Color::srgba(1.0, 0.85, 0.3, 1.0);
const BEST_GRAY: Color = Color::srgba(0.6, 0.6, 0.6, 1.0);
// Action-button backgrounds: same green family as the snake, stepping
// brighter on hover and brighter again while pressed.
const BUTTON_BG: Color = Color::srgba(0.10, 0.32, 0.10, 1.0);
const BUTTON_BG_HOVERED: Color = Color::srgba(0.14, 0.45, 0.14, 1.0);
const BUTTON_BG_PRESSED: Color = Color::srgba(0.18, 0.58, 0.18, 1.0);

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
                // Must run after the two spawn systems: the end screens
                // compare the final score against the *previous* record to
                // decide whether to show "NEW HIGH SCORE!".
                update_high_score,
                // Cosmetic hover/pressed feedback — no ordering constraints,
                // it just lives at the end of the UI chain.
                button_feedback,
            )
                .chain()
                .in_set(GameSet::Ui),
        );
    }
}

// Type alias for querying snake entities
type SnakeEntityQuery<'w, 's> = Query<'w, 's, Entity, Or<(With<SnakeSegment>, With<SnakeHead>)>>;
// Every kind of transient eating effect, for restart cleanup.
type EffectEntityQuery<'w, 's> =
    Query<'w, 's, Entity, Or<(With<PulseEffect>, With<Particle>, With<ScorePopup>)>>;

/// Initial setup system - camera, arena, score text, start menu.
///
/// Runs once at app boot when `GameState::default()` is in `Menu` phase, so
/// the start menu can be spawned unconditionally.
fn setup_system(mut commands: Commands, high_score: Res<HighScore>) {
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

    // Subtle checkerboard over the arena so movement reads against the grid.
    for x in 0..ARENA_WIDTH as i32 {
        for y in 0..ARENA_HEIGHT as i32 {
            if (x + y) % 2 == 0 {
                continue;
            }
            commands.spawn((
                Sprite {
                    color: ARENA_COLOR_ALT,
                    custom_size: Some(Vec2::splat(CELL_SIZE)),
                    ..default()
                },
                Transform::from_xyz(
                    (x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE,
                    (y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE,
                    Z_BACKGROUND + 0.05,
                ),
            ));
        }
    }

    // Glowing arena border using hollow rectangle. Stays a plain spawn:
    // `ShapeBundle` is a bundle, not a component, so it can't appear in `bsn!`.
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

    commands.spawn_scene(score_hud());

    // Show start menu (we're always in `Menu` phase at Startup).
    commands.spawn_scene(start_menu(high_score.score));
}

/// The score HUD — hidden at boot (Menu phase) and toggled by
/// `update_score_visibility` based on the current `GamePhase`.
fn score_hud() -> impl Scene {
    bsn! {
        ScoreText
        Text("Score: 0")
        TextFont {
            font_size: { FontSize::Px(20.0) },
            weight: FontWeight::BOLD,
        }
        TextColor(Color::WHITE)
        Node {
            position_type: PositionType::Absolute,
            top: px(10),
            left: px(10),
        }
        Visibility::Hidden
    }
}

/// Full-screen centered column over a translucent black scrim — the shared
/// scaffold of the start menu and both end screens.
fn overlay<L: SceneList>(scrim_alpha: f32, content: L) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
        }
        BackgroundColor({ Color::srgba(0.0, 0.0, 0.0, scrim_alpha) })
        Children [{ content }]
    }
}

/// One line of bold text with a gap below it — every text row on every
/// screen is one of these.
///
/// Font size and gap are in `vmin` units (percent of the window's smaller
/// dimension) so the menu and end screens scale with the window instead of
/// staying fixed at pixel sizes. At the default 520×520 window, 1 vmin =
/// 5.2 px. The score HUD intentionally does *not* use this helper: it stays
/// pixel-sized because it is tuned to the arena's fixed-size food-exclusion
/// zone (`SCORE_AREA_COLS`/`SCORE_AREA_ROWS`).
fn label(text: String, size_vmin: f32, color: Color, gap_below_vmin: f32) -> impl Scene {
    bsn! {
        Text(text)
        TextFont {
            font_size: { FontSize::VMin(size_vmin) },
            weight: FontWeight::BOLD,
        }
        TextColor(color)
        Node { margin: { UiRect::bottom(Val::VMin(gap_below_vmin)) } }
    }
}

/// A clickable action button (START / RESTART / PLAY AGAIN).
///
/// Built on the headless `bevy_ui_widgets` button: the widget manages the
/// `Pressed` state and emits [`Activate`] on click (and on Enter/Space when
/// focused). The observer turns that into a [`StartRequested`] message so
/// buttons and the SPACE key share one start/restart code path. `Hovered` is
/// kept up to date by UI picking; `button_feedback` maps both states to
/// background colors.
fn action_button(text: String) -> impl Scene {
    bsn! {
        WidgetButton
        Hovered
        Node {
            padding: { UiRect::axes(Val::VMin(4.6), Val::VMin(1.7)) },
            border: { UiRect::all(Val::VMin(0.4)) },
            border_radius: { BorderRadius::all(Val::VMin(1.9)) },
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            margin: { UiRect::bottom(Val::VMin(1.9)) },
        }
        BackgroundColor(BUTTON_BG)
        BorderColor::from(START_GREEN)
        on(|_: On<Activate>, mut requests: MessageWriter<StartRequested>| {
            requests.write(StartRequested);
        })
        Children [(
            Text(text)
            TextFont {
                font_size: { FontSize::VMin(4.6) },
                weight: FontWeight::BOLD,
            }
            TextColor(Color::WHITE)
        )]
    }
}

/// Applies hover/pressed background feedback to the action buttons.
fn button_feedback(
    mut buttons: Query<(&mut BackgroundColor, &Hovered, Has<Pressed>), With<WidgetButton>>,
) {
    for (mut bg, hovered, pressed) in &mut buttons {
        let target = if pressed {
            BUTTON_BG_PRESSED
        } else if hovered.get() {
            BUTTON_BG_HOVERED
        } else {
            BUTTON_BG
        };
        if bg.0 != target {
            bg.0 = target;
        }
    }
}

/// The start menu screen.
///
/// The `(marker, scene)` tuples here and in the end screens merge both parts
/// onto the same root entity — tuples of scenes implement [`Scene`].
fn start_menu(high_score: usize) -> impl Scene {
    (
        bsn! { MenuUI },
        overlay(
            0.85,
            bsn_list![
                label("SNAKE".into(), 15.4, TITLE_GREEN, 7.7),
                { menu_high_score(high_score) },
                label("CONTROLS".into(), 4.6, Color::WHITE, 2.9),
                label("Arrow Keys or WASD to move".into(), 3.5, HINT_GRAY, 1.9),
                label("Eat the red apples to grow".into(), 3.5, HINT_GRAY, 1.9),
                label("Don't run into yourself!".into(), 3.5, HINT_GRAY, 7.7),
                action_button("START".into()),
                label("or press SPACE".into(), 2.9, BEST_GRAY, 0.0),
            ],
        ),
    )
}

/// Persistent best score from previous sessions — only shown once the player
/// has actually scored something (`None` spawns nothing).
fn menu_high_score(high_score: usize) -> Option<impl SceneList> {
    (high_score > 0)
        .then(|| bsn_list![label(format!("High Score: {high_score}"), 3.8, MENU_GOLD, 5.8)])
}

/// Shared layout of the game-over and win screens: title, final score,
/// record comparison, restart hint.
///
/// The scrim alpha is high enough to make the overlay text dominant, but
/// still translucent so the player can see where they died.
fn end_screen(
    title: String,
    title_color: Color,
    score: usize,
    previous_best: usize,
    button_text: String,
) -> impl Scene {
    overlay(
        0.82,
        bsn_list![
            label(title, 11.5, title_color, 3.8),
            label(format!("Final Score: {}", score), 5.8, Color::WHITE, 2.3),
            record_line(score, previous_best),
            action_button(button_text),
            label("or press SPACE".into(), 2.9, BEST_GRAY, 0.0),
        ],
    )
}

/// The line on an end screen that reports how the run compared to the stored
/// record: a gold "NEW HIGH SCORE!" banner when the run beat it, or a dim
/// "Best: N" reminder otherwise.
///
/// Callers must pass the record as it was *before* this run is persisted —
/// see the ordering note on `update_high_score` in the plugin's system chain.
fn record_line(score: usize, previous_best: usize) -> impl Scene {
    let (text, color) = if score > previous_best {
        ("NEW HIGH SCORE!".to_string(), RECORD_GOLD)
    } else {
        (format!("Best: {}", previous_best), BEST_GRAY)
    };
    label(text, 4.2, color, 5.8)
}

/// The game over screen.
fn game_over_screen(score: usize, previous_best: usize) -> impl Scene {
    (
        bsn! { GameOverUI },
        end_screen(
            "GAME OVER".into(),
            GAME_OVER_RED,
            score,
            previous_best,
            "RESTART".into(),
        ),
    )
}

/// The win screen, shown when the player fills the arena.
fn win_screen(score: usize, previous_best: usize) -> impl Scene {
    (
        bsn! { WinUI },
        end_screen(
            "YOU WIN!".into(),
            TITLE_GREEN,
            score,
            previous_best,
            "PLAY AGAIN".into(),
        ),
    )
}

/// System to spawn game over screen when game ends.
fn spawn_game_over_screen_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    game_over_ui: Query<Entity, With<GameOverUI>>,
    high_score: Res<HighScore>,
) {
    // Only spawn if game just ended and no UI exists yet
    if game_state.is_changed() && game_state.phase == GamePhase::GameOver && game_over_ui.is_empty()
    {
        commands.spawn_scene(game_over_screen(game_state.score, high_score.score));
    }
}

/// System to spawn the win screen when the player fills the arena.
fn spawn_win_screen_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    win_ui: Query<Entity, With<WinUI>>,
    high_score: Res<HighScore>,
) {
    if game_state.is_changed() && game_state.phase == GamePhase::Won && win_ui.is_empty() {
        commands.spawn_scene(win_screen(game_state.score, high_score.score));
    }
}

/// Persists a new record when a run ends (game over or win).
///
/// Ordered after the end-screen spawn systems in the plugin's chain so those
/// systems still see the previous record when deciding whether to show
/// "NEW HIGH SCORE!". The save is asynchronous (file I/O happens on another
/// thread) and crash-safe: `bevy_settings` writes to a temp file and renames.
fn update_high_score(
    mut commands: Commands,
    game_state: Res<GameState>,
    mut high_score: ResMut<HighScore>,
) {
    let run_ended = game_state.is_changed()
        && matches!(game_state.phase, GamePhase::GameOver | GamePhase::Won);
    if run_ended && game_state.score > high_score.score {
        high_score.score = game_state.score;
        commands.queue(SaveSettings::IfChanged);
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

/// System to start the game from the menu, on SPACE or the START button.
fn start_game_from_menu(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut start_requests: MessageReader<StartRequested>,
    mut game_state: ResMut<GameState>,
    mut camera_shake: ResMut<CameraShake>,
    menu_ui: Query<Entity, With<MenuUI>>,
) {
    // Drain unconditionally so a request from another phase can't linger.
    let button_clicked = start_requests.read().count() > 0;
    let requested = button_clicked || keyboard_input.just_pressed(KeyCode::Space);
    if game_state.phase == GamePhase::Menu && requested {
        for entity in menu_ui.iter() {
            commands.entity(entity).despawn();
        }
        begin_new_game(&mut commands, &mut game_state, &mut camera_shake);
    }
}

/// System to restart the game from the game-over or win screen, on SPACE or
/// the RESTART / PLAY AGAIN button.
#[allow(clippy::too_many_arguments)]
fn restart_game(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut start_requests: MessageReader<StartRequested>,
    mut game_state: ResMut<GameState>,
    mut input_buffer: ResMut<InputBuffer>,
    mut camera_shake: ResMut<CameraShake>,
    segments: SnakeEntityQuery,
    food: Query<Entity, With<Food>>,
    effects: EffectEntityQuery,
    game_over_ui: Query<Entity, With<GameOverUI>>,
    win_ui: Query<Entity, With<WinUI>>,
) {
    // Drain unconditionally so a request from another phase can't linger.
    let button_clicked = start_requests.read().count() > 0;
    let requested = button_clicked || keyboard_input.just_pressed(KeyCode::Space);
    let restartable = matches!(game_state.phase, GamePhase::GameOver | GamePhase::Won);
    if !(restartable && requested) {
        return;
    }

    // Despawn snake, food, and any in-flight eating effects (flashes,
    // particles, score popups) so the new game starts visually clean.
    for entity in segments.iter().chain(food.iter()).chain(effects.iter()) {
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
