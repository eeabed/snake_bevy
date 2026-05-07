//! Food plugin - handles food spawning, collision detection, and related effects.

use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use rand::prelude::*;

use crate::game::{
    ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, FOOD_COLOR, Food, FoodEatenEvent, FoodPulse, GamePhase,
    GameSet, GameState, GrowthEvent, Position, PreviousPosition, SCORE_AREA_COLS, SCORE_AREA_ROWS,
    SnakeHead, SnakeSegment,
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
/// the complement. Returns without spawning when the arena is completely full
/// (every cell is occupied by the snake).
pub fn spawn_food(commands: &mut Commands, snake_positions: &[Position]) {
    let mut rng = rand::rng();

    // Build a hash-set of occupied cells for O(1) lookup.
    let occupied: std::collections::HashSet<(i32, i32)> =
        snake_positions.iter().map(|p| (p.x, p.y)).collect();

    // Collect every cell in the arena that is free.
    // Exclude the top-left area where the score text is displayed (≈ 3 × 2 cells).
    let free: Vec<Position> = (0..ARENA_WIDTH as i32)
        .flat_map(|x| (0..ARENA_HEIGHT as i32).map(move |y| Position { x, y }))
        .filter(|p| {
            let is_score_area =
                p.x < SCORE_AREA_COLS && p.y >= (ARENA_HEIGHT as i32 - SCORE_AREA_ROWS);
            !occupied.contains(&(p.x, p.y)) && !is_score_area
        })
        .collect();

    // If every cell is occupied there is nowhere to place food — skip spawning.
    if free.is_empty() {
        return;
    }
    let position = free[rng.random_range(0..free.len())];

    let radius = CELL_SIZE / 2.0;

    commands.spawn((
        ShapeBundle::circle(
            &ShapeConfig {
                color: FOOD_COLOR,
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
}

/// System to detect food collision and trigger growth.
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

    if let Some(head_pos) = head_positions.iter().next() {
        for (food_entity, food_pos) in food_positions.iter() {
            if head_pos.collides_with(food_pos) {
                commands.entity(food_entity).despawn();
                game_state.score += 1;
                growth_writer.write(GrowthEvent);
                food_eaten_writer.write(FoodEatenEvent {
                    position: *food_pos,
                });

                // Collect all snake positions to avoid spawning food on the snake
                let snake_positions: Vec<Position> = all_snake_positions.iter().copied().collect();
                spawn_food(&mut commands, &snake_positions);
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
