# AGENTS.md - tetris-rs Development Guide

This file contains guidelines and commands for agentic coding agents working on the tetris-rs codebase.

## Project Overview

This is a terminal-based Tetris game written in Rust (edition 2024) using the crossterm crate for cross-platform terminal UI. The game follows standard Tetris mechanics with support for configuration files, different tetrimino types, and full game loop implementation.

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
- `cargo test` - Run all tests (none currently exist)
- `cargo test <test_name>` - Run a specific test
- `cargo test -- --nocapture` - Run tests with stdout output

### Development Workflow
- Always run `cargo check` after making changes to ensure compilation
- Run `cargo clippy` before committing to catch potential issues
- Use `cargo fmt` to maintain consistent code formatting
- Test by running `cargo run` to verify game functionality

## Code Style Guidelines

### Module Structure and Imports
- Keep module declarations at the top of `main.rs` in alphabetical order
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
- Board coordinates: use `(x, y)` with `x` horizontal, `y` vertical
- Tetrimino positions: store as `i32` to allow negative values during validation
- Game loop: target 60 FPS with frame rate limiting
- Input handling: use enums for different action types
- State management: separate game state from rendering logic

### Testing Strategy (Future)
- Unit tests for individual modules in their respective files using `#[cfg(test)]`
- Integration tests for game flow in `tests/` directory
- Property-based testing for tetrimino rotation and collision logic
- Benchmarks for performance-critical game loop code

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
src/
├── main.rs          # Entry point and application setup
├── game.rs          # Main game loop and input handling
├── game_state.rs    # Game state management and logic
├── board.rs         # Game board and collision detection
├── tetrimino.rs     # Tetrimino definitions and rotations
├── input.rs         # Keyboard input processing
├── ui.rs            # Terminal rendering and display
└── config.rs        # Game configuration and file I/O
```

## Common Gotchas

- Terminal mode: Always use RAII pattern for terminal cleanup
- Coordinate system: Y increases downward, be consistent with board bounds
- Input handling: Poll in main loop, don't block for extended periods
- Frame timing: Use `Instant::now()` and `Duration` for accurate timing
- Error handling: Terminal operations can fail, handle gracefully with `Result`

## Configuration

- Game config uses JSON format with `serde` for serialization
- Default values are provided through `Default` trait
- Config files are optional; missing files fall back to defaults
- Configuration should be validated on load

## Development Notes

- This is a "vibe-coded" project - prioritize readability and simplicity
- Game follows standard Tetris guidelines for scoring and gravity
- Terminal-only implementation, no GUI dependencies
- Cross-platform compatibility via crossterm
- Single-threaded game loop with frame rate limiting