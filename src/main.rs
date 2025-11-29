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
const INITIAL_SNAKE_POSITION: Position = Position { x: 3, y: 3 };

// Z-index constants for rendering layers
const Z_BACKGROUND: f32 = 0.0;
const Z_FOOD: f32 = 1.0;
const Z_SNAKE_SEGMENT: f32 = 1.5;
const Z_SNAKE_HEAD: f32 = 2.0;

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

impl Position {
    fn collides_with(&self, other: &Position) -> bool {
        self.x == other.x && self.y == other.y
    }
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

    fn from_input(keyboard_input: &ButtonInput<KeyCode>, current: Direction) -> Direction {
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

// Game state resource
#[derive(Resource)]
struct GameState {
    snake_segments: Vec<Entity>,
    score: usize,
    game_over: bool,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            snake_segments: Vec::new(),
            score: 0,
            game_over: false,
        }
    }
}

// Input buffer to queue direction changes
#[derive(Resource, Default)]
struct InputBuffer {
    queued_directions: Vec<Direction>,
}

impl InputBuffer {
    fn queue_direction(&mut self, direction: Direction) {
        // Only store up to 2 buffered inputs
        if self.queued_directions.len() < 2 {
            self.queued_directions.push(direction);
        }
    }

    fn pop_direction(&mut self) -> Option<Direction> {
        if !self.queued_directions.is_empty() {
            Some(self.queued_directions.remove(0))
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.queued_directions.clear();
    }
}

// Message triggered when snake grows
#[derive(Message)]
struct GrowthEvent;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(
                    (ARENA_WIDTH as f32 * CELL_SIZE + 20.0) as u32,
                    (ARENA_HEIGHT as f32 * CELL_SIZE + 20.0) as u32,
                ),
                title: "Snake Game".to_string(),
                ..Default::default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .init_resource::<GameState>()
        .init_resource::<InputBuffer>()
        .add_message::<GrowthEvent>()
        .add_systems(Startup, setup_system)
        .add_systems(
            Update,
            (
                snake_movement_input,
                snake_movement.run_if(on_timer(MOVE_INTERVAL)),
                food_collision,
                snake_growth,
                position_translation,
                game_over_check,
                restart_game,
                update_score_text,
            )
                .chain(),
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
    commands.spawn((
        Text::from("Score: 0"),
        TextFont {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 20.0,
            font_smoothing: Default::default(),
            ..default()
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

    // Spawn initial food (pass initial snake position)
    spawn_food(&mut commands, &[INITIAL_SNAKE_POSITION]);

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
    });
}

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct GameOverUI;

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
            INITIAL_SNAKE_POSITION,
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

fn spawn_food(commands: &mut Commands, snake_positions: &[Position]) {
    let mut rng = rand::rng();
    let mut position;

    // Keep generating positions until we find one that doesn't overlap with the snake or score display
    loop {
        position = Position {
            x: rng.random_range(0..ARENA_WIDTH as i32),
            y: rng.random_range(0..ARENA_HEIGHT as i32),
        };

        // Exclude top-left area where score is displayed (roughly 3x2 cells)
        let is_score_area = position.x <= 2 && position.y >= (ARENA_HEIGHT as i32 - 2);

        // Check if this position overlaps with any snake segment or the score area
        if !snake_positions.contains(&position) && !is_score_area {
            break;
        }
    }

    commands.spawn((
        Sprite {
            color: FOOD_COLOR,
            custom_size: Some(Vec2::new(CELL_SIZE, CELL_SIZE)),
            ..default()
        },
        Transform::default(),
        Food,
        position,
    ));
}

fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut input_buffer: ResMut<InputBuffer>,
    heads: Query<&SnakeHead>,
) {
    if let Some(head) = heads.iter().next() {
        // Get the last direction in buffer or current head direction
        let last_direction = input_buffer
            .queued_directions
            .last()
            .copied()
            .unwrap_or(head.direction);

        // Get new direction from input
        let new_direction = Direction::from_input(&keyboard_input, last_direction);

        // If direction changed and it's not opposite to the last direction, queue it
        if new_direction != last_direction && new_direction != last_direction.opposite() {
            input_buffer.queue_direction(new_direction);
        }
    }
}

fn snake_movement(
    game_state: ResMut<GameState>,
    mut input_buffer: ResMut<InputBuffer>,
    mut query_set: ParamSet<(
        Query<(Entity, &mut SnakeHead, &mut Position)>,
        Query<&mut Position>,
    )>,
    _segments: Query<Entity, With<SnakeSegment>>,
) {
    if game_state.game_over {
        return;
    }

    // Step 1: Get the head entity and its current direction and position
    // Also consume buffered input if available
    // We use ParamSet because we need to query positions mutably in multiple steps
    let (head_entity, head_direction, head_position) = {
        let mut heads_query = query_set.p0();
        if let Some((entity, mut head, position)) = heads_query.iter_mut().next() {
            // Try to consume buffered direction
            if let Some(buffered_direction) = input_buffer.pop_direction() {
                head.direction = buffered_direction;
            }
            (entity, head.direction, *position)
        } else {
            return; // No head found, exit early
        }
    };

    // Step 2: Record the current position of each segment before any movement
    // This is the classic "snake movement" pattern: each segment moves to where
    // the segment in front of it was
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

    // Step 3: Move the head in the current direction
    {
        let mut heads_query = query_set.p0();
        if let Some((_, _, mut head_pos)) = heads_query.iter_mut().next() {
            // Move the head one cell in the current direction
            match head_direction {
                Direction::Left => head_pos.x -= 1,
                Direction::Right => head_pos.x += 1,
                Direction::Up => head_pos.y += 1,
                Direction::Down => head_pos.y -= 1,
            }

            // Wrap around if the snake goes off the edge (creates a toroidal arena)
            head_pos.x = (head_pos.x + ARENA_WIDTH as i32) % ARENA_WIDTH as i32;
            head_pos.y = (head_pos.y + ARENA_HEIGHT as i32) % ARENA_HEIGHT as i32;
        }
    }

    // Step 4: Move each body segment to the position of the segment in front of it
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
    mut growth_writer: MessageWriter<GrowthEvent>,
    mut game_state: ResMut<GameState>,
    head_positions: Query<&Position, With<SnakeHead>>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    all_snake_positions: Query<&Position, Or<(With<SnakeHead>, With<SnakeSegment>)>>,
) {
    if game_state.game_over {
        return;
    }

    if let Some(head_pos) = head_positions.iter().next() {
        for (food_entity, food_pos) in food_positions.iter() {
            if head_pos.collides_with(food_pos) {
                commands.entity(food_entity).despawn();
                game_state.score += 1;
                growth_writer.write(GrowthEvent);

                // Collect all snake positions to avoid spawning food on the snake
                let snake_positions: Vec<Position> = all_snake_positions.iter().copied().collect();
                spawn_food(&mut commands, &snake_positions);
            }
        }
    }
}

fn snake_growth(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut growth_reader: MessageReader<GrowthEvent>,
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
            Z_SNAKE_HEAD
        } else if segment.is_some() {
            Z_SNAKE_SEGMENT
        } else if food.is_some() {
            Z_FOOD
        } else {
            Z_BACKGROUND
        };

