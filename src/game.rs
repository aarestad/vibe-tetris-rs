use crate::audio::{AudioPlayer, THEME};
use crate::config::GameConfig;
use crate::input::InputHandler;
use crate::menu::{Menu, MenuResult};
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

/// Connects the menu and each game's `Session` to the real terminal and sound device. All game
/// behavior is in those types, so this only moves input, time and drawing between them.
pub struct Game {
    config: GameConfig,
    menu: Menu,
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
            menu: Menu::new(&config),
            config,
            renderer,
            input: InputHandler::new(),
            audio,
        })
    }

    /// Runs the Main Menu and the games started from it until the player exits.
    pub fn run(&mut self) -> Result<()> {
        let terminal_guard = TerminalGuard::new()?;

        while self.run_menu()? {
            let wants_to_leave = self.play_game(terminal_guard.keyboard_enhanced)?;
            if wants_to_leave {
                break;
            }
        }

        Ok(())
    }

    /// Audio messages to show once the terminal is back to normal.
    pub fn take_warnings(&mut self) -> Vec<String> {
        self.audio.take_warnings()
    }

    /// Shows the menu until the player chooses. Returns true to start a game, false to exit.
    fn run_menu(&mut self) -> Result<bool> {
        loop {
            while let Some(input) = self.input.poll_menu_input() {
                match self.menu.handle_input(input) {
                    MenuResult::Stay => {}
                    MenuResult::StartGame => return Ok(true),
                    MenuResult::Exit => return Ok(false),
                }
            }
            self.renderer.render_menu(&self.menu)?;
            std::thread::sleep(FRAME_DURATION);
        }
    }

    /// Plays one game through its Game Over screen. Returns true if the player wants to leave
    /// the program instead of going back to the menu.
    fn play_game(&mut self, key_release_events: bool) -> Result<bool> {
        let mut session = Session::new(self.menu.game_config(&self.config));
        session.set_key_release_events(key_release_events);

        self.start_music();

        let mut last_tick = Instant::now();

        loop {
            let frame_start = Instant::now();
            let dt = frame_start.duration_since(last_tick);
            last_tick = frame_start;

            self.handle_pending_input(&mut session);
            session.update(dt);
            self.renderer.render(session.state(), session.screen())?;

            if session.state().is_game_over() {
                self.audio.stop();
                self.renderer.render_game_over(session.state())?;
                stdout().flush()?;
                return self.input.wait_for_key_press(GAME_OVER_KEY_GUARD);
            }

            std::thread::sleep(FRAME_DURATION.saturating_sub(frame_start.elapsed()));
        }
    }

    fn start_music(&mut self) {
        if THEME.is_empty() {
            return;
        }
        self.audio.set_volume(0.5);
        self.audio.play_background_music(THEME);
    }

    /// Passes all input waiting this frame to the session.
    fn handle_pending_input(&mut self, session: &mut Session) {
        while let Some(input) = self.input.poll_input() {
            match session.handle_input(input) {
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
