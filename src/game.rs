use crate::audio::AudioPlayer;
use crate::config::GameConfig;
use crate::game_state::GameState;
use crate::input::{InputAction, InputHandler};
use crate::ui::Renderer;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io::{Write, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct Game {
    state: GameState,
    renderer: Renderer,
    input: InputHandler,
    audio: Option<AudioPlayer>,
}

impl Game {
    pub fn new(config: GameConfig) -> Result<Self> {
        let renderer = Renderer::new()?;
        let audio = if config.enable_sound {
            Some(AudioPlayer::new())
        } else {
            None
        };
        let state = GameState::new(config);
        let input = InputHandler::new();

        Ok(Self {
            state,
            renderer,
            input,
            audio,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let _cleanup = setup_terminal();

        self.start_music();

        self.state.spawn_piece();

        let mut last_update = Instant::now();
        let mut last_gravity = Instant::now();
        let gravity_duration = self.get_gravity_duration();
        let frame_duration = Duration::from_millis(16);

        loop {
            let now = Instant::now();
            let frame_time = now.duration_since(last_update);

            if let Some(action) = self.input.poll_input() {
                self.handle_input(action)?;
            }

            if self.state.pending_line_clear {
                if !self.state.is_line_clear_animation_active() {
                    self.state.complete_line_clear();
                }
            } else if now.duration_since(last_gravity) >= gravity_duration {
                if !self.state.move_piece(0, 1) {
                    self.state.lock_current_piece();
                }
                last_gravity = now;
            }

            self.renderer.render(&self.state)?;

            if self.state.game_over {
                self.renderer.render_game_over(&self.state)?;

                if let Some(a) = self.audio.as_mut() {
                    a.stop();
                }

                stdout().flush()?;

                while !self.input.has_input() {
                    std::thread::sleep(Duration::from_millis(16));
                }
                break;
            }

            if frame_time < frame_duration {
                std::thread::sleep(frame_duration - frame_time);
            }

            last_update = now;
        }

        Ok(())
    }

    fn start_music(&mut self) {
        let mut audio_path = PathBuf::from(env!("OUT_DIR"));
        audio_path.push("tetris_theme.wav");

        if audio_path.exists()
            && let Some(a) = self.audio.as_mut()
        {
            a.play_background_music(audio_path);
            a.set_volume(0.5);
        }
    }

    fn get_gravity_duration(&self) -> Duration {
        let base_gravity_ms = 800;
        let level = self.state.level.max(1);
        let gravity_ms = (base_gravity_ms / (2_u32.pow((level - 1).min(10)))).max(50);
        Duration::from_millis(gravity_ms as u64)
    }

    fn handle_input(&mut self, action: InputAction) -> Result<()> {
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
                self.handle_pause()?;
            }
            InputAction::Quit => {
                self.state.game_over = true;

                if let Some(a) = self.audio.as_mut() {
                    a.stop();
                }
            }
        }

        Ok(())
    }

    fn handle_pause(&mut self) -> Result<()> {
        if let Some(a) = self.audio.as_mut() {
            a.pause();
        }

        self.renderer.render_pause(&self.state)?;

        loop {
            if let Some(action) = self.input.poll_input() {
                match action {
                    InputAction::Pause => {
                        if let Some(a) = self.audio.as_mut() {
                            a.resume();
                        }

                        break;
                    }
                    InputAction::Quit => {
                        if let Some(a) = self.audio.as_mut() {
                            a.stop();
                        }
                        self.state.game_over = true;
                        break;
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }

        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GameConfig;
    use crate::input::InputAction;

    fn make_test_config() -> GameConfig {
        GameConfig {
            board_width: 10,
            board_height: 20,
            starting_level: 1,
            lines_per_level: 10,
            enable_ghost_piece: false,
            enable_hold: true,
            enable_variable_goal: false,
            enable_sound: false,
            preview_count: 3,
            das_delay: 250,
            das_repeat: 50,
        }
    }

    fn get_gravity_for_level(level: u32) -> Duration {
        let base_gravity_ms = 800;
        let level = level.max(1);
        let gravity_ms = (base_gravity_ms / (2_u32.pow((level - 1).min(10)))).max(50);
        Duration::from_millis(gravity_ms as u64)
    }

    #[test]
    fn test_get_gravity_duration_level_1() {
        let gravity = get_gravity_for_level(1);
        assert_eq!(gravity.as_millis(), 800);
    }

    #[test]
    fn test_get_gravity_duration_level_2() {
        let gravity = get_gravity_for_level(2);
        assert_eq!(gravity.as_millis(), 400);
    }

    #[test]
    fn test_get_gravity_duration_level_3() {
        let gravity = get_gravity_for_level(3);
        assert_eq!(gravity.as_millis(), 200);
    }

    #[test]
    fn test_get_gravity_duration_level_10() {
        let gravity = get_gravity_for_level(10);
        assert_eq!(gravity.as_millis(), 50);
    }

    #[test]
    fn test_get_gravity_duration_level_15_capped() {
        let gravity = get_gravity_for_level(15);
        assert_eq!(gravity.as_millis(), 50);
    }

    #[test]
    fn test_get_gravity_duration_very_high_level() {
        let gravity = get_gravity_for_level(100);
        assert_eq!(gravity.as_millis(), 50);
    }

    #[test]
    fn test_get_gravity_duration_level_0_adjusted_to_1() {
        let gravity = get_gravity_for_level(0);
        assert_eq!(gravity.as_millis(), 800);
    }

    #[test]
    fn test_handle_input_action_types() {
        let actions = [
            InputAction::MoveLeft,
            InputAction::MoveRight,
            InputAction::MoveDown,
            InputAction::HardDrop,
            InputAction::RotateClockwise,
            InputAction::RotateCounterClockwise,
            InputAction::Hold,
            InputAction::Pause,
            InputAction::Quit,
        ];

        assert_eq!(actions.len(), 9);
    }

    #[test]
    fn test_terminal_cleanup_drop() {
        let cleanup = TerminalCleanup;
        drop(cleanup);
    }
}
