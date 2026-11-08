use crate::config::{GameConfig, GameMode};
use std::time::Duration;

/// Marathon ends after this level (Guideline 12.1).
const MAX_LEVEL: u32 = 15;
/// Lines a Sprint asks for (Guideline 12.2).
pub const SPRINT_LINES: u32 = 40;

/// Kind of T-Spin recognized when a Tetrimino Locks Down (Guideline 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TSpin {
    None,
    Mini,
    Full,
}

/// What an awarded action means for the rest of the game.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionResult {
    /// Text to announce, if the Guideline wants this action announced (Guideline 13.1.5).
    pub notification: Option<String>,
    /// True if this action finished the game: the final Marathon level, or the Sprint's lines.
    /// An Ultra ends on time instead, so its actions never finish it.
    pub game_complete: bool,
    /// True if this action earned the Back-to-Back bonus.
    pub back_to_back: bool,
    /// True if this action reached a new level.
    pub level_up: bool,
}

/// Counts of what the player did in one game (Guideline A1.2.7, Game Statistics).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Statistics {
    /// Tetriminos Locked Down, including one that Locks Out.
    pub tetriminos: u32,
    /// Line Clears of each size, with or without a T-Spin.
    pub singles: u32,
    pub doubles: u32,
    pub triples: u32,
    pub tetrises: u32,
    /// T-Spins of every kind, including Minis and T-Spins that clear no lines.
    pub tspins: u32,
}

impl Statistics {
    /// Average of `count` per minute over `elapsed`, or 0 before any time has passed.
    pub fn per_minute(count: u32, elapsed: Duration) -> f64 {
        let minutes = elapsed.as_secs_f64() / 60.0;
        if minutes > 0.0 {
            count as f64 / minutes
        } else {
            0.0
        }
    }

    fn record_action(&mut self, tspin: TSpin, lines: u32) {
        match lines {
            1 => self.singles += 1,
            2 => self.doubles += 1,
            3 => self.triples += 1,
            4 => self.tetrises += 1,
            _ => {}
        }
        if tspin != TSpin::None {
            self.tspins += 1;
        }
    }
}

/// Score, level and Goal for one game (Guideline 6 and 8).
pub struct Scoring {
    pub score: u64,
    /// The part of the score from Soft and Hard Drops, so the scoring demonstration can show
    /// an action's points apart from the drop that placed it.
    pub drop_points: u64,
    pub level: u32,
    pub lines_cleared: u32,
    pub statistics: Statistics,
    /// Lines (or awarded Line Clears) left to reach the Goal.
    pub lines_until_next_level: u32,
    starting_level: u32,
    lines_per_level: u32,
    variable_goal: bool,
    mode: GameMode,
    back_to_back_active: bool,
}

impl Scoring {
    pub fn new(config: &GameConfig) -> Self {
        let mut scoring = Self {
            score: 0,
            drop_points: 0,
            level: config.starting_level,
            lines_cleared: 0,
            statistics: Statistics::default(),
            lines_until_next_level: 0,
            starting_level: config.starting_level,
            lines_per_level: config.lines_per_level,
            variable_goal: config.enable_variable_goal,
            mode: config.game_mode,
            back_to_back_active: false,
        };
        scoring.lines_until_next_level = scoring.starting_goal();
        scoring
    }

    /// Awards 1 point per Soft Dropped row, independent of level (Guideline 8).
    pub fn add_soft_drop(&mut self, rows: u64) {
        self.score += rows;
        self.drop_points += rows;
    }

    /// Awards 2 points per Hard Dropped row, independent of level (Guideline 8).
    pub fn add_hard_drop(&mut self, rows: u64) {
        self.score += 2 * rows;
        self.drop_points += 2 * rows;
    }

    /// Scores the result of a Lock Down, and updates Back-to-Back and the Goal.
    pub fn award_action(&mut self, tspin: TSpin, lines: u32) -> ActionResult {
        self.lines_cleared += lines;

        // The scoring table has no Mini T-Spin clearing 2 or more lines.
        let tspin = match tspin {
            TSpin::Mini if lines >= 2 => TSpin::Full,
            other => other,
        };
        self.statistics.record_action(tspin, lines);

        let action_total = action_points(tspin, lines);
        if action_total == 0 {
            return ActionResult::default();
        }

        let qualifies = lines > 0 && (lines == 4 || tspin != TSpin::None);
        let is_back_to_back = qualifies && self.back_to_back_active;
        let bonus = if is_back_to_back { action_total / 2 } else { 0 };
        let total = action_total + bonus;

        self.score += total * self.level as u64;

        // Only a Line Clear changes the sequence: a no-clear T-Spin neither starts nor breaks it.
        if lines > 0 {
            self.back_to_back_active = qualifies;
        }

        let goal_credit = if self.variable_goal && self.mode == GameMode::Marathon {
            (total / 100) as u32
        } else {
            lines
        };

        let level_before = self.level;
        let game_complete = self.advance_goal(goal_credit);
        ActionResult {
            notification: notification_text(tspin, lines, is_back_to_back),
            game_complete,
            back_to_back: is_back_to_back,
            level_up: self.level > level_before,
        }
    }

