//! Game events (messages).

use bevy::prelude::*;

use super::Position;

/// Message triggered when snake should grow.
#[derive(Message)]
pub struct GrowthEvent;

/// Message triggered when food is eaten (for visual effects).
#[derive(Message)]
pub struct FoodEatenEvent {
    pub position: Position,
}

/// Message written by the UI action buttons (START / RESTART / PLAY AGAIN)
/// requesting a new game. Handled by the same systems that handle the
/// SPACE key, so buttons and keyboard share one start/restart code path.
#[derive(Message)]
pub struct StartRequested;
