//! Game constants for arena size, colors, timing, and rendering layers.

use bevy::prelude::*;
use std::time::Duration;

// Arena dimensions
pub const ARENA_WIDTH: u32 = 20;
pub const ARENA_HEIGHT: u32 = 20;

// Visual settings
pub const CELL_SIZE: f32 = 25.0;
pub const CORNER_RADIUS: f32 = 4.0;

/// Pixels of padding added on each side of the arena when sizing the OS window.
pub const WINDOW_PADDING: f32 = 20.0;

// Timing
pub const MOVE_INTERVAL: Duration = Duration::from_millis(150);

// Colors - using HDR values (> 1.0) for bloom glow effects.
// Head and body share the same hue so they read as one organism; head is
// pushed into HDR (>1.0 in green channel) so the bloom pass picks it up,
// while the body stays just below 1.0 — vivid but non-blooming.
pub const SNAKE_HEAD_COLOR: Color = Color::srgba(0.6, 1.5, 0.6, 1.0);
pub const SNAKE_SEGMENT_COLOR: Color = Color::srgba(0.3, 0.9, 0.3, 1.0);
pub const FOOD_COLOR: Color = Color::srgba(2.5, 0.3, 0.3, 1.0); // HDR red for glow
pub const ARENA_COLOR: Color = Color::srgba(0.08, 0.08, 0.1, 1.0);
pub const BACKGROUND_COLOR: Color = Color::srgba(0.02, 0.02, 0.03, 1.0);

// Effect colors - HDR for bloom
pub const FOOD_EATEN_COLOR: Color = Color::srgba(3.0, 3.0, 1.0, 0.8); // Bright yellow flash
pub const ARENA_BORDER_COLOR: Color = Color::srgba(0.3, 0.5, 0.8, 0.6); // Blue border glow

// Score-text exclusion zone: cells near the top-left corner that the UI overlaps.
// Food will not be spawned in the rectangle x ∈ [0, SCORE_AREA_COLS) × y ∈ (ARENA_HEIGHT - SCORE_AREA_ROWS, ARENA_HEIGHT].
pub const SCORE_AREA_COLS: i32 = 3;
pub const SCORE_AREA_ROWS: i32 = 2;

// Z-index constants for rendering layers
pub const Z_BACKGROUND: f32 = 0.0;
pub const Z_FOOD: f32 = 1.0;
pub const Z_SNAKE_SEGMENT: f32 = 1.5;
pub const Z_SNAKE_HEAD: f32 = 2.0;
