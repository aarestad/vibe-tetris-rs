# AGENTS.md - tetris-rs Development Guide

This file contains guidelines and commands for agentic coding agents working on the tetris-rs codebase.

## Project Overview

This is a terminal-based Tetris game written in Rust (edition 2024) using ratatui and crossterm for cross-platform terminal UI, and rodio for audio. The game follows standard Tetris mechanics with support for configuration files, different tetrimino types, and full game loop implementation.

The design of the game aims to implement as much as possible of the official
[Tetris Design Guidelines](./2009%20Tetris%20Design%20Guideline.pdf), including T-Spins, Variable Goal System, and Super
Rotation.

## Build, Test, and Development Commands

### Core Commands
- `cargo build` - Build the project in debug mode
- `cargo build --release` - Build optimized release version
- `cargo run` - Build and run the game
- `cargo check` - Quick compile check without building
- `cargo clippy` - Run linter for code quality checks
- `cargo fmt` - Format code according to Rust standards
- `cargo doc --open` - Generate and view documentation

### Testing Commands
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run a specific test
- `cargo test -- --nocapture` - Run tests with stdout output

### Development Workflow
- Always run `cargo check` after making changes to ensure compilation
- Run `cargo clippy` before committing to catch potential issues
- Use `cargo fmt` to maintain consistent code formatting
- Test by running `cargo run` to verify game functionality

## Code Style Guidelines

### Module Structure and Imports
- Keep module declarations in `lib.rs` in alphabetical order; `main.rs` uses the library
- Use `use crate::` for internal modules, external crates go after
- Group imports by type: std library, external crates, internal modules
- Prefer explicit imports over glob imports (`use std::io::{Write, stdout}` vs `use std::io::*`)

### Naming Conventions
- **Structs/Enums**: PascalCase (`GameConfig`, `TetriminoType`)
- **Functions/Methods**: snake_case (`get_blocks`, `handle_input`)
- **Variables/Fields**: snake_case (`board_width`, `last_update`)
- **Constants**: SCREAMING_SNAKE_CASE for global constants
- **Private fields**: Use underscore prefix only if absolutely necessary

### Error Handling
- Use `anyhow::Result<T>` for application-level error handling
- Use `?` operator for error propagation
- Prefer `unwrap_or_else` with fallback values over bare `unwrap()`
- Return `Result<()>` from functions that can fail but don't return meaningful data

### Code Organization
- Keep functions focused and single-purpose
- Use `impl` blocks to group related functionality
- Add doc comments (`///`) for public APIs
- Use regular comments (`//`) for implementation details
- Limit line length to 100 characters where reasonable

### Data Structures
- Prefer structs over tuples for named fields
- Use enums for type-safe state representation
- Implement `Default` for configuration structs
- Use `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` for simple enums and value types
- Use `#[derive(Debug, Clone)]` for structs with owned data

### Function Design
- Keep functions under 50 lines when possible
- Use early returns to reduce nesting
- Prefer explicit returns over implicit returns for complex expressions
- Group related parameters together
- Use builder pattern for complex struct construction

### Memory and Performance
- Use references (`&`) instead of copies where appropriate
- Prefer `Vec` over arrays when size is dynamic or unknown at compile time
- Use `Option<T>` for nullable values
- Consider `Cow<str>` for string data that might be borrowed or owned
- Use `Duration` from `std::time` for time-based calculations

### Game-Specific Patterns
- Board coordinates: use `(x, y)` with `x` horizontal, `y` vertical; row 0 is the top Matrix row and negative rows are the Buffer Zone
- Tetrimino positions: store as `i32` to allow negative values during validation
- Game loop: target 60 FPS with frame rate limiting
- Input handling: use enums for different action types
- State management: separate game state from rendering logic
- Game phases: `Phase` holds the Tetrimino in play and the Line Clear, so add new states as variants instead of flags

### Testing Strategy
- Unit tests for individual modules in their respective files using `#[cfg(test)]`
- Integration tests for game flow in `tests/` directory
- Render tests with ratatui's `TestBackend`, in `ui.rs`
- Time passes only through `GameState::tick`, so timing rules can be tested without waiting

### Documentation Requirements
- All public APIs must have doc comments
- Complex algorithms should have inline comments
- Game mechanics should be documented with reference to Tetris guidelines
- Configuration options should explain their effects on gameplay

### Dependencies and Version Management
- Lock dependencies in `Cargo.lock` should be committed
- Use semver-compatible version updates
- Prefer dependencies with active maintenance
- Review new dependency additions for security and compatibility

## File Structure

```
build.rs             # Build script to transform MIDI to WAV; requires timidity for music
tests/
└── random_play.rs   # Random input games that check the rules always hold
src/
├── main.rs          # Entry point and application setup
├── lib.rs           # Module declarations
├── audio.rs         # Audio handling
├── game.rs          # Main game loop and input handling
├── game_state.rs    # Phases, movement, Lock Down, Hold and T-Spin recognition
├── scoring.rs       # Score, Back-to-Back, level and Goal
├── board.rs         # Game board and collision detection
├── tetrimino.rs     # Tetrimino definitions and rotations
├── input.rs         # Keyboard input processing
├── ui.rs            # Terminal rendering and display
└── config.rs        # Game configuration and file I/O
```

## Common Gotchas

- Terminal mode: Always use RAII pattern for terminal cleanup
- Coordinate system: Y increases downward, so y-up rotation tables need their sign reversed
- Key releases: only terminals with keyboard enhancement report them; other terminals get one Soft Drop row or one move per key event, with OS key repeat
- Input handling: Poll in main loop, don't block for extended periods
- Frame timing: Use `Instant::now()` and `Duration` for accurate timing
- Error handling: Terminal operations can fail, handle gracefully with `Result`

## Configuration

- Game config uses JSON format with `serde` for serialization
- Default values are provided through `Default` trait
- Config files are optional; missing files fall back to defaults, and missing fields take their defaults
- Configuration is validated on load, and an invalid file is an error

## Development Notes

- This is a "vibe-coded" project - prioritize readability and simplicity
- Game follows standard Tetris guidelines for scoring and gravity
- Terminal-only implementation, no GUI dependencies
- Cross-platform compatibility via crossterm
- Single-threaded game loop with frame rate limiting