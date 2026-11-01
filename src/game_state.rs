use crate::board::Board;
use crate::config::{GameConfig, LockDownMode};
use crate::tetrimino::{Tetrimino, TetriminoType, rotation_points};
use rand::rng;
use rand::seq::SliceRandom;
use std::time::Duration;

/// Time a landed Tetrimino has before it Locks Down (Guideline 5.7).
const LOCK_DELAY: Duration = Duration::from_millis(500);
/// Moves or rotations allowed per lowest row in Extended Placement (Guideline 5.7).
const MAX_LOCK_MOVES: u32 = 15;
/// Delay between Lock Down and the Next Tetrimino (Guideline A1.2.1).
const GENERATION_DELAY: Duration = Duration::from_millis(200);
/// Marathon ends after this level (Guideline 12.1).
const MAX_LEVEL: u32 = 15;
/// Soft Drop speed relative to normal Fall Speed (Guideline 5.5).
const SOFT_DROP_FACTOR: u32 = 20;
/// How long an action notification stays on screen.
const NOTIFICATION_TIME: Duration = Duration::from_secs(2);
const LINE_CLEAR_BASE: Duration = Duration::from_millis(100);
const LINE_CLEAR_PER_LINE: Duration = Duration::from_millis(150);
const LINE_CLEAR_BLINK: Duration = Duration::from_millis(75);

/// Kind of T-Spin recognized when a Tetrimino Locks Down (Guideline 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TSpin {
    None,
    Mini,
    Full,
}

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

pub struct GameState {
    pub board: Board,
    pub current_piece: Option<Tetrimino>,
    pub held_piece: Option<TetriminoType>,
    pub next_pieces: Vec<TetriminoType>,
    pub score: u64,
    pub level: u32,
    pub lines_cleared: u32,
    pub game_over: bool,
    /// True when the game ended by finishing level 15, not by a Game Over Condition.
    pub marathon_complete: bool,
    pub config: GameConfig,
    /// Lines (or awarded Line Clears) left to reach the Goal.
    pub lines_until_next_level: u32,
    pub line_clear_animation: Option<LineClearAnimation>,
    pub pending_line_clear: bool,
    /// Time played, excluding pauses.
    pub elapsed: Duration,
    /// True while the help overlay is open, which stops play.
    pub show_help: bool,
    bag: Vec<TetriminoType>,
    back_to_back_active: bool,
    hold_used: bool,
    soft_dropping: bool,
    fall_progress: Duration,
    lock: LockState,
    generation_delay: Option<Duration>,
    last_move_was_rotation: bool,
    used_rotation_point_5: bool,
    pending_tspin: TSpin,
    notification: Option<(String, Duration)>,
}

pub struct LineClearAnimation {
    pub cleared_rows: Vec<i32>,
    pub elapsed: Duration,
    pub total_lines: u32,
}

impl LineClearAnimation {
    fn duration(&self) -> Duration {
        LINE_CLEAR_BASE + LINE_CLEAR_PER_LINE * self.total_lines
    }
}

impl GameState {
    pub fn new(config: GameConfig) -> Self {
        let starting_level = config.starting_level;

        let mut game_state = Self {
            board: Board::new(config.board_width, config.board_height),
            current_piece: None,
            held_piece: None,
            next_pieces: Vec::new(),
            score: 0,
            level: starting_level,
            lines_cleared: 0,
            game_over: false,
            marathon_complete: false,
            lines_until_next_level: 0,
            config,
            line_clear_animation: None,
            pending_line_clear: false,
            elapsed: Duration::ZERO,
            show_help: false,
            bag: Vec::new(),
            back_to_back_active: false,
            hold_used: false,
            soft_dropping: false,
            fall_progress: Duration::ZERO,
            lock: LockState::new(0),
            generation_delay: None,
            last_move_was_rotation: false,
            used_rotation_point_5: false,
            pending_tspin: TSpin::None,
            notification: None,
        };

        game_state.lines_until_next_level = game_state.starting_goal();
        game_state.refill_bag();
        game_state.populate_next_pieces();
        game_state
    }

