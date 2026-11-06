use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Lock Down ruleset (Guideline 5.7 to 5.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockDownMode {
    /// Moves and rotations reset the timer, up to 15 times per lowest row reached.
    #[default]
    Extended,
    /// Moves and rotations always reset the timer.
    Infinite,
    /// Only falling below the lowest row reached resets the timer.
    Classic,
}

/// Which game is played (Guideline 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    /// Play for points through 15 levels (Guideline 12.1).
    #[default]
    Marathon,
    /// Clear 40 lines as fast as possible, with no Level Up (Guideline 12.2).
    Sprint,
    /// Score as many points as possible in three minutes, with no Level Up (Guideline 12.3).
    Ultra,
}

/// Game settings. Any field missing from the config file takes its default.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameConfig {
    /// Matrix width in cells. The Guideline standard is 10.
    pub board_width: usize,
    /// Matrix height in cells, not counting the Buffer Zone. The Guideline standard is 20.
    pub board_height: usize,
    /// Marathon, Sprint or Ultra.
    pub game_mode: GameMode,
    /// Level to start from, 1 to 15. Higher starts raise the first Goal.
    pub starting_level: u32,
    /// Lines per level in the Fixed Goal System. Sprint and Ultra ignore it.
    /// The Guideline standard is 10.
    pub lines_per_level: u32,
    /// Shows where the Tetrimino in play will land.
    pub enable_ghost_piece: bool,
    /// Allows the Hold command.
    pub enable_hold: bool,
    /// Uses the Variable Goal System instead of the Fixed Goal System. Sprint and Ultra
    /// ignore it.
    pub enable_variable_goal: bool,
    /// Opens the sound device. When false, the game is silent whatever the other settings.
    pub enable_sound: bool,
    /// Plays background music during games (Guideline 2.5.5).
    pub enable_music: bool,
    /// Plays sound effects (Guideline 2.5.6).
    pub enable_sound_effects: bool,
    /// Tetriminos shown in the Next Queue, 1 to 6.
    pub preview_count: usize,
    /// Delay before Auto-Repeat starts, in milliseconds.
    pub das_delay: u64,
    /// Time between Auto-Repeat moves, in milliseconds.
    pub das_repeat: u64,
    /// Lock Down ruleset.
    pub lock_down_mode: LockDownMode,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            board_width: 10,
            board_height: 20,
            game_mode: GameMode::Marathon,
            starting_level: 1,
            lines_per_level: 10,
            enable_ghost_piece: true,
            enable_hold: true,
            enable_variable_goal: false,
            enable_sound: true,
            enable_music: true,
            enable_sound_effects: true,
            preview_count: 3,
            das_delay: 300,
            das_repeat: 50,
            lock_down_mode: LockDownMode::Extended,
        }
    }
}

