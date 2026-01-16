// ============================================================================
// Game Loop
// ============================================================================

use crate::config::GameConfig;
use crate::game_state::GameState;
use crate::input::{InputAction, InputHandler};
use crate::ui::Renderer;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io::{Write, stdout};
use std::time::{Duration, Instant};

pub struct Game {
    state: GameState,
    renderer: Renderer,
    input: InputHandler,
}

impl Game {
    pub fn new(config: GameConfig) -> Self {
        let renderer = Renderer::new(config.clone());
        let state = GameState::new(config);
        let input = InputHandler::new();

        Self {
            state,
            renderer,
            input,
        }
    }

    pub fn run(&mut self) {
        // Setup terminal
        let _cleanup = setup_terminal();

        self.state.spawn_piece();

        // Game loop timing variables
        let mut last_update = Instant::now();
        let mut last_gravity = Instant::now();
        let gravity_duration = self.get_gravity_duration();
        let frame_duration = Duration::from_millis(16); // ~60 FPS

        loop {
            let now = Instant::now();
            let frame_time = now.duration_since(last_update);

            // Handle input
            if let Some(action) = self.input.poll_input() {
                self.handle_input(action);
            }

            // Apply gravity
            if now.duration_since(last_gravity) >= gravity_duration {
                if !self.state.move_piece(0, 1) {
                    self.state.lock_current_piece();
                }
                last_gravity = now;
            }

            // Render
            self.renderer.clear_screen();
            self.renderer.render(&self.state);

            // Check for game over
            if self.state.game_over {
                self.renderer.render_game_over(&self.state);
                // Wait for any key press before exiting
                let _ = std::io::stdout().flush();
                while self.input.poll_input().is_none() {
                    std::thread::sleep(Duration::from_millis(16));
                }
                break;
            }

            // Frame rate limiting
            if frame_time < frame_duration {
                std::thread::sleep(frame_duration - frame_time);
            }
            last_update = now;
        }
    }

    fn get_gravity_duration(&self) -> Duration {
        // Calculate gravity duration based on level
        // Using Tetris guidelines: gravity increases with level
        let base_gravity_ms = 800; // Level 1: 800ms per drop
        let level = self.state.level.max(1);
        let gravity_ms = (base_gravity_ms / (2_u32.pow((level - 1).min(10)))).max(50);
        Duration::from_millis(gravity_ms as u64)
    }

    fn handle_input(&mut self, action: InputAction) {
        match action {
            InputAction::MoveLeft => {
                self.state.move_piece(-1, 0);
            }
            InputAction::MoveRight => {
                self.state.move_piece(1, 0);
            }
            InputAction::MoveDown => {
                self.state.move_piece(0, 1);
            }
            InputAction::HardDrop => {
                self.state.hard_drop();
            }
            InputAction::RotateClockwise => {
                self.state.rotate_piece(true);
            }
            InputAction::RotateCounterClockwise => {
                self.state.rotate_piece(false);
            }
            InputAction::Hold => {
                self.state.hold_piece();
            }
            InputAction::Pause => {
                self.handle_pause();
            }
            InputAction::Quit => {
                self.state.game_over = true;
            }
        }
    }

    fn handle_pause(&mut self) {
        // Display pause screen
        self.renderer.clear_screen();
        self.renderer.render_pause(&self.state);

        // Wait for pause to be toggled again
        loop {
            if let Some(action) = self.input.poll_input() {
                match action {
                    InputAction::Pause => break, // Resume game
                    InputAction::Quit => {
                        self.state.game_over = true;
                        break;
                    }
                    _ => {
                        // Ignore other inputs while paused
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(16)); // 60 FPS check
        }

        // Reset timing to prevent gravity jumps after unpausing
        // let now = Instant::now();
        // Note: In a full implementation, we'd reset last_gravity and last_update here
        // For now, the timing will reset naturally in the next game loop iteration
    }
}

// Terminal cleanup guard using RAII
struct TerminalCleanup;

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = stdout().flush();
    }
}

fn setup_terminal() -> TerminalCleanup {
    enable_raw_mode().expect("Failed to enable raw mode");
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)
        .expect("Failed to enter alternate screen");

    TerminalCleanup
}
