//! Food plugin - handles food spawning, collision detection, and related effects.

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use rand::prelude::*;

use crate::game::{
    ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, FOOD_COLOR, Food, FoodEatenEvent, FoodPulse, GamePhase,
    GameState, GrowthEvent, Position, PreviousPosition, SnakeHead, SnakeSegment,
};

/// Plugin for food-related systems.
pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (food_collision, food_pulse_animation).chain());
    }
}

// Type alias for querying all snake parts
type SnakePartsQuery<'w, 's> =
    Query<'w, 's, &'static Position, Or<(With<SnakeHead>, With<SnakeSegment>)>>;

/// Spawns food at a random position that doesn't overlap with the snake.
pub fn spawn_food(commands: &mut Commands, snake_positions: &[Position]) {
    let mut rng = rand::rng();
    let mut position;

    // Keep generating positions until we find one that doesn't overlap with the snake or score display
    loop {
        position = Position {
            x: rng.random_range(0..ARENA_WIDTH as i32),
            y: rng.random_range(0..ARENA_HEIGHT as i32),
        };

        // Exclude top-left area where score is displayed (roughly 3x2 cells)
        let is_score_area = position.x <= 2 && position.y >= (ARENA_HEIGHT as i32 - 2);

        // Check if this position overlaps with any snake segment or the score area
        if !snake_positions.contains(&position) && !is_score_area {
            break;
        }
    }

    let shape = shapes::Circle {
        radius: CELL_SIZE / 2.0,
        center: Vec2::ZERO,
    };

    commands.spawn((
        ShapeBuilder::with(&shape).fill(FOOD_COLOR).build(),
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
