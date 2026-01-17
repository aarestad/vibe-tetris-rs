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
    audio: AudioPlayer,
}

impl Game {
    pub fn new(config: GameConfig) -> Result<Self> {
        let renderer = Renderer::new()?;
        let state = GameState::new(config);
        let input = InputHandler::new();
        let audio = AudioPlayer::new();

        Ok(Self {
            state,
            renderer,
            input,
            audio,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let _cleanup = setup_terminal();

        // Start background music
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

            if now.duration_since(last_gravity) >= gravity_duration {
                if !self.state.move_piece(0, 1) {
                    self.state.lock_current_piece();
                }
                last_gravity = now;
            }

            self.renderer.render(&self.state)?;

            if self.state.game_over {
                self.renderer.render_game_over(&self.state)?;

                // Stop music on game over
                self.audio.stop();

                let _ = stdout().flush();
                while self.input.poll_input().is_none() {
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
        audio_path.push("tetris_theme.ogg");

        if audio_path.exists() {
            self.audio.play_background_music(audio_path);
            self.audio.set_volume(0.5);
        } else {
            let midi_path = PathBuf::from("tetris_main_theme.mid");
            if midi_path.exists() {
                self.audio.play_background_music(midi_path);
            }
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
                self.audio.stop();
            }
        }

        Ok(())
    }

    fn handle_pause(&mut self) -> Result<()> {
        // Pause music when pausing game
        self.audio.pause();

        self.renderer.render_pause(&self.state)?;

        loop {
            if let Some(action) = self.input.poll_input() {
                match action {
                    InputAction::Pause => {
                        // Resume music when unpausing
                        self.audio.resume();
                        break;
                    }
                    InputAction::Quit => {
                        self.audio.stop();
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
