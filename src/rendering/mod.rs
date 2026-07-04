//! Rendering plugin - handles position interpolation, rotation, visual effects, and camera.

use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use rand::prelude::*;

use std::time::Duration;

use crate::game::{
    ARENA_HEIGHT, ARENA_WIDTH, CELL_SIZE, CameraShake, Direction, FOOD_EATEN_COLOR, FoodEatenEvent,
    GamePhase, GameSet, GameState, GrowingSegment, MOVE_INTERVAL, PARTICLE_COLORS, Particle,
    Position, PreviousPosition, PulseEffect, SCORE_POPUP_COLOR, ScorePopup, SnakeHead, Z_FOOD,
};

/// Plugin for rendering and visual effects.
pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                position_translation,
                update_head_rotation,
                pulse_effect_system,
                spawn_food_eaten_effect,
                particle_update,
                score_popup_update,
                camera_shake_system,
                growing_segment_animation,
                trigger_camera_shake_on_game_over,
            )
                .chain()
                .in_set(GameSet::Rendering),
        );
    }
}

// Type alias for transform interpolation query.
// Every entity matched here carries Position + PreviousPosition (snake parts
// and food). The z-layer was set at spawn time and is preserved by the
// translation system below.
type TransformInterpolationQuery<'w, 's> =
    Query<'w, 's, (&'static Position, &'static PreviousPosition, &'static mut Transform)>;

/// System to interpolate entity positions for smooth movement.
///
/// Owns its own interpolation accumulator (`Local<Duration>`). Each frame:
///   - if any snake-head's `Position` was mutated this frame (by the move-tick
///     in `snake_movement`), the accumulator is reset to zero — meaning we
///     start the new tick at progress = 0.0 with no visual snap;
///   - otherwise the accumulator advances by `time.delta()`.
///
/// This eliminates the need for a shared `MoveTimer` resource and the inter-
/// system coordination that came with it.
fn position_translation(
    mut transforms: TransformInterpolationQuery,
    head_changed: Query<(), (With<SnakeHead>, Changed<Position>)>,
    mut accum: Local<Duration>,
    time: Res<Time>,
    game_state: Res<GameState>,
) {
    // Outside of `Playing`, snap the accumulator back to zero so the next play
    // session starts cleanly, and skip the interpolation work entirely.
    if game_state.phase != GamePhase::Playing {
        *accum = Duration::ZERO;
        return;
    }

    if head_changed.is_empty() {
        *accum += time.delta();
    } else {
        *accum = Duration::ZERO;
    }

    // Calculate interpolation progress (0.0 to 1.0)
    let progress = (accum.as_secs_f32() / MOVE_INTERVAL.as_secs_f32()).min(1.0);

    for (pos, prev_pos, mut transform) in &mut transforms {
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

        // Preserve z (set once at spawn) — only update x/y.
        transform.translation.x = prev_x + dx * progress;
        transform.translation.y = prev_y + dy * progress;
    }
}

/// System to update snake head rotation based on direction.
///
/// Only runs while playing — after death the head's direction is fixed and
/// rewriting the same Quat every frame would be busywork.
fn update_head_rotation(
    mut heads: Query<(&SnakeHead, &mut Transform), Changed<SnakeHead>>,
) {
    for (head, mut transform) in &mut heads {
        let rotation = match head.direction {
            Direction::Right => 0.0,
            Direction::Up => std::f32::consts::FRAC_PI_2,
            Direction::Left => std::f32::consts::PI,
            Direction::Down => -std::f32::consts::FRAC_PI_2,
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
        let radius = CELL_SIZE / 2.0;
        let x = (event.position.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE;
        let y = (event.position.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE;

        // Main bright flash with HDR color for bloom glow
        commands.spawn((
            ShapeBundle::circle(
                &ShapeConfig {
                    color: FOOD_EATEN_COLOR,
                    alpha_mode: ShapeAlphaMode::Add, // Additive blending for bright glow
                    transform: Transform::from_xyz(x, y, Z_FOOD + 0.5),
                    ..ShapeConfig::default_2d()
                },
                radius,
            ),
            PulseEffect {
                timer: Timer::from_seconds(0.30, TimerMode::Once),
                start_scale: 0.8,
                end_scale: 2.4,
            },
        ));

        // Secondary ring effect for extra visual punch
        commands.spawn((
            ShapeBundle::circle(
                &ShapeConfig {
                    color: Color::srgba(2.0, 1.5, 0.5, 0.5),
                    alpha_mode: ShapeAlphaMode::Add,
                    hollow: true,
                    thickness: 3.0,
                    transform: Transform::from_xyz(x, y, Z_FOOD + 0.4),
                    ..ShapeConfig::default_2d()
                },
                radius * 0.8,
            ),
            PulseEffect {
                timer: Timer::from_seconds(0.4, TimerMode::Once),
                start_scale: 0.5,
                end_scale: 4.0,
            },
        ));

        // Juice burst: little HDR droplets that fly out, slow down, and
        // fade — the main "crunch" feedback for eating an apple.
        let mut rng = rand::rng();
        for _ in 0..18 {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let speed = rng.random_range(50.0..170.0);
            let color = PARTICLE_COLORS[rng.random_range(0..PARTICLE_COLORS.len())];
            commands.spawn((
                ShapeBundle::circle(
                    &ShapeConfig {
                        color,
                        alpha_mode: ShapeAlphaMode::Add,
                        transform: Transform::from_xyz(x, y, Z_FOOD + 0.6),
                        ..ShapeConfig::default_2d()
                    },
                    rng.random_range(2.0..4.5),
                ),
                Particle {
                    velocity: Vec2::from_angle(angle) * speed,
                    timer: Timer::from_seconds(rng.random_range(0.35..0.6), TimerMode::Once),
                },
            ));
        }

        // Floating "+1" over the bite.
        commands.spawn((
            Text2d::new("+1"),
            TextFont {
                font_size: FontSize::Px(24.0),
                weight: bevy::text::FontWeight::BOLD,
                ..default()
            },
            TextColor(SCORE_POPUP_COLOR),
            Transform::from_xyz(x, y + CELL_SIZE * 0.3, Z_FOOD + 0.7),
            ScorePopup {
                timer: Timer::from_seconds(0.7, TimerMode::Once),
            },
        ));
    }
}

/// Moves, decelerates, shrinks, and fades the food-eaten juice droplets.
fn particle_update(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Transform, &mut Particle, &mut ShapeFill)>,
) {
    for (entity, mut transform, mut particle, mut fill) in particles.iter_mut() {
        particle.timer.tick(time.delta());

        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let dt = time.delta_secs();
        transform.translation += (particle.velocity * dt).extend(0.0);
        // Drag so the burst blooms outward then hangs for a beat.
        particle.velocity *= (1.0 - 3.0 * dt).max(0.0);

        let t = particle.timer.fraction();
        transform.scale = Vec3::splat(1.0 - t * t);
        fill.color.set_alpha(1.0 - t);
    }
}

/// Floats the "+1" popup upward while fading it out.
fn score_popup_update(
    mut commands: Commands,
    time: Res<Time>,
    mut popups: Query<(Entity, &mut Transform, &mut TextColor, &mut ScorePopup)>,
) {
    for (entity, mut transform, mut color, mut popup) in popups.iter_mut() {
        popup.timer.tick(time.delta());

        if popup.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let t = popup.timer.fraction();
        // Rise fast at first, then ease off as it fades.
        transform.translation.y += 45.0 * time.delta_secs() * (1.0 - t * 0.6);
        color.0.set_alpha(1.0 - t * t);
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
