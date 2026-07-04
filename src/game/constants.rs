//! Game constants for arena size, colors, timing, and rendering layers.

use bevy::prelude::*;
use std::time::Duration;

// Arena dimensions
pub const ARENA_WIDTH: u32 = 20;
pub const ARENA_HEIGHT: u32 = 20;

// Visual settings
pub const CELL_SIZE: f32 = 25.0;

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
/// Checkerboard tint for alternating arena cells — barely lighter than
/// `ARENA_COLOR` so the grid reads without competing with the pieces.
pub const ARENA_COLOR_ALT: Color = Color::srgba(0.095, 0.095, 0.12, 1.0);
pub const BACKGROUND_COLOR: Color = Color::srgba(0.02, 0.02, 0.03, 1.0);

// Apple detailing
pub const APPLE_STEM_COLOR: Color = Color::srgba(0.45, 0.28, 0.12, 1.0);
pub const APPLE_LEAF_COLOR: Color = Color::srgba(0.35, 1.05, 0.4, 1.0); // just-HDR green
pub const APPLE_HIGHLIGHT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.35);

// Snake detailing
pub const TONGUE_COLOR: Color = Color::srgba(0.95, 0.25, 0.3, 1.0);

// Effect colors - HDR for bloom
// Lower alpha + smaller end-scale than the old flash: it should read as a
// pop, not a blowout that hides the juice particles and the "+1".
pub const FOOD_EATEN_COLOR: Color = Color::srgba(3.0, 3.0, 1.0, 0.55);
pub const ARENA_BORDER_COLOR: Color = Color::srgba(0.3, 0.5, 0.8, 0.25); // Subtle blue border
pub const SCORE_POPUP_COLOR: Color = Color::srgba(1.8, 1.5, 0.5, 1.0); // HDR gold
/// Juice-droplet palette for the food-eaten particle burst (all HDR).
pub const PARTICLE_COLORS: [Color; 3] = [
    Color::srgba(2.5, 0.4, 0.3, 1.0),
    Color::srgba(2.2, 1.2, 0.3, 1.0),
    Color::srgba(1.8, 0.5, 0.2, 1.0),
];

// Score-text exclusion zone: cells near the top-left corner that the UI overlaps.
// Food will not be spawned in the rectangle x ∈ [0, SCORE_AREA_COLS) × y ∈ (ARENA_HEIGHT - SCORE_AREA_ROWS, ARENA_HEIGHT].
pub const SCORE_AREA_COLS: i32 = 3;
pub const SCORE_AREA_ROWS: i32 = 2;

// Z-index constants for rendering layers
pub const Z_BACKGROUND: f32 = 0.0;
pub const Z_FOOD: f32 = 1.0;
pub const Z_SNAKE_SEGMENT: f32 = 1.5;
pub const Z_SNAKE_HEAD: f32 = 2.0;
