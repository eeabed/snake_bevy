use bevy::{
    ecs::system::ParamSet, prelude::*, time::common_conditions::on_timer, window::WindowResolution,
};
use rand::prelude::*;
use std::time::Duration;

// Game constants
const ARENA_WIDTH: u32 = 20;
const ARENA_HEIGHT: u32 = 20;
const SNAKE_HEAD_COLOR: Color = Color::srgba(0.9, 0.9, 0.9, 1.0); // Brighter head color
const SNAKE_SEGMENT_COLOR: Color = Color::srgba(0.5, 0.5, 0.5, 1.0); // Brighter segment color
const FOOD_COLOR: Color = Color::srgba(1.0, 0.0, 0.0, 1.0);
const ARENA_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 1.0);
const BACKGROUND_COLOR: Color = Color::srgba(0.04, 0.04, 0.04, 1.0);
const CELL_SIZE: f32 = 25.0;
const MOVE_INTERVAL: Duration = Duration::from_millis(150);

// Component to mark the snake's head
#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

// Component to mark snake body segments
#[derive(Component)]
struct SnakeSegment;

// Component to mark the food
#[derive(Component)]
struct Food;

// Grid position component
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
struct Position {
    x: i32,
    y: i32,
}

// Direction enum
#[derive(PartialEq, Copy, Clone, Debug)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn opposite(&self) -> Self {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}

// Game state resource
#[derive(Resource)]
struct GameState {
    snake_segments: Vec<Entity>,
    score: usize,
    game_over: bool,
    just_eaten: bool,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            snake_segments: Vec::new(),
            score: 0,
            game_over: false,
            just_eaten: false,
        }
    }
}

// Event triggered when snake moves
#[derive(Event)]
struct GrowthEvent;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(
                    ARENA_WIDTH as f32 * CELL_SIZE + 20.0,
                    ARENA_HEIGHT as f32 * CELL_SIZE + 20.0,
                ),
                title: "Snake Game".to_string(),
                ..Default::default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .init_resource::<GameState>()
        .add_event::<GrowthEvent>()
        .add_systems(Startup, setup_system)
        .add_systems(
            Update,
            (
                snake_movement_input.before(snake_movement),
                snake_movement.run_if(on_timer(MOVE_INTERVAL)),
                food_collision.after(snake_movement),
                snake_growth.after(food_collision),
                position_translation,
                game_over_check.after(snake_movement),
                restart_game,
                update_score_text.after(food_collision),
            ),
        )
        .run();
}

