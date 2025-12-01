//! Game resources (singleton state).

use bevy::prelude::*;
use std::time::Duration;

use super::Direction;

/// Game phase enum to track which state the game is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GamePhase {
    #[default]
    Menu,
    Playing,
    GameOver,
}

/// Main game state resource.
#[derive(Resource)]
pub struct GameState {
    pub snake_segments: Vec<Entity>,
    pub score: usize,
    pub game_over: bool,
    pub phase: GamePhase,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            snake_segments: Vec::new(),
            score: 0,
            game_over: false,
            phase: GamePhase::Menu,
        }
    }
}

/// Input buffer to queue direction changes.
#[derive(Resource, Default)]
pub struct InputBuffer {
    queued_directions: Vec<Direction>,
}

impl InputBuffer {
    /// Queue a direction change (max 2 buffered inputs).
    pub fn queue_direction(&mut self, direction: Direction) {
        if self.queued_directions.len() < 2 {
            self.queued_directions.push(direction);
        }
    }

    /// Pop the next queued direction.
    pub fn pop_direction(&mut self) -> Option<Direction> {
        if !self.queued_directions.is_empty() {
            Some(self.queued_directions.remove(0))
        } else {
            None
        }
    }

    /// Get the last queued direction without removing it.
    pub fn last_direction(&self) -> Option<Direction> {
        self.queued_directions.last().copied()
    }

    /// Clear all queued directions.
    pub fn clear(&mut self) {
        self.queued_directions.clear();
    }
}

/// Resource to track time since last move for interpolation.
#[derive(Resource)]
pub struct MoveTimer {
    pub elapsed: Duration,
}

impl Default for MoveTimer {
    fn default() -> Self {
        MoveTimer {
            elapsed: Duration::ZERO,
        }
    }
}

/// Resource for camera shake effect.
#[derive(Resource)]
pub struct CameraShake {
    pub timer: Timer,
    pub intensity: f32,
}

impl Default for CameraShake {
    fn default() -> Self {
        CameraShake {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            intensity: 0.0,
        }
    }
}
