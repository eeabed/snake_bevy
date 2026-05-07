//! Snake plugin - handles snake movement, input, collision detection, and spawning.

use bevy::{ecs::system::ParamSet, prelude::*, time::common_conditions::on_timer};
use bevy_vector_shapes::prelude::*;

use crate::game::{
    ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, CORNER_RADIUS, Direction, GamePhase, GameSet, GameState,
    GrowingSegment, GrowthEvent, INITIAL_SNAKE_POSITION, InputBuffer, MOVE_INTERVAL, Position,
    PreviousPosition, SNAKE_HEAD_COLOR, SNAKE_SEGMENT_COLOR, SnakeEye, SnakeHead, SnakeSegment,
    Z_SNAKE_HEAD, Z_SNAKE_SEGMENT,
};

// Visual sizing: head fills almost the full cell so it reads as larger than
// the body, and the body sits inside its cell so adjacent segments show a
// visible gap (~14% of CELL_SIZE).
const HEAD_SIZE_FACTOR: f32 = 0.92;
const SEGMENT_SIZE_FACTOR: f32 = 0.86;

// Tail tapering — the last few segments are scaled down progressively so the
// snake doesn't end abruptly. Index 0 is the tail itself.
const TAIL_TAPER: [f32; 3] = [0.65, 0.78, 0.90];

// Body color gradient. Each body segment's color is interpolated between
// `BODY_COLOR_NEAR_HEAD` (segment closest to the head) and
// `BODY_COLOR_NEAR_TAIL` (the tail) based on its position in the snake.
// Same hue as `SNAKE_SEGMENT_COLOR`, just brighter near the head and dimmer
// at the tail. The tail color is intentionally close to the playfield's
// background so the tail visually dissolves into the arena.
const BODY_COLOR_NEAR_HEAD: Color = Color::srgba(0.40, 0.95, 0.40, 1.0);
const BODY_COLOR_NEAR_TAIL: Color = Color::srgba(0.08, 0.28, 0.08, 1.0);

/// Exponent applied to the gradient parameter `t` (in 0.0..=1.0).
///
/// A value > 1 gives a concave-up curve: segments near the head stay close
/// to `BODY_COLOR_NEAR_HEAD`, then darken faster as they approach the tail.
/// Visually: a bright, even body up front and a fade that accelerates into
/// the tail — much more readable per-segment than a linear ramp.
const BODY_GRADIENT_EXPONENT: f32 = 1.6;

/// Plugin for snake-related systems.
pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
        // Movement set: read input, then advance the snake on each move-tick.
        // The renderer's interpolation accumulator lives inside
        // `position_translation` itself (no shared resource needed).
        app.add_systems(
            Update,
            (snake_movement_input, snake_movement.run_if(on_timer(MOVE_INTERVAL)))
                .chain()
                .in_set(GameSet::Movement),
        );
        // Growth and game-over run after food collision (GameSet::Effects).
        app.add_systems(
            Update,
            (snake_growth, game_over_check).chain().in_set(GameSet::Effects),
        );
        // Visual body styling (tail taper + head→tail color gradient) belongs
        // in the Rendering set so it runs after `growing_segment_animation`
        // (whose final-frame `scale = 1.0` write we want to overwrite for
        // tail segments).
        app.add_systems(Update, style_snake_body.in_set(GameSet::Rendering));
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
type BodyStyleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut ShapeFill),
    (With<SnakeSegment>, Without<GrowingSegment>),
>;

