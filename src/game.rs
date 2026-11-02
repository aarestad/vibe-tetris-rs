use crate::audio::{AudioPlayer, THEME};
use crate::config::GameConfig;
use crate::input::InputHandler;
use crate::session::{MusicChange, Session};
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

/// Connects a `Session` to the real terminal and sound device. All game behavior is in
/// `Session`, so this only moves input, time and drawing between them.
pub struct Game {
    session: Session,
    renderer: Renderer,
    input: InputHandler,
    audio: AudioPlayer,
}

impl Game {
    pub fn new(config: GameConfig) -> Result<Self> {
        let renderer = Renderer::new()?;
        let audio = if config.enable_sound {
            AudioPlayer::new()
        } else {
            AudioPlayer::no_sound()
        };

        Ok(Self {
            session: Session::new(config),
            renderer,
            input: InputHandler::new(),
            audio,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let terminal_guard = TerminalGuard::new()?;
        self.session
            .set_key_release_events(terminal_guard.keyboard_enhanced);

        self.start_music();

        let mut last_tick = Instant::now();

        loop {
            let frame_start = Instant::now();
            let dt = frame_start.duration_since(last_tick);
            last_tick = frame_start;

            self.handle_pending_input();
            self.session.update(dt);
            self.renderer
                .render(self.session.state(), self.session.screen())?;

            if self.session.state().is_game_over() {
                self.audio.stop();
                self.renderer.render_game_over(self.session.state())?;
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

    /// Passes all input waiting this frame to the session.
    fn handle_pending_input(&mut self) {
        while let Some(input) = self.input.poll_input() {
            match self.session.handle_input(input) {
                Some(MusicChange::Pause) => self.audio.pause(),
                Some(MusicChange::Resume) => self.audio.resume(),
                None => {}
            }
        }
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
