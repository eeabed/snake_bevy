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
