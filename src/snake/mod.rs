//! Snake plugin - handles snake movement, input, collision detection, and spawning.

use bevy::{ecs::system::ParamSet, prelude::*, time::common_conditions::on_timer};
use bevy_vector_shapes::prelude::*;

use crate::game::{
    ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, CORNER_RADIUS, Direction, GamePhase, GameState,
    GrowingSegment, GrowthEvent, INITIAL_SNAKE_POSITION, InputBuffer, MOVE_INTERVAL, MoveTimer,
    Position, PreviousPosition, SNAKE_HEAD_COLOR, SNAKE_HEAD_GLOW_COLOR, SNAKE_SEGMENT_COLOR,
    SnakeEye, SnakeHead, SnakeSegment, Z_SNAKE_HEAD,
};

/// Plugin for snake-related systems.
pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                snake_movement_input,
                snake_movement.run_if(on_timer(MOVE_INTERVAL)),
                snake_growth,
                game_over_check,
            )
                .chain(),
        );
    }
}

// Type aliases for complex queries
type SnakeHeadQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut SnakeHead,
        &'static mut Position,
        &'static mut PreviousPosition,
    ),
>;
type PositionQuery<'w, 's> = Query<'w, 's, (&'static mut Position, &'static mut PreviousPosition)>;

/// Spawns the snake head entity with eyes.
pub fn spawn_snake_head(commands: &mut Commands) -> Entity {
    let size = CELL_SIZE * 0.9;
    // Normalize corner radius relative to the shape size (0.0 to 1.0 range)
    let corner_radius_normalized = CORNER_RADIUS / (size / 2.0);

    commands
        .spawn((
            ShapeBundle::rect(
                &ShapeConfig {
                    color: SNAKE_HEAD_COLOR,
                    corner_radii: Vec4::splat(corner_radius_normalized),
                    transform: Transform::from_xyz(
                        (3.0 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE,
                        (3.0 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE,
                        Z_SNAKE_HEAD,
                    ),
                    ..ShapeConfig::default_2d()
                },
                Vec2::splat(size),
            ),
            SnakeHead {
                direction: Direction::Right,
            },
            INITIAL_SNAKE_POSITION,
            PreviousPosition {
                pos: INITIAL_SNAKE_POSITION,
            },
        ))
        .with_children(|parent| {
            // Glow effect behind the head (rendered first, behind everything)
            parent.spawn(ShapeBundle::circle(
                &ShapeConfig {
                    color: SNAKE_HEAD_GLOW_COLOR,
                    alpha_mode: ShapeAlphaMode::Add,
                    transform: Transform::from_xyz(0.0, 0.0, -0.1),
                    ..ShapeConfig::default_2d()
                },
                CELL_SIZE * 0.8,
            ));

            let eye_radius = CELL_SIZE * 0.08;

            // Right eye (relative to Right direction)
            parent.spawn((
                ShapeBundle::circle(
                    &ShapeConfig {
                        color: Color::srgba(0.0, 0.0, 0.0, 1.0),
                        transform: Transform::from_xyz(CELL_SIZE * 0.15, CELL_SIZE * 0.15, 0.1),
                        ..ShapeConfig::default_2d()
                    },
                    eye_radius,
                ),
                SnakeEye,
            ));

            // Left eye (relative to Right direction)
            parent.spawn((
                ShapeBundle::circle(
                    &ShapeConfig {
                        color: Color::srgba(0.0, 0.0, 0.0, 1.0),
                        transform: Transform::from_xyz(CELL_SIZE * 0.15, -CELL_SIZE * 0.15, 0.1),
                        ..ShapeConfig::default_2d()
                    },
                    eye_radius,
                ),
                SnakeEye,
            ));
        })
        .id()
}

/// Spawns a snake body segment at the given position.
pub fn spawn_snake_segment(commands: &mut Commands, position: Position) -> Entity {
    let size = CELL_SIZE;
    // Normalize corner radius relative to the shape size (0.0 to 1.0 range)
    let corner_radius_normalized = CORNER_RADIUS / (size / 2.0);

    commands
        .spawn((
            ShapeBundle::rect(
                &ShapeConfig {
                    color: SNAKE_SEGMENT_COLOR,
                    corner_radii: Vec4::splat(corner_radius_normalized),
                    ..ShapeConfig::default_2d()
                },
                Vec2::splat(size),
            ),
            SnakeSegment,
            position,
            PreviousPosition { pos: position },
        ))
        .id()
}

/// System to read keyboard input and queue direction changes.
fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut input_buffer: ResMut<InputBuffer>,
    heads: Query<&SnakeHead>,
    game_state: Res<GameState>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }

    if let Some(head) = heads.iter().next() {
        // Get the last direction in buffer or current head direction
        let last_direction = input_buffer.last_direction().unwrap_or(head.direction);

        // Get new direction from input
        let new_direction = Direction::from_input(&keyboard_input, last_direction);

        // If direction changed and it's not opposite to the last direction, queue it
        if new_direction != last_direction && new_direction != last_direction.opposite() {
            input_buffer.queue_direction(new_direction);
        }
    }
}

