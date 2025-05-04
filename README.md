# Snake Game in Rust with Bevy

A simple Snake game implemented in Rust using the Bevy game engine.

## Features

- Classic Snake gameplay mechanics
- Score tracking
- Game over state with restart functionality
- Wrap-around screen edges

## Controls

- Arrow keys or WASD to control the snake
- Space to restart after game over

## How to Run

1. Make sure you have Rust and Cargo installed:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Navigate to the project directory:
   ```bash
   cd ~/src/snake_rust
   ```

3. Run the game:
   ```bash
   cargo run --release
   ```

## Game Rules

- Control the snake to eat the red food dots
- Each food item increases your score and makes the snake longer
- The game ends if the snake collides with itself
- The snake can wrap around the edges of the screen

## Dependencies

- Bevy 0.13.0
- rand 0.8.5

## Project Structure

- `src/main.rs`: Contains all game code
- `assets/fonts/`: Contains font files for text rendering

## Notes

This game requires a font file at `assets/fonts/FiraSans-Bold.ttf` for displaying the score. If you don't have this font, download it or replace it with another font and update the path in the code.