    /// Advances all timers: Line Clear animation, generation, gravity and Lock Down.
    pub fn tick(&mut self, dt: Duration) {
        if self.game_over || self.show_help {
            return;
        }
        self.elapsed += dt;
        self.update_notification(dt);

        if self.pending_line_clear {
            self.update_line_clear_animation(dt);
            return;
        }

        if let Some(remaining) = self.generation_delay {
            if remaining <= dt {
                self.generation_delay = None;
                self.spawn_piece();
            } else {
                self.generation_delay = Some(remaining - dt);
            }
            return;
        }

        if self.current_piece.is_some() {
            self.apply_gravity(dt);
            self.update_lock_down(dt);
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

    fn update_line_clear_animation(&mut self, dt: Duration) {
        if let Some(animation) = &mut self.line_clear_animation {
            animation.elapsed += dt;
        }
        if !self.is_line_clear_animation_active() {
            self.complete_line_clear();
        }
    }

    fn apply_gravity(&mut self, dt: Duration) {
        if !self.can_fall() {
            self.fall_progress = Duration::ZERO;
            return;
        }

        let mut interval = fall_interval(self.level);
        if self.soft_dropping {
            interval /= SOFT_DROP_FACTOR;
        }

        self.fall_progress += dt;
        while self.fall_progress >= interval {
            self.fall_progress -= interval;
            if !self.fall_one_row() {
                self.fall_progress = Duration::ZERO;
                break;
            }
        }
    }

    fn update_lock_down(&mut self, dt: Duration) {
        if self.current_piece.is_none() || self.can_fall() {
            return;
        }
        self.lock.has_landed = true;

        let out_of_moves =
            self.config.lock_down_mode == LockDownMode::Extended && self.lock.moves_left == 0;
        if out_of_moves || self.lock.timer <= dt {
            self.lock_current_piece();
        } else {
            self.lock.timer -= dt;
        }
    }

    fn can_fall(&self) -> bool {
        self.current_piece.is_some_and(|piece| {
            let below = Tetrimino {
                y: piece.y + 1,
                ..piece
            };
            self.board.is_valid_position(&below)
        })
    }

    fn fall_one_row(&mut self) -> bool {
        if !self.move_piece(0, 1) {
            return false;
        }
        if self.soft_dropping {
            self.score += 1;
        }
        self.last_move_was_rotation = false;
        self.note_new_lowest_row();
        true
    }

    /// Falling below the lowest row reached restores the timer and move count.
    fn note_new_lowest_row(&mut self) {
        if let Some(piece) = self.current_piece
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
    pub fn spawn_piece(&mut self) {
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
        self.current_piece = Some(piece);
        self.lock = LockState::new(piece.y);
        self.fall_progress = Duration::ZERO;
        self.last_move_was_rotation = false;
        self.used_rotation_point_5 = false;

        if !self.board.is_valid_position(&piece) {
            self.game_over = true;
            return;
        }
        self.fall_one_row_silently();
    }

    fn fall_one_row_silently(&mut self) {
        if self.move_piece(0, 1) {
            self.note_new_lowest_row();
        }
    }

    /// Moves the Tetrimino in play if the target position is free. Applies no game rules.
    pub fn move_piece(&mut self, dx: i32, dy: i32) -> bool {
        if let Some(ref mut piece) = self.current_piece {
            piece.x += dx;
            piece.y += dy;

            if !self.board.is_valid_position(piece) {
                piece.x -= dx;
                piece.y -= dy;
                return false;
            }
            true
        } else {
            false
        }
    }

    /// Player move left or right (Guideline 5.1).
    pub fn shift_piece(&mut self, dx: i32) -> bool {
        let moved = self.move_piece(dx, 0);
        if moved {
            self.last_move_was_rotation = false;
            self.note_manipulation();
        }
        moved
    }

    /// Turns continuous Soft Drop on or off (Guideline 5.5).
    pub fn set_soft_drop(&mut self, active: bool) {
        if active == self.soft_dropping {
            return;
        }
        self.soft_dropping = active;
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
        let moved = self.move_piece(0, 1);
        if moved {
            self.score += 1;
            self.last_move_was_rotation = false;
            self.note_new_lowest_row();
        }
        moved
    }

    /// Rotates using the Super Rotation System (Guideline A1.4). The first rotation point
    /// that fits is used. Returns false if none fit.
    pub fn rotate_piece(&mut self, clockwise: bool) -> bool {
        let Some(piece) = self.current_piece else {
            return false;
        };
        // All O-Tetrimino facings occupy the same cells, so rotating changes nothing.
        if piece.kind == TetriminoType::O {
            return false;
        }

        let from = piece.rotation % 4;
        let to = if clockwise {
            (from + 1) % 4
        } else {
            (from + 3) % 4
        };

        for (point, (dx, dy)) in rotation_points(piece.kind, from, to) {
            let candidate = Tetrimino {
                x: piece.x + dx,
                y: piece.y + dy,
                rotation: to,
                ..piece
            };
            if self.board.is_valid_position(&candidate) {
                self.current_piece = Some(candidate);
                self.last_move_was_rotation = true;
                self.used_rotation_point_5 |= point == 5;
                self.note_manipulation();
                return true;
            }
        }
        false
    }

    /// Drops to the Surface below and Locks Down at once, for 2 points per row (Guideline 8).
    pub fn hard_drop(&mut self) {
        if self.current_piece.is_none() {
            return;
        }
        while self.move_piece(0, 1) {
            self.score += 2;
            self.last_move_was_rotation = false;
        }
        self.lock_current_piece();
    }

    /// Locks the Tetrimino in play and starts the Pattern and Animate phases.
    pub fn lock_current_piece(&mut self) {
        let Some(piece) = self.current_piece else {
            return;
        };
        self.pending_tspin = self.detect_tspin(&piece);
        self.current_piece = None;
        self.board.lock_tetromino(&piece);
        self.hold_used = false;

        // A whole Tetrimino above the Skyline is a Lock Out (Guideline 10).
        if piece.get_blocks().iter().all(|(_, dy)| piece.y + dy < 0) {
            self.game_over = true;
            return;
        }

        let cleared_rows = self.board.get_full_lines();
        if cleared_rows.is_empty() {
            self.award_action(0);
            self.finish_piece();
        } else {
            self.line_clear_animation = Some(LineClearAnimation {
                total_lines: cleared_rows.len() as u32,
                cleared_rows,
                elapsed: Duration::ZERO,
            });
            self.pending_line_clear = true;
        }
    }

    /// Eliminate and Completion phases: removes the lines and awards the action.
    pub fn complete_line_clear(&mut self) {
        if !self.pending_line_clear {
            return;
        }
        self.pending_line_clear = false;
        self.line_clear_animation = None;

        let lines = self.board.clear_lines();
        self.lines_cleared += lines;
        self.award_action(lines);
        self.finish_piece();
    }

    fn finish_piece(&mut self) {
        if !self.game_over {
            self.generation_delay = Some(GENERATION_DELAY);
        }
    }

    /// Recognizes a T-Spin from the four cells diagonal to the T's center (Guideline 9.1).
    /// Walls and floor count as occupied. A and B are on the side the T points to.
    fn detect_tspin(&self, piece: &Tetrimino) -> TSpin {
        if piece.kind != TetriminoType::T || !self.last_move_was_rotation {
            return TSpin::None;
        }

        const CORNERS_CLOCKWISE: [(i32, i32); 4] = [(-1, -1), (1, -1), (1, 1), (-1, 1)];
        let facing = piece.rotation % 4;
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

    /// Scores the action, updates Back-to-Back and the Goal (Guideline 6, 8).
    fn award_action(&mut self, lines: u32) {
        // The scoring table has no Mini T-Spin clearing 2 or more lines.
        let tspin = match self.pending_tspin {
            TSpin::Mini if lines >= 2 => TSpin::Full,
            other => other,
        };
        self.pending_tspin = TSpin::None;

        let action_total = action_points(tspin, lines);
        if action_total == 0 {
            return;
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

        if let Some(name) = action_name(tspin, lines) {
            let text = if is_back_to_back {
                format!("Back-to-Back {name}")
            } else {
                name.to_string()
            };
            self.notification = Some((text, NOTIFICATION_TIME));
        }

        let goal_credit = if self.config.enable_variable_goal {
            (total / 100) as u32
        } else {
            lines
        };
        self.advance_goal(goal_credit);
    }

    /// Goal for a level reached by playing through the one before it.
    fn goal_for_level(&self, level: u32) -> u32 {
        if self.config.enable_variable_goal {
            5 * level
        } else {
            self.config.lines_per_level
        }
    }

    /// First Goal of a game. Starting higher includes the levels skipped (Guideline 6).
    fn starting_goal(&self) -> u32 {
        if self.config.enable_variable_goal {
            5 * self.level
        } else {
            self.config.lines_per_level * self.level
        }
    }

    fn advance_goal(&mut self, credit: u32) {
        let mut remaining = credit;
        while remaining >= self.lines_until_next_level {
            remaining -= self.lines_until_next_level;
            if self.level >= MAX_LEVEL {
                self.lines_until_next_level = 0;
                self.game_over = true;
                self.marathon_complete = true;
                return;
            }
            self.level += 1;
            self.lines_until_next_level = self.goal_for_level(self.level);
        }
        self.lines_until_next_level -= remaining;
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
        new_bag.shuffle(&mut rng());
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
        let Some(current) = self.current_piece.take() else {
            return;
        };

        self.hold_used = true;
        match self.held_piece.replace(current.kind) {
            Some(held) => self.place_piece(held),
            None => self.spawn_piece(),
        }
    }

    pub fn is_line_clear_animation_active(&self) -> bool {
        self.line_clear_animation
            .as_ref()
            .is_some_and(|animation| animation.elapsed < animation.duration())
    }

    pub fn should_show_cleared_rows(&self) -> bool {
        self.line_clear_animation.as_ref().is_some_and(|animation| {
            (animation.elapsed.as_millis() / LINE_CLEAR_BLINK.as_millis()).is_multiple_of(2)
        })
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Progress toward the Goal as (done, total), in lines or awarded Line Clears.
    pub fn goal_progress(&self) -> (u32, u32) {
        let total = if self.level == self.config.starting_level {
            self.starting_goal()
        } else {
            self.goal_for_level(self.level)
        };
        (total.saturating_sub(self.lines_until_next_level), total)
    }

    #[cfg(test)]
    pub fn award_notification_for_test(&mut self, text: &str) {
        self.notification = Some((text.to_string(), NOTIFICATION_TIME));
    }

    /// Text of the current action notification, if one is showing (Guideline 13.1.5).
    pub fn notification_text(&self) -> Option<&str> {
        self.notification.as_ref().map(|(text, _)| text.as_str())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
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

    fn piece(kind: TetriminoType, x: i32, y: i32, rotation: usize) -> Tetrimino {
        Tetrimino {
            kind,
            x,
            y,
            rotation,
        }
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
        state.current_piece = Some(piece(TetriminoType::I, -2, 16, 1));
        lock_and_clear(state);
    }

    /// Sets up a T-Spin Double slot and rotates a T into it (Point 1, East to South).
    fn do_tspin_double(state: &mut GameState) {
        fill_row_except(state, 19, &[4]);
        fill_row_except(state, 18, &[3, 4, 5]);
        fill(state, &[(3, 17)]);
        state.current_piece = Some(piece(TetriminoType::T, 3, 17, 1));
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
        let p = state.current_piece.unwrap();
        assert_eq!((p.x, p.y, p.rotation), (3, -1, 0));
        assert!(!state.game_over);
    }

    #[test]
    fn test_spawn_stays_put_when_row_below_blocked() {
        let mut state = new_state();
        state.next_pieces[0] = TetriminoType::T;
        fill(&mut state, &[(4, 0)]);
        state.spawn_piece();
        assert_eq!(state.current_piece.unwrap().y, -2);
        assert!(!state.game_over);
    }

    #[test]
    fn test_block_out_ends_game() {
        let mut state = new_state();
        state.next_pieces[0] = TetriminoType::T;
        fill(&mut state, &[(4, -1)]);
        state.spawn_piece();
        assert!(state.game_over);
    }

    #[test]
    fn test_lock_out_ends_game() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::O, 4, -3, 0));
        state.lock_current_piece();
        assert!(state.game_over);
    }

    #[test]
    fn test_peeking_lock_is_not_game_over() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::O, 4, -1, 0));
        state.lock_current_piece();
        assert!(!state.game_over);
    }

    #[test]
    fn test_next_piece_arrives_after_generation_delay() {
        let mut state = new_state();
        state.spawn_piece();
        state.hard_drop();
        assert!(state.current_piece.is_none());

        state.tick(GENERATION_DELAY - Duration::from_millis(1));
        assert!(state.current_piece.is_none());

        state.tick(Duration::from_millis(1));
        assert!(state.current_piece.is_some());
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
        state.current_piece = Some(piece(TetriminoType::T, 3, 5, 0));
        assert!(state.rotate_piece(false));
        assert_eq!(state.current_piece.unwrap().rotation, 3);
        assert!(state.rotate_piece(true));
        assert_eq!(state.current_piece.unwrap().rotation, 0);
    }

    #[test]
    fn test_four_rotations_return_to_start_in_open_space() {
        for kind in ALL_TYPES {
            for clockwise in [true, false] {
                let mut state = new_state();
                let start = piece(kind, 3, 5, 0);
                state.current_piece = Some(start);
                for _ in 0..4 {
                    state.rotate_piece(clockwise);
                }
                assert_eq!(state.current_piece.unwrap(), start, "{kind:?}");
            }
        }
    }

    #[test]
    fn test_rotate_kicks_off_left_wall() {
        // East-facing T against the left wall: South would hit the wall, so Point 2 shifts right.
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, -1, 5, 1));
        assert!(state.rotate_piece(true));
        let p = state.current_piece.unwrap();
        assert_eq!((p.x, p.y, p.rotation), (0, 5, 2));
    }

    #[test]
    fn test_rotate_kicks_off_floor_with_i() {
        // I North on the floor, rotating to East: only Point 5 (right 1, up 2) can fit.
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::I, 3, 18, 0));
        assert!(state.rotate_piece(true));
        let p = state.current_piece.unwrap();
        assert_eq!((p.x, p.y, p.rotation), (4, 16, 1));
    }

    #[test]
    fn test_rotate_uses_point_5_and_records_it() {
        // Points 1 to 3 are blocked, and T North has no Point 4.
        let mut state = new_state();
        fill(&mut state, &[(4, 12), (3, 10)]);
        state.current_piece = Some(piece(TetriminoType::T, 3, 10, 0));
        assert!(state.rotate_piece(true));
        let p = state.current_piece.unwrap();
        assert_eq!((p.x, p.y, p.rotation), (2, 12, 1));
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
        let start = piece(TetriminoType::T, 3, 9, 0);
        state.current_piece = Some(start);
        assert!(!state.rotate_piece(true));
        assert_eq!(state.current_piece.unwrap(), start);
    }

    #[test]
    fn test_rotating_o_changes_nothing() {
        let mut state = new_state();
        let start = piece(TetriminoType::O, 4, 5, 0);
        state.current_piece = Some(start);
        assert!(!state.rotate_piece(true));
        assert_eq!(state.current_piece.unwrap(), start);
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
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        state.tick(Duration::from_millis(999));
        assert_eq!(state.current_piece.unwrap().y, 0);
        state.tick(Duration::from_millis(1));
        assert_eq!(state.current_piece.unwrap().y, 1);

        state.level = 5;
        state.tick(fall_interval(5));
        assert_eq!(state.current_piece.unwrap().y, 2);
    }

    #[test]
    fn test_high_gravity_falls_many_rows_in_one_tick() {
        let mut state = new_state();
        state.level = 15;
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        state.tick(Duration::from_millis(16));
        assert!(state.current_piece.unwrap().y >= 2);
    }

    #[test]
    fn test_soft_drop_is_20_times_faster_and_scores_per_row() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        state.set_soft_drop(true);
        state.tick(Duration::from_millis(50));
        assert_eq!(state.current_piece.unwrap().y, 1);
        state.tick(Duration::from_millis(50));
        assert_eq!(state.current_piece.unwrap().y, 2);
        assert_eq!(state.score, 2);

        state.set_soft_drop(false);
        state.tick(Duration::from_millis(50));
        assert_eq!(state.current_piece.unwrap().y, 2);
        assert_eq!(state.score, 2);
    }

    #[test]
    fn test_starting_soft_drop_does_not_release_built_up_gravity() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        state.tick(Duration::from_millis(900));
        assert_eq!(state.current_piece.unwrap().y, 0);

        state.set_soft_drop(true);
        state.tick(Duration::from_millis(1));
        assert_eq!(state.current_piece.unwrap().y, 0);
        assert_eq!(state.score, 0);

        // The remaining 10% of the row now takes 5 ms at Soft Drop speed.
        state.tick(Duration::from_millis(4));
        assert_eq!(state.current_piece.unwrap().y, 1);
    }

    #[test]
    fn test_ending_soft_drop_keeps_fraction_of_row() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        state.set_soft_drop(true);
        state.tick(Duration::from_millis(45));
        state.set_soft_drop(false);
        state.tick(Duration::from_millis(99));
        assert_eq!(state.current_piece.unwrap().y, 0);
        state.tick(Duration::from_millis(1));
        assert_eq!(state.current_piece.unwrap().y, 1);
    }

    #[test]
    fn test_setting_soft_drop_twice_changes_nothing() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        state.tick(Duration::from_millis(800));
        state.set_soft_drop(false);
        assert_eq!(state.fall_progress, Duration::from_millis(800));
    }

    #[test]
    fn test_soft_drop_step_scores_one_point() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        assert!(state.soft_drop_step());
        assert_eq!(state.score, 1);
    }

    #[test]
    fn test_soft_drop_landing_waits_for_lock_timer() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::O, 4, 18, 0));
        assert!(!state.soft_drop_step());
        assert!(state.current_piece.is_some());
    }

    #[test]
    fn test_hard_drop_scores_two_per_row_and_locks() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::O, 4, 8, 0));
        state.hard_drop();
        assert_eq!(state.score, 20);
        assert!(state.current_piece.is_none());
        assert_eq!(state.board.get_cell(4, 19), Some(TetriminoType::O));
    }

    #[test]
    fn test_hard_drop_scores_level_independently() {
        let mut state = new_state();
        state.level = 7;
        state.current_piece = Some(piece(TetriminoType::O, 4, 17, 0));
        state.hard_drop();
        assert_eq!(state.score, 2);
    }

    #[test]
    fn test_gravity_does_not_score() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        state.tick(Duration::from_secs(1));
        assert_eq!(state.score, 0);
    }

    // --- Lock Down ---

    fn landed_o(mode: LockDownMode) -> GameState {
        let mut state = state_with_mode(mode);
        let p = piece(TetriminoType::O, 4, 18, 0);
        state.current_piece = Some(p);
        state.lock = LockState::new(p.y);
        state
    }

    #[test]
    fn test_landing_waits_half_a_second_before_locking() {
        let mut state = landed_o(LockDownMode::Extended);
        state.tick(Duration::from_millis(499));
        assert!(state.current_piece.is_some());
        state.tick(Duration::from_millis(1));
        assert!(state.current_piece.is_none());
    }

    #[test]
    fn test_extended_move_resets_timer() {
        let mut state = landed_o(LockDownMode::Extended);
        state.tick(Duration::from_millis(400));
        assert!(state.shift_piece(1));
        state.tick(Duration::from_millis(400));
        assert!(state.current_piece.is_some());
        state.tick(Duration::from_millis(100));
        assert!(state.current_piece.is_none());
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
            assert_eq!(state.current_piece.is_some(), still_in_play, "move {i}");
        }
    }

    #[test]
    fn test_extended_moves_only_count_after_landing() {
        let mut state = state_with_mode(LockDownMode::Extended);
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        for _ in 0..3 {
            state.shift_piece(1);
            state.shift_piece(-1);
        }
        assert_eq!(state.lock.moves_left, MAX_LOCK_MOVES);
    }

    #[test]
    fn test_extended_new_lowest_row_restores_moves() {
        let mut state = state_with_mode(LockDownMode::Extended);
        let p = piece(TetriminoType::O, 0, 10, 0);
        state.current_piece = Some(p);
        state.lock = LockState::new(p.y);
        fill(&mut state, &[(0, 12), (1, 12)]);
        state.tick(Duration::from_millis(1));
        state.lock.moves_left = 2;
        // Moving off the ledge lets it fall below the lowest row.
        state.shift_piece(1);
        state.shift_piece(1);
        state.tick(Duration::from_secs(1));
        assert_eq!(state.current_piece.unwrap().y, 11);
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
        assert!(state.current_piece.is_some());
    }

    #[test]
    fn test_classic_move_does_not_reset_timer() {
        let mut state = landed_o(LockDownMode::Classic);
        state.tick(Duration::from_millis(400));
        assert!(state.shift_piece(1));
        state.tick(Duration::from_millis(100));
        assert!(state.current_piece.is_none());
    }

    #[test]
    fn test_timer_stops_while_airborne() {
        let mut state = landed_o(LockDownMode::Classic);
        state.tick(Duration::from_millis(400));
        let before = state.lock.timer;
        state.current_piece = Some(piece(TetriminoType::O, 4, 16, 0));
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
        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.held_piece = Some(TetriminoType::I);
        state.hold_piece();
        assert_eq!(state.current_piece.unwrap().kind, TetriminoType::T);
        assert_eq!(state.held_piece, Some(TetriminoType::I));
    }

    #[test]
    fn test_hold_with_no_piece_in_play_does_nothing() {
        let mut state = new_state();
        state.held_piece = Some(TetriminoType::T);
        state.hold_piece();
        assert_eq!(state.current_piece, None);
        assert_eq!(state.held_piece, Some(TetriminoType::T));
    }

    #[test]
    fn test_first_hold_takes_from_next_queue() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 3, 5, 2));
        state.next_pieces[0] = TetriminoType::I;
        state.hold_piece();
        assert_eq!(state.held_piece, Some(TetriminoType::T));
        assert_eq!(state.current_piece.unwrap().kind, TetriminoType::I);
        assert_eq!(state.next_pieces.len(), 3);
    }

    #[test]
    fn test_hold_swap_returns_to_generation_position_facing_north() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 5, 10, 2));
        state.held_piece = Some(TetriminoType::J);
        let next_before = state.next_pieces.clone();

        state.hold_piece();

        let p = state.current_piece.unwrap();
        assert_eq!((p.kind, p.x, p.y, p.rotation), (TetriminoType::J, 3, -1, 0));
        assert_eq!(state.held_piece, Some(TetriminoType::T));
        assert_eq!(state.next_pieces, next_before);
    }

    #[test]
    fn test_hold_allowed_once_per_lock_down() {
        let mut state = new_state();
        state.current_piece = Some(Tetrimino::at_generation(TetriminoType::T, 10));
        state.held_piece = Some(TetriminoType::I);

        state.hold_piece();
        let after_first = state.current_piece.unwrap();
        state.hold_piece();

        assert_eq!(state.current_piece.unwrap(), after_first);
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
        assert_eq!(state.score, 800 + 1200);
    }

    // --- Scoring ---

    #[test]
    fn test_line_clear_scores_match_guideline() {
        for (lines, points) in [(1, 100), (2, 300), (3, 500), (4, 800)] {
            let mut state = new_state();
            state.level = 3;
            state.award_action(lines);
            assert_eq!(state.score, points * 3, "{lines} lines");
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
            let mut state = new_state();
            state.pending_tspin = tspin;
            state.award_action(lines);
            assert_eq!(state.score, points, "{tspin:?} {lines}");
        }
    }

    #[test]
    fn test_plain_lock_scores_nothing() {
        let mut state = new_state();
        state.award_action(0);
        assert_eq!(state.score, 0);
    }

    #[test]
    fn test_back_to_back_example_totals_5400() {
        // Guideline 8.1: Tetris, T-Spin Double, T-Spin, Tetris, T-Spin Single at level 1.
        let mut state = GameState::new(GameConfig {
            lines_per_level: 1000,
            ..config()
        });
        state.award_action(4);
        state.pending_tspin = TSpin::Full;
        state.award_action(2);
        state.pending_tspin = TSpin::Full;
        state.award_action(0);
        state.award_action(4);
        state.pending_tspin = TSpin::Full;
        state.award_action(1);
        assert_eq!(state.score, 5400);
    }

    #[test]
    fn test_single_breaks_back_to_back() {
        let mut state = new_state();
        state.award_action(4);
        state.award_action(1);
        let before = state.score;
        state.award_action(4);
        assert_eq!(state.score - before, 800);
    }

    #[test]
    fn test_plain_lock_keeps_back_to_back() {
        let mut state = new_state();
        state.award_action(4);
        state.award_action(0);
        assert!(state.back_to_back_active);
    }

    #[test]
    fn test_no_clear_tspin_cannot_start_back_to_back() {
        let mut state = new_state();
        state.pending_tspin = TSpin::Full;
        state.award_action(0);
        assert!(!state.back_to_back_active);
    }

    // --- T-Spin recognition ---

    #[test]
    fn test_tspin_double_through_real_rotation() {
        let mut state = new_state();
        do_tspin_double(&mut state);
        assert_eq!(state.lines_cleared, 2);
        assert_eq!(state.score, 1200);
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
        assert_eq!(state.score, 800 + 1800);
    }

    #[test]
    fn test_notification_expires() {
        let mut state = new_state();
        state.award_action(4);
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
        state.current_piece = Some(piece(TetriminoType::T, 3, 17, 2));
        lock_and_clear(&mut state);
        assert_eq!(state.score, 300);
    }

    #[test]
    fn test_move_after_rotation_cancels_tspin() {
        let mut state = new_state();
        let p = piece(TetriminoType::T, 3, 10, 0);
        state.current_piece = Some(p);
        state.rotate_piece(true);
        state.shift_piece(1);
        assert!(!state.last_move_was_rotation);
    }

    #[test]
    fn test_non_t_is_not_a_tspin() {
        let mut state = new_state();
        state.last_move_was_rotation = true;
        let p = piece(TetriminoType::S, 0, 18, 0);
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
            (0, [tl, tr, bl], TSpin::Full),
            (0, [bl, br, tl], TSpin::Mini),
            (1, [tr, br, tl], TSpin::Full),
            (1, [tl, bl, tr], TSpin::Mini),
            (2, [br, bl, tr], TSpin::Full),
            (2, [tl, tr, br], TSpin::Mini),
            (3, [bl, tl, br], TSpin::Full),
            (3, [tr, br, tl], TSpin::Mini),
        ];
        for (facing, corners, expected) in cases {
            let mut state = new_state();
            for (x, y) in corners {
                state.board.set_cell(x, y, Some(TetriminoType::I));
            }
            state.last_move_was_rotation = true;
            let p = piece(TetriminoType::T, 3, 9, facing);
            assert_eq!(state.detect_tspin(&p), expected, "facing {facing}");
        }
    }

    #[test]
    fn test_two_corners_is_not_a_tspin() {
        let mut state = new_state();
        fill(&mut state, &[(3, 9), (5, 9)]);
        state.last_move_was_rotation = true;
        let p = piece(TetriminoType::T, 3, 9, 0);
        assert_eq!(state.detect_tspin(&p), TSpin::None);
    }

    #[test]
    fn test_walls_and_floor_count_as_corners() {
        let mut state = new_state();
        fill(&mut state, &[(3, 18)]);
        state.last_move_was_rotation = true;
        let p = piece(TetriminoType::T, 3, 18, 0);
        assert_eq!(state.detect_tspin(&p), TSpin::Mini);
    }

    #[test]
    fn test_point_5_upgrades_mini_to_tspin() {
        let mut state = new_state();
        fill(&mut state, &[(3, 18)]);
        state.last_move_was_rotation = true;
        state.used_rotation_point_5 = true;
        let p = piece(TetriminoType::T, 3, 18, 0);
        assert_eq!(state.detect_tspin(&p), TSpin::Full);
    }

    // --- Goals ---

    #[test]
    fn test_fixed_goal_levels_up_every_10_lines_and_carries_overflow() {
        let mut state = new_state();
        assert_eq!(state.lines_until_next_level, 10);
        state.advance_goal(5);
        assert_eq!((state.level, state.lines_until_next_level), (1, 5));
        state.advance_goal(5);
        assert_eq!((state.level, state.lines_until_next_level), (2, 10));
        state.advance_goal(13);
        assert_eq!((state.level, state.lines_until_next_level), (3, 7));
    }

    #[test]
    fn test_fixed_goal_higher_start_includes_skipped_levels() {
        // Guideline 6: starting on level 4 requires 40 lines, then 10 for level 5.
        let mut state = GameState::new(GameConfig {
            starting_level: 4,
            ..config()
        });
        assert_eq!(state.lines_until_next_level, 40);
        state.advance_goal(40);
        assert_eq!((state.level, state.lines_until_next_level), (5, 10));
    }

    #[test]
    fn test_variable_goal_is_5_times_level() {
        let mut state = GameState::new(GameConfig {
            enable_variable_goal: true,
            ..config()
        });
        assert_eq!(state.lines_until_next_level, 5);
        state.advance_goal(5);
        assert_eq!((state.level, state.lines_until_next_level), (2, 10));
        state.advance_goal(10);
        assert_eq!((state.level, state.lines_until_next_level), (3, 15));
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
            let mut state = GameState::new(GameConfig {
                enable_variable_goal: true,
                starting_level: 15,
                ..config()
            });
            let before = state.lines_until_next_level;
            state.pending_tspin = tspin;
            state.award_action(lines);
            assert_eq!(
                before - state.lines_until_next_level,
                awarded,
                "{tspin:?} {lines}"
            );
        }
    }

    #[test]
    fn test_variable_goal_back_to_back_adds_half() {
        let mut state = GameState::new(GameConfig {
            enable_variable_goal: true,
            starting_level: 15,
            ..config()
        });
        state.award_action(4);
        let before = state.lines_until_next_level;
        state.award_action(4);
        assert_eq!(before - state.lines_until_next_level, 12);
    }

    #[test]
    fn test_marathon_ends_after_level_15() {
        let mut state = GameState::new(GameConfig {
            starting_level: 15,
            ..config()
        });
        state.lines_until_next_level = 2;
        state.advance_goal(2);
        assert!(state.game_over);
        assert!(state.marathon_complete);
        assert_eq!(state.level, 15);
    }

    #[test]
    fn test_goal_progress_follows_the_current_goal() {
        let mut state = GameState::new(GameConfig {
            starting_level: 4,
            ..config()
        });
        assert_eq!(state.goal_progress(), (0, 40));
        state.advance_goal(15);
        assert_eq!(state.goal_progress(), (15, 40));
        state.advance_goal(28);
        assert_eq!((state.level, state.goal_progress()), (5, (3, 10)));
    }

    #[test]
    fn test_goal_progress_with_variable_goal() {
        let mut state = GameState::new(GameConfig {
            enable_variable_goal: true,
            ..config()
        });
        state.advance_goal(7);
        assert_eq!((state.level, state.goal_progress()), (2, (2, 10)));
    }

    // --- Help ---

    #[test]
    fn test_help_stops_gravity_and_time() {
        let mut state = new_state();
        state.current_piece = Some(piece(TetriminoType::T, 3, 0, 0));
        state.toggle_help();
        state.tick(Duration::from_secs(5));
        assert_eq!(state.current_piece.unwrap().y, 0);
        assert_eq!(state.elapsed, Duration::ZERO);

        state.toggle_help();
        state.tick(Duration::from_secs(1));
        assert_eq!(state.current_piece.unwrap().y, 1);
    }

    // --- Line Clear flow ---

    #[test]
    fn test_line_clear_animation_then_clear_then_generation() {
        let mut state = new_state();
        fill_row_except(&mut state, 19, &[4, 5]);
        state.current_piece = Some(piece(TetriminoType::O, 4, 18, 0));
        state.lock_current_piece();

        assert!(state.pending_line_clear);
        assert_eq!(
            state.line_clear_animation.as_ref().unwrap().cleared_rows,
            vec![19]
        );
        assert_eq!(state.lines_cleared, 0);

        state.tick(LINE_CLEAR_BASE + LINE_CLEAR_PER_LINE);
        assert!(!state.pending_line_clear);
        assert_eq!(state.lines_cleared, 1);
        assert_eq!(state.score, 100);
        assert!(state.current_piece.is_none());

        state.tick(GENERATION_DELAY);
        assert!(state.current_piece.is_some());
    }

    #[test]
    fn test_more_lines_animate_longer() {
        let animation = |total_lines| LineClearAnimation {
            cleared_rows: vec![],
            elapsed: Duration::ZERO,
            total_lines,
        };
        assert!(animation(4).duration() > animation(1).duration());
    }

    #[test]
    fn test_blink_alternates() {
        let mut state = new_state();
        assert!(!state.should_show_cleared_rows());
        state.line_clear_animation = Some(LineClearAnimation {
            cleared_rows: vec![19],
            elapsed: Duration::ZERO,
            total_lines: 1,
        });
        assert!(state.should_show_cleared_rows());
        state.line_clear_animation.as_mut().unwrap().elapsed = LINE_CLEAR_BLINK;
        assert!(!state.should_show_cleared_rows());
    }

    #[test]
    fn test_complete_line_clear_without_pending_does_nothing() {
        let mut state = new_state();
        state.complete_line_clear();
        assert!(state.generation_delay.is_none());
    }

    #[test]
    fn test_tick_counts_time_and_stops_at_game_over() {
        let mut state = new_state();
        state.tick(Duration::from_secs(3));
        assert_eq!(state.elapsed, Duration::from_secs(3));
        state.game_over = true;
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
        assert_eq!(state.level, 5);
        assert_eq!(state.lines_until_next_level, 75);
    }
}