impl GameConfig {
    /// Loads a config file, using defaults only when the file does not exist.
    /// A file that exists but is invalid is an error, not a silent fallback.
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::load_from_file(path)
    }

    /// Loads and validates a config file.
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let contents =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let config: Self = serde_json::from_str(&contents)
            .with_context(|| format!("parsing {}", path.display()))?;
        config
            .validate()
            .with_context(|| format!("invalid settings in {}", path.display()))?;
        Ok(config)
    }

    /// Writes these settings to a config file as formatted JSON, replacing the file.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("formatting settings")?;
        fs::write(path, json + "\n").with_context(|| format!("writing {}", path.display()))
    }

    /// Checks that all values can produce a working game.
    pub fn validate(&self) -> Result<()> {
        ensure!(self.board_width >= 4, "board_width must be at least 4");
        ensure!(self.board_height >= 4, "board_height must be at least 4");
        ensure!(
            (1..=15).contains(&self.starting_level),
            "starting_level must be from 1 to 15"
        );
        ensure!(
            self.lines_per_level >= 1,
            "lines_per_level must be at least 1"
        );
        ensure!(
            (1..=6).contains(&self.preview_count),
            "preview_count must be from 1 to 6"
        );
        ensure!(self.das_repeat >= 1, "das_repeat must be at least 1");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_valid() {
        assert!(GameConfig::default().validate().is_ok());
    }

    #[test]
    fn test_partial_json_keeps_other_defaults() {
        let config: GameConfig = serde_json::from_str(r#"{"starting_level": 4}"#).unwrap();
        assert_eq!(config.starting_level, 4);
        assert_eq!(config.board_width, 10);
        assert_eq!(config.lock_down_mode, LockDownMode::Extended);
    }

    #[test]
    fn test_music_and_effects_default_on_and_can_be_turned_off_separately() {
        // Guideline 2.5.5 and 2.5.6: both may be turned off, and both default to on.
        let default = GameConfig::default();
        assert!(default.enable_music && default.enable_sound_effects);

        let config: GameConfig = serde_json::from_str(r#"{"enable_music": false}"#).unwrap();
        assert!(!config.enable_music);
        assert!(config.enable_sound_effects);
    }

    #[test]
    fn test_game_mode_defaults_to_marathon_and_parses_from_snake_case() {
        assert_eq!(GameConfig::default().game_mode, GameMode::Marathon);
        let config: GameConfig = serde_json::from_str(r#"{"game_mode": "sprint"}"#).unwrap();
        assert_eq!(config.game_mode, GameMode::Sprint);
        let config: GameConfig = serde_json::from_str(r#"{"game_mode": "ultra"}"#).unwrap();
        assert_eq!(config.game_mode, GameMode::Ultra);
    }

    #[test]
    fn test_lock_down_mode_parses_from_snake_case() {
        let config: GameConfig = serde_json::from_str(r#"{"lock_down_mode": "classic"}"#).unwrap();
        assert_eq!(config.lock_down_mode, LockDownMode::Classic);
    }

    #[test]
    fn test_validate_rejects_bad_values() {
        let bad = [
            GameConfig {
                board_width: 3,
                ..Default::default()
            },
            GameConfig {
                board_height: 0,
                ..Default::default()
            },
            GameConfig {
                starting_level: 0,
                ..Default::default()
            },
            GameConfig {
                starting_level: 16,
                ..Default::default()
            },
            GameConfig {
                lines_per_level: 0,
                ..Default::default()
            },
            GameConfig {
                preview_count: 7,
                ..Default::default()
            },
            GameConfig {
                das_repeat: 0,
                ..Default::default()
            },
        ];
        for config in bad {
            assert!(config.validate().is_err(), "{config:?} should be rejected");
        }
    }

    #[test]
    fn test_missing_file_uses_defaults() {
        let config = GameConfig::load_or_default(Path::new("no_such_config.json")).unwrap();
        assert_eq!(config.board_height, 20);
    }

    #[test]
    fn test_saved_file_loads_back_the_same_settings() {
        let path = std::env::temp_dir().join(format!("tetris_save_{}.json", std::process::id()));
        let config = GameConfig {
            game_mode: GameMode::Ultra,
            preview_count: 5,
            enable_hold: false,
            lock_down_mode: LockDownMode::Classic,
            enable_music: false,
            ..GameConfig::default()
        };
        config.save_to_file(&path).unwrap();
        let loaded = GameConfig::load_from_file(&path);
        fs::remove_file(&path).ok();

        let loaded = loaded.unwrap();
        assert_eq!(loaded.game_mode, GameMode::Ultra);
        assert_eq!(loaded.preview_count, 5);
        assert!(!loaded.enable_hold);
        assert_eq!(loaded.lock_down_mode, LockDownMode::Classic);
        assert!(!loaded.enable_music);
        assert!(loaded.enable_sound_effects);
    }

    #[test]
    fn test_invalid_file_is_an_error() {
        let path = std::env::temp_dir().join(format!("tetris_bad_{}.json", std::process::id()));
        fs::write(&path, "{ not json").unwrap();
        let result = GameConfig::load_or_default(&path);
        fs::remove_file(&path).ok();
        assert!(result.is_err());
    }
}
