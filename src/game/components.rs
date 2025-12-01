//! ECS components for the snake game.

use bevy::prelude::*;

/// Grid position component for entities on the arena.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    /// Check if this position collides with another position.
    pub fn collides_with(&self, other: &Position) -> bool {
        self.x == other.x && self.y == other.y
    }
}

/// Component to track previous position for smooth interpolation.
#[derive(Component, Clone, Copy, Debug)]
pub struct PreviousPosition {
    pub pos: Position,
}

/// Direction enum for snake movement.
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// Returns the opposite direction.
    pub fn opposite(&self) -> Self {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }

    /// Reads keyboard input and returns the corresponding direction.
    pub fn from_input(keyboard_input: &ButtonInput<KeyCode>, current: Direction) -> Direction {
        if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::ArrowRight)
            || keyboard_input.pressed(KeyCode::KeyD)
        {
            Direction::Right
        } else if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW)
        {
            Direction::Up
        } else if keyboard_input.pressed(KeyCode::ArrowDown)
            || keyboard_input.pressed(KeyCode::KeyS)
        {
            Direction::Down
        } else {
            current
        }
    }
}

/// Component to mark the snake's head.
#[derive(Component)]
pub struct SnakeHead {
    pub direction: Direction,
}

/// Component to mark snake head eyes (children of head).
#[derive(Component)]
pub struct SnakeEye;

/// Component to mark snake body segments.
#[derive(Component)]
pub struct SnakeSegment;

/// Component to mark food entities.
#[derive(Component)]
pub struct Food;

/// Component for food pulsing animation.
#[derive(Component)]
pub struct FoodPulse {
    pub timer: Timer,
}

/// Component for entities that should flash/pulse.
#[derive(Component)]
pub struct PulseEffect {
    pub timer: Timer,
    pub start_scale: f32,
    pub end_scale: f32,
}

/// Component for animating newly grown segments.
#[derive(Component)]
pub struct GrowingSegment {
    pub timer: Timer,
}

/// Component to mark the score display UI element.
#[derive(Component)]
pub struct ScoreText;

/// Component to mark the game over overlay UI.
#[derive(Component)]
pub struct GameOverUI;

/// Component to mark the start menu UI.
#[derive(Component)]
pub struct MenuUI;
