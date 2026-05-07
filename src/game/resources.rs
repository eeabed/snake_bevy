//! Game resources (singleton state).

use bevy::prelude::*;
use std::collections::VecDeque;

use super::Direction;

/// Maximum number of direction changes that can be queued at once.
pub const INPUT_BUFFER_CAPACITY: usize = 2;

/// Game phase enum to track which state the game is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GamePhase {
    #[default]
    Menu,
    Playing,
    GameOver,
    /// Player filled the entire arena with the snake — win condition.
    Won,
}

/// Main game state resource.
#[derive(Resource)]
pub struct GameState {
    pub snake_segments: Vec<Entity>,
    pub score: usize,
    pub phase: GamePhase,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            snake_segments: Vec::new(),
            score: 0,
            phase: GamePhase::Menu,
        }
    }
}

/// Input buffer to queue direction changes.
#[derive(Resource, Default)]
pub struct InputBuffer {
    queued_directions: VecDeque<Direction>,
}

impl InputBuffer {
    /// Queue a direction change. Drops the input if the buffer is full
    /// (capacity is [`INPUT_BUFFER_CAPACITY`]).
    pub fn queue_direction(&mut self, direction: Direction) {
        if self.queued_directions.len() < INPUT_BUFFER_CAPACITY {
            self.queued_directions.push_back(direction);
        }
    }

    /// Pop the next queued direction.
    pub fn pop_direction(&mut self) -> Option<Direction> {
        self.queued_directions.pop_front()
    }

    /// Get the last queued direction without removing it.
    pub fn last_direction(&self) -> Option<Direction> {
        self.queued_directions.back().copied()
    }

    /// Clear all queued directions.
    pub fn clear(&mut self) {
        self.queued_directions.clear();
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
