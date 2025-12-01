//! Snake game built with Bevy.

use bevy::{prelude::*, window::WindowResolution};
use bevy_vector_shapes::prelude::*;

mod food;
mod game;
mod rendering;
mod snake;
mod ui;

use food::FoodPlugin;
use game::{
    ARENA_HEIGHT, ARENA_WIDTH, BACKGROUND_COLOR, CELL_SIZE, CameraShake, FoodEatenEvent, GameState,
    GrowthEvent, InputBuffer, MoveTimer,
};
use rendering::RenderingPlugin;
use snake::SnakePlugin;
use ui::UiPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        (ARENA_WIDTH as f32 * CELL_SIZE + 20.0) as u32,
                        (ARENA_HEIGHT as f32 * CELL_SIZE + 20.0) as u32,
                    ),
                    title: "Snake Game".to_string(),
                    ..Default::default()
                }),
                ..default()
            }),
            Shape2dPlugin::default(),
        ))
        // Game plugins
        .add_plugins((SnakePlugin, FoodPlugin, RenderingPlugin, UiPlugin))
        // Resources
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .init_resource::<GameState>()
        .init_resource::<InputBuffer>()
        .init_resource::<MoveTimer>()
        .init_resource::<CameraShake>()
        // Events
        .add_message::<GrowthEvent>()
        .add_message::<FoodEatenEvent>()
        .run();
}