fn setup_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    asset_server: Res<AssetServer>,
) {
    // Setup camera
    commands.spawn(Camera2d::default());

    // Arena background
    commands.spawn((
        Sprite {
            color: ARENA_COLOR,
            custom_size: Some(Vec2::new(
                ARENA_WIDTH as f32 * CELL_SIZE,
                ARENA_HEIGHT as f32 * CELL_SIZE,
            )),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    ));

    // Score text
    commands
        .spawn((
            Text::from("Score: 0"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 20.0,
                font_smoothing: Default::default(),
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ScoreText,
        ));

    // Spawn initial snake
    game_state.snake_segments.clear();
    game_state.score = 0;
    game_state.game_over = false;

    let head_entity = spawn_snake_head(&mut commands);
    game_state.snake_segments.push(head_entity);

    // Print debug info
    println!("Snake head spawned at position: (3, 3)");

    // Spawn initial food
    spawn_food(&mut commands);

    // Force immediate position update
    commands.queue(move |world: &mut World| {
        let mut position_query = world.query::<(&Position, &mut Transform)>();
        for (pos, mut transform) in position_query.iter_mut(world) {
            transform.translation = Vec3::new(
                (pos.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE,
                (pos.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE,
                1.0, // Set z-index to 1.0 to ensure it renders above background
            );
        }
        println!("Position translation applied immediately");
    });
}

#[derive(Component)]
struct ScoreText;

fn spawn_snake_head(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Sprite {
                color: SNAKE_HEAD_COLOR,
                custom_size: Some(Vec2::new(CELL_SIZE * 0.9, CELL_SIZE * 0.9)), // Slightly smaller for visual clarity
                ..default()
            },
            // Pre-position it properly in the center of the grid
            Transform::from_xyz(
                (3.0 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE,
                (3.0 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE,
                2.0, // Higher z-index to ensure visibility
            ),
            SnakeHead {
                direction: Direction::Right,
            },
            Position { x: 3, y: 3 },
        ))
        .id()
}

fn spawn_snake_segment(commands: &mut Commands, position: Position) -> Entity {
    commands
        .spawn((
            Sprite {
                color: SNAKE_SEGMENT_COLOR,
                custom_size: Some(Vec2::new(CELL_SIZE, CELL_SIZE)),
                ..default()
            },
            Transform::default(),
            SnakeSegment,
            position,
        ))
        .id()
}

fn spawn_food(commands: &mut Commands) {
    let mut rng = rand::thread_rng();
    let x = rng.gen_range(0..ARENA_WIDTH) as i32;
    let y = rng.gen_range(0..ARENA_HEIGHT) as i32;

    commands
        .spawn((
            Sprite {
                color: FOOD_COLOR,
                custom_size: Some(Vec2::new(CELL_SIZE, CELL_SIZE)),
                ..default()
            },
            Transform::default(),
            Food,
            Position { x, y },
        ));
}

fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut heads: Query<&mut SnakeHead>,
) {
    if let Some(mut head) = heads.iter_mut().next() {
        let dir = if keyboard_input.pressed(KeyCode::ArrowLeft)
            || keyboard_input.pressed(KeyCode::KeyA)
        {
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
            head.direction
        };

        // Prevent the snake from reversing direction
        if dir != head.direction.opposite() {
            head.direction = dir;
        }
    }
}

fn snake_movement(
    game_state: ResMut<GameState>,
    mut query_set: ParamSet<(
        Query<(Entity, &SnakeHead, &mut Position)>,
        Query<&mut Position>,
    )>,
    _segments: Query<Entity, With<SnakeSegment>>,
) {
    if game_state.game_over {
        return;
    }

    // First, get the head entity and its current direction and position
    let (head_entity, head_direction, head_position) = {
        let mut heads_query = query_set.p0();
        if let Some((entity, head, position)) = heads_query.iter_mut().next() {
            (entity, head.direction, *position)
        } else {
            return; // No head found, exit early
        }
    };

    // Record the current position of each segment
    let segments_positions = {
        let mut positions = Vec::new();
        let positions_query = query_set.p1();

        for &segment_entity in &game_state.snake_segments {
            if segment_entity == head_entity {
                positions.push(head_position);
            } else if let Ok(segment_pos) = positions_query.get(segment_entity) {
                positions.push(*segment_pos);
            }
        }
        positions
    };

    // Now update the head position
    {
        let mut heads_query = query_set.p0();
        if let Some((_, _, mut head_pos)) = heads_query.iter_mut().next() {
            // Move the head
            match head_direction {
                Direction::Left => head_pos.x -= 1,
                Direction::Right => head_pos.x += 1,
                Direction::Up => head_pos.y += 1,
                Direction::Down => head_pos.y -= 1,
            }

            // Wrap around if the snake goes off the edge
            head_pos.x = (head_pos.x + ARENA_WIDTH as i32) % ARENA_WIDTH as i32;
            head_pos.y = (head_pos.y + ARENA_HEIGHT as i32) % ARENA_HEIGHT as i32;
        }
    }

    // Move the rest of the snake
    {
        let mut positions_query = query_set.p1();
        for (i, segment_entity) in game_state.snake_segments.iter().skip(1).enumerate() {
            if let Ok(mut segment_pos) = positions_query.get_mut(*segment_entity) {
                *segment_pos = segments_positions[i];
            }
        }
    }
}

fn food_collision(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    mut game_state: ResMut<GameState>,
    head_positions: Query<&Position, With<SnakeHead>>,
    food_positions: Query<(Entity, &Position), With<Food>>,
) {
    if game_state.game_over {
        return;
    }

    if let Some(head_pos) = head_positions.iter().next() {
        for (food_entity, food_pos) in food_positions.iter() {
            if head_pos.x == food_pos.x && head_pos.y == food_pos.y {
                commands.entity(food_entity).despawn();
                game_state.just_eaten = true;
                game_state.score += 1;
                growth_writer.send(GrowthEvent);
                spawn_food(&mut commands);
            }
        }
    }
}

fn snake_growth(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut growth_reader: EventReader<GrowthEvent>,
    positions: Query<&Position>,
) {
    if growth_reader.read().next().is_some() {
        if let Some(&last_segment_entity) = game_state.snake_segments.last() {
            if let Ok(last_pos) = positions.get(last_segment_entity) {
                let new_segment = spawn_snake_segment(&mut commands, *last_pos);
                game_state.snake_segments.push(new_segment);
            }
        }
    }
}

fn position_translation(
    mut transforms: Query<(
        &Position,
        &mut Transform,
        Option<&SnakeHead>,
        Option<&SnakeSegment>,
        Option<&Food>,
    )>,
) {
    for (pos, mut transform, head, segment, food) in transforms.iter_mut() {
        // Set z-index based on entity type to ensure proper layering
        let z = if head.is_some() {
            2.0 // Snake head on top
        } else if segment.is_some() {
            1.5 // Snake segments in middle
        } else if food.is_some() {
            1.0 // Food above background
        } else {
            0.0 // Background
        };

        transform.translation = Vec3::new(
            (pos.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE,
            (pos.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE,
            z,
        );
    }
}

fn game_over_check(
    mut game_state: ResMut<GameState>,
    head_positions: Query<&Position, With<SnakeHead>>,
    segment_positions: Query<(&Position, Entity), With<SnakeSegment>>,
) {
    if game_state.game_over {
        return;
    }

    if let Some(head_pos) = head_positions.iter().next() {
        for (segment_pos, segment_entity) in segment_positions.iter() {
            if head_pos.x == segment_pos.x && head_pos.y == segment_pos.y {
                if game_state.snake_segments.len() > 1
                    && game_state.snake_segments[1] != segment_entity
                {
                    game_state.game_over = true;
                    println!("Game Over! Final score: {}", game_state.score);
                }
            }
        }
    }
}

fn restart_game(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    segments: Query<Entity, Or<(With<SnakeSegment>, With<SnakeHead>)>>,
    food: Query<Entity, With<Food>>,
    _asset_server: Res<AssetServer>,
) {
    if game_state.game_over && keyboard_input.just_pressed(KeyCode::Space) {
        // Despawn all existing snake segments and food
        for entity in segments.iter().chain(food.iter()) {
            commands.entity(entity).despawn();
        }

        // Reset game state
        game_state.snake_segments.clear();
        game_state.score = 0;
        game_state.game_over = false;

        // Spawn new snake head
        let head_entity = spawn_snake_head(&mut commands);
        game_state.snake_segments.push(head_entity);

        // Spawn new food
        spawn_food(&mut commands);
    }
}

fn update_score_text(game_state: Res<GameState>, mut query: Query<&mut Text, With<ScoreText>>) {
    if let Ok(mut text) = query.get_single_mut() {
        *text = Text::from(format!("Score: {}", game_state.score));
    }
}
