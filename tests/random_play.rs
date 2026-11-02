//! Plays many games with random input and checks that the rules always hold.

use std::time::Duration;
use tetris_rs::config::{GameConfig, LockDownMode};
use tetris_rs::input::{InputAction, InputEvent};
use tetris_rs::session::Session;

const ACTIONS: [InputAction; 9] = [
    InputAction::MoveLeft,
    InputAction::MoveRight,
    InputAction::MoveDown,
    InputAction::RotateClockwise,
    InputAction::RotateCounterClockwise,
    InputAction::HardDrop,
    InputAction::Hold,
    InputAction::Pause,
    InputAction::ToggleHelp,
];

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

fn random_input(random: &mut Lcg) -> Option<InputEvent> {
    // Most frames have no input, as in real play.
    if random.next(3) != 0 {
        return None;
    }
    let action = ACTIONS[random.next(ACTIONS.len() as u64) as usize];
    Some(if random.next(2) == 0 {
        InputEvent::Pressed(action)
    } else {
        InputEvent::Released(action)
    })
}

fn play(config: GameConfig, key_release_events: bool, seed: u64) {
    let mut random = Lcg(seed);
    let mut session = Session::new(config);
    session.set_key_release_events(key_release_events);

    for _ in 0..20_000 {
        if let Some(input) = random_input(&mut random) {
            session.handle_input(input);
        }

        let score_before = session.state().scoring.score;
        let elapsed_before = session.state().elapsed;
        session.update(Duration::from_millis(random.next(40)));

        let state = session.state();
        assert!(state.scoring.score >= score_before);
        if !session.screen().is_playing() {
            assert_eq!(state.elapsed, elapsed_before);
        }
        if let Some(piece) = state.current_piece()
            && !state.is_game_over()
        {
            assert!(state.board.is_valid_position(&piece), "{piece:?}");
        }
        assert!((1..=15).contains(&state.scoring.level));
        assert_eq!(state.next_pieces.len(), state.config.preview_count);

        if state.is_game_over() {
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
                play(config.clone(), false, seed);
                play(config, true, seed);
            }
        }
    }
}
