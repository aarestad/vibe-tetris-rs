use crate::audio::{AudioPlayer, THEME};
use crate::config::GameConfig;
use crate::demo::Demo;
use crate::end_reward::EndReward;
use crate::game_state::GameState;
use crate::high_scores::{HighScore, HighScores, NameEntry, NameEntryResult, TextInput};
use crate::input::InputHandler;
use crate::menu::{Menu, MenuInput, MenuResult, OptionItem};
use crate::session::{MusicChange, Session};
use crate::sound::SoundEffect;
use crate::ui::{GameOverView, Renderer};
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
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// About 60 frames per second.
const FRAME_DURATION: Duration = Duration::from_millis(16);
/// Keys pressed this soon after Game Over do not dismiss the screen.
const GAME_OVER_KEY_GUARD: Duration = Duration::from_millis(500);

/// Connects the menu and each game's `Session` to the real terminal and sound device. All game
/// behavior is in those types, so this only moves input, time and drawing between them.
pub struct Game {
    /// The settings from the config file, with the Options page's changes applied.
    config: GameConfig,
    /// Where the Options page saves the settings.
    config_path: PathBuf,
    scores: HighScores,
    /// Where new high scores are saved.
    scores_path: PathBuf,
    menu: Menu,
    renderer: Renderer,
    input: InputHandler,
    audio: AudioPlayer,
    warnings: Vec<String>,
}