/// System to execute snake movement on a timer.
fn snake_movement(
    game_state: ResMut<GameState>,
    mut input_buffer: ResMut<InputBuffer>,
    mut move_timer: ResMut<MoveTimer>,
    mut query_set: ParamSet<(SnakeHeadQuery, PositionQuery)>,
    _segments: Query<Entity, With<SnakeSegment>>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }

    // Reset the move timer
    move_timer.elapsed = std::time::Duration::ZERO;

    // Step 1: Get the head entity and its current direction and position
    let (head_entity, head_direction, head_position) = {
        let mut heads_query = query_set.p0();
        if let Some((entity, mut head, position, _)) = heads_query.iter_mut().next() {
            // Try to consume buffered direction
            if let Some(buffered_direction) = input_buffer.pop_direction() {
                head.direction = buffered_direction;
            }
            (entity, head.direction, *position)
        } else {
            return;
        }
    };

    // Step 2: Record the current position of each segment before any movement
    let segments_positions = {
        let mut positions = Vec::new();
        let positions_query = query_set.p1();

        for &segment_entity in &game_state.snake_segments {
            if segment_entity == head_entity {
                positions.push(head_position);
            } else if let Ok((segment_pos, _)) = positions_query.get(segment_entity) {
                positions.push(*segment_pos);
            }
        }
        positions
    };

    // Step 3: Move the head in the current direction
    {
        let mut heads_query = query_set.p0();
        if let Some((_, _, mut head_pos, mut prev_pos)) = heads_query.iter_mut().next() {
            // Save current position as previous position for interpolation
            prev_pos.pos = *head_pos;

            // Move the head one cell in the current direction
            match head_direction {
                Direction::Left => head_pos.x -= 1,
                Direction::Right => head_pos.x += 1,
                Direction::Up => head_pos.y += 1,
                Direction::Down => head_pos.y -= 1,
            }

            // Wrap around if the snake goes off the edge (creates a toroidal arena)
            head_pos.x = (head_pos.x + ARENA_WIDTH as i32) % ARENA_WIDTH as i32;
            head_pos.y = (head_pos.y + ARENA_HEIGHT as i32) % ARENA_HEIGHT as i32;
        }
    }

    // Step 4: Move each body segment to the position of the segment in front of it
    {
        let mut positions_query = query_set.p1();
        for (i, segment_entity) in game_state.snake_segments.iter().skip(1).enumerate() {
            if let Ok((mut segment_pos, mut prev_pos)) = positions_query.get_mut(*segment_entity) {
                prev_pos.pos = *segment_pos;
                *segment_pos = segments_positions[i];
            }
        }
    }
}

/// System to handle snake growth when GrowthEvent is received.
fn snake_growth(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut growth_reader: MessageReader<GrowthEvent>,
    positions: Query<&Position>,
) {
    if growth_reader.read().next().is_some()
        && let Some(&last_segment_entity) = game_state.snake_segments.last()
        && let Ok(last_pos) = positions.get(last_segment_entity)
    {
        let new_segment = spawn_snake_segment(&mut commands, *last_pos);

        // Add growing animation component
        commands.entity(new_segment).insert(GrowingSegment {
            timer: Timer::from_seconds(0.2, TimerMode::Once),
        });

        game_state.snake_segments.push(new_segment);
    }
}

/// System to check for game over (self-collision).
fn game_over_check(
    mut game_state: ResMut<GameState>,
    head_positions: Query<&Position, With<SnakeHead>>,
    segment_positions: Query<(&Position, Entity), With<SnakeSegment>>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }

    if let Some(head_pos) = head_positions.iter().next() {
        for (segment_pos, segment_entity) in segment_positions.iter() {
            if head_pos.collides_with(segment_pos)
                && game_state.snake_segments.len() > 1
                && game_state.snake_segments[1] != segment_entity
            {
                game_state.game_over = true;
                game_state.phase = GamePhase::GameOver;
                println!("Game Over! Final score: {}", game_state.score);
            }
        }
    }
}
