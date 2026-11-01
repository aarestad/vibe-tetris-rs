//! Plays many games with random input and checks that the rules always hold.

use std::time::Duration;
use tetris_rs::config::{GameConfig, LockDownMode};
use tetris_rs::game_state::GameState;

/// Small fixed-seed generator, so that a failure can be repeated.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, limit: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % limit
    }
}

fn play(config: GameConfig, seed: u64) {
    let mut random = Lcg(seed);
    let mut state = GameState::new(config);
    state.spawn_piece();

    for _ in 0..20_000 {
        match random.next(12) {
            0 => {
                state.shift_piece(-1);
            }
            1 => {
                state.shift_piece(1);
            }
            2 => {
                state.rotate_piece(true);
            }
            3 => {
                state.rotate_piece(false);
            }
            4 => state.hold_piece(),
            5 => state.hard_drop(),
            6 => {
                state.soft_drop_step();
            }
            7 => state.set_soft_drop(random.next(2) == 0),
            _ => {}
        }

        let score_before = state.score;
        state.tick(Duration::from_millis(random.next(40)));
        assert!(state.score >= score_before);

        if let Some(piece) = state.current_piece
            && !state.game_over
        {
            assert!(state.board.is_valid_position(&piece), "{piece:?}");
        }
        assert!((1..=15).contains(&state.level));
        assert_eq!(state.next_pieces.len(), state.config.preview_count);

        if state.game_over {
            return;
        }
    }
}

#[test]
fn test_random_play_keeps_rules_in_every_mode() {
    let modes = [
        LockDownMode::Extended,
        LockDownMode::Infinite,
        LockDownMode::Classic,
    ];
    for seed in 0..30 {
        for lock_down_mode in modes {
            for enable_variable_goal in [false, true] {
                let config = GameConfig {
                    lock_down_mode,
                    enable_variable_goal,
                    enable_sound: false,
                    starting_level: (seed % 15 + 1) as u32,
                    preview_count: (seed % 6 + 1) as usize,
                    ..GameConfig::default()
                };
                play(config, seed);
            }
        }
    }
}
