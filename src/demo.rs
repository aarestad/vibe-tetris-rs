use crate::config::{GameConfig, GameMode};
use crate::game_state::{GameState, Phase};
use crate::input::{InputAction, InputEvent};
use crate::menu::MenuInput;
use crate::session::Session;
use crate::sound::SoundEffect;
use crate::tetrimino::TetriminoType;
use std::time::Duration;

/// The demo runs in steps of this length, whatever the frame rate, so every scene plays out
/// exactly as its tests check it.
const TICK: Duration = Duration::from_millis(10);
/// Time to look at a scene's Matrix before its first key.
const FIRST_KEY_DELAY: Duration = Duration::from_millis(700);
/// Time between scripted key presses, longer than the repeat guard's gap.
const KEY_DELAY: Duration = Duration::from_millis(160);
/// Time between scripted Soft Drop presses.
const DOWN_DELAY: Duration = Duration::from_millis(30);
/// How long a scene's result stays on screen before the next scene.
const RESULT_TIME: Duration = Duration::from_millis(2500);

/// One scripted key press.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    Press(InputAction),
    /// Waits for the scene's next Tetrimino to be in play.
    NextPiece,
}

/// One point-scoring action, shown on the real engine (Guideline 8).
#[derive(Debug)]
pub struct Scene {
    /// The action's name, as in the Guideline's scoring table.
    pub name: &'static str,
    pub description: &'static str,
    /// The action's points as the Guideline gives them.
    pub points: &'static str,
    /// The Matrix's bottom rows, top first: `.` is empty and anything else is a Block.
    rows: &'static [&'static str],
    pieces: &'static [TetriminoType],
    /// Keys as `L`, `R`, `D`, `CW`, `CCW`, `HD` and `NEXT`, each with an optional `*count`.
    script: &'static str,
}

/// Every point-scoring action the Guideline asks the game to demonstrate, in its table's
/// order (Guideline 8).
pub const SCENES: [Scene; 13] = [
    Scene {
        name: "Single",
        description: "1 line of Blocks is cleared.",
        points: "100 x Level",
        rows: &["#.........", "#####....#"],
        pieces: &[TetriminoType::I],
        script: "R*2 HD",
    },
    Scene {
        name: "Double",
        description: "2 lines of Blocks are cleared at once.",
        points: "300 x Level",
        rows: &["##..###...", "########..", "########.."],
        pieces: &[TetriminoType::O],
        script: "R*4 HD",
    },
    Scene {
        name: "Triple",
        description: "3 lines of Blocks are cleared at once.",
        points: "500 x Level",
        rows: &["######....", "#########.", "#########.", "#########."],
        pieces: &[TetriminoType::I],
        script: "CW R*4 HD",
    },
    Scene {
        name: "Tetris",
        description: "4 lines of Blocks are cleared at once.",
        points: "800 x Level",
        rows: &[
            "....######",
            ".#########",
            ".#########",
            ".#########",
            ".#########",
        ],
        pieces: &[TetriminoType::I],
        script: "CW L*5 HD",
    },
    Scene {
        name: "Mini T-Spin",
        description: "An easier T-Spin with no Line Clear: the T rotates into a spot against \
                      the wall.",
        points: "100 x Level",
        rows: &["...####...", ".########."],
        pieces: &[TetriminoType::T],
        script: "L*3 D*18 CW",
    },
    Scene {
        name: "Mini T-Spin Single",
        description: "An easier T-Spin clearing 1 line of Blocks.",
        points: "200 x Level",
        rows: &["...####...", ".#########"],
        pieces: &[TetriminoType::T],
        script: "L*3 D*18 CW",
    },
    Scene {
        name: "T-Spin",
        description: "The T-Tetrimino is spun into a T-Slot with no Line Clear.",
        points: "400 x Level",
        rows: &["...#......", "###...###.", "####.####."],
        pieces: &[TetriminoType::T],
        script: "D*16 CW D*2 CW",
    },
    Scene {
        name: "T-Spin Single",
        description: "A T-Spin clearing 1 line of Blocks.",
        points: "800 x Level",
        rows: &["...#......", "###...###.", "####.#####"],
        pieces: &[TetriminoType::T],
        script: "D*16 CW D*2 CW",
    },
    Scene {
        name: "T-Spin Double",
        description: "A T-Spin clearing 2 lines of Blocks at once.",
        points: "1200 x Level",
        rows: &["...#......", "###...####", "####.#####"],
        pieces: &[TetriminoType::T],
        script: "D*16 CW D*2 CW",
    },
    Scene {
        name: "T-Spin Triple",
        description: "A T-Spin clearing 3 lines of Blocks at once. The T slides under the \
                      overhang, and its rotation kicks it down into the slot.",
        points: "1600 x Level",
        rows: &[
            "##........",
            "#.........",
            "#.########",
            "#..#######",
            "#.########",
        ],
        pieces: &[TetriminoType::T],
        script: "L D*16 L CW",
    },
    Scene {
        name: "Back-to-Back",
        description: "Tetrises and T-Spin Line Clears in a row, with no Single, Double or \
                      Triple between, earn half as much again.",
        points: "0.5 x Action Total",
        rows: &[
            ".#########",
            ".#########",
            ".#########",
            ".#########",
            ".#########",
            ".#########",
            ".#########",
            ".#########",
        ],
        pieces: &[TetriminoType::I, TetriminoType::I],
        script: "CW L*5 HD NEXT CW L*5 HD",
    },
    Scene {
        name: "Soft Drop",
        description: "The Tetrimino is Soft Dropped for n lines.",
        points: "1 x n",
        rows: &["##..##..##"],
        pieces: &[TetriminoType::O],
        script: "D*20",
    },
    Scene {
        name: "Hard Drop",
        description: "The Tetrimino is Hard Dropped for m lines.",
        points: "2 x m",
        rows: &["##..##..##"],
        pieces: &[TetriminoType::O],
        script: "L*2 HD",
    },
];

