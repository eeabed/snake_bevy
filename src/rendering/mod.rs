//! Rendering plugin - handles position interpolation, rotation, visual effects, and camera.

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use rand::prelude::*;

use crate::game::{
    ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, CameraShake, Food, FoodEatenEvent, GamePhase, GameState,
    GrowingSegment, MOVE_INTERVAL, MoveTimer, Position, PreviousPosition, PulseEffect, SnakeHead,
    SnakeSegment, Z_BACKGROUND, Z_FOOD, Z_SNAKE_HEAD, Z_SNAKE_SEGMENT,
};

/// Plugin for rendering and visual effects.
pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_move_timer,
                position_translation,
                update_head_rotation,
                pulse_effect_system,
                spawn_food_eaten_effect,
                camera_shake_system,
                growing_segment_animation,
                trigger_camera_shake_on_game_over,
            )
                .chain(),
        );
    }
}

// Type alias for transform interpolation query
type TransformInterpolationQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static PreviousPosition,
        &'static mut Transform,
        Option<&'static SnakeHead>,
        Option<&'static SnakeSegment>,
        Option<&'static Food>,
    ),
>;

/// System to track elapsed time for interpolation.
fn update_move_timer(mut move_timer: ResMut<MoveTimer>, time: Res<Time>) {
    move_timer.elapsed += time.delta();
}

/// System to interpolate entity positions for smooth movement.
fn position_translation(mut transforms: TransformInterpolationQuery, move_timer: Res<MoveTimer>) {
    // Calculate interpolation progress (0.0 to 1.0)
    let progress = (move_timer.elapsed.as_secs_f32() / MOVE_INTERVAL.as_secs_f32()).min(1.0);

    for (pos, prev_pos, mut transform, head, segment, food) in transforms.iter_mut() {
        // Set z-index based on entity type to ensure proper layering
        let z = if head.is_some() {
            Z_SNAKE_HEAD
        } else if segment.is_some() {
            Z_SNAKE_SEGMENT
        } else if food.is_some() {
            Z_FOOD
        } else {
            Z_BACKGROUND
        };

        // Interpolate between previous and current position
        let curr_x = (pos.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE;
        let curr_y = (pos.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE;

        let prev_x = (prev_pos.pos.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE;
        let prev_y = (prev_pos.pos.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE;

        // Handle wrapping for toroidal arena
        let dx = if (curr_x - prev_x).abs() > CELL_SIZE * ARENA_WIDTH as f32 / 2.0 {
            if curr_x > prev_x {
                curr_x - prev_x - CELL_SIZE * ARENA_WIDTH as f32
            } else {
                curr_x - prev_x + CELL_SIZE * ARENA_WIDTH as f32
            }
        } else {
            curr_x - prev_x
        };

        let dy = if (curr_y - prev_y).abs() > CELL_SIZE * ARENA_HEIGHT as f32 / 2.0 {
            if curr_y > prev_y {
                curr_y - prev_y - CELL_SIZE * ARENA_HEIGHT as f32
            } else {
                curr_y - prev_y + CELL_SIZE * ARENA_HEIGHT as f32
            }
        } else {
            curr_y - prev_y
        };

        let interpolated_x = prev_x + dx * progress;
        let interpolated_y = prev_y + dy * progress;

        transform.translation = Vec3::new(interpolated_x, interpolated_y, z);
    }
}

/// System to update snake head rotation based on direction.
fn update_head_rotation(mut heads: Query<(&SnakeHead, &mut Transform)>) {
    for (head, mut transform) in heads.iter_mut() {
        let rotation = match head.direction {
            crate::game::Direction::Right => 0.0,
            crate::game::Direction::Up => std::f32::consts::FRAC_PI_2,
            crate::game::Direction::Left => std::f32::consts::PI,
            crate::game::Direction::Down => -std::f32::consts::FRAC_PI_2,
        };

        transform.rotation = Quat::from_rotation_z(rotation);
    }
}

/// System to handle pulse effects (for eaten food flash).
fn pulse_effect_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut Transform, &mut PulseEffect)>,
) {
    for (entity, mut transform, mut effect) in effects.iter_mut() {
        effect.timer.tick(time.delta());

        if effect.timer.is_finished() {
            commands.entity(entity).despawn();
        } else {
            let progress = effect.timer.fraction();
            let scale = effect.start_scale + (effect.end_scale - effect.start_scale) * progress;
            transform.scale = Vec3::splat(scale);
        }
    }
}

/// System to spawn visual effect when food is eaten.
fn spawn_food_eaten_effect(
    mut commands: Commands,
    mut food_eaten_reader: MessageReader<FoodEatenEvent>,
) {
    for event in food_eaten_reader.read() {
        let shape = shapes::Circle {
            radius: CELL_SIZE / 2.0,
            center: Vec2::ZERO,
        };

        let x = (event.position.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE;
        let y = (event.position.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE;

        commands.spawn((
            ShapeBuilder::with(&shape)
                .fill(Color::srgba(1.0, 1.0, 0.3, 0.8))
                .build(),
            Transform::from_xyz(x, y, Z_FOOD + 0.5),
            PulseEffect {
                timer: Timer::from_seconds(0.3, TimerMode::Once),
                start_scale: 1.0,
                end_scale: 2.5,
            },
        ));
    }
}

/// System to trigger camera shake on game over.
fn trigger_camera_shake_on_game_over(
    game_state: Res<GameState>,
    mut camera_shake: ResMut<CameraShake>,
) {
    // Detect transition to GameOver phase
    if game_state.is_changed() && game_state.phase == GamePhase::GameOver {
        camera_shake.timer = Timer::from_seconds(0.5, TimerMode::Once);
        camera_shake.intensity = 8.0;
    }
}

/// System to apply camera shake effect.
fn camera_shake_system(
    time: Res<Time>,
    mut camera_shake: ResMut<CameraShake>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    if !camera_shake.timer.is_finished() {
        camera_shake.timer.tick(time.delta());

        if let Ok(mut camera_transform) = camera_query.single_mut() {
            if camera_shake.timer.is_finished() {
                // Reset camera position when shake is done
                camera_transform.translation.x = 0.0;
                camera_transform.translation.y = 0.0;
            } else {
                // Apply random shake based on intensity
                let progress = camera_shake.timer.fraction();
                let decay = 1.0 - progress;

                let mut rng = rand::rng();
                let shake_x = (rng.random::<f32>() - 0.5) * camera_shake.intensity * decay;
                let shake_y = (rng.random::<f32>() - 0.5) * camera_shake.intensity * decay;

                camera_transform.translation.x = shake_x;
                camera_transform.translation.y = shake_y;
            }
        }
    }
}

/// System to animate growing segments.
fn growing_segment_animation(
    mut commands: Commands,
    time: Res<Time>,
    mut growing: Query<(Entity, &mut Transform, &mut GrowingSegment)>,
) {
    for (entity, mut transform, mut growing_segment) in growing.iter_mut() {
        growing_segment.timer.tick(time.delta());

        if growing_segment.timer.is_finished() {
            transform.scale = Vec3::splat(1.0);
            commands.entity(entity).remove::<GrowingSegment>();
        } else {
            let progress = growing_segment.timer.fraction();
            // Use ease-out for a bouncy effect
            let scale = progress * (2.0 - progress);
            transform.scale = Vec3::splat(scale);
        }
    }
}
