//! System sets that establish execution order across all plugins.
//!
//! Guaranteed order every frame:
//!   Movement → Collision → Effects → Rendering

use bevy::prelude::*;

/// Top-level system sets for the snake game, executed in declaration order.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Snake input reading and movement.
    Movement,
    /// Food collision detection.
    Collision,
    /// Post-collision effects: snake growth, game-over check.
    Effects,
    /// Position interpolation, animations, camera, and UI updates.
    Rendering,
}