impl Scene {
    fn steps(&self) -> Vec<Step> {
        let mut steps = Vec::new();
        for token in self.script.split_whitespace() {
            let (name, count) = match token.split_once('*') {
                Some((name, count)) => (name, count.parse().unwrap_or(1)),
                None => (token, 1),
            };
            let step = match name {
                "L" => Step::Press(InputAction::MoveLeft),
                "R" => Step::Press(InputAction::MoveRight),
                "D" => Step::Press(InputAction::MoveDown),
                "CW" => Step::Press(InputAction::RotateClockwise),
                "CCW" => Step::Press(InputAction::RotateCounterClockwise),
                "HD" => Step::Press(InputAction::HardDrop),
                _ => Step::NextPiece,
            };
            steps.extend(std::iter::repeat_n(step, count));
        }
        steps
    }
}

/// What a finished scene scored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneResult {
    /// Points for the scene's last action, with any Back-to-Back bonus.
    pub action_points: u64,
    /// Points for Soft and Hard Drops in the whole scene.
    pub drop_points: u64,
}

/// The scoring demonstration: each point-scoring action played by a script on the real
/// engine, one scene after another (Guideline 8). Has no terminal, so tests can run it.
pub struct Demo {
    index: usize,
    session: Session,
    steps: Vec<Step>,
    next_step: usize,
    /// Time until the next step may run.
    wait: Duration,
    /// Time not yet run, less than one tick.
    pending: Duration,
    /// Points not from drops when the scene's last Tetrimino came into play.
    points_before_last_piece: Option<u64>,
    /// How long the finished scene's result has been showing.
    showing_result: Option<Duration>,
}

impl Demo {
    /// Starts on the first scene. Sound and the Matrix size come from `base`; everything
    /// else is fixed so that every scene plays out the same way.
    pub fn new(base: &GameConfig) -> Self {
        let mut demo = Self {
            index: 0,
            session: Self::scene_session(base, &SCENES[0]),
            steps: Vec::new(),
            next_step: 0,
            wait: Duration::ZERO,
            pending: Duration::ZERO,
            points_before_last_piece: None,
            showing_result: None,
        };
        demo.start_scene(base, 0);
        demo
    }