    /// Progress toward the Goal as (done, total), in lines or awarded Line Clears.
    pub fn goal_progress(&self) -> (u32, u32) {
        let total = if self.mode == GameMode::Sprint || self.level == self.starting_level {
            self.starting_goal()
        } else {
            self.goal_for_level(self.level)
        };
        (total.saturating_sub(self.lines_until_next_level), total)
    }

    /// Goal for a level reached by playing through the one before it.
    fn goal_for_level(&self, level: u32) -> u32 {
        if self.variable_goal {
            5 * level
        } else {
            self.lines_per_level
        }
    }

    /// First Goal of a game. Starting higher includes the levels skipped (Guideline 6).
    /// A Sprint has only this Goal, and an Ultra has none.
    fn starting_goal(&self) -> u32 {
        if self.mode == GameMode::Ultra {
            0
        } else if self.mode == GameMode::Sprint {
            SPRINT_LINES
        } else if self.variable_goal {
            5 * self.level
        } else {
            self.lines_per_level * self.level
        }
    }

    /// Applies credit toward the Goal. Returns true if it finished the game.
    fn advance_goal(&mut self, credit: u32) -> bool {
        // An Ultra never Levels Up and has no Goal (Guideline 12.3).
        if self.mode == GameMode::Ultra {
            return false;
        }
        // A Sprint never Levels Up (Guideline 12.2).
        if self.mode == GameMode::Sprint {
            self.lines_until_next_level = self.lines_until_next_level.saturating_sub(credit);
            return self.lines_until_next_level == 0;
        }
        let mut remaining = credit;
        while remaining >= self.lines_until_next_level {
            remaining -= self.lines_until_next_level;
            if self.level >= MAX_LEVEL {
                self.lines_until_next_level = 0;
                return true;
            }
            self.level += 1;
            self.lines_until_next_level = self.goal_for_level(self.level);
        }
        self.lines_until_next_level -= remaining;
        false
    }
}

/// Every scoring action with its name and points at level 1, in the order of the Guideline's
/// scoring table (Guideline 8), for Help to show.
pub fn scoring_table() -> Vec<(&'static str, u64)> {
    [
        ("Single", TSpin::None, 1),
        ("Double", TSpin::None, 2),
        ("Triple", TSpin::None, 3),
        ("Tetris", TSpin::None, 4),
        ("Mini T-Spin", TSpin::Mini, 0),
        ("Mini T-Spin Single", TSpin::Mini, 1),
        ("T-Spin", TSpin::Full, 0),
        ("T-Spin Single", TSpin::Full, 1),
        ("T-Spin Double", TSpin::Full, 2),
        ("T-Spin Triple", TSpin::Full, 3),
    ]
    .into_iter()
    .map(|(name, tspin, lines)| (name, action_points(tspin, lines)))
    .collect()
}

/// Action total at level 1 from the scoring table (Guideline 8).
fn action_points(tspin: TSpin, lines: u32) -> u64 {
    match (tspin, lines) {
        (TSpin::None, 1) => 100,
        (TSpin::None, 2) => 300,
        (TSpin::None, 3) => 500,
        (TSpin::None, 4) => 800,
        (TSpin::Mini, 0) => 100,
        (TSpin::Mini, 1) => 200,
        (TSpin::Full, 0) => 400,
        (TSpin::Full, 1) => 800,
        (TSpin::Full, 2) => 1200,
        (TSpin::Full, 3) => 1600,
        _ => 0,
    }
}

/// Name shown for actions the Guideline wants announced (Guideline 13.1.5).
fn action_name(tspin: TSpin, lines: u32) -> Option<&'static str> {
    match (tspin, lines) {
        (TSpin::None, 4) => Some("Tetris"),
        (TSpin::Mini, 0) => Some("Mini T-Spin"),
        (TSpin::Mini, 1) => Some("Mini T-Spin Single"),
        (TSpin::Full, 0) => Some("T-Spin"),
        (TSpin::Full, 1) => Some("T-Spin Single"),
        (TSpin::Full, 2) => Some("T-Spin Double"),
        (TSpin::Full, 3) => Some("T-Spin Triple"),
        _ => None,
    }
}