        transform.translation = Vec3::new(
            (pos.x as f32 - ARENA_WIDTH as f32 / 2.0 + 0.5) * CELL_SIZE,
            (pos.y as f32 - ARENA_HEIGHT as f32 / 2.0 + 0.5) * CELL_SIZE,
            z,
        );
    }
}

fn game_over_check(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    head_positions: Query<&Position, With<SnakeHead>>,
    segment_positions: Query<(&Position, Entity), With<SnakeSegment>>,
    asset_server: Res<AssetServer>,
) {
    if game_state.game_over {
        return;
    }

    if let Some(head_pos) = head_positions.iter().next() {
        for (segment_pos, segment_entity) in segment_positions.iter() {
            if head_pos.collides_with(segment_pos) {
                if game_state.snake_segments.len() > 1
                    && game_state.snake_segments[1] != segment_entity
                {
                    game_state.game_over = true;
                    println!("Game Over! Final score: {}", game_state.score);

                    // Spawn game over overlay
                    spawn_game_over_screen(&mut commands, &asset_server, game_state.score);
                }
            }
        }
    }
}

fn spawn_game_over_screen(commands: &mut Commands, asset_server: &Res<AssetServer>, score: usize) {
    // Semi-transparent dark overlay
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            GameOverUI,
        ))
        .with_children(|parent| {
            // "GAME OVER" text
            parent.spawn((
                Text::from("GAME OVER"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 0.3, 0.3, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Final score text
            parent.spawn((
                Text::from(format!("Final Score: {}", score)),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ));

            // Restart instructions
            parent.spawn((
                Text::from("Press SPACE to restart"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            ));
        });
}

fn restart_game(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut input_buffer: ResMut<InputBuffer>,
    segments: Query<Entity, Or<(With<SnakeSegment>, With<SnakeHead>)>>,
    food: Query<Entity, With<Food>>,
    game_over_ui: Query<Entity, With<GameOverUI>>,
    _asset_server: Res<AssetServer>,
) {
    if game_state.game_over && keyboard_input.just_pressed(KeyCode::Space) {
        // Despawn all existing snake segments and food
        for entity in segments.iter().chain(food.iter()) {
            commands.entity(entity).despawn();
        }

        // Despawn game over UI (despawn children first, then parent)
        for entity in game_over_ui.iter() {
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
        }

        // Reset game state
        game_state.snake_segments.clear();
        game_state.score = 0;
        game_state.game_over = false;

        // Clear input buffer
        input_buffer.clear();

        // Spawn new snake head
        let head_entity = spawn_snake_head(&mut commands);
        game_state.snake_segments.push(head_entity);

        // Spawn new food (pass initial snake position)
        spawn_food(&mut commands, &[INITIAL_SNAKE_POSITION]);
    }
}

fn update_score_text(game_state: Res<GameState>, mut query: Query<&mut Text, With<ScoreText>>) {
    if let Ok(mut text) = query.single_mut() {
        *text = Text::from(format!("Score: {}", game_state.score));
    }
}