/// Spawns the snake head entity with eyes.
///
/// The head is colored in HDR-green (matches the body's hue but pushed past
/// 1.0 so the bloom pass picks it up — no separate "glow disc" child needed).
/// Eyes are positioned toward the front of the head (assumes the head spawns
/// facing `Right`; `update_head_rotation` rotates the children to follow).
pub fn spawn_snake_head(commands: &mut Commands) -> Entity {
    let size = CELL_SIZE * HEAD_SIZE_FACTOR;
    // Normalize corner radius relative to the shape size (0.0 to 1.0 range)
    let corner_radius_normalized = CORNER_RADIUS / (size / 2.0);

    // Eye geometry, in the head's local pixel space.
    //   forward: pushed toward the front (positive x = "Right" direction)
    //   lateral: spaced wider apart so the two eyes don't read as a colon
    //   radius:  large enough to be visibly two distinct dots at ~25 px cells
    let eye_forward = CELL_SIZE * 0.18;
    let eye_lateral = CELL_SIZE * 0.22;
    let eye_radius = CELL_SIZE * 0.13;

    commands
        .spawn((
            ShapeBundle::rect(
                &ShapeConfig {
                    color: SNAKE_HEAD_COLOR,
                    corner_radii: Vec4::splat(corner_radius_normalized),
                    transform: Transform::from_xyz(
                        (INITIAL_SNAKE_POSITION.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5)
                            * CELL_SIZE,
                        (INITIAL_SNAKE_POSITION.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5)
                            * CELL_SIZE,
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
            // Front-right eye (relative to spawn direction = Right).
            parent.spawn((
                ShapeBundle::circle(
                    &ShapeConfig {
                        color: Color::srgba(0.0, 0.0, 0.0, 1.0),
                        transform: Transform::from_xyz(eye_forward, eye_lateral, 0.1),
                        ..ShapeConfig::default_2d()
                    },
                    eye_radius,
                ),
                SnakeEye,
            ));

            // Front-left eye.
            parent.spawn((
                ShapeBundle::circle(
                    &ShapeConfig {
                        color: Color::srgba(0.0, 0.0, 0.0, 1.0),
                        transform: Transform::from_xyz(eye_forward, -eye_lateral, 0.1),
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
///
/// Sized below the cell so adjacent segments leave a small visible gap —
/// the body reads as a chain of pills rather than a continuous rectangle.
pub fn spawn_snake_segment(commands: &mut Commands, position: Position) -> Entity {
    let size = CELL_SIZE * SEGMENT_SIZE_FACTOR;
    // Normalize corner radius relative to the shape size (0.0 to 1.0 range)
    let corner_radius_normalized = CORNER_RADIUS / (size / 2.0);

    // Compute world-space spawn coordinates so the segment renders at the right
    // z-layer immediately. `position_translation` will overwrite x/y next frame
    // but preserve z.
    let world_x = (position.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE;
    let world_y = (position.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE;

    commands
        .spawn((
            ShapeBundle::rect(
                &ShapeConfig {
                    color: SNAKE_SEGMENT_COLOR,
                    corner_radii: Vec4::splat(corner_radius_normalized),
                    transform: Transform::from_xyz(world_x, world_y, Z_SNAKE_SEGMENT),
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

/// Maps the current keyboard state to a [`Direction`], falling back to
/// `current` when no directional key is held.
///
/// Lives here rather than on `Direction` itself because it depends on a Bevy
/// input resource — a concern that doesn't belong on a plain data enum.
fn direction_from_input(keyboard_input: &ButtonInput<KeyCode>, current: Direction) -> Direction {
    if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
        Direction::Left
    } else if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD)
    {
        Direction::Right
    } else if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
        Direction::Up
    } else if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
        Direction::Down
    } else {
        current
    }
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

    let Ok(head) = heads.single() else { return };

    // Get the last direction in buffer or current head direction
    let last_direction = input_buffer.last_direction().unwrap_or(head.direction);

    // Get new direction from input
    let new_direction = direction_from_input(&keyboard_input, last_direction);

    // If direction changed and it's not opposite to the last direction, queue it
    if new_direction != last_direction && new_direction != last_direction.opposite() {
        input_buffer.queue_direction(new_direction);
    }
}

/// System to execute snake movement on a timer.
fn snake_movement(
    game_state: Res<GameState>,
    mut input_buffer: ResMut<InputBuffer>,
    mut query_set: ParamSet<(SnakeHeadQuery, PositionQuery)>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }

    // Step 1: Get the head entity and its current direction and position.
    let (head_entity, head_direction, head_position) = {
        let mut heads_query = query_set.p0();
        let Ok((entity, mut head, position, _)) = heads_query.single_mut() else {
            return;
        };
        // Try to consume buffered direction
        if let Some(buffered_direction) = input_buffer.pop_direction() {
            head.direction = buffered_direction;
        }
        (entity, head.direction, *position)
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
        let Ok((_, _, mut head_pos, mut prev_pos)) = heads_query.single_mut() else {
            return;
        };
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

/// Handles every [`GrowthEvent`] in the queue this frame by appending a new
/// segment for each one.
///
/// The new segment is spawned at the **previous** position of the current tail
/// — i.e. the cell the tail just vacated this tick. This avoids the head/segment
/// overlap that would otherwise occur on the length-1 → length-2 transition,
/// which is why no special-case "skip segment[1]" logic is needed in
/// [`game_over_check`] anymore.
fn snake_growth(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut growth_reader: MessageReader<GrowthEvent>,
    prev_positions: Query<&PreviousPosition>,
) {
    for _ in growth_reader.read() {
        let Some(&last_segment_entity) = game_state.snake_segments.last() else {
            return;
        };
        let Ok(last_prev) = prev_positions.get(last_segment_entity) else {
            return;
        };

        let new_segment = spawn_snake_segment(&mut commands, last_prev.pos);

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
    segment_positions: Query<&Position, With<SnakeSegment>>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }

    let Ok(head_pos) = head_positions.single() else {
        return;
    };

    // Because new segments now spawn at the tail's previous position
    // (see `snake_growth`), no body segment can ever share the head's cell
    // immediately after a growth — so a plain equality check is sufficient.
    for segment_pos in segment_positions.iter() {
        if head_pos == segment_pos {
            game_state.phase = GamePhase::GameOver;
            info!("Game Over! Final score: {}", game_state.score);
            break;
        }
    }
}

/// Linearly interpolate between two `srgba` colors. Used for the body's
/// head→tail brightness gradient.
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let a = a.to_srgba();
    let b = b.to_srgba();
    Color::srgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}

/// Per-frame visual styling for snake body segments: applies the tail
/// scale-taper and the head→tail color gradient.
///
/// Segments still in the grow-in animation (`GrowingSegment`) are skipped so
/// the two scale writers don't fight; once that animation removes its
/// component, this system takes over on the next frame.
///
/// Both scale and color writes are guarded so they only fire when the value
/// actually changes — avoids spurious change-detection ticks for segments
/// in the steady-state middle of the body.
fn style_snake_body(game_state: Res<GameState>, mut segments: BodyStyleQuery) {
    let total = game_state.snake_segments.len();
    if total < 2 {
        return; // only the head — no body to style.
    }
    let body_count = total - 1; // segments excluding the head.

    // game_state.snake_segments[0] is the head; body segments are at 1..total.
    for (i, &entity) in game_state.snake_segments.iter().enumerate().skip(1) {
        // Position in body, 0 = closest to head, body_count - 1 = tail.
        let body_index = i - 1;
        let from_tail = body_count - 1 - body_index;

        // Scale taper for the last few segments; full scale otherwise.
        let scale_factor = TAIL_TAPER.get(from_tail).copied().unwrap_or(1.0);

        // Color gradient: t = 0.0 at the segment closest to the head,
        // t = 1.0 at the tail. Single-segment body collapses to t = 0.0.
        // The exponent biases the curve so brightness change-per-segment is
        // small near the head and large near the tail — perceptually clearer
        // than a linear ramp on long snakes.
        let t_linear = if body_count <= 1 {
            0.0
        } else {
            body_index as f32 / (body_count - 1) as f32
        };
        let t = t_linear.powf(BODY_GRADIENT_EXPONENT);
        let color = lerp_color(BODY_COLOR_NEAR_HEAD, BODY_COLOR_NEAR_TAIL, t);

        let Ok((mut transform, mut fill)) = segments.get_mut(entity) else {
            continue;
        };

        let target_scale = Vec3::splat(scale_factor);
        if transform.scale != target_scale {
            transform.scale = target_scale;
        }
        if fill.color != color {
            fill.color = color;
        }
    }
}