impl Game {
    pub fn new(
        config: GameConfig,
        config_path: PathBuf,
        scores: HighScores,
        scores_path: PathBuf,
    ) -> Result<Self> {
        let renderer = Renderer::new()?;
        let mut audio = if config.enable_sound {
            AudioPlayer::new()
        } else {
            AudioPlayer::no_sound()
        };
        audio.set_effects_volume(config.sound_effects_gain());

        Ok(Self {
            menu: Menu::new(&config),
            input: InputHandler::new(config.controls.clone()),
            config,
            config_path,
            scores,
            scores_path,
            renderer,
            audio,
            warnings: Vec::new(),
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

    /// Audio and saving messages to show once the terminal is back to normal.
    pub fn take_warnings(&mut self) -> Vec<String> {
        let mut warnings = self.audio.take_warnings();
        warnings.append(&mut self.warnings);
        warnings
    }

    /// Shows the menu until the player chooses. Returns true to start a game, false to exit.
    fn run_menu(&mut self) -> Result<bool> {
        loop {
            while let Some(input) = self.input.poll_menu_input() {
                match self.menu.handle_input(input) {
                    MenuResult::Stay => {}
                    MenuResult::StartGame => return Ok(true),
                    MenuResult::StartDemo => {
                        if self.run_demo()? {
                            return Ok(false);
                        }
                    }
                    MenuResult::OptionsChanged => self.save_options(),
                    MenuResult::Exit => return Ok(false),
                }
            }
            self.renderer.render_menu(&self.menu, &self.scores)?;
            std::thread::sleep(FRAME_DURATION);
        }
    }

    /// Plays the scoring demonstration until the player leaves it. Returns true if the
    /// player wants to leave the program.
    fn run_demo(&mut self) -> Result<bool> {
        let mut demo = Demo::new(&self.config);
        let mut last_tick = Instant::now();
        loop {
            while let Some(input) = self.input.poll_menu_input() {
                if input == MenuInput::Quit {
                    return Ok(true);
                }
                if !demo.handle_input(&self.config, input) {
                    return Ok(false);
                }
            }
            let now = Instant::now();
            demo.update(&self.config, now.duration_since(last_tick));
            last_tick = now;
            for effect in demo.take_sound_effects() {
                if self.config.enable_sound_effects {
                    self.audio.play_effect(effect);
                }
            }
            self.renderer.render_demo(&demo)?;
            std::thread::sleep(FRAME_DURATION);
        }
    }

    /// Applies the Options page's settings and saves them to the config file. A failed save
    /// keeps the settings for this run and is reported when the program exits.
    fn save_options(&mut self) {
        self.menu.options().apply_to(&mut self.config);
        self.audio
            .set_effects_volume(self.config.sound_effects_gain());
        self.preview_sound_effects();
        if let Err(error) = self.config.save_to_file(&self.config_path) {
            self.warn(format!("could not save options: {error:#}"));
        }
    }

    /// Saves the High Score Table. A failed save keeps the scores for this run and is reported
    /// when the program exits.
    fn save_scores(&mut self) {
        if let Err(error) = self.scores.save_to_file(&self.scores_path) {
            self.warn(format!("could not save high scores: {error:#}"));
        }
    }

    fn warn(&mut self, warning: String) {
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }

    /// Plays one game through its Game Over screen. Returns true if the player wants to leave
    /// the program instead of going back to the menu.
    fn play_game(&mut self, key_release_events: bool) -> Result<bool> {
        let mut session = Session::new(self.menu.game_config(&self.config));
        session.set_key_release_events(key_release_events);
        // Shown during play as the high score to beat (Guideline 2.4).
        let mode = session.state().config.game_mode;
        let best = self.scores.table(mode).first().cloned();
        // Shift and Ctrl are play keys (Guideline 4.1), but only enhanced terminals report
        // them alone, and only during play, so name entry still gets shifted letters.
        let modifier_keys = ModifierKeyReporting::start(key_release_events)?;
        self.input.set_modifier_keys(modifier_keys.active);

        self.start_music();

        let mut last_tick = Instant::now();

        loop {
            let frame_start = Instant::now();
            let dt = frame_start.duration_since(last_tick);
            last_tick = frame_start;

            self.handle_pending_input(&mut session);
            session.update(dt);
            self.play_sound_effects(&mut session);
            self.renderer
                .render(session.state(), session.screen(), best.as_ref())?;

            if session.state().is_game_over() {
                drop(modifier_keys);
                self.input.set_modifier_keys(false);
                self.audio.stop();
                return self.finish_game(session.state(), best.as_ref());
            }

            std::thread::sleep(FRAME_DURATION.saturating_sub(frame_start.elapsed()));
        }
    }

    /// Shows the Game Over screen, with any End Reward. A result that makes the High Score
    /// Table asks for a name, is saved, and opens the table for the menu to show. Returns true
    /// if the player wants to leave the program.
    fn finish_game(&mut self, state: &GameState, best: Option<&HighScore>) -> Result<bool> {
        let mode = state.config.game_mode;
        let new_high_score = HighScore::from_game(state)
            .and_then(|score| Some((self.scores.rank_of(mode, &score)?, score)));
        let rank = new_high_score.as_ref().map(|(rank, _)| *rank);
        let table_was_empty = self.scores.table(mode).is_empty();
        let view = GameOverView {
            reward: EndReward::for_game(state, rank, table_was_empty),
            best,
            ..GameOverView::default()
        };

        let Some((rank, mut score)) = new_high_score else {
            return self.wait_on_game_over(state, view);
        };
        let Some(name) = self.enter_name(state, rank, view)? else {
            return Ok(true);
        };
        score.name = name;
        let rank = self.scores.insert(mode, score);
        self.save_scores();
        self.menu.show_high_scores(mode, rank);
        Ok(false)
    }

    /// Shows the Game Over screen until a key is pressed. Keys pressed in the first moments
    /// are ignored, so a key held during play does not dismiss it at once. Returns true for
    /// Ctrl+C, which means the player wants to leave.
    fn wait_on_game_over(&mut self, state: &GameState, view: GameOverView) -> Result<bool> {
        let start = Instant::now();
        loop {
            while let Some(quit) = self.input.poll_key_press() {
                if quit {
                    return Ok(true);
                }
                if start.elapsed() >= GAME_OVER_KEY_GUARD {
                    return Ok(false);
                }
            }
            let view = GameOverView {
                since: start.elapsed(),
                ..view
            };
            self.renderer.render_game_over(state, view)?;
            std::thread::sleep(FRAME_DURATION);
        }
    }

    /// Asks for a name for a new high score. Returns None if the player pressed Ctrl+C, which
    /// leaves without saving.
    fn enter_name(
        &mut self,
        state: &GameState,
        rank: usize,
        view: GameOverView,
    ) -> Result<Option<String>> {
        let mut entry = NameEntry::new(self.scores.last_name());
        let start = Instant::now();
        loop {
            while let Some(input) = self.input.poll_text_input() {
                // Keys still held from play must not type or confirm a name.
                if input != TextInput::Quit && start.elapsed() < GAME_OVER_KEY_GUARD {
                    continue;
                }
                match entry.handle_input(input) {
                    NameEntryResult::Typing => {}
                    NameEntryResult::Done(name) => return Ok(Some(name)),
                    NameEntryResult::Quit => return Ok(None),
                }
            }
            let view = GameOverView {
                new_high_score: Some((rank, entry.name())),
                since: start.elapsed(),
                ..view
            };
            self.renderer.render_game_over(state, view)?;
            std::thread::sleep(FRAME_DURATION);
        }
    }

    /// Plays a sample effect when the effects are turned on or their volume changes, so the
    /// player can hear the new setting.
    fn preview_sound_effects(&mut self) {
        let effects_option = matches!(
            self.menu.selected_option(),
            OptionItem::SoundEffects | OptionItem::SoundEffectsVolume
        );
        if effects_option && self.config.enable_sound_effects {
            self.audio.play_effect(SoundEffect::LineClear(1));
        }
    }

    fn play_sound_effects(&mut self, session: &mut Session) {
        for effect in session.take_sound_effects() {
            if self.config.enable_sound_effects {
                self.audio.play_effect(effect);
            }
        }
    }

    fn start_music(&mut self) {
        if !self.config.enable_music || THEME.is_empty() {
            return;
        }
        self.audio.set_volume(self.config.music_gain());
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

/// Has the terminal report every key as an escape code, including Shift and Ctrl alone, until
/// dropped. Does nothing on terminals without keyboard enhancement.
struct ModifierKeyReporting {
    active: bool,
}

impl ModifierKeyReporting {
    fn start(keyboard_enhanced: bool) -> Result<Self> {
        if keyboard_enhanced {
            execute!(
                stdout(),
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                )
            )?;
        }
        Ok(Self {
            active: keyboard_enhanced,
        })
    }
}

impl Drop for ModifierKeyReporting {
    fn drop(&mut self) {
        if self.active {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
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
