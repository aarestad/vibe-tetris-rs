use crate::config::GameMode;
use crate::game_state::{GameOutcome, GameState};
use crate::scoring::SPRINT_LINES;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Entries kept in each variant's table.
pub const TABLE_SIZE: usize = 10;
/// Longest name the table holds.
pub const NAME_MAX_LEN: usize = 10;
/// The name saved when the player enters none.
pub const DEFAULT_NAME: &str = "Player";

/// One finished game in the High Score Table (Guideline 11.2). Every variant records the same
/// fields, and each shows only the columns that matter to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighScore {
    pub name: String,
    pub score: u64,
    pub level: u32,
    pub lines: u32,
    /// Time played in milliseconds. A Sprint ranks by it.
    pub time_ms: u64,
}

impl HighScore {
    /// The result of a finished game, with no name yet, or None if the game cannot enter the
    /// table. A game the player quit never counts, nor does a Sprint that did not reach its
    /// lines, nor a game that scored nothing.
    pub fn from_game(state: &GameState) -> Option<Self> {
        let outcome = state.outcome()?;
        let mode = state.config.game_mode;
        let counts = match outcome {
            GameOutcome::Quit => false,
            _ if mode == GameMode::Sprint => outcome == GameOutcome::SprintComplete,
            _ => state.scoring.score > 0,
        };
        if !counts {
            return None;
        }
        let lines = match mode {
            GameMode::Sprint => state.scoring.lines_cleared.min(SPRINT_LINES),
            GameMode::Marathon | GameMode::Ultra => state.scoring.lines_cleared,
        };
        Some(Self {
            name: String::new(),
            score: state.scoring.score,
            level: state.scoring.level,
            lines,
            time_ms: state.elapsed.as_millis() as u64,
        })
    }

    pub fn time(&self) -> Duration {
        Duration::from_millis(self.time_ms)
    }
}

/// How two results compare in a variant's table: `Less` means `a` ranks above `b`.
fn compare(mode: GameMode, a: &HighScore, b: &HighScore) -> Ordering {
    match mode {
        GameMode::Marathon => (b.score, b.level, b.lines).cmp(&(a.score, a.level, a.lines)),
        GameMode::Sprint => a.time_ms.cmp(&b.time_ms).then(b.score.cmp(&a.score)),
        GameMode::Ultra => (b.score, b.lines).cmp(&(a.score, a.lines)),
    }
}

/// The best games of each variant, kept between runs (Guideline 11.2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HighScores {
    marathon: Vec<HighScore>,
    sprint: Vec<HighScore>,
    ultra: Vec<HighScore>,
    /// The name entered most recently, offered again at the next name entry.
    last_name: String,
}

impl HighScores {
    /// Loads the scores file, starting with empty tables only when the file does not exist.
    /// A file that exists but cannot be read is an error, so it is never overwritten.
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::load_from_file(path)
    }

    /// Loads a scores file. Tables are put back in order and cut to size, so a hand-edited file
    /// still works.
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let contents =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut scores: Self = serde_json::from_str(&contents)
            .with_context(|| format!("parsing {}", path.display()))?;
        for mode in [GameMode::Marathon, GameMode::Sprint, GameMode::Ultra] {
            let table = scores.table_mut(mode);
            table.sort_by(|a, b| compare(mode, a, b));
            table.truncate(TABLE_SIZE);
            for entry in table.iter_mut() {
                entry.name = clean_name(&entry.name);
            }
        }
        Ok(scores)
    }

    /// Writes the tables to a scores file as formatted JSON, replacing the file.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("formatting high scores")?;
        fs::write(path, json + "\n").with_context(|| format!("writing {}", path.display()))
    }

    /// A variant's table, best first.
    pub fn table(&self, mode: GameMode) -> &[HighScore] {
        match mode {
            GameMode::Marathon => &self.marathon,
            GameMode::Sprint => &self.sprint,
            GameMode::Ultra => &self.ultra,
        }
    }

    fn table_mut(&mut self, mode: GameMode) -> &mut Vec<HighScore> {
        match mode {
            GameMode::Marathon => &mut self.marathon,
            GameMode::Sprint => &mut self.sprint,
            GameMode::Ultra => &mut self.ultra,
        }
    }

    /// The place, from 0, that a result would take, or None if it would not make the table.
    /// A result tied with one already there goes below it.
    pub fn rank_of(&self, mode: GameMode, score: &HighScore) -> Option<usize> {
        let rank = self
            .table(mode)
            .iter()
            .take_while(|entry| compare(mode, entry, score) != Ordering::Greater)
            .count();
        (rank < TABLE_SIZE).then_some(rank)
    }

    /// Adds a result if it makes the table, and returns its place from 0. Its name becomes the
    /// one offered at the next name entry.
    pub fn insert(&mut self, mode: GameMode, mut score: HighScore) -> Option<usize> {
        let rank = self.rank_of(mode, &score)?;
        score.name = clean_name(&score.name);
        self.last_name = score.name.clone();
        let table = self.table_mut(mode);
        table.insert(rank, score);
        table.truncate(TABLE_SIZE);
        Some(rank)
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }
}