fn notification_text(tspin: TSpin, lines: u32, is_back_to_back: bool) -> Option<String> {
    let name = action_name(tspin, lines)?;
    if is_back_to_back {
        Some(format!("Back-to-Back {name}"))
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scoring() -> Scoring {
        Scoring::new(&GameConfig::default())
    }

    fn scoring_with(config: GameConfig) -> Scoring {
        Scoring::new(&config)
    }

    #[test]
    fn test_line_clear_scores_match_guideline() {
        for (lines, points) in [(1, 100), (2, 300), (3, 500), (4, 800)] {
            let mut scoring = scoring();
            scoring.level = 3;
            scoring.award_action(TSpin::None, lines);
            assert_eq!(scoring.score, points * 3, "{lines} lines");
        }
    }

    #[test]
    fn test_tspin_scores_match_guideline() {
        let cases = [
            (TSpin::Mini, 0, 100),
            (TSpin::Mini, 1, 200),
            (TSpin::Full, 0, 400),
            (TSpin::Full, 1, 800),
            (TSpin::Full, 2, 1200),
            (TSpin::Full, 3, 1600),
        ];
        for (tspin, lines, points) in cases {
            let mut scoring = scoring();
            scoring.award_action(tspin, lines);
            assert_eq!(scoring.score, points, "{tspin:?} {lines}");
        }
    }

    #[test]
    fn test_scoring_table_lists_every_action_once_with_points() {
        let table = scoring_table();
        assert_eq!(table.len(), 10);
        assert!(table.iter().all(|&(_, points)| points > 0));
        assert_eq!(table[3], ("Tetris", 800));
        assert_eq!(table[9], ("T-Spin Triple", 1600));
        // Every announced action appears under the name it is announced by.
        for tspin in [TSpin::None, TSpin::Mini, TSpin::Full] {
            for lines in 0..=4 {
                if let Some(name) = action_name(tspin, lines) {
                    assert!(table.iter().any(|&(n, _)| n == name), "{name}");
                }
            }
        }
    }

    #[test]
    fn test_mini_tspin_clearing_two_lines_scores_as_tspin_double() {
        let mut scoring = scoring();
        let result = scoring.award_action(TSpin::Mini, 2);
        assert_eq!(scoring.score, 1200);
        assert_eq!(result.notification.as_deref(), Some("T-Spin Double"));
    }

    #[test]
    fn test_plain_lock_scores_nothing() {
        let mut scoring = scoring();
        assert_eq!(
            scoring.award_action(TSpin::None, 0),
            ActionResult::default()
        );
        assert_eq!(scoring.score, 0);
    }

    #[test]
    fn test_drop_points_ignore_level_and_are_counted_apart_from_actions() {
        let mut scoring = scoring();
        scoring.level = 7;
        scoring.add_soft_drop(3);
        scoring.add_hard_drop(5);
        assert_eq!(scoring.score, 13);
        assert_eq!(scoring.drop_points, 13);
        scoring.award_action(TSpin::None, 1);
        assert_eq!(scoring.drop_points, 13);
    }

    #[test]
    fn test_lines_cleared_accumulates() {
        let mut scoring = scoring();
        scoring.award_action(TSpin::None, 2);
        scoring.award_action(TSpin::None, 0);
        scoring.award_action(TSpin::None, 3);
        assert_eq!(scoring.lines_cleared, 5);
    }

    #[test]
    fn test_statistics_count_line_clears_and_tspins() {
        let mut scoring = scoring_with(GameConfig {
            lines_per_level: 1000,
            ..GameConfig::default()
        });
        for (tspin, lines) in [
            (TSpin::None, 1),
            (TSpin::None, 2),
            (TSpin::None, 3),
            (TSpin::None, 4),
            (TSpin::None, 4),
            (TSpin::None, 0),
            (TSpin::Mini, 0),
            (TSpin::Full, 2),
        ] {
            scoring.award_action(tspin, lines);
        }
        let expected = Statistics {
            tetriminos: 0,
            singles: 1,
            doubles: 2,
            triples: 1,
            tetrises: 2,
            tspins: 2,
        };
        assert_eq!(scoring.statistics, expected);
    }

    #[test]
    fn test_per_minute_averages_over_elapsed_time() {
        assert_eq!(Statistics::per_minute(30, Duration::from_secs(90)), 20.0);
        assert_eq!(Statistics::per_minute(5, Duration::ZERO), 0.0);
    }

    #[test]
    fn test_back_to_back_example_totals_5400() {
        // Guideline 8.1: Tetris, T-Spin Double, T-Spin, Tetris, T-Spin Single at level 1.
        let mut scoring = scoring_with(GameConfig {
            lines_per_level: 1000,
            ..GameConfig::default()
        });
        scoring.award_action(TSpin::None, 4);
        scoring.award_action(TSpin::Full, 2);
        scoring.award_action(TSpin::Full, 0);
        scoring.award_action(TSpin::None, 4);
        scoring.award_action(TSpin::Full, 1);
        assert_eq!(scoring.score, 5400);
    }

    #[test]
    fn test_single_breaks_back_to_back() {
        let mut scoring = scoring();
        scoring.award_action(TSpin::None, 4);
        scoring.award_action(TSpin::None, 1);
        let before = scoring.score;
        scoring.award_action(TSpin::None, 4);
        assert_eq!(scoring.score - before, 800);
    }

    #[test]
    fn test_plain_lock_keeps_back_to_back() {
        let mut scoring = scoring();
        scoring.award_action(TSpin::None, 4);
        scoring.award_action(TSpin::None, 0);
        assert!(scoring.back_to_back_active);
    }

    #[test]
    fn test_no_clear_tspin_cannot_start_back_to_back() {
        let mut scoring = scoring();
        scoring.award_action(TSpin::Full, 0);
        assert!(!scoring.back_to_back_active);
    }

    #[test]
    fn test_notifications_name_the_action() {
        let mut scoring = scoring();
        let text = |result: ActionResult| result.notification;
        assert_eq!(text(scoring.award_action(TSpin::None, 1)), None);
        assert_eq!(
            text(scoring.award_action(TSpin::None, 4)).as_deref(),
            Some("Tetris")
        );
        assert_eq!(
            text(scoring.award_action(TSpin::Full, 2)).as_deref(),
            Some("Back-to-Back T-Spin Double")
        );
    }

    #[test]
    fn test_fixed_goal_levels_up_every_10_lines_and_carries_overflow() {
        let mut scoring = scoring();
        assert_eq!(scoring.lines_until_next_level, 10);
        scoring.advance_goal(5);
        assert_eq!((scoring.level, scoring.lines_until_next_level), (1, 5));
        scoring.advance_goal(5);
        assert_eq!((scoring.level, scoring.lines_until_next_level), (2, 10));
        scoring.advance_goal(13);
        assert_eq!((scoring.level, scoring.lines_until_next_level), (3, 7));
    }

    #[test]
    fn test_fixed_goal_higher_start_includes_skipped_levels() {
        // Guideline 6: starting on level 4 requires 40 lines, then 10 for level 5.
        let mut scoring = scoring_with(GameConfig {
            starting_level: 4,
            ..GameConfig::default()
        });
        assert_eq!(scoring.lines_until_next_level, 40);
        scoring.advance_goal(40);
        assert_eq!((scoring.level, scoring.lines_until_next_level), (5, 10));
    }

    #[test]
    fn test_variable_goal_is_5_times_level() {
        let mut scoring = scoring_with(GameConfig {
            enable_variable_goal: true,
            ..GameConfig::default()
        });
        assert_eq!(scoring.lines_until_next_level, 5);
        scoring.advance_goal(5);
        assert_eq!((scoring.level, scoring.lines_until_next_level), (2, 10));
        scoring.advance_goal(10);
        assert_eq!((scoring.level, scoring.lines_until_next_level), (3, 15));
    }

    #[test]
    fn test_variable_goal_awards_table_line_clears() {
        // Guideline 6.1: score at level 1 / 100, with Back-to-Back adding half.
        let cases = [
            (TSpin::None, 1, 1),
            (TSpin::Mini, 0, 1),
            (TSpin::Mini, 1, 2),
            (TSpin::None, 2, 3),
            (TSpin::Full, 0, 4),
            (TSpin::None, 3, 5),
            (TSpin::None, 4, 8),
            (TSpin::Full, 1, 8),
            (TSpin::Full, 2, 12),
            (TSpin::Full, 3, 16),
        ];
        for (tspin, lines, awarded) in cases {
            let mut scoring = scoring_with(GameConfig {
                enable_variable_goal: true,
                starting_level: 15,
                ..GameConfig::default()
            });
            let before = scoring.lines_until_next_level;
            scoring.award_action(tspin, lines);
            assert_eq!(
                before - scoring.lines_until_next_level,
                awarded,
                "{tspin:?} {lines}"
            );
        }
    }

    #[test]
    fn test_variable_goal_back_to_back_adds_half() {
        let mut scoring = scoring_with(GameConfig {
            enable_variable_goal: true,
            starting_level: 15,
            ..GameConfig::default()
        });
        scoring.award_action(TSpin::None, 4);
        let before = scoring.lines_until_next_level;
        scoring.award_action(TSpin::None, 4);
        assert_eq!(before - scoring.lines_until_next_level, 12);
    }

    #[test]
    fn test_marathon_ends_after_level_15() {
        let mut scoring = scoring_with(GameConfig {
            starting_level: 15,
            ..GameConfig::default()
        });
        scoring.lines_until_next_level = 2;
        assert!(scoring.award_action(TSpin::None, 2).game_complete);
        assert_eq!(scoring.level, 15);
    }

    #[test]
    fn test_result_reports_back_to_back_and_level_up() {
        let mut scoring = scoring();
        let first = scoring.award_action(TSpin::None, 4);
        assert!(!first.back_to_back && !first.level_up);

        let second = scoring.award_action(TSpin::None, 4);
        assert!(second.back_to_back);
        assert!(!second.level_up);

        let third = scoring.award_action(TSpin::None, 2);
        assert!(!third.back_to_back);
        assert!(third.level_up);
        assert_eq!(scoring.level, 2);
    }

    #[test]
    fn test_marathon_continues_below_level_15() {
        let mut scoring = scoring();
        assert!(!scoring.award_action(TSpin::None, 4).game_complete);
    }

    #[test]
    fn test_goal_progress_follows_the_current_goal() {
        let mut scoring = scoring_with(GameConfig {
            starting_level: 4,
            ..GameConfig::default()
        });
        assert_eq!(scoring.goal_progress(), (0, 40));
        scoring.advance_goal(15);
        assert_eq!(scoring.goal_progress(), (15, 40));
        scoring.advance_goal(28);
        assert_eq!((scoring.level, scoring.goal_progress()), (5, (3, 10)));
    }

    fn sprint(starting_level: u32, variable_goal: bool) -> Scoring {
        scoring_with(GameConfig {
            game_mode: GameMode::Sprint,
            starting_level,
            enable_variable_goal: variable_goal,
            ..GameConfig::default()
        })
    }

    #[test]
    fn test_sprint_ends_on_its_40th_line_and_never_levels_up() {
        let mut scoring = sprint(5, false);
        assert_eq!(scoring.goal_progress(), (0, 40));
        for _ in 0..9 {
            let result = scoring.award_action(TSpin::None, 4);
            assert!(!result.game_complete && !result.level_up);
        }
        assert_eq!((scoring.level, scoring.goal_progress()), (5, (36, 40)));

        scoring.award_action(TSpin::None, 3);
        assert!(scoring.award_action(TSpin::None, 4).game_complete);
        assert_eq!((scoring.level, scoring.goal_progress()), (5, (40, 40)));
    }

    #[test]
    fn test_sprint_counts_lines_even_with_variable_goal() {
        let mut scoring = sprint(1, true);
        scoring.award_action(TSpin::Full, 2);
        assert_eq!(scoring.goal_progress(), (2, 40));
        scoring.award_action(TSpin::Full, 0);
        assert_eq!(scoring.goal_progress(), (2, 40));
    }

    #[test]
    fn test_sprint_still_scores_at_its_level() {
        let mut scoring = sprint(3, false);
        scoring.award_action(TSpin::None, 4);
        assert_eq!(scoring.score, 800 * 3);
    }

    #[test]
    fn test_ultra_never_levels_up_or_completes() {
        let mut scoring = scoring_with(GameConfig {
            game_mode: GameMode::Ultra,
            starting_level: 4,
            enable_variable_goal: true,
            ..GameConfig::default()
        });
        for _ in 0..100 {
            let result = scoring.award_action(TSpin::None, 4);
            assert!(!result.game_complete && !result.level_up);
        }
        assert_eq!(scoring.level, 4);
        assert_eq!(scoring.lines_cleared, 400);
        assert_eq!(scoring.score, 800 * 4 + 1200 * 4 * 99);
    }

    #[test]
    fn test_goal_progress_with_variable_goal() {
        let mut scoring = scoring_with(GameConfig {
            enable_variable_goal: true,
            ..GameConfig::default()
        });
        scoring.advance_goal(7);
        assert_eq!((scoring.level, scoring.goal_progress()), (2, (2, 10)));
    }
}
