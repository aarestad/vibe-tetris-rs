//! Plays whole games with a simple bot that places each Tetrimino well, sending real key events.
//! Random input tops out quickly, so this is what reaches many Line Clears and high levels.
//!
//! The games are seeded, because this simple bot does sometimes lose to an unlucky order of
//! Tetriminos. A different result for these seeds means the game's behavior has changed.

use std::time::Duration;
use tetris_rs::config::GameConfig;
use tetris_rs::game_state::{GameOutcome, GameState};
use tetris_rs::input::{InputAction, InputEvent};
use tetris_rs::session::Session;
use tetris_rs::tetrimino::{Facing, Tetrimino};

const FRAME: Duration = Duration::from_millis(16);
/// Rows of the grid the bot scores: the Matrix plus a few above the Skyline.
const ROWS_ABOVE: i32 = 4;
const MAX_PIECES: u32 = 2000;

type Grid = Vec<Vec<bool>>;

fn grid_of(state: &GameState) -> Grid {
    let board = state.board();
    let height = board.get_height() as i32;
    (-ROWS_ABOVE..height)
        .map(|y| {
            (0..board.get_width())
                .map(|x| board.get_cell(x, y).is_some())
                .collect()
        })
        .collect()
}

/// Rates a position after a Tetrimino lands. Higher is better.
fn rate(grid: &mut Grid) -> f64 {
    let width = grid[0].len();
    let rows = grid.len();
    let full_before = grid.len();
    grid.retain(|row| row.iter().any(|cell| !cell));
    let lines = full_before - grid.len();
    for _ in 0..lines {
        grid.insert(0, vec![false; width]);
    }

    let mut heights = vec![0usize; width];
    let mut holes = 0;
    for (x, height) in heights.iter_mut().enumerate() {
        let top = (0..rows).find(|&y| grid[y][x]);
        if let Some(top) = top {
            *height = rows - top;
            holes += (top..rows).filter(|&y| !grid[y][x]).count();
        }
    }
    let total: usize = heights.iter().sum();
    let bumpiness: usize = heights.windows(2).map(|h| h[0].abs_diff(h[1])).sum();

    0.76 * lines as f64 - 0.51 * total as f64 - 0.36 * holes as f64 - 0.18 * bumpiness as f64
}

/// The best facing and column for the Tetrimino in play.
fn best_placement(state: &GameState, piece: Tetrimino) -> Option<(usize, i32)> {
    let board = state.board();
    let base = grid_of(state);
    let mut best: Option<(f64, usize, i32)> = None;

    for (turns, facing) in Facing::ALL.into_iter().enumerate() {
        for x in -3..board.get_width() as i32 {
            let mut landing = Tetrimino {
                x,
                y: piece.y,
                facing,
                ..piece
            };
            if !board.is_valid_position(&landing) {
                continue;
            }
            while board.is_valid_position(&Tetrimino {
                y: landing.y + 1,
                ..landing
            }) {
                landing.y += 1;
            }

            let mut grid = base.clone();
            let blocks = landing.get_blocks();
            if blocks.iter().any(|(_, dy)| landing.y + dy < -ROWS_ABOVE) {
                continue;
            }
            for (dx, dy) in blocks {
                grid[(landing.y + dy + ROWS_ABOVE) as usize][(landing.x + dx) as usize] = true;
            }

            let score = rate(&mut grid);
            if best.is_none_or(|(top, _, _)| score > top) {
                best = Some((score, turns, x));
            }
        }
    }
    best.map(|(_, turns, x)| (turns, x))
}

fn press(session: &mut Session, action: InputAction) {
    session.handle_input(InputEvent::Pressed(action));
    session.handle_input(InputEvent::Released(action));
}

/// Plays one Tetrimino with key presses, as a person would.
fn place(session: &mut Session, piece: Tetrimino) {
    if let Some((turns, column)) = best_placement(session.state(), piece) {
        for _ in 0..turns {
            press(session, InputAction::RotateClockwise);
        }
        let current = session.state().current_piece().unwrap();
        let moves = column - current.x;
        let action = if moves < 0 {
            InputAction::MoveLeft
        } else {
            InputAction::MoveRight
        };
        for _ in 0..moves.abs() {
            press(session, action);
        }
    }
    press(session, InputAction::HardDrop);
}

fn play_marathon(config: GameConfig, seed: u64) -> Session {
    let mut session = Session::with_seed(config, seed);

    for _ in 0..MAX_PIECES {
        // Run frames until the next Tetrimino is in play, checking the rules on the way.
        while session.state().current_piece().is_none() {
            session.update(FRAME);
        }
        if session.state().is_game_over() {
            return session;
        }

        let before = (
            session.state().scoring().score,
            session.state().scoring().lines_cleared,
            session.state().scoring().level,
        );
        let piece = session.state().current_piece().unwrap();
        assert!(session.state().board().is_valid_position(&piece));

        place(&mut session, piece);

        for _ in 0..200 {
            if session.state().current_piece().is_some() || session.state().is_game_over() {
                break;
            }
            session.update(FRAME);
        }

        let scoring = session.state().scoring();
        // A drop that moves no rows and clears nothing scores nothing.
        assert!(scoring.score >= before.0);
        assert!((0..=4).contains(&(scoring.lines_cleared - before.1)));
        assert!(scoring.level >= before.2 && scoring.level <= 15);

        if session.state().is_game_over() {
            return session;
        }
    }
    panic!("the game did not end within {MAX_PIECES} Tetriminos");
}

fn config() -> GameConfig {
    GameConfig {
        enable_sound: false,
        ..GameConfig::default()
    }
}

/// Seeds the bot wins with. Chosen by running it, then fixed so that the tests are repeatable.
const SEEDS: [u64; 3] = [1, 2, 3];

#[test]
fn test_skilled_fixed_goal_marathon_reaches_the_end() {
    for seed in SEEDS {
        let session = play_marathon(config(), seed);
        let state = session.state();

        assert_eq!(
            state.outcome(),
            Some(GameOutcome::MarathonComplete),
            "{seed}"
        );
        assert_eq!(state.scoring().level, 15, "{seed}");
        // 10 lines for each of 15 levels, and the last clear can overshoot by up to 3.
        assert!(
            (150..=153).contains(&state.scoring().lines_cleared),
            "{seed}"
        );
    }
}

#[test]
fn test_skilled_variable_goal_marathon_reaches_the_end() {
    for seed in SEEDS {
        // From level 1 this needs 600 awarded Line Clears, more than this bot reliably survives.
        // Starting at level 13 still plays through three Goals: 65, 70 and 75.
        let session = play_marathon(
            GameConfig {
                enable_variable_goal: true,
                starting_level: 13,
                ..config()
            },
            seed,
        );
        let state = session.state();

        assert_eq!(
            state.outcome(),
            Some(GameOutcome::MarathonComplete),
            "{seed}"
        );
        assert_eq!(state.scoring().level, 15, "{seed}");
        // 210 awarded Line Clears, and no line earns more than 8 outside a Back-to-Back.
        assert!(state.scoring().lines_cleared >= 210 / 12, "{seed}");
    }
}