/// Characters a name may contain.
fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || " -_.'!".contains(c)
}

/// A name trimmed to the allowed characters and length, or the default name if none is left.
fn clean_name(name: &str) -> String {
    let name: String = name
        .chars()
        .filter(|c| is_name_char(*c))
        .take(NAME_MAX_LEN)
        .collect();
    let name = name.trim();
    if name.is_empty() {
        DEFAULT_NAME.to_string()
    } else {
        name.to_string()
    }
}

/// A key press while typing a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInput {
    Char(char),
    Backspace,
    Enter,
    /// Ctrl+C: leave the program without saving the result.
    Quit,
}

/// What should happen after a key press during name entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameEntryResult {
    Typing,
    /// The player confirmed this name.
    Done(String),
    Quit,
}

/// Typing a name for a new high score (Guideline 11.2), with no terminal.
#[derive(Debug, Clone)]
pub struct NameEntry {
    name: String,
}

impl NameEntry {
    /// Starts with a name already filled in, usually the last one entered.
    pub fn new(initial: &str) -> Self {
        let name = initial
            .chars()
            .filter(|c| is_name_char(*c))
            .take(NAME_MAX_LEN)
            .collect();
        Self { name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn handle_input(&mut self, input: TextInput) -> NameEntryResult {
        match input {
            TextInput::Char(c) if is_name_char(c) && self.name.len() < NAME_MAX_LEN => {
                self.name.push(c);
            }
            TextInput::Char(_) => {}
            TextInput::Backspace => {
                self.name.pop();
            }
            TextInput::Enter => return NameEntryResult::Done(clean_name(&self.name)),
            TextInput::Quit => return NameEntryResult::Quit,
        }
        NameEntryResult::Typing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GameConfig;
    use crate::game_state::Phase;

    fn entry(name: &str, score: u64, level: u32, lines: u32, time_ms: u64) -> HighScore {
        HighScore {
            name: name.to_string(),
            score,
            level,
            lines,
            time_ms,
        }
    }

    fn scored(score: u64) -> HighScore {
        entry("A", score, 1, 0, 0)
    }

    fn timed(time_ms: u64) -> HighScore {
        entry("A", 0, 1, SPRINT_LINES, time_ms)
    }

    fn finished_game(mode: GameMode, outcome: GameOutcome, score: u64) -> GameState {
        let mut state = GameState::new(GameConfig {
            game_mode: mode,
            enable_sound: false,
            ..GameConfig::default()
        });
        state.scoring.score = score;
        state.scoring.lines_cleared = 43;
        state.elapsed = Duration::from_millis(83_456);
        state.phase = Phase::GameOver {
            outcome,
            piece: None,
        };
        state
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("tetris_{name}_{}.json", std::process::id()))
    }

    #[test]
    fn test_marathon_and_ultra_rank_by_score_and_sprint_by_time() {
        let mut scores = HighScores::default();
        for score in [300, 900, 600] {
            scores.insert(GameMode::Marathon, scored(score));
            scores.insert(GameMode::Ultra, scored(score));
        }
        for time in [70_000, 50_000, 60_000] {
            scores.insert(GameMode::Sprint, timed(time));
        }
        for mode in [GameMode::Marathon, GameMode::Ultra] {
            let order: Vec<u64> = scores.table(mode).iter().map(|e| e.score).collect();
            assert_eq!(order, [900, 600, 300], "{mode:?}");
        }
        let order: Vec<u64> = scores
            .table(GameMode::Sprint)
            .iter()
            .map(|e| e.time_ms)
            .collect();
        assert_eq!(order, [50_000, 60_000, 70_000]);
    }

    #[test]
    fn test_ties_break_on_level_then_lines_and_go_below_equal_results() {
        let mut scores = HighScores::default();
        scores.insert(GameMode::Marathon, entry("Low", 500, 3, 20, 0));
        assert_eq!(
            scores.insert(GameMode::Marathon, entry("High", 500, 4, 20, 0)),
            Some(0)
        );
        assert_eq!(
            scores.insert(GameMode::Marathon, entry("Same", 500, 4, 20, 0)),
            Some(1)
        );
        assert_eq!(
            scores.insert(GameMode::Ultra, entry("First", 500, 1, 30, 0)),
            Some(0)
        );
        assert_eq!(
            scores.insert(GameMode::Ultra, entry("More", 500, 1, 31, 0)),
            Some(0)
        );
        scores.insert(GameMode::Sprint, entry("Slow", 100, 1, 40, 60_000));
        assert_eq!(
            scores.insert(GameMode::Sprint, entry("Rich", 200, 1, 40, 60_000)),
            Some(0)
        );
    }

    #[test]
    fn test_table_keeps_ten_and_a_result_below_them_does_not_qualify() {
        let mut scores = HighScores::default();
        for score in 1..=12 {
            scores.insert(GameMode::Marathon, scored(score * 100));
        }
        let table = scores.table(GameMode::Marathon);
        assert_eq!(table.len(), TABLE_SIZE);
        assert_eq!(table[0].score, 1200);
        assert_eq!(table[9].score, 300);

        assert_eq!(scores.rank_of(GameMode::Marathon, &scored(300)), None);
        assert_eq!(scores.insert(GameMode::Marathon, scored(200)), None);
        assert_eq!(scores.rank_of(GameMode::Marathon, &scored(301)), Some(9));
    }

    #[test]
    fn test_variants_keep_separate_tables() {
        let mut scores = HighScores::default();
        scores.insert(GameMode::Ultra, scored(100));
        assert!(scores.table(GameMode::Marathon).is_empty());
        assert!(scores.table(GameMode::Sprint).is_empty());
        assert_eq!(scores.table(GameMode::Ultra).len(), 1);
    }

    #[test]
    fn test_finished_games_record_their_results() {
        let state = finished_game(GameMode::Marathon, GameOutcome::BlockOut, 1234);
        let result = HighScore::from_game(&state).unwrap();
        assert_eq!(result.score, 1234);
        assert_eq!(result.level, 1);
        assert_eq!(result.lines, 43);
        assert_eq!(result.time_ms, 83_456);

        let state = finished_game(GameMode::Sprint, GameOutcome::SprintComplete, 0);
        let result = HighScore::from_game(&state).unwrap();
        assert_eq!(
            result.lines, SPRINT_LINES,
            "lines past the Goal are not counted"
        );
        assert_eq!(result.time(), Duration::from_millis(83_456));

        for outcome in [GameOutcome::UltraComplete, GameOutcome::LockOut] {
            let state = finished_game(GameMode::Ultra, outcome, 50);
            assert!(HighScore::from_game(&state).is_some(), "{outcome:?}");
        }
    }

    #[test]
    fn test_quit_unfinished_sprints_and_empty_scores_never_count() {
        let cases = [
            (GameMode::Marathon, GameOutcome::Quit, 5000),
            (GameMode::Ultra, GameOutcome::Quit, 5000),
            (GameMode::Sprint, GameOutcome::BlockOut, 5000),
            (GameMode::Sprint, GameOutcome::Quit, 5000),
            (GameMode::Marathon, GameOutcome::BlockOut, 0),
            (GameMode::Ultra, GameOutcome::UltraComplete, 0),
        ];
        for (mode, outcome, score) in cases {
            let state = finished_game(mode, outcome, score);
            assert_eq!(
                HighScore::from_game(&state),
                None,
                "{mode:?} {outcome:?} {score}"
            );
        }
        let playing = GameState::new(GameConfig::default());
        assert_eq!(HighScore::from_game(&playing), None);
    }

    #[test]
    fn test_saved_file_loads_back_the_same_tables() {
        let path = temp_path("scores_save");
        let mut scores = HighScores::default();
        scores.insert(GameMode::Marathon, entry("Ann", 4000, 5, 44, 300_000));
        scores.insert(GameMode::Sprint, entry("Bo", 900, 1, 40, 61_230));
        scores.insert(GameMode::Ultra, entry("Cy", 7000, 1, 50, 180_000));
        scores.save_to_file(&path).unwrap();
        let loaded = HighScores::load_or_default(&path);
        fs::remove_file(&path).ok();

        let loaded = loaded.unwrap();
        assert_eq!(loaded, scores);
        assert_eq!(loaded.last_name(), "Cy");
    }

    #[test]
    fn test_missing_file_gives_empty_tables_and_invalid_file_is_an_error() {
        let missing = HighScores::load_or_default(Path::new("no_such_scores.json")).unwrap();
        assert_eq!(missing, HighScores::default());

        let path = temp_path("scores_bad");
        fs::write(&path, "{ not json").unwrap();
        let result = HighScores::load_or_default(&path);
        fs::remove_file(&path).ok();
        assert!(result.is_err());
    }

    #[test]
    fn test_hand_edited_file_is_sorted_cut_to_size_and_cleaned() {
        let path = temp_path("scores_edited");
        let marathon: Vec<String> = (1..=12)
            .map(|score| {
                format!(
                    r#"{{"name": "Name{score}IsFarTooLong", "score": {score}, "level": 1,
                    "lines": 0, "time_ms": 0}}"#
                )
            })
            .collect();
        let json = format!(
            r#"{{"marathon": [{}], "ultra": [{{"name": "  ", "score": 5, "level": 1,
            "lines": 0, "time_ms": 0}}]}}"#,
            marathon.join(",")
        );
        fs::write(&path, json).unwrap();
        let loaded = HighScores::load_from_file(&path);
        fs::remove_file(&path).ok();

        let loaded = loaded.unwrap();
        let table = loaded.table(GameMode::Marathon);
        assert_eq!(table.len(), TABLE_SIZE);
        assert_eq!(table[0].score, 12);
        assert_eq!(table[0].name, "Name12IsFa");
        assert_eq!(loaded.table(GameMode::Ultra)[0].name, DEFAULT_NAME);
        assert!(loaded.table(GameMode::Sprint).is_empty());
    }

    #[test]
    fn test_name_entry_types_deletes_and_stops_at_the_length_limit() {
        let mut entry = NameEntry::new("");
        for c in "Ann Lee-Smith!".chars() {
            assert_eq!(
                entry.handle_input(TextInput::Char(c)),
                NameEntryResult::Typing
            );
        }
        assert_eq!(entry.name(), "Ann Lee-Sm");
        entry.handle_input(TextInput::Backspace);
        entry.handle_input(TextInput::Backspace);
        entry.handle_input(TextInput::Char('x'));
        assert_eq!(entry.name(), "Ann Lee-x");
        assert_eq!(
            entry.handle_input(TextInput::Enter),
            NameEntryResult::Done("Ann Lee-x".to_string())
        );
    }

    #[test]
    fn test_name_entry_ignores_other_characters_and_backspace_on_empty() {
        let mut entry = NameEntry::new("");
        entry.handle_input(TextInput::Backspace);
        for c in ['é', '/', '\t', '{'] {
            entry.handle_input(TextInput::Char(c));
        }
        assert_eq!(entry.name(), "");
    }

    #[test]
    fn test_empty_or_blank_name_saves_as_the_default() {
        for typed in ["", "   "] {
            let mut entry = NameEntry::new(typed);
            assert_eq!(
                entry.handle_input(TextInput::Enter),
                NameEntryResult::Done(DEFAULT_NAME.to_string()),
                "{typed:?}"
            );
        }
        let mut entry = NameEntry::new("");
        entry.handle_input(TextInput::Char(' '));
        entry.handle_input(TextInput::Char('Z'));
        entry.handle_input(TextInput::Char(' '));
        assert_eq!(
            entry.handle_input(TextInput::Enter),
            NameEntryResult::Done("Z".to_string())
        );
    }

    #[test]
    fn test_name_entry_starts_from_the_last_name_and_ctrl_c_quits() {
        let mut scores = HighScores::default();
        scores.insert(GameMode::Marathon, entry("Dee", 100, 1, 0, 0));
        let mut entry = NameEntry::new(scores.last_name());
        assert_eq!(entry.name(), "Dee");
        assert_eq!(entry.handle_input(TextInput::Quit), NameEntryResult::Quit);
    }
}
