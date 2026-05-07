//! System sets that establish execution order across all plugins.
//!
//! Guaranteed order every frame:
//!   Movement → Collision → Effects → Rendering → Ui

use bevy::prelude::*;

/// Top-level system sets for the snake game, executed in declaration order.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Move-tick accumulator, snake input reading, and snake movement.
    Movement,
    /// Food collision detection.
    Collision,
    /// Post-collision effects: snake growth, game-over check.
    Effects,
    /// Position interpolation, rotation, particle effects, camera shake.
    Rendering,
    /// Game-flow UI: start/restart input, score text, overlays.
    /// Runs last so all reads see the fully resolved frame state.
    Ui,
}
