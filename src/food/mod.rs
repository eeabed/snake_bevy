//! Food plugin - handles food spawning, collision detection, and related effects.

use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use rand::prelude::*;

use crate::game::{
    APPLE_HIGHLIGHT_COLOR, APPLE_LEAF_COLOR, APPLE_STEM_COLOR, ARENA_HEIGHT, ARENA_WIDTH,
    CELL_SIZE, FOOD_COLOR, Food, FoodEatenEvent, FoodPulse, GamePhase, GameSet, GameState,
    GrowthEvent, Position, PreviousPosition, SCORE_AREA_COLS, SCORE_AREA_ROWS, SnakeHead,
    SnakeSegment, SpawnPop, Z_FOOD,
};

/// Plugin for food-related systems.
pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (food_collision, spawn_pop_animation, food_pulse_animation)
                .chain()
                .in_set(GameSet::Collision),
        );
    }
}

// Type alias for querying all snake parts
type SnakePartsQuery<'w, 's> =
    Query<'w, 's, &'static Position, Or<(With<SnakeHead>, With<SnakeSegment>)>>;
// Food-pulse query, excluding apples still in their spawn pop-in.
type FoodPulseQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut FoodPulse),
    (With<Food>, Without<SpawnPop>),
>;

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

    let radius = CELL_SIZE * 0.40;

    // Pre-compute world-space coordinates so the food spawns at its final
    // z-layer immediately (avoids a one-frame z=0 flash before the renderer
    // catches up next frame).
    let world_x = (position.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE;
    let world_y = (position.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE;

    // The apple: HDR-red body with a glossy highlight, a stem, and a leaf.
    // Children ride the parent transform, so the pop-in / pulse animations
    // scale and wobble the whole fruit as one piece.
    commands
        .spawn((
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
            SpawnPop {
                timer: Timer::from_seconds(0.25, TimerMode::Once),
            },
        ))
        .with_children(|apple| {
            // Glossy highlight, upper-left.
            apple.spawn(ShapeBundle::circle(
                &ShapeConfig {
                    color: APPLE_HIGHLIGHT_COLOR,
                    transform: Transform::from_xyz(-radius * 0.35, radius * 0.35, 0.01),
                    ..ShapeConfig::default_2d()
                },
                radius * 0.26,
            ));
            // Stem: small brown pill poking out of the top, slightly tilted.
            apple.spawn(ShapeBundle::rect(
                &ShapeConfig {
                    color: APPLE_STEM_COLOR,
                    corner_radii: Vec4::splat(1.0),
                    transform: Transform::from_xyz(0.0, radius * 1.0, -0.01)
                        .with_rotation(Quat::from_rotation_z(0.25)),
                    ..ShapeConfig::default_2d()
                },
                Vec2::new(radius * 0.18, radius * 0.55),
            ));
            // Leaf: green pill rotated off the stem.
            apple.spawn(ShapeBundle::rect(
                &ShapeConfig {
                    color: APPLE_LEAF_COLOR,
                    corner_radii: Vec4::splat(1.0),
                    transform: Transform::from_xyz(radius * 0.45, radius * 1.05, -0.005)
                        .with_rotation(Quat::from_rotation_z(0.9)),
                    ..ShapeConfig::default_2d()
                },
                Vec2::new(radius * 0.7, radius * 0.32),
            ));
        });
    true
}

/// Eases the apple in with a springy overshoot (ease-out-back) when it
/// spawns, then hands scale control back to `food_pulse_animation`.
fn spawn_pop_animation(
    mut commands: Commands,
    time: Res<Time>,
    mut popping: Query<(Entity, &mut Transform, &mut SpawnPop)>,
) {
    for (entity, mut transform, mut pop) in popping.iter_mut() {
        pop.timer.tick(time.delta());

        if pop.timer.is_finished() {
            transform.scale = Vec3::splat(1.0);
            commands.entity(entity).remove::<SpawnPop>();
        } else {
            // Ease-out-back: overshoots ~10% then settles at 1.0.
            const C1: f32 = 1.70158;
            const C3: f32 = C1 + 1.0;
            let t = pop.timer.fraction() - 1.0;
            let scale = 1.0 + C3 * t * t * t + C1 * t * t;
            transform.scale = Vec3::splat(scale.max(0.0));
        }
    }
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

/// System to animate food with a gentle pulse and a playful wobble.
///
/// Skips apples still in their `SpawnPop` intro so the two animations don't
/// fight over `Transform::scale`.
fn food_pulse_animation(time: Res<Time>, mut foods: FoodPulseQuery) {
    for (mut transform, mut pulse) in foods.iter_mut() {
        pulse.timer.tick(time.delta());

        let angle = pulse.timer.fraction() * std::f32::consts::TAU;
        transform.scale = Vec3::splat(1.0 + angle.sin() * 0.10);
        // Rock side to side at the same rate, a quarter-phase behind the
        // pulse so the two motions read as one organic wiggle.
        transform.rotation = Quat::from_rotation_z(angle.cos() * 0.09);
    }
}
