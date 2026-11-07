use crate::game_state::{GameOutcome, GameState};

/// A congratulatory message and fireworks for a game that went particularly well
/// (Guideline 11.1). Better results get more fireworks, as the Guideline suggests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReward {
    /// The best result yet in this variant's High Score Table, beating at least one earlier
    /// game.
    Record,
    /// A Marathon played through its final level.
    MarathonComplete,
    /// A place in the High Score Table.
    HighScore,
}

impl EndReward {
    /// The reward a finished game earns, if any. `rank` is its place in the High Score Table
    /// from 0, if it made the table, and `table_was_empty` says whether the table had no
    /// entries before this game, when first place is no achievement.
    pub fn for_game(state: &GameState, rank: Option<usize>, table_was_empty: bool) -> Option<Self> {
        if rank == Some(0) && !table_was_empty {
            return Some(Self::Record);
        }
        if state.outcome() == Some(GameOutcome::MarathonComplete) {
            return Some(Self::MarathonComplete);
        }
        rank.map(|_| Self::HighScore)
    }

    /// The congratulatory message for the Game Over box.
    pub fn message(self) -> &'static str {
        match self {
            Self::Record => "A new record! Bravo!",
            Self::MarathonComplete => "You conquered level 15!",
            Self::HighScore => "Well played!",
        }
    }

    /// Number of fireworks shown at once behind the Game Over box.
    pub fn fireworks(self) -> usize {
        match self {
            Self::Record => 6,
            Self::MarathonComplete => 4,
            Self::HighScore => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GameConfig;
    use crate::game_state::Phase;

    fn finished(outcome: GameOutcome) -> GameState {
        let mut state = GameState::new(GameConfig {
            enable_sound: false,
            ..GameConfig::default()
        });
        state.phase = Phase::GameOver {
            outcome,
            piece: None,
        };
        state
    }

    #[test]
    fn test_rewards_rank_by_achievement() {
        let top_out = finished(GameOutcome::BlockOut);
        let complete = finished(GameOutcome::MarathonComplete);
        let cases = [
            (&top_out, Some(0), false, Some(EndReward::Record)),
            (&complete, Some(0), false, Some(EndReward::Record)),
            (&top_out, Some(0), true, Some(EndReward::HighScore)),
            (&complete, Some(0), true, Some(EndReward::MarathonComplete)),
            (&complete, None, false, Some(EndReward::MarathonComplete)),
            (&top_out, Some(4), false, Some(EndReward::HighScore)),
            (&top_out, None, false, None),
        ];
        for (state, rank, empty, expected) in cases {
            assert_eq!(
                EndReward::for_game(state, rank, empty),
                expected,
                "{:?} {rank:?} {empty}",
                state.outcome()
            );
        }
    }

    #[test]
    fn test_better_rewards_have_more_fireworks() {
        assert!(EndReward::Record.fireworks() > EndReward::MarathonComplete.fireworks());
        assert!(EndReward::MarathonComplete.fireworks() > EndReward::HighScore.fireworks());
    }
}
