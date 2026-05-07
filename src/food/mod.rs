//! Food plugin - handles food spawning, collision detection, and related effects.

use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use rand::prelude::*;

use crate::game::{
    ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, FOOD_COLOR, Food, FoodEatenEvent, FoodPulse, GamePhase,
    GameSet, GameState, GrowthEvent, Position, PreviousPosition, SCORE_AREA_COLS, SCORE_AREA_ROWS,
    SnakeHead, SnakeSegment, Z_FOOD,
};

/// Plugin for food-related systems.
pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (food_collision, food_pulse_animation)
                .chain()
                .in_set(GameSet::Collision),
        );
    }
}

// Type alias for querying all snake parts
type SnakePartsQuery<'w, 's> =
    Query<'w, 's, &'static Position, Or<(With<SnakeHead>, With<SnakeSegment>)>>;

/// Spawns food at a random free cell that doesn't overlap the snake.
///
/// Builds the complete set of occupied cells first, then picks uniformly from
/// the complement. Returns `false` when the arena is completely full (no free
/// cell exists), allowing the caller to transition to a win state. Returns
/// `true` when a food entity was successfully spawned.
pub fn spawn_food(commands: &mut Commands, snake_positions: &[Position]) -> bool {
    let mut rng = rand::rng();

    // Build a hash-set of occupied cells for O(1) lookup.
    let occupied: std::collections::HashSet<Position> = snake_positions.iter().copied().collect();

    // Collect every cell in the arena that is free.
    // Exclude the top-left area where the score text is displayed (≈ 3 × 2 cells).
    let free: Vec<Position> = (0..ARENA_WIDTH as i32)
        .flat_map(|x| (0..ARENA_HEIGHT as i32).map(move |y| Position { x, y }))
        .filter(|p| {
            let is_score_area =
                p.x < SCORE_AREA_COLS && p.y >= (ARENA_HEIGHT as i32 - SCORE_AREA_ROWS);
            !occupied.contains(p) && !is_score_area
        })
        .collect();

    // If every cell is occupied there is nowhere to place food — caller decides
    // what to do (e.g. transition to GamePhase::Won).
    if free.is_empty() {
        return false;
    }
    let position = free[rng.random_range(0..free.len())];

    let radius = CELL_SIZE / 2.0;

    // Pre-compute world-space coordinates so the food spawns at its final
    // z-layer immediately (avoids a one-frame z=0 flash before the renderer
    // catches up next frame).
    let world_x = (position.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE;
    let world_y = (position.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE;

    commands.spawn((
        ShapeBundle::circle(
            &ShapeConfig {
                color: FOOD_COLOR,
                transform: Transform::from_xyz(world_x, world_y, Z_FOOD),
                ..ShapeConfig::default_2d()
            },
            radius,
        ),
        Food,
        position,
        PreviousPosition { pos: position },
        FoodPulse {
            timer: Timer::from_seconds(0.8, TimerMode::Repeating),
        },
    ));
    true
}

/// System to detect food collision, trigger growth, and respawn food.
///
/// If the arena is full after eating (no free cell to place new food), the
/// game transitions to [`GamePhase::Won`].
fn food_collision(
    mut commands: Commands,
    mut growth_writer: MessageWriter<GrowthEvent>,
    mut food_eaten_writer: MessageWriter<FoodEatenEvent>,
    mut game_state: ResMut<GameState>,
    head_positions: Query<&Position, With<SnakeHead>>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    all_snake_positions: SnakePartsQuery,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }

    let Ok(head_pos) = head_positions.single() else {
        return;
    };

    for (food_entity, food_pos) in food_positions.iter() {
        if head_pos == food_pos {
            // Capture the food position before despawning the entity.
            let eaten_at = *food_pos;

            // Update game state and emit messages first, then despawn the entity.
            game_state.score += 1;
            growth_writer.write(GrowthEvent);
            food_eaten_writer.write(FoodEatenEvent { position: eaten_at });
            commands.entity(food_entity).despawn();

            // Collect all snake positions to avoid spawning food on the snake.
            let snake_positions: Vec<Position> = all_snake_positions.iter().copied().collect();
            if !spawn_food(&mut commands, &snake_positions) {
                // No free cell remained — the snake fills the arena. Win!
                game_state.phase = GamePhase::Won;
                info!("You Win! Final score: {}", game_state.score);
            }
        }
    }
}

/// System to animate food with a pulsing effect.
fn food_pulse_animation(
    time: Res<Time>,
    mut foods: Query<(&mut Transform, &mut FoodPulse), With<Food>>,
) {
    for (mut transform, mut pulse) in foods.iter_mut() {
        pulse.timer.tick(time.delta());

        // Use sine wave for smooth pulsing
        let progress = pulse.timer.fraction();
        let scale = 1.0 + (progress * std::f32::consts::PI * 2.0).sin() * 0.15;

        transform.scale = Vec3::splat(scale);
    }
}
