use crate::audio::{AudioPlayer, THEME};
use crate::config::GameConfig;
use crate::game_state::GameState;
use crate::input::{AutoRepeat, InputAction, InputEvent, InputHandler};
use crate::ui::Renderer;
use anyhow::Result;
use crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        supports_keyboard_enhancement,
    },
};
use std::io::{Write, stdout};
use std::time::{Duration, Instant};

/// About 60 frames per second.
const FRAME_DURATION: Duration = Duration::from_millis(16);
/// Keys pressed this soon after Game Over do not dismiss the screen.
const GAME_OVER_KEY_GUARD: Duration = Duration::from_millis(500);

pub struct Game {
    state: GameState,
    renderer: Renderer,
    input: InputHandler,
    audio: AudioPlayer,
    auto_repeat: AutoRepeat,
    /// True when the terminal reports key releases, which continuous input needs.
    key_release_events: bool,
}

impl Game {
    pub fn new(config: GameConfig) -> Result<Self> {
        let renderer = Renderer::new()?;
        let audio = if config.enable_sound {
            AudioPlayer::new()
        } else {
            AudioPlayer::no_sound()
        };
        let auto_repeat = AutoRepeat::new(
            Duration::from_millis(config.das_delay),
            Duration::from_millis(config.das_repeat),
        );
        let state = GameState::new(config);
        let input = InputHandler::new();

        Ok(Self {
            state,
            renderer,
            input,
            audio,
            auto_repeat,
            key_release_events: false,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let terminal_guard = TerminalGuard::new()?;
        self.key_release_events = terminal_guard.keyboard_enhanced;

        self.start_music();
        self.state.spawn_piece();

        let mut last_tick = Instant::now();

        loop {
            let frame_start = Instant::now();
            let dt = frame_start.duration_since(last_tick);
            last_tick = frame_start;

            if self.handle_pending_input()? {
                // A pause took real time that must not count as play.
                last_tick = Instant::now();
            }

            self.apply_auto_repeat(dt);
            self.state.tick(dt);
            self.renderer.render(&self.state)?;

            if self.state.game_over {
                self.audio.stop();
                self.renderer.render_game_over(&self.state)?;
                stdout().flush()?;
                self.input.wait_for_key_press(GAME_OVER_KEY_GUARD)?;
                break;
            }

            std::thread::sleep(FRAME_DURATION.saturating_sub(frame_start.elapsed()));
        }

        Ok(())
    }

    /// Audio messages to show once the terminal is back to normal.
    pub fn take_warnings(&mut self) -> Vec<String> {
        self.audio.take_warnings()
    }

    fn start_music(&mut self) {
        if THEME.is_empty() {
            return;
        }
        self.audio.play_background_music(THEME);
        self.audio.set_volume(0.5);
    }

    /// Handles all input waiting this frame. Returns true if the game was paused.
    fn handle_pending_input(&mut self) -> Result<bool> {
        let mut paused = false;
        while let Some(input) = self.input.poll_input() {
            paused |= self.handle_input(input)?;
        }
        Ok(paused)
    }

    fn apply_auto_repeat(&mut self, dt: Duration) {
        let (direction, moves) = self.auto_repeat.update(dt);
        if self.state.show_help {
            return;
        }
        for _ in 0..moves {
            self.state.shift_piece(direction);
        }
    }

    fn handle_input(&mut self, input: InputEvent) -> Result<bool> {
        match input {
            InputEvent::Pressed(action) => self.handle_press(action),
            InputEvent::Released(action) => {
                self.handle_release(action);
                Ok(false)
            }
        }
    }

    fn handle_press(&mut self, action: InputAction) -> Result<bool> {
        // Play is stopped behind the help overlay, so only these keys do anything.
        if self.state.show_help && !matches!(action, InputAction::ToggleHelp | InputAction::Quit) {
            return Ok(false);
        }

        match action {
            InputAction::MoveLeft => self.press_direction(-1),
            InputAction::MoveRight => self.press_direction(1),
            InputAction::MoveDown => {
                if self.key_release_events {
                    self.state.set_soft_drop(true);
                } else {
                    self.state.soft_drop_step();
                }
            }
            InputAction::HardDrop => self.state.hard_drop(),
            InputAction::RotateClockwise => {
                self.state.rotate_piece(true);
            }
            InputAction::RotateCounterClockwise => {
                self.state.rotate_piece(false);
            }
            InputAction::Hold => self.state.hold_piece(),
            InputAction::Pause => {
                self.handle_pause()?;
                return Ok(true);
            }
            InputAction::ToggleHelp => self.state.toggle_help(),
            InputAction::Quit => self.state.game_over = true,
        }
        Ok(false)
    }

    fn press_direction(&mut self, direction: i32) {
        self.state.shift_piece(direction);
        if self.key_release_events {
            self.auto_repeat.press(direction);
        }
    }

    fn handle_release(&mut self, action: InputAction) {
        match action {
            InputAction::MoveLeft => self.auto_repeat.release(-1),
            InputAction::MoveRight => self.auto_repeat.release(1),
            InputAction::MoveDown => self.state.set_soft_drop(false),
            _ => {}
        }
    }

    /// Stops play and hides the Matrix and queues until Pause is pressed again.
    fn handle_pause(&mut self) -> Result<()> {
        self.audio.pause();

        loop {
            // Drawn every time through so that a resize during the pause is handled.
            self.renderer.render_pause(&self.state)?;

            while let Some(input) = self.input.poll_input() {
                match input {
                    InputEvent::Pressed(InputAction::Pause) => {
                        self.audio.resume();
                        self.release_all_held_keys();
                        return Ok(());
                    }
                    InputEvent::Pressed(InputAction::Quit) => {
                        self.state.game_over = true;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            std::thread::sleep(FRAME_DURATION);
        }
    }

    fn release_all_held_keys(&mut self) {
        self.auto_repeat.clear();
        self.state.set_soft_drop(false);
    }
}

/// Restores the terminal when dropped, including during a panic.
struct TerminalGuard {
    keyboard_enhanced: bool,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        // Created now so that any failure below still leaves raw mode.
        let mut guard = Self {
            keyboard_enhanced: false,
        };
        execute!(stdout(), EnterAlternateScreen)?;

        if supports_keyboard_enhancement().unwrap_or(false) {
            execute!(
                stdout(),
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            )?;
            guard.keyboard_enhanced = true;
        }
        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.keyboard_enhanced {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        let _ = stdout().flush();
    }
}