    fn scene_session(base: &GameConfig, scene: &Scene) -> Session {
        let config = GameConfig {
            board_width: 10,
            board_height: 20,
            game_mode: GameMode::Marathon,
            starting_level: 1,
            starting_blocks: 0,
            enable_ghost_piece: true,
            enable_hold: false,
            enable_variable_goal: false,
            preview_count: 1,
            ..base.clone()
        };
        let mut session = Session::with_seed(config, 0);
        let state = session.state_mut();
        let top = 20 - scene.rows.len() as i32;
        for (i, row) in scene.rows.iter().enumerate() {
            let y = top + i as i32;
            for (x, cell) in row.chars().enumerate() {
                // Rows take turns among the Tetrimino colors.
                let color = TetriminoType::ALL[y as usize % TetriminoType::ALL.len()];
                state.board.set_cell(x, y, (cell != '.').then_some(color));
            }
        }
        state.next_pieces = scene.pieces.to_vec();
        session
    }

    fn start_scene(&mut self, base: &GameConfig, index: usize) {
        let scene = &SCENES[index];
        self.index = index;
        self.session = Self::scene_session(base, scene);
        self.steps = scene.steps();
        self.next_step = 0;
        self.wait = FIRST_KEY_DELAY;
        self.pending = Duration::ZERO;
        self.points_before_last_piece = None;
        self.showing_result = None;
    }

