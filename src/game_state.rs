use crate::board::Board;
use crate::config::{GameConfig, GameMode, LockDownMode};
use crate::scoring::{Scoring, TSpin};
use crate::sound::SoundEffect;
use crate::tetrimino::{Tetrimino, TetriminoType, rotation_points};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};
use std::time::Duration;

/// Length of an Ultra. Guideline 12.3 allows two or three minutes.
pub const ULTRA_TIME: Duration = Duration::from_secs(180);
/// Time a landed Tetrimino has before it Locks Down (Guideline 5.7).
const LOCK_DELAY: Duration = Duration::from_millis(500);
/// Moves or rotations allowed per lowest row in Extended Placement (Guideline 5.7).
const MAX_LOCK_MOVES: u32 = 15;
/// Delay between Lock Down and the Next Tetrimino (Guideline A1.2.1).
const GENERATION_DELAY: Duration = Duration::from_millis(200);
/// Soft Drop speed relative to normal Fall Speed (Guideline 5.5).
const SOFT_DROP_FACTOR: u32 = 20;
/// How long an action notification stays on screen.
const NOTIFICATION_TIME: Duration = Duration::from_secs(2);
const LINE_CLEAR_BASE: Duration = Duration::from_millis(100);
const LINE_CLEAR_PER_LINE: Duration = Duration::from_millis(150);
const LINE_CLEAR_BLINK: Duration = Duration::from_millis(75);

/// Normal Fall Speed in seconds per line for a level (Guideline 7).
pub fn fall_interval(level: u32) -> Duration {
    let steps = level.max(1) - 1;
    Duration::from_secs_f64((0.8 - steps as f64 * 0.007).powi(steps as i32))
}

/// Lock Down bookkeeping for the Tetrimino in play.
struct LockState {
    timer: Duration,
    moves_left: u32,
    lowest_y: i32,
    has_landed: bool,
}

impl LockState {
    fn new(y: i32) -> Self {
        Self {
            timer: LOCK_DELAY,
            moves_left: MAX_LOCK_MOVES,
            lowest_y: y,
            has_landed: false,
        }
    }
}

/// Why a game ended (Guideline 10 and 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOutcome {
    /// A new Tetrimino's generation position was blocked.
    BlockOut,
    /// A whole Tetrimino Locked Down above the Skyline.
    LockOut,
    /// The player quit.
    Quit,
    /// The player finished the final Marathon level.
    MarathonComplete,
    /// The player cleared all the lines of a Sprint.
    SprintComplete,
    /// An Ultra's time ran out.
    UltraComplete,
}

/// The stage a game is in. Each Tetrimino goes through these in turn (Guideline A1.1).
pub enum Phase {
    /// Waiting to generate the Next Tetrimino.
    Generation { remaining: Duration },
    /// A Tetrimino is in play, falling or waiting to Lock Down.
    Falling(Tetrimino),
    /// Full lines are animating and have not been removed yet.
    LineClear(LineClearAnimation),
    /// The game has ended. `piece` is a Tetrimino still on screen, if there is one.
    GameOver {
        outcome: GameOutcome,
        piece: Option<Tetrimino>,
    },
}

pub struct GameState {
    pub(crate) board: Board,
    pub(crate) held_piece: Option<TetriminoType>,
    pub(crate) next_pieces: Vec<TetriminoType>,
    pub(crate) scoring: Scoring,
    pub(crate) config: GameConfig,
    pub(crate) phase: Phase,
    /// Time played, excluding pauses.
    pub(crate) elapsed: Duration,
    bag: Vec<TetriminoType>,
    bag_rng: StdRng,
    hold_used: bool,
    soft_dropping: bool,
    fall_progress: Duration,
    lock: LockState,
    last_move_was_rotation: bool,
    used_rotation_point_5: bool,
    pub(crate) notification: Option<(String, Duration)>,
    /// Sounds for events since the last `take_sound_effects`.
    pub(crate) sound_effects: Vec<SoundEffect>,
}

pub struct LineClearAnimation {
    pub cleared_rows: Vec<i32>,
    pub elapsed: Duration,
    pub total_lines: u32,
    /// T-Spin recognized at Lock Down, scored once the lines are removed.
    tspin: TSpin,
}

impl LineClearAnimation {
    fn duration(&self) -> Duration {
        LINE_CLEAR_BASE + LINE_CLEAR_PER_LINE * self.total_lines
    }
}

impl GameState {
    /// Starts a game with an unpredictable order of Tetriminos.
    pub fn new(config: GameConfig) -> Self {
        Self::with_rng(config, rand::make_rng())
    }

    /// Starts a game whose order of Tetriminos is fixed by the seed, so it can be repeated.
    pub fn with_seed(config: GameConfig, seed: u64) -> Self {
        Self::with_rng(config, StdRng::seed_from_u64(seed))
    }

    fn with_rng(config: GameConfig, bag_rng: StdRng) -> Self {
        let mut game_state = Self {
            board: Board::new(config.board_width, config.board_height),
            held_piece: None,
            next_pieces: Vec::new(),
            scoring: Scoring::new(&config),
            config,
            phase: Phase::Generation {
                remaining: Duration::ZERO,
            },
            elapsed: Duration::ZERO,
            bag: Vec::new(),
            bag_rng,
            hold_used: false,
            soft_dropping: false,
            fall_progress: Duration::ZERO,
            lock: LockState::new(0),
            last_move_was_rotation: false,
            used_rotation_point_5: false,
            notification: None,
            sound_effects: Vec::new(),
        };

        game_state.refill_bag();
        game_state.populate_next_pieces();
        game_state.add_starting_blocks();
        game_state
    }

