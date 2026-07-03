//! ECS components for the snake game.

use bevy::prelude::*;

/// Grid position component for entities on the arena.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Initial spawn cell for the snake's head. Lives next to `Position` so the
/// constants module doesn't need to import a component type.
pub const INITIAL_SNAKE_POSITION: Position = Position { x: 3, y: 3 };

/// Component to track previous position for smooth interpolation.
#[derive(Component, Clone, Copy, Debug)]
pub struct PreviousPosition {
    pub pos: Position,
}

/// Direction enum for snake movement.
#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
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

// The UI markers below derive `Default + Clone` in addition to `Component`
// because they are spawned through `bsn!` scenes, whose template machinery
// requires both.

/// Component to mark the score display UI element.
#[derive(Component, Default, Clone)]
pub struct ScoreText;

/// Component to mark the game over overlay UI.
#[derive(Component, Default, Clone)]
pub struct GameOverUI;

/// Component to mark the win-screen overlay UI.
#[derive(Component, Default, Clone)]
pub struct WinUI;

/// Component to mark the start menu UI.
#[derive(Component, Default, Clone)]
pub struct MenuUI;
