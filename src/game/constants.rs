//! Game constants for arena size, colors, timing, and rendering layers.

use bevy::prelude::*;
use std::time::Duration;

use super::Position;

// Arena dimensions
pub const ARENA_WIDTH: u32 = 20;
pub const ARENA_HEIGHT: u32 = 20;

// Visual settings
pub const CELL_SIZE: f32 = 25.0;
pub const CORNER_RADIUS: f32 = 4.0;

// Timing
pub const MOVE_INTERVAL: Duration = Duration::from_millis(150);

// Initial positions
pub const INITIAL_SNAKE_POSITION: Position = Position { x: 3, y: 3 };

// Colors - using HDR values (> 1.0) for bloom glow effects
pub const SNAKE_HEAD_COLOR: Color = Color::srgba(1.2, 1.2, 1.2, 1.0);
pub const SNAKE_SEGMENT_COLOR: Color = Color::srgba(0.4, 0.6, 0.4, 1.0);
pub const FOOD_COLOR: Color = Color::srgba(2.5, 0.3, 0.3, 1.0); // HDR red for glow
pub const ARENA_COLOR: Color = Color::srgba(0.08, 0.08, 0.1, 1.0);
pub const BACKGROUND_COLOR: Color = Color::srgba(0.02, 0.02, 0.03, 1.0);

// Effect colors - HDR for bloom
pub const FOOD_EATEN_COLOR: Color = Color::srgba(3.0, 3.0, 1.0, 0.8); // Bright yellow flash
pub const SNAKE_HEAD_GLOW_COLOR: Color = Color::srgba(0.5, 1.5, 0.5, 0.4); // Subtle green glow
pub const ARENA_BORDER_COLOR: Color = Color::srgba(0.3, 0.5, 0.8, 0.6); // Blue border glow

// Z-index constants for rendering layers
pub const Z_BACKGROUND: f32 = 0.0;
pub const Z_FOOD: f32 = 1.0;
pub const Z_SNAKE_SEGMENT: f32 = 1.5;
pub const Z_SNAKE_HEAD: f32 = 2.0;