    /// Fills the bottom rows of the Matrix with Starting Blocks (Guideline 2.4). Each cell
    /// is a Mino of a random Tetrimino's color half the time, and each row has at least one
    /// gap and at least one Mino. Uses the same seeded randomness as the bag, after the first
    /// bag is drawn, so a game with no Starting Blocks keeps the same order of Tetriminos.
    fn add_starting_blocks(&mut self) {
        const COLORS: [TetriminoType; 7] = [
            TetriminoType::I,
            TetriminoType::O,
            TetriminoType::T,
            TetriminoType::S,
            TetriminoType::Z,
            TetriminoType::J,
            TetriminoType::L,
        ];
        let width = self.board.get_width();
        let bottom = self.board.get_height() as i32 - 1;
        for row in 0..self.config.starting_blocks as i32 {
            let mut cells: Vec<Option<TetriminoType>> = (0..width)
                .map(|_| {
                    let color = COLORS[self.bag_rng.random_range(0..COLORS.len())];
                    self.bag_rng.random_bool(0.5).then_some(color)
                })
                .collect();
            let gap = self.bag_rng.random_range(0..width);
            cells[gap] = None;
            if cells.iter().all(Option::is_none) {
                cells[(gap + 1) % width] = Some(COLORS[gap % COLORS.len()]);
            }
            for (x, cell) in cells.into_iter().enumerate() {
                self.board.set_cell(x, bottom - row, cell);
            }
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn held_piece(&self) -> Option<TetriminoType> {
        self.held_piece
    }

    pub fn next_pieces(&self) -> &[TetriminoType] {
        &self.next_pieces
    }

    pub fn scoring(&self) -> &Scoring {
        &self.scoring
    }

    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    /// Time played, excluding pauses.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns and clears the sounds for events since the last call.
    pub fn take_sound_effects(&mut self) -> Vec<SoundEffect> {
        std::mem::take(&mut self.sound_effects)
    }

    /// Advances the current phase and all timers.
    pub fn tick(&mut self, dt: Duration) {
        if self.is_game_over() {
            return;
        }
        self.elapsed += dt;
        if self.config.game_mode == GameMode::Ultra && self.elapsed >= ULTRA_TIME {
            self.end_ultra();
            return;
        }
        self.update_notification(dt);

        match &mut self.phase {
            Phase::Generation { remaining } => {
                if *remaining <= dt {
                    self.spawn_piece();
                } else {
                    *remaining -= dt;
                }
            }
            Phase::Falling(_) => {
                self.apply_gravity(dt);
                self.update_lock_down(dt);
            }
            Phase::LineClear(animation) => {
                animation.elapsed += dt;
                if animation.elapsed >= animation.duration() {
                    self.complete_line_clear();
                }
            }
            Phase::GameOver { .. } => {}
        }
    }

    /// Ends an Ultra when its time runs out (Guideline 12.3). Lines already clearing still
    /// score, and the Tetrimino in play stays on screen.
    fn end_ultra(&mut self) {
        self.elapsed = ULTRA_TIME;
        if matches!(self.phase, Phase::LineClear(_)) {
            self.complete_line_clear();
        }
        self.sound_effects.push(SoundEffect::GameComplete);
        self.phase = Phase::GameOver {
            outcome: GameOutcome::UltraComplete,
            piece: self.falling_piece(),
        };
    }

    /// The Tetrimino in play, if the game is in the Falling phase.
    fn falling_piece(&self) -> Option<Tetrimino> {
        match self.phase {
            Phase::Falling(piece) => Some(piece),
            _ => None,
        }
    }

    /// The Tetrimino to draw: the one in play, or the one left on screen when the game ended.
    pub fn current_piece(&self) -> Option<Tetrimino> {
        match self.phase {
            Phase::Falling(piece) => Some(piece),
            Phase::GameOver { piece, .. } => piece,
            _ => None,
        }
    }

    pub fn is_game_over(&self) -> bool {
        matches!(self.phase, Phase::GameOver { .. })
    }

    /// Why the game ended, if it has.
    pub fn outcome(&self) -> Option<GameOutcome> {
        match self.phase {
            Phase::GameOver { outcome, .. } => Some(outcome),
            _ => None,
        }
    }

    /// The running Line Clear animation, if there is one.
    pub fn line_clear_animation(&self) -> Option<&LineClearAnimation> {
        match &self.phase {
            Phase::LineClear(animation) => Some(animation),
            _ => None,
        }
    }

    /// Ends the game because the player quit. A game that has already ended keeps its outcome.
    pub fn quit(&mut self) {
        if !self.is_game_over() {
            self.phase = Phase::GameOver {
                outcome: GameOutcome::Quit,
                piece: self.current_piece(),
            };
        }
    }

    fn update_notification(&mut self, dt: Duration) {
        if let Some((_, remaining)) = &mut self.notification {
            *remaining = remaining.saturating_sub(dt);
            if remaining.is_zero() {
                self.notification = None;
            }
        }
    }

    fn apply_gravity(&mut self, dt: Duration) {
        if !self.can_fall() {
            self.fall_progress = Duration::ZERO;
            return;
        }

        let mut interval = fall_interval(self.scoring.level);
        if self.soft_dropping {
            interval /= SOFT_DROP_FACTOR;
        }

        self.fall_progress += dt;
        while self.fall_progress >= interval {
            self.fall_progress -= interval;
            let points = u64::from(self.soft_dropping);
            if !self.drop_one_row(points) {
                self.fall_progress = Duration::ZERO;
                break;
            }
        }
    }

    fn update_lock_down(&mut self, dt: Duration) {
        if self.falling_piece().is_none() || self.can_fall() {
            return;
        }
        if !self.lock.has_landed {
            self.lock.has_landed = true;
            self.sound_effects.push(SoundEffect::Land);
        }

        let out_of_moves =
            self.config.lock_down_mode == LockDownMode::Extended && self.lock.moves_left == 0;
        if out_of_moves || self.lock.timer <= dt {
            self.lock_piece(false);
        } else {
            self.lock.timer -= dt;
        }
    }

    fn can_fall(&self) -> bool {
        self.falling_piece().is_some_and(|piece| {
            let below = Tetrimino {
                y: piece.y + 1,
                ..piece
            };
            self.board.is_valid_position(&below)
        })
    }

    /// Moves down one row if there is room, awarding the given Soft Drop points for it.
    fn drop_one_row(&mut self, soft_drop_points: u64) -> bool {
        if !self.move_piece(0, 1) {
            return false;
        }
        self.scoring.add_soft_drop(soft_drop_points);
        self.last_move_was_rotation = false;
        self.note_new_lowest_row();
        true
    }

    /// Falling below the lowest row reached restores the timer and move count.
    fn note_new_lowest_row(&mut self) {
        if let Some(piece) = self.falling_piece()
            && piece.y > self.lock.lowest_y
        {
            self.lock = LockState::new(piece.y);
        }
    }

    /// Applies the Lock Down rules after a successful move or rotation.
    fn note_manipulation(&mut self) {
        self.note_new_lowest_row();
        match self.config.lock_down_mode {
            LockDownMode::Extended => {
                if self.lock.has_landed && self.lock.moves_left > 0 {
                    self.lock.moves_left -= 1;
                    self.lock.timer = LOCK_DELAY;
                }
            }
            LockDownMode::Infinite => self.lock.timer = LOCK_DELAY,
            LockDownMode::Classic => {}
        }
    }

    /// Generates the Next Tetrimino from the Next Queue.
    fn spawn_piece(&mut self) {
        if self.next_pieces.is_empty() {
            return;
        }
        let kind = self.next_pieces.remove(0);
        self.populate_next_pieces();
        self.place_piece(kind);
    }

    /// Puts a Tetrimino at its generation position, then drops it one row if there is room.
    /// A blocked generation position is a Block Out (Guideline 10).
    fn place_piece(&mut self, kind: TetriminoType) {
        let piece = Tetrimino::at_generation(kind, self.board.get_width());
        self.lock = LockState::new(piece.y);
        self.fall_progress = Duration::ZERO;
        self.last_move_was_rotation = false;
        self.used_rotation_point_5 = false;

        if !self.board.is_valid_position(&piece) {
            self.phase = Phase::GameOver {
                outcome: GameOutcome::BlockOut,
                piece: Some(piece),
            };
            self.sound_effects.push(SoundEffect::GameOver);
            return;
        }
        self.phase = Phase::Falling(piece);
        self.drop_one_row(0);
    }

    /// Moves the Tetrimino in play if the target position is free. Applies no game rules.
    fn move_piece(&mut self, dx: i32, dy: i32) -> bool {
        let Phase::Falling(piece) = &mut self.phase else {
            return false;
        };
        let moved = Tetrimino {
            x: piece.x + dx,
            y: piece.y + dy,
            ..*piece
        };
        if !self.board.is_valid_position(&moved) {
            return false;
        }
        *piece = moved;
        true
    }

    /// Player move left or right (Guideline 5.1).
    pub fn shift_piece(&mut self, dx: i32) -> bool {
        let moved = self.move_piece(dx, 0);
        if moved {
            self.last_move_was_rotation = false;
            self.note_manipulation();
            self.sound_effects.push(SoundEffect::Move);
        }
        moved
    }

    /// Turns continuous Soft Drop on or off (Guideline 5.5).
    pub fn set_soft_drop(&mut self, active: bool) {
        if active == self.soft_dropping {
            return;
        }
        self.soft_dropping = active;
        if active && self.falling_piece().is_some() {
            self.sound_effects.push(SoundEffect::SoftDrop);
        }
        // Keep the same fraction of a row, so changing speed cannot release a burst of rows.
        if active {
            self.fall_progress /= SOFT_DROP_FACTOR;
        } else {
            self.fall_progress *= SOFT_DROP_FACTOR;
        }
    }

    /// Drops one row and awards one Soft Drop point. Used on terminals that cannot report
    /// key releases, where each key event stands for one step of Soft Drop.
    pub fn soft_drop_step(&mut self) -> bool {
        let moved = self.drop_one_row(1);
        if moved {
            self.sound_effects.push(SoundEffect::SoftDrop);
        }
        moved
    }

    /// Rotates using the Super Rotation System (Guideline A1.4). The first rotation point
    /// that fits is used. Returns false if none fit.
    pub fn rotate_piece(&mut self, clockwise: bool) -> bool {
        let Some(piece) = self.falling_piece() else {
            return false;
        };
        // All O-Tetrimino facings occupy the same cells, so rotating changes nothing. It still
        // sounds like a rotation, as the player asked for one and nothing blocked it.
        if piece.kind == TetriminoType::O {
            self.sound_effects.push(SoundEffect::Rotate);
            return false;
        }

        let to = if clockwise {
            piece.facing.clockwise()
        } else {
            piece.facing.counterclockwise()
        };

        for (point, (dx, dy)) in rotation_points(piece.kind, piece.facing, clockwise) {
            let candidate = Tetrimino {
                x: piece.x + dx,
                y: piece.y + dy,
                facing: to,
                ..piece
            };
            if self.board.is_valid_position(&candidate) {
                self.phase = Phase::Falling(candidate);
                self.last_move_was_rotation = true;
                self.used_rotation_point_5 |= point == 5;
                self.note_manipulation();
                self.sound_effects.push(SoundEffect::Rotate);
                return true;
            }
        }
        self.sound_effects.push(SoundEffect::RotateBlocked);
        false
    }

    /// Drops to the Surface below and Locks Down at once, for 2 points per row (Guideline 8).
    pub fn hard_drop(&mut self) {
        if self.falling_piece().is_none() {
            return;
        }
        let mut rows = 0;
        while self.move_piece(0, 1) {
            rows += 1;
            self.last_move_was_rotation = false;
        }
        self.scoring.add_hard_drop(rows);
        self.sound_effects.push(SoundEffect::HardDrop);
        self.lock_piece(true);
    }

    /// Locks the Tetrimino in play and starts the Pattern and Animate phases.
    #[cfg(test)]
    pub(crate) fn lock_current_piece(&mut self) {
        self.lock_piece(false);
    }

    /// Locks the Tetrimino in play. A Hard Drop already has its own sound, so it plays no
    /// Lock Down sound, and a Line Clear or T-Spin sound replaces the Lock Down sound.
    fn lock_piece(&mut self, hard_dropped: bool) {
        let Some(piece) = self.falling_piece() else {
            return;
        };
        let tspin = self.detect_tspin(&piece);
        self.board.lock_tetrimino(&piece);
        self.scoring.statistics.tetriminos += 1;
        self.hold_used = false;

        // A whole Tetrimino above the Skyline is a Lock Out (Guideline 10).
        if piece.get_blocks().iter().all(|(_, dy)| piece.y + dy < 0) {
            self.phase = Phase::GameOver {
                outcome: GameOutcome::LockOut,
                piece: None,
            };
            self.sound_effects.push(SoundEffect::GameOver);
            return;
        }

        let cleared_rows = self.board.get_full_lines();
        let effect = match tspin {
            TSpin::Full => Some(SoundEffect::TSpin),
            TSpin::Mini => Some(SoundEffect::MiniTSpin),
            TSpin::None if !cleared_rows.is_empty() => {
                Some(SoundEffect::LineClear(cleared_rows.len() as u32))
            }
            TSpin::None if hard_dropped => None,
            TSpin::None => Some(SoundEffect::LockDown),
        };
        self.sound_effects.extend(effect);
        if cleared_rows.is_empty() {
            self.finish_action(tspin, 0);
        } else {
            self.phase = Phase::LineClear(LineClearAnimation {
                total_lines: cleared_rows.len() as u32,
                cleared_rows,
                elapsed: Duration::ZERO,
                tspin,
            });
        }
    }

    /// Eliminate phase: removes the lines, then scores them.
    pub(crate) fn complete_line_clear(&mut self) {
        let Phase::LineClear(animation) = &self.phase else {
            return;
        };
        let tspin = animation.tspin;
        let lines = self.board.clear_lines();
        self.finish_action(tspin, lines);
    }

    /// Completion phase: scores the action, then moves on to the next phase.
    fn finish_action(&mut self, tspin: TSpin, lines: u32) {
        let result = self.scoring.award_action(tspin, lines);
        if let Some(text) = result.notification {
            self.notification = Some((text, NOTIFICATION_TIME));
        }
        if result.back_to_back {
            self.sound_effects.push(SoundEffect::BackToBack);
        }
        if result.game_complete {
            self.sound_effects.push(SoundEffect::GameComplete);
        } else if result.level_up {
            self.sound_effects.push(SoundEffect::LevelUp);
        }
        self.phase = if result.game_complete {
            Phase::GameOver {
                outcome: match self.config.game_mode {
                    GameMode::Marathon => GameOutcome::MarathonComplete,
                    GameMode::Sprint => GameOutcome::SprintComplete,
                    GameMode::Ultra => GameOutcome::UltraComplete,
                },
                piece: None,
            }
        } else {
            Phase::Generation {
                remaining: GENERATION_DELAY,
            }
        };
    }

    /// Recognizes a T-Spin from the four cells diagonal to the T's center (Guideline 9.1).
    /// Walls and floor count as occupied. A and B are on the side the T points to.
    fn detect_tspin(&self, piece: &Tetrimino) -> TSpin {
        if piece.kind != TetriminoType::T || !self.last_move_was_rotation {
            return TSpin::None;
        }

        const CORNERS_CLOCKWISE: [(i32, i32); 4] = [(-1, -1), (1, -1), (1, 1), (-1, 1)];
        let facing = piece.facing as usize;
        let occupied = |offset: usize| {
            let (dx, dy) = CORNERS_CLOCKWISE[(facing + offset) % 4];
            self.board.is_occupied(piece.x + 1 + dx, piece.y + 1 + dy)
        };
        let (a, b, d, c) = (occupied(0), occupied(1), occupied(2), occupied(3));

        if a && b && (c || d) {
            TSpin::Full
        } else if c && d && (a || b) {
            if self.used_rotation_point_5 {
                TSpin::Full
            } else {
                TSpin::Mini
            }
        } else {
            TSpin::None
        }
    }

    fn refill_bag(&mut self) {
        let mut new_bag = vec![
            TetriminoType::I,
            TetriminoType::O,
            TetriminoType::T,
            TetriminoType::S,
            TetriminoType::Z,
            TetriminoType::J,
            TetriminoType::L,
        ];
        new_bag.shuffle(&mut self.bag_rng);
        self.bag = new_bag;
    }

    fn populate_next_pieces(&mut self) {
        let target_count = self.config.preview_count.clamp(1, 6);

        while self.next_pieces.len() < target_count {
            if self.bag.is_empty() {
                self.refill_bag();
            }
            if let Some(piece) = self.bag.pop() {
                self.next_pieces.push(piece);
            }
        }
    }

    /// Moves the Tetrimino in play to the Hold Queue. Allowed once per Lock Down (Guideline 5.6).
    pub fn hold_piece(&mut self) {
        if !self.config.enable_hold || self.hold_used {
            return;
        }
        let Some(current) = self.falling_piece() else {
            return;
        };

        self.hold_used = true;
        self.sound_effects.push(SoundEffect::Hold);
        match self.held_piece.replace(current.kind) {
            Some(held) => self.place_piece(held),
            None => self.spawn_piece(),
        }
    }

    pub fn should_show_cleared_rows(&self) -> bool {
        self.line_clear_animation().is_some_and(|animation| {
            (animation.elapsed.as_millis() / LINE_CLEAR_BLINK.as_millis()).is_multiple_of(2)
        })
    }

    /// Text of the current action notification, if one is showing (Guideline 13.1.5).
    pub fn notification_text(&self) -> Option<&str> {
        self.notification.as_ref().map(|(text, _)| text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tetrimino::Facing;
    use std::collections::HashSet;

    const ALL_TYPES: [TetriminoType; 7] = [
        TetriminoType::I,
        TetriminoType::O,
        TetriminoType::T,
        TetriminoType::S,
        TetriminoType::Z,
        TetriminoType::J,
        TetriminoType::L,
    ];

    fn config() -> GameConfig {
        GameConfig {
            enable_sound: false,
            ..GameConfig::default()
        }
    }

    fn new_state() -> GameState {
        GameState::new(config())
    }

    fn state_with_mode(mode: LockDownMode) -> GameState {
        GameState::new(GameConfig {
            lock_down_mode: mode,
            ..config()
        })
    }

    fn piece(kind: TetriminoType, x: i32, y: i32, facing: Facing) -> Tetrimino {
        Tetrimino { kind, x, y, facing }
    }

    fn fill(state: &mut GameState, cells: &[(usize, i32)]) {
        for &(x, y) in cells {
            state.board.set_cell(x, y, Some(TetriminoType::I));
        }
    }

    fn fill_row_except(state: &mut GameState, y: i32, gaps: &[usize]) {
        for x in 0..10 {
            if !gaps.contains(&x) {
                state.board.set_cell(x, y, Some(TetriminoType::I));
            }
        }
    }

    /// Locks the Tetrimino in play and finishes its Line Clear without waiting for the animation.
    fn lock_and_clear(state: &mut GameState) {
        state.lock_current_piece();
        state.complete_line_clear();
    }

    /// Performs a Tetris with an I-Tetrimino dropped into column 0.
    fn do_tetris(state: &mut GameState) {
        for y in 16..20 {
            fill_row_except(state, y, &[0]);
        }
        state.phase = Phase::Falling(piece(TetriminoType::I, -2, 16, Facing::East));
        lock_and_clear(state);
    }

    /// Sets up a T-Spin Double slot and rotates a T into it (Point 1, East to South).
    fn do_tspin_double(state: &mut GameState) {
        fill_row_except(state, 19, &[4]);
        fill_row_except(state, 18, &[3, 4, 5]);
        fill(state, &[(3, 17)]);
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 17, Facing::East));
        assert!(state.rotate_piece(true));
        lock_and_clear(state);
    }

    // --- Generation ---

    #[test]
    fn test_generation_cells_match_guideline() {
        // Guideline 3.4, as 0-based x; row 21 is y = -1 and row 22 is y = -2.
        let expected: [(TetriminoType, &[(i32, i32)]); 3] = [
            (TetriminoType::I, &[(3, -1), (4, -1), (5, -1), (6, -1)]),
            (TetriminoType::O, &[(4, -2), (5, -2), (4, -1), (5, -1)]),
            (TetriminoType::T, &[(4, -2), (3, -1), (4, -1), (5, -1)]),
        ];
        for (kind, cells) in expected {
            let p = Tetrimino::at_generation(kind, 10);
            let actual: Vec<_> = p
                .get_blocks()
                .iter()
                .map(|(dx, dy)| (p.x + dx, p.y + dy))
                .collect();
            assert_eq!(actual, cells, "{kind:?}");
        }
    }

    #[test]
    fn test_spawn_drops_one_row_when_clear() {
        let mut state = new_state();
        state.next_pieces[0] = TetriminoType::T;
        state.spawn_piece();
        let p = state.current_piece().unwrap();
        assert_eq!((p.x, p.y, p.facing), (3, -1, Facing::North));
        assert!(!state.is_game_over());
    }

    #[test]
    fn test_spawn_stays_put_when_row_below_blocked() {
        let mut state = new_state();
        state.next_pieces[0] = TetriminoType::T;
        fill(&mut state, &[(4, 0)]);
        state.spawn_piece();
        assert_eq!(state.current_piece().unwrap().y, -2);
        assert!(!state.is_game_over());
    }

    #[test]
    fn test_block_out_ends_game() {
        let mut state = new_state();
        state.next_pieces[0] = TetriminoType::T;
        fill(&mut state, &[(4, -1)]);
        state.spawn_piece();
        assert!(state.is_game_over());
    }

    #[test]
    fn test_lock_out_ends_game() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, -3, Facing::North));
        state.lock_current_piece();
        assert!(state.is_game_over());
    }

    #[test]
    fn test_peeking_lock_is_not_game_over() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, -1, Facing::North));
        state.lock_current_piece();
        assert!(!state.is_game_over());
    }

    #[test]
    fn test_next_piece_arrives_after_generation_delay() {
        let mut state = new_state();
        state.spawn_piece();
        state.hard_drop();
        assert!(state.current_piece().is_none());

        state.tick(GENERATION_DELAY - Duration::from_millis(1));
        assert!(state.current_piece().is_none());

        state.tick(Duration::from_millis(1));
        assert!(state.current_piece().is_some());
    }

    #[test]
    fn test_bag_gives_each_type_once_per_seven() {
        let mut state = new_state();
        for _ in 0..4 {
            let mut seen = HashSet::new();
            state.next_pieces.clear();
            state.bag.clear();
            for _ in 0..7 {
                state.populate_next_pieces();
                let kind = state.next_pieces.remove(0);
                seen.insert(format!("{kind:?}"));
            }
            assert_eq!(seen.len(), 7);
        }
    }

    #[test]
    fn test_same_seed_gives_the_same_tetriminos() {
        let sequence = |seed| {
            let mut state = GameState::with_seed(config(), seed);
            (0..50)
                .map(|_| {
                    state.spawn_piece();
                    state.current_piece().unwrap().kind
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(sequence(7), sequence(7));
        assert_ne!(sequence(7), sequence(8));
    }

    #[test]
    fn test_next_queue_stays_full() {
        let mut state = new_state();
        for _ in 0..20 {
            state.spawn_piece();
            assert_eq!(state.next_pieces.len(), 3);
        }
    }

    // --- Rotation ---

    #[test]
    fn test_ccw_then_cw_does_not_overflow() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 5, Facing::North));
        assert!(state.rotate_piece(false));
        assert_eq!(state.current_piece().unwrap().facing, Facing::West);
        assert!(state.rotate_piece(true));
        assert_eq!(state.current_piece().unwrap().facing, Facing::North);
    }

    #[test]
    fn test_four_rotations_return_to_start_in_open_space() {
        for kind in ALL_TYPES {
            for clockwise in [true, false] {
                let mut state = new_state();
                let start = piece(kind, 3, 5, Facing::North);
                state.phase = Phase::Falling(start);
                for _ in 0..4 {
                    state.rotate_piece(clockwise);
                }
                assert_eq!(state.current_piece().unwrap(), start, "{kind:?}");
            }
        }
    }

    #[test]
    fn test_rotate_kicks_off_left_wall() {
        // East-facing T against the left wall: South would hit the wall, so Point 2 shifts right.
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, -1, 5, Facing::East));
        assert!(state.rotate_piece(true));
        let p = state.current_piece().unwrap();
        assert_eq!((p.x, p.y, p.facing), (0, 5, Facing::South));
    }

    #[test]
    fn test_rotate_kicks_off_floor_with_i() {
        // I North on the floor, rotating to East: only Point 5 (right 1, up 2) can fit.
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::I, 3, 18, Facing::North));
        assert!(state.rotate_piece(true));
        let p = state.current_piece().unwrap();
        assert_eq!((p.x, p.y, p.facing), (4, 16, Facing::East));
    }

    #[test]
    fn test_rotate_uses_point_5_and_records_it() {
        // Points 1 to 3 are blocked, and T North has no Point 4.
        let mut state = new_state();
        fill(&mut state, &[(4, 12), (3, 10)]);
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 10, Facing::North));
        assert!(state.rotate_piece(true));
        let p = state.current_piece().unwrap();
        assert_eq!((p.x, p.y, p.facing), (2, 12, Facing::East));
        assert!(state.used_rotation_point_5);
    }

    #[test]
    fn test_rotate_fails_when_no_point_fits() {
        let mut state = new_state();
        for y in 0..20 {
            for x in 0..10 {
                let in_piece = (y == 10 && (3..=5).contains(&x)) || (y == 9 && x == 4);
                if !in_piece {
                    fill(&mut state, &[(x, y)]);
                }
            }
        }
        let start = piece(TetriminoType::T, 3, 9, Facing::North);
        state.phase = Phase::Falling(start);
        assert!(!state.rotate_piece(true));
        assert_eq!(state.current_piece().unwrap(), start);
    }

    #[test]
    fn test_rotating_o_changes_nothing() {
        let mut state = new_state();
        let start = piece(TetriminoType::O, 4, 5, Facing::North);
        state.phase = Phase::Falling(start);
        assert!(!state.rotate_piece(true));
        assert_eq!(state.current_piece().unwrap(), start);
    }

    // --- Fall Speed and drops ---

    #[test]
    fn test_fall_interval_matches_guideline_table() {
        let table = [
            (1, 1.0),
            (2, 0.793),
            (3, 0.618),
            (4, 0.473),
            (5, 0.355),
            (6, 0.262),
            (7, 0.190),
            (8, 0.135),
            (9, 0.094),
            (10, 0.064),
            (11, 0.043),
            (12, 0.028),
            (13, 0.018),
            (14, 0.011),
            (15, 0.007),
        ];
        for (level, seconds) in table {
            let actual = fall_interval(level).as_secs_f64();
            assert!((actual - seconds).abs() < 0.001, "level {level}: {actual}");
        }
    }

    #[test]
    fn test_gravity_follows_current_level() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        state.tick(Duration::from_millis(999));
        assert_eq!(state.current_piece().unwrap().y, 0);
        state.tick(Duration::from_millis(1));
        assert_eq!(state.current_piece().unwrap().y, 1);

        state.scoring.level = 5;
        state.tick(fall_interval(5));
        assert_eq!(state.current_piece().unwrap().y, 2);
    }

    #[test]
    fn test_high_gravity_falls_many_rows_in_one_tick() {
        let mut state = new_state();
        state.scoring.level = 15;
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        state.tick(Duration::from_millis(16));
        assert!(state.current_piece().unwrap().y >= 2);
    }

    #[test]
    fn test_soft_drop_is_20_times_faster_and_scores_per_row() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        state.set_soft_drop(true);
        state.tick(Duration::from_millis(50));
        assert_eq!(state.current_piece().unwrap().y, 1);
        state.tick(Duration::from_millis(50));
        assert_eq!(state.current_piece().unwrap().y, 2);
        assert_eq!(state.scoring.score, 2);

        state.set_soft_drop(false);
        state.tick(Duration::from_millis(50));
        assert_eq!(state.current_piece().unwrap().y, 2);
        assert_eq!(state.scoring.score, 2);
    }

    #[test]
    fn test_starting_soft_drop_does_not_release_built_up_gravity() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        state.tick(Duration::from_millis(900));
        assert_eq!(state.current_piece().unwrap().y, 0);

        state.set_soft_drop(true);
        state.tick(Duration::from_millis(1));
        assert_eq!(state.current_piece().unwrap().y, 0);
        assert_eq!(state.scoring.score, 0);

        // The remaining 10% of the row now takes 5 ms at Soft Drop speed.
        state.tick(Duration::from_millis(4));
        assert_eq!(state.current_piece().unwrap().y, 1);
    }

    #[test]
    fn test_ending_soft_drop_keeps_fraction_of_row() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        state.set_soft_drop(true);
        state.tick(Duration::from_millis(45));
        state.set_soft_drop(false);
        state.tick(Duration::from_millis(99));
        assert_eq!(state.current_piece().unwrap().y, 0);
        state.tick(Duration::from_millis(1));
        assert_eq!(state.current_piece().unwrap().y, 1);
    }

    #[test]
    fn test_setting_soft_drop_twice_changes_nothing() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        state.tick(Duration::from_millis(800));
        state.set_soft_drop(false);
        assert_eq!(state.fall_progress, Duration::from_millis(800));
    }

    #[test]
    fn test_soft_drop_step_scores_one_point() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        assert!(state.soft_drop_step());
        assert_eq!(state.scoring.score, 1);
    }

    #[test]
    fn test_soft_drop_landing_waits_for_lock_timer() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, 18, Facing::North));
        assert!(!state.soft_drop_step());
        assert!(state.current_piece().is_some());
    }

    #[test]
    fn test_hard_drop_scores_two_per_row_and_locks() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, 8, Facing::North));
        state.hard_drop();
        assert_eq!(state.scoring.score, 20);
        assert!(state.current_piece().is_none());
        assert_eq!(state.board.get_cell(4, 19), Some(TetriminoType::O));
    }

    fn with_starting_blocks(rows: u32, seed: u64) -> GameState {
        GameState::with_seed(
            GameConfig {
                starting_blocks: rows,
                enable_sound: false,
                ..GameConfig::default()
            },
            seed,
        )
    }

    #[test]
    fn test_starting_blocks_fill_only_the_bottom_rows_and_leave_gaps() {
        for seed in 0..50 {
            let state = with_starting_blocks(6, seed);
            let board = &state.board;
            let (width, height) = (board.get_width(), board.get_height() as i32);
            for y in -2..height {
                let minos = (0..width)
                    .filter(|&x| board.get_cell(x, y).is_some())
                    .count();
                if y >= height - 6 {
                    assert!((1..width).contains(&minos), "seed {seed} row {y}: {minos}");
                } else {
                    assert_eq!(minos, 0, "seed {seed} row {y}");
                }
            }
            assert!(board.get_full_lines().is_empty(), "seed {seed}");
        }
    }

    #[test]
    fn test_starting_blocks_repeat_with_the_seed_and_keep_the_first_tetriminos() {
        let first = with_starting_blocks(5, 7);
        let again = with_starting_blocks(5, 7);
        assert_eq!(first.board.cells(), again.board.cells());
        assert_eq!(first.next_pieces, with_starting_blocks(0, 7).next_pieces);
        assert_ne!(
            first.board.cells(),
            with_starting_blocks(5, 8).board.cells()
        );
    }

    #[test]
    fn test_each_lock_down_counts_a_tetrimino() {
        let mut state = new_state();
        for _ in 0..2 {
            state.phase = Phase::Falling(piece(TetriminoType::O, 4, 8, Facing::North));
            state.hard_drop();
        }
        assert_eq!(state.scoring.statistics.tetriminos, 2);
    }

    #[test]
    fn test_hard_drop_scores_level_independently() {
        let mut state = new_state();
        state.scoring.level = 7;
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, 17, Facing::North));
        state.hard_drop();
        assert_eq!(state.scoring.score, 2);
    }

    #[test]
    fn test_gravity_does_not_score() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        state.tick(Duration::from_secs(1));
        assert_eq!(state.scoring.score, 0);
    }

    // --- Lock Down ---

    fn landed_o(mode: LockDownMode) -> GameState {
        let mut state = state_with_mode(mode);
        let p = piece(TetriminoType::O, 4, 18, Facing::North);
        state.phase = Phase::Falling(p);
        state.lock = LockState::new(p.y);
        state
    }

    #[test]
    fn test_landing_waits_half_a_second_before_locking() {
        let mut state = landed_o(LockDownMode::Extended);
        state.tick(Duration::from_millis(499));
        assert!(state.current_piece().is_some());
        state.tick(Duration::from_millis(1));
        assert!(state.current_piece().is_none());
    }

    #[test]
    fn test_extended_move_resets_timer() {
        let mut state = landed_o(LockDownMode::Extended);
        state.tick(Duration::from_millis(400));
        assert!(state.shift_piece(1));
        state.tick(Duration::from_millis(400));
        assert!(state.current_piece().is_some());
        state.tick(Duration::from_millis(100));
        assert!(state.current_piece().is_none());
    }

    #[test]
    fn test_extended_locks_at_once_after_15_moves() {
        let mut state = landed_o(LockDownMode::Extended);
        state.tick(Duration::from_millis(1));
        for i in 0..MAX_LOCK_MOVES {
            let dx = if i % 2 == 0 { 1 } else { -1 };
            assert!(state.shift_piece(dx));
            state.tick(Duration::from_millis(1));
            let still_in_play = i + 1 < MAX_LOCK_MOVES;
            assert_eq!(state.current_piece().is_some(), still_in_play, "move {i}");
        }
    }

    #[test]
    fn test_extended_moves_only_count_after_landing() {
        let mut state = state_with_mode(LockDownMode::Extended);
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 0, Facing::North));
        for _ in 0..3 {
            state.shift_piece(1);
            state.shift_piece(-1);
        }
        assert_eq!(state.lock.moves_left, MAX_LOCK_MOVES);
    }

    #[test]
    fn test_extended_new_lowest_row_restores_moves() {
        let mut state = state_with_mode(LockDownMode::Extended);
        let p = piece(TetriminoType::O, 0, 10, Facing::North);
        state.phase = Phase::Falling(p);
        state.lock = LockState::new(p.y);
        fill(&mut state, &[(0, 12), (1, 12)]);
        state.tick(Duration::from_millis(1));
        state.lock.moves_left = 2;
        // Moving off the ledge lets it fall below the lowest row.
        state.shift_piece(1);
        state.shift_piece(1);
        state.tick(Duration::from_secs(1));
        assert_eq!(state.current_piece().unwrap().y, 11);
        assert_eq!(state.lock.moves_left, MAX_LOCK_MOVES);
    }

    #[test]
    fn test_infinite_placement_never_runs_out_of_moves() {
        let mut state = landed_o(LockDownMode::Infinite);
        for i in 0..40 {
            state.tick(Duration::from_millis(400));
            let dx = if i % 2 == 0 { 1 } else { -1 };
            assert!(state.shift_piece(dx));
        }
        assert!(state.current_piece().is_some());
    }

    #[test]
    fn test_classic_move_does_not_reset_timer() {
        let mut state = landed_o(LockDownMode::Classic);
        state.tick(Duration::from_millis(400));
        assert!(state.shift_piece(1));
        state.tick(Duration::from_millis(100));
        assert!(state.current_piece().is_none());
    }

    #[test]
    fn test_timer_stops_while_airborne() {
        let mut state = landed_o(LockDownMode::Classic);
        state.tick(Duration::from_millis(400));
        let before = state.lock.timer;
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, 16, Facing::North));
        state.apply_gravity(Duration::ZERO);
        state.update_lock_down(Duration::from_millis(100));
        assert_eq!(state.lock.timer, before);
    }

    // --- Hold ---

    #[test]
    fn test_hold_disabled_does_nothing() {
        let mut state = GameState::new(GameConfig {
            enable_hold: false,
            ..config()
        });
        state.phase = Phase::Falling(Tetrimino::new(TetriminoType::T));
        state.held_piece = Some(TetriminoType::I);
        state.hold_piece();
        assert_eq!(state.current_piece().unwrap().kind, TetriminoType::T);
        assert_eq!(state.held_piece, Some(TetriminoType::I));
    }

    #[test]
    fn test_hold_with_no_piece_in_play_does_nothing() {
        let mut state = new_state();
        state.held_piece = Some(TetriminoType::T);
        state.hold_piece();
        assert_eq!(state.current_piece(), None);
        assert_eq!(state.held_piece, Some(TetriminoType::T));
    }

    #[test]
    fn test_first_hold_takes_from_next_queue() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 5, Facing::South));
        state.next_pieces[0] = TetriminoType::I;
        state.hold_piece();
        assert_eq!(state.held_piece, Some(TetriminoType::T));
        assert_eq!(state.current_piece().unwrap().kind, TetriminoType::I);
        assert_eq!(state.next_pieces.len(), 3);
    }

    #[test]
    fn test_hold_swap_returns_to_generation_position_facing_north() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 5, 10, Facing::South));
        state.held_piece = Some(TetriminoType::J);
        let next_before = state.next_pieces.clone();

        state.hold_piece();

        let p = state.current_piece().unwrap();
        assert_eq!(
            (p.kind, p.x, p.y, p.facing),
            (TetriminoType::J, 3, -1, Facing::North)
        );
        assert_eq!(state.held_piece, Some(TetriminoType::T));
        assert_eq!(state.next_pieces, next_before);
    }

    #[test]
    fn test_hold_allowed_once_per_lock_down() {
        let mut state = new_state();
        state.phase = Phase::Falling(Tetrimino::at_generation(TetriminoType::T, 10));
        state.held_piece = Some(TetriminoType::I);

        state.hold_piece();
        let after_first = state.current_piece().unwrap();
        state.hold_piece();

        assert_eq!(state.current_piece().unwrap(), after_first);
        assert_eq!(state.held_piece, Some(TetriminoType::T));

        state.hard_drop();
        state.next_pieces[0] = TetriminoType::J;
        state.tick(GENERATION_DELAY);
        state.hold_piece();
        assert_eq!(state.held_piece, Some(TetriminoType::J));
    }

    #[test]
    fn test_hold_does_not_break_back_to_back() {
        let mut state = new_state();
        do_tetris(&mut state);
        state.tick(GENERATION_DELAY);
        state.hold_piece();
        assert!(state.held_piece.is_some());
        do_tetris(&mut state);
        assert_eq!(state.scoring.score, 800 + 1200);
    }

    // --- T-Spin recognition ---

    #[test]
    fn test_tspin_double_through_real_rotation() {
        let mut state = new_state();
        do_tspin_double(&mut state);
        assert_eq!(state.scoring.lines_cleared, 2);
        assert_eq!(state.scoring.score, 1200);
        assert_eq!(state.notification_text(), Some("T-Spin Double"));
    }

    #[test]
    fn test_back_to_back_notification() {
        let mut state = new_state();
        do_tetris(&mut state);
        assert_eq!(state.notification_text(), Some("Tetris"));
        state.board = Board::new(10, 20);
        do_tspin_double(&mut state);
        assert_eq!(
            state.notification_text(),
            Some("Back-to-Back T-Spin Double")
        );
        assert_eq!(state.scoring.score, 800 + 1800);
    }

    #[test]
    fn test_notification_expires() {
        let mut state = new_state();
        do_tetris(&mut state);
        state.tick(NOTIFICATION_TIME - Duration::from_millis(1));
        assert!(state.notification_text().is_some());
        state.tick(Duration::from_millis(1));
        assert!(state.notification_text().is_none());
    }

    #[test]
    fn test_moved_into_slot_is_not_a_tspin() {
        let mut state = new_state();
        fill_row_except(&mut state, 19, &[4]);
        fill_row_except(&mut state, 18, &[3, 4, 5]);
        fill(&mut state, &[(3, 17)]);
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 17, Facing::South));
        lock_and_clear(&mut state);
        assert_eq!(state.scoring.score, 300);
    }

    #[test]
    fn test_move_after_rotation_cancels_tspin() {
        let mut state = new_state();
        let p = piece(TetriminoType::T, 3, 10, Facing::North);
        state.phase = Phase::Falling(p);
        state.rotate_piece(true);
        state.shift_piece(1);
        assert!(!state.last_move_was_rotation);
    }

    #[test]
    fn test_non_t_is_not_a_tspin() {
        let mut state = new_state();
        state.last_move_was_rotation = true;
        let p = piece(TetriminoType::S, 0, 18, Facing::North);
        assert_eq!(state.detect_tspin(&p), TSpin::None);
    }

    #[test]
    fn test_corner_rules_for_every_facing() {
        // Center at (4, 10). Corners listed as (TL, TR, BR, BL); front corners depend on facing.
        let tl = (3, 9);
        let tr = (5, 9);
        let br = (5, 11);
        let bl = (3, 11);
        let cases = [
            (Facing::North, [tl, tr, bl], TSpin::Full),
            (Facing::North, [bl, br, tl], TSpin::Mini),
            (Facing::East, [tr, br, tl], TSpin::Full),
            (Facing::East, [tl, bl, tr], TSpin::Mini),
            (Facing::South, [br, bl, tr], TSpin::Full),
            (Facing::South, [tl, tr, br], TSpin::Mini),
            (Facing::West, [bl, tl, br], TSpin::Full),
            (Facing::West, [tr, br, tl], TSpin::Mini),
        ];
        for (facing, corners, expected) in cases {
            let mut state = new_state();
            for (x, y) in corners {
                state.board.set_cell(x, y, Some(TetriminoType::I));
            }
            state.last_move_was_rotation = true;
            let p = piece(TetriminoType::T, 3, 9, facing);
            assert_eq!(state.detect_tspin(&p), expected, "{facing:?}");
        }
    }

    #[test]
    fn test_two_corners_is_not_a_tspin() {
        let mut state = new_state();
        fill(&mut state, &[(3, 9), (5, 9)]);
        state.last_move_was_rotation = true;
        let p = piece(TetriminoType::T, 3, 9, Facing::North);
        assert_eq!(state.detect_tspin(&p), TSpin::None);
    }

    #[test]
    fn test_walls_and_floor_count_as_corners() {
        let mut state = new_state();
        fill(&mut state, &[(3, 18)]);
        state.last_move_was_rotation = true;
        let p = piece(TetriminoType::T, 3, 18, Facing::North);
        assert_eq!(state.detect_tspin(&p), TSpin::Mini);
    }

    #[test]
    fn test_point_5_upgrades_mini_to_tspin() {
        let mut state = new_state();
        fill(&mut state, &[(3, 18)]);
        state.last_move_was_rotation = true;
        state.used_rotation_point_5 = true;
        let p = piece(TetriminoType::T, 3, 18, Facing::North);
        assert_eq!(state.detect_tspin(&p), TSpin::Full);
    }

    // --- Sound effects ---

    fn sounds(state: &mut GameState) -> Vec<SoundEffect> {
        state.take_sound_effects()
    }

    #[test]
    fn test_move_and_rotate_sound_only_when_they_happen() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 5, Facing::North));
        state.shift_piece(1);
        state.rotate_piece(true);
        assert_eq!(sounds(&mut state), [SoundEffect::Move, SoundEffect::Rotate]);

        // Against the left wall, a move fails silently here; the session plays that sound.
        state.phase = Phase::Falling(piece(TetriminoType::T, 0, 5, Facing::North));
        assert!(!state.shift_piece(-1));
        assert!(sounds(&mut state).is_empty());
    }

    #[test]
    fn test_blocked_rotation_has_its_own_sound() {
        let mut state = new_state();
        for y in 0..20 {
            for x in 0..10 {
                let in_piece = (y == 10 && (3..=5).contains(&x)) || (y == 9 && x == 4);
                if !in_piece {
                    fill(&mut state, &[(x, y)]);
                }
            }
        }
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 9, Facing::North));
        assert!(!state.rotate_piece(true));
        assert_eq!(sounds(&mut state), [SoundEffect::RotateBlocked]);
    }

    #[test]
    fn test_rotating_o_sounds_like_a_rotation() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, 5, Facing::North));
        state.rotate_piece(true);
        assert_eq!(sounds(&mut state), [SoundEffect::Rotate]);
    }

    #[test]
    fn test_landing_sounds_once_then_lock_down_sounds() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, 17, Facing::North));
        for _ in 0..63 {
            state.tick(Duration::from_millis(16));
        }
        assert_eq!(state.current_piece().unwrap().y, 18);
        assert_eq!(sounds(&mut state), [SoundEffect::Land]);

        state.tick(Duration::from_millis(100));
        assert!(sounds(&mut state).is_empty());

        state.tick(LOCK_DELAY);
        assert_eq!(sounds(&mut state), [SoundEffect::LockDown]);
    }

    #[test]
    fn test_hard_drop_has_its_own_sound_instead_of_lock_down() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, 5, Facing::North));
        state.hard_drop();
        assert_eq!(sounds(&mut state), [SoundEffect::HardDrop]);
    }

    #[test]
    fn test_soft_drop_sounds_when_it_starts_and_per_step() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 5, Facing::North));
        state.set_soft_drop(true);
        state.set_soft_drop(true);
        assert_eq!(sounds(&mut state), [SoundEffect::SoftDrop]);

        state.set_soft_drop(false);
        state.soft_drop_step();
        assert_eq!(sounds(&mut state), [SoundEffect::SoftDrop]);
    }

    #[test]
    fn test_hold_sounds_only_when_allowed() {
        let mut state = new_state();
        state.phase = Phase::Falling(Tetrimino::at_generation(TetriminoType::T, 10));
        state.hold_piece();
        state.hold_piece();
        let held = sounds(&mut state);
        assert_eq!(held.iter().filter(|e| **e == SoundEffect::Hold).count(), 1);
    }

    #[test]
    fn test_line_clears_sound_by_number_of_lines() {
        for lines in 1..=4 {
            let mut state = new_state();
            for y in (20 - lines)..20 {
                fill_row_except(&mut state, y, &[0]);
            }
            state.phase = Phase::Falling(piece(TetriminoType::I, -2, 16, Facing::East));
            state.lock_current_piece();
            assert_eq!(
                sounds(&mut state),
                [SoundEffect::LineClear(lines as u32)],
                "{lines} lines"
            );
        }
    }

    #[test]
    fn test_tspin_sound_replaces_line_clear_and_back_to_back_follows() {
        let mut state = new_state();
        do_tetris(&mut state);
        sounds(&mut state);
        state.board = Board::new(10, 20);
        do_tspin_double(&mut state);
        assert_eq!(
            sounds(&mut state),
            [
                SoundEffect::Rotate,
                SoundEffect::TSpin,
                SoundEffect::BackToBack
            ]
        );
    }

    #[test]
    fn test_level_up_and_game_complete_sound() {
        let mut state = new_state();
        state.scoring.lines_until_next_level = 4;
        do_tetris(&mut state);
        assert_eq!(
            sounds(&mut state),
            [SoundEffect::LineClear(4), SoundEffect::LevelUp]
        );

        let mut state = GameState::new(GameConfig {
            starting_level: 15,
            ..config()
        });
        state.scoring.lines_until_next_level = 4;
        do_tetris(&mut state);
        assert_eq!(
            sounds(&mut state),
            [SoundEffect::LineClear(4), SoundEffect::GameComplete]
        );
    }

    #[test]
    fn test_game_over_sounds_for_top_outs_but_not_for_quitting() {
        let mut state = new_state();
        state.next_pieces[0] = TetriminoType::T;
        fill(&mut state, &[(4, -1)]);
        state.spawn_piece();
        assert_eq!(sounds(&mut state), [SoundEffect::GameOver]);

        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, -3, Facing::North));
        state.lock_current_piece();
        assert_eq!(sounds(&mut state), [SoundEffect::GameOver]);

        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 5, Facing::North));
        state.quit();
        assert!(sounds(&mut state).is_empty());
    }

    #[test]
    fn test_taking_sounds_clears_them() {
        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 5, Facing::North));
        state.shift_piece(1);
        assert_eq!(sounds(&mut state).len(), 1);
        assert!(sounds(&mut state).is_empty());
    }

    // --- Phases ---

    #[test]
    fn test_game_starts_in_generation_with_no_piece() {
        let mut state = new_state();
        assert!(state.current_piece().is_none());
        state.tick(Duration::ZERO);
        assert!(state.current_piece().is_some());
    }

    #[test]
    fn test_input_does_nothing_outside_the_falling_phase() {
        let mut state = new_state();
        assert!(!state.shift_piece(1));
        assert!(!state.rotate_piece(true));
        assert!(!state.soft_drop_step());
        state.hard_drop();
        state.hold_piece();
        state.lock_current_piece();
        assert!(matches!(state.phase, Phase::Generation { .. }));
        assert_eq!(state.scoring.score, 0);
    }

    #[test]
    fn test_game_over_ignores_input_but_keeps_piece_on_screen() {
        let mut state = new_state();
        let p = piece(TetriminoType::T, 3, 5, Facing::North);
        state.phase = Phase::Falling(p);
        state.quit();
        assert_eq!(state.outcome(), Some(GameOutcome::Quit));
        assert_eq!(state.current_piece(), Some(p));

        assert!(!state.shift_piece(1));
        state.hard_drop();
        assert_eq!(state.current_piece(), Some(p));
        assert_eq!(state.scoring.score, 0);
    }

    #[test]
    fn test_quit_keeps_the_first_outcome() {
        let mut state = new_state();
        state.phase = Phase::GameOver {
            outcome: GameOutcome::LockOut,
            piece: None,
        };
        state.quit();
        assert_eq!(state.outcome(), Some(GameOutcome::LockOut));
    }

    #[test]
    fn test_outcomes_for_block_out_and_lock_out() {
        let mut state = new_state();
        state.next_pieces[0] = TetriminoType::T;
        fill(&mut state, &[(4, -1)]);
        state.spawn_piece();
        assert_eq!(state.outcome(), Some(GameOutcome::BlockOut));
        assert!(state.current_piece().is_some());

        let mut state = new_state();
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, -3, Facing::North));
        state.lock_current_piece();
        assert_eq!(state.outcome(), Some(GameOutcome::LockOut));
    }

    #[test]
    fn test_finishing_level_15_ends_the_game() {
        let mut state = GameState::new(GameConfig {
            starting_level: 15,
            ..config()
        });
        state.scoring.lines_until_next_level = 4;
        do_tetris(&mut state);
        assert_eq!(state.outcome(), Some(GameOutcome::MarathonComplete));
    }

    fn ultra_state() -> GameState {
        GameState::new(GameConfig {
            game_mode: GameMode::Ultra,
            ..config()
        })
    }

    #[test]
    fn test_ultra_ends_when_time_runs_out_with_the_piece_left_on_screen() {
        let mut state = ultra_state();
        state.tick(Duration::ZERO);
        let piece = state.current_piece();
        assert!(piece.is_some());

        state.elapsed = ULTRA_TIME - Duration::from_millis(20);
        state.tick(Duration::from_millis(16));
        assert!(!state.is_game_over());
        state.take_sound_effects();

        state.tick(Duration::from_millis(16));
        assert_eq!(state.outcome(), Some(GameOutcome::UltraComplete));
        assert_eq!(state.elapsed(), ULTRA_TIME);
        assert_eq!(sounds(&mut state), [SoundEffect::GameComplete]);
        assert_eq!(state.current_piece().map(|p| p.kind), piece.map(|p| p.kind));

        state.tick(Duration::from_secs(5));
        assert_eq!(state.elapsed(), ULTRA_TIME);
    }

    #[test]
    fn test_ultra_scores_lines_still_clearing_when_time_runs_out() {
        let mut state = ultra_state();
        for y in 16..20 {
            fill_row_except(&mut state, y, &[0]);
        }
        state.phase = Phase::Falling(piece(TetriminoType::I, -2, 16, Facing::East));
        state.lock_current_piece();
        assert!(state.line_clear_animation().is_some());

        state.tick(ULTRA_TIME);
        assert_eq!(state.outcome(), Some(GameOutcome::UltraComplete));
        assert_eq!(state.scoring.lines_cleared, 4);
        assert_eq!(state.scoring.score, 800);
        assert_eq!(state.current_piece(), None);
    }

    #[test]
    fn test_ultra_does_not_end_on_lines() {
        let mut state = ultra_state();
        for _ in 0..20 {
            do_tetris(&mut state);
        }
        assert!(!state.is_game_over());
        assert_eq!(state.scoring.level, 1);
    }

    #[test]
    fn test_clearing_40_lines_ends_a_sprint_and_stops_the_clock() {
        let mut state = GameState::new(GameConfig {
            game_mode: GameMode::Sprint,
            ..config()
        });
        state.scoring.lines_until_next_level = 4;
        state.tick(Duration::from_secs(90));
        do_tetris(&mut state);
        assert_eq!(state.outcome(), Some(GameOutcome::SprintComplete));
        assert_eq!(
            sounds(&mut state),
            [SoundEffect::LineClear(4), SoundEffect::GameComplete]
        );

        let time = state.elapsed();
        state.tick(Duration::from_secs(5));
        assert_eq!(state.elapsed(), time);
    }

    #[test]
    fn test_tspin_survives_the_line_clear_animation() {
        let mut state = new_state();
        fill_row_except(&mut state, 19, &[4]);
        fill_row_except(&mut state, 18, &[3, 4, 5]);
        fill(&mut state, &[(3, 17)]);
        state.phase = Phase::Falling(piece(TetriminoType::T, 3, 17, Facing::East));
        assert!(state.rotate_piece(true));
        state.lock_current_piece();
        assert_eq!(state.scoring.score, 0);

        state.tick(Duration::from_secs(1));
        assert_eq!(state.scoring.score, 1200);
    }

    // --- Line Clear flow ---

    #[test]
    fn test_line_clear_animation_then_clear_then_generation() {
        let mut state = new_state();
        fill_row_except(&mut state, 19, &[4, 5]);
        state.phase = Phase::Falling(piece(TetriminoType::O, 4, 18, Facing::North));
        state.lock_current_piece();

        assert_eq!(state.line_clear_animation().unwrap().cleared_rows, vec![19]);
        assert_eq!(state.scoring.lines_cleared, 0);

        state.tick(LINE_CLEAR_BASE + LINE_CLEAR_PER_LINE);
        assert!(state.line_clear_animation().is_none());
        assert_eq!(state.scoring.lines_cleared, 1);
        assert_eq!(state.scoring.score, 100);
        assert!(state.current_piece().is_none());

        state.tick(GENERATION_DELAY);
        assert!(state.current_piece().is_some());
    }

    #[test]
    fn test_more_lines_animate_longer() {
        let animation = |total_lines| LineClearAnimation {
            cleared_rows: vec![],
            elapsed: Duration::ZERO,
            total_lines,
            tspin: TSpin::None,
        };
        assert!(animation(4).duration() > animation(1).duration());
    }

    #[test]
    fn test_blink_alternates() {
        let mut state = new_state();
        assert!(!state.should_show_cleared_rows());
        state.phase = Phase::LineClear(LineClearAnimation {
            cleared_rows: vec![19],
            elapsed: Duration::ZERO,
            total_lines: 1,
            tspin: TSpin::None,
        });
        assert!(state.should_show_cleared_rows());
        state.phase = Phase::LineClear(LineClearAnimation {
            cleared_rows: vec![19],
            elapsed: LINE_CLEAR_BLINK,
            total_lines: 1,
            tspin: TSpin::None,
        });
        assert!(!state.should_show_cleared_rows());
    }

    #[test]
    fn test_complete_line_clear_without_pending_does_nothing() {
        let mut state = new_state();
        state.complete_line_clear();
        assert!(matches!(state.phase, Phase::Generation { .. }));
    }

    #[test]
    fn test_tick_counts_time_and_stops_at_game_over() {
        let mut state = new_state();
        state.tick(Duration::from_secs(3));
        assert_eq!(state.elapsed, Duration::from_secs(3));
        state.quit();
        state.tick(Duration::from_secs(3));
        assert_eq!(state.elapsed, Duration::from_secs(3));
    }

    #[test]
    fn test_custom_board_size_and_starting_level() {
        let state = GameState::new(GameConfig {
            board_width: 15,
            board_height: 25,
            starting_level: 5,
            lines_per_level: 15,
            ..config()
        });
        assert_eq!(state.board.get_width(), 15);
        assert_eq!(state.board.get_height(), 25);
        assert_eq!(state.scoring.level, 5);
        assert_eq!(state.scoring.lines_until_next_level, 75);
    }
}