    pub fn scene(&self) -> &'static Scene {
        &SCENES[self.index]
    }

    /// The scene's place, from 0.
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn state(&self) -> &GameState {
        self.session.state()
    }

    /// What the scene scored, once it has finished.
    pub fn result(&self) -> Option<SceneResult> {
        self.showing_result?;
        let scoring = &self.state().scoring;
        let action_total = scoring.score - scoring.drop_points;
        Some(SceneResult {
            action_points: action_total - self.points_before_last_piece.unwrap_or(0),
            drop_points: scoring.drop_points,
        })
    }

    /// Returns and clears the sounds for events since the last call.
    pub fn take_sound_effects(&mut self) -> Vec<SoundEffect> {
        self.session.take_sound_effects()
    }

    /// Left and Right go to the previous and next scene. Returns false when the player wants
    /// to leave the demo.
    pub fn handle_input(&mut self, base: &GameConfig, input: MenuInput) -> bool {
        let count = SCENES.len();
        match input {
            MenuInput::Left => self.start_scene(base, (self.index + count - 1) % count),
            MenuInput::Right => self.start_scene(base, (self.index + 1) % count),
            MenuInput::Back | MenuInput::Select | MenuInput::Quit => return false,
            MenuInput::Up | MenuInput::Down | MenuInput::Remove => {}
        }
        true
    }

    /// Advances the demo, going on to the next scene after each result.
    pub fn update(&mut self, base: &GameConfig, dt: Duration) {
        self.pending += dt;
        while self.pending >= TICK {
            self.pending -= TICK;
            self.tick(base);
        }
    }

    fn tick(&mut self, base: &GameConfig) {
        if let Some(shown) = &mut self.showing_result {
            *shown += TICK;
            if *shown >= RESULT_TIME {
                self.start_scene(base, (self.index + 1) % SCENES.len());
                return;
            }
        } else {
            self.run_due_steps();
            self.check_finished();
        }
        self.session.update(TICK);
    }

    fn run_due_steps(&mut self) {
        self.wait = self.wait.saturating_sub(TICK);
        while self.wait.is_zero() {
            let Some(&step) = self.steps.get(self.next_step) else {
                return;
            };
            match step {
                Step::NextPiece => {
                    if self.state().current_piece().is_none() {
                        return;
                    }
                    self.wait = KEY_DELAY;
                }
                Step::Press(action) => {
                    self.session.handle_input(InputEvent::Pressed(action));
                    self.wait = match action {
                        InputAction::MoveDown => DOWN_DELAY,
                        _ => KEY_DELAY,
                    };
                }
            }
            self.next_step += 1;
        }
    }

    /// Notes when the last Tetrimino comes into play, and ends the scene once it has Locked
    /// Down and its lines are gone.
    fn check_finished(&mut self) {
        let pieces = self.scene().pieces.len() as u32;
        let state = self.session.state();
        let locked = state.scoring.statistics.tetriminos;
        let scoring = &state.scoring;
        if locked + 1 == pieces && state.current_piece().is_some() {
            self.points_before_last_piece
                .get_or_insert(scoring.score - scoring.drop_points);
        }
        if locked == pieces && state.line_clear_animation().is_none() {
            // Hold back the next Tetrimino, so the result stays on screen as it was.
            self.session.state_mut().phase = Phase::Generation {
                remaining: Duration::from_secs(3600),
            };
            self.showing_result = Some(Duration::ZERO);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::scoring_table;

    fn config() -> GameConfig {
        GameConfig {
            enable_sound: false,
            ..GameConfig::default()
        }
    }

    /// Runs the demo until the current scene shows its result.
    fn finish_scene(demo: &mut Demo) -> SceneResult {
        for _ in 0..2000 {
            if let Some(result) = demo.result() {
                return result;
            }
            demo.update(&config(), Duration::from_millis(16));
        }
        panic!("{} did not finish", demo.scene().name);
    }

    fn level_points(points: &str) -> u64 {
        points.split(' ').next().unwrap().parse().unwrap()
    }

    #[test]
    fn test_every_scene_scores_its_action_as_the_scoring_table_says() {
        let mut demo = Demo::new(&config());
        let table = scoring_table();
        for scene in &SCENES[..10] {
            assert_eq!(demo.scene().name, scene.name);
            let result = finish_scene(&mut demo);
            let expected = table
                .iter()
                .find(|(name, _)| *name == scene.name)
                .unwrap()
                .1;
            assert_eq!(result.action_points, expected, "{}", scene.name);
            assert_eq!(level_points(scene.points), expected, "{}", scene.name);
            // Only Tetrises and T-Spins are announced (Guideline 13.1.5).
            let announced = !matches!(scene.name, "Single" | "Double" | "Triple");
            assert_eq!(
                demo.state().notification_text(),
                announced.then_some(scene.name),
                "{}",
                scene.name
            );
            demo.update(&config(), RESULT_TIME);
        }
    }

    #[test]
    fn test_back_to_back_scene_earns_half_again_for_the_second_tetris() {
        let mut demo = Demo::new(&config());
        demo.start_scene(&config(), 10);
        let result = finish_scene(&mut demo);
        assert_eq!(result.action_points, 1200);
        assert_eq!(demo.state().scoring.lines_cleared, 8);
        assert_eq!(
            demo.state().notification_text(),
            Some("Back-to-Back Tetris")
        );
    }

    #[test]
    fn test_drop_scenes_score_only_their_drops() {
        let mut demo = Demo::new(&config());
        demo.start_scene(&config(), 11);
        let soft = finish_scene(&mut demo);
        // Gravity takes one of the 18 rows while the scene starts.
        assert_eq!(
            soft,
            SceneResult {
                action_points: 0,
                drop_points: 17
            }
        );

        demo.start_scene(&config(), 12);
        let hard = finish_scene(&mut demo);
        assert_eq!(
            hard,
            SceneResult {
                action_points: 0,
                drop_points: 2 * 18
            }
        );
    }

    #[test]
    fn test_a_finished_scene_holds_still_then_the_next_one_starts() {
        let mut demo = Demo::new(&config());
        finish_scene(&mut demo);
        demo.update(&config(), RESULT_TIME / 2);
        assert!(demo.state().current_piece().is_none());
        assert_eq!(demo.index(), 0);
        demo.update(&config(), RESULT_TIME / 2);
        assert_eq!(demo.index(), 1);
        assert!(demo.result().is_none());
    }

    #[test]
    fn test_left_and_right_change_scenes_round_in_a_circle_and_back_leaves() {
        let mut demo = Demo::new(&config());
        assert!(demo.handle_input(&config(), MenuInput::Left));
        assert_eq!(demo.index(), SCENES.len() - 1);
        assert!(demo.handle_input(&config(), MenuInput::Right));
        assert_eq!(demo.index(), 0);
        for input in [MenuInput::Back, MenuInput::Select, MenuInput::Quit] {
            assert!(!demo.handle_input(&config(), input));
        }
    }

    #[test]
    fn test_scene_rows_fit_the_matrix() {
        for scene in &SCENES {
            assert!(
                scene.rows.iter().all(|row| row.len() == 10),
                "{}",
                scene.name
            );
            assert!(!scene.steps().is_empty(), "{}", scene.name);
        }
    }
}
