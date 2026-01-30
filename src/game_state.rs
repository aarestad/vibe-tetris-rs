use crate::board::Board;
use crate::config::GameConfig;
use crate::tetrimino::{Tetrimino, TetriminoType};
use rand::rng;
use rand::seq::SliceRandom;
use std::time::Instant;

pub struct GameState {
    pub board: Board,
    pub current_piece: Option<Tetrimino>,
    pub held_piece: Option<TetriminoType>,
    pub next_pieces: Vec<TetriminoType>,
    pub score: u64,
    pub level: u32,
    pub lines_cleared: u32,
    pub game_over: bool,
    pub config: GameConfig,
    bag: Vec<TetriminoType>,
    pub lines_until_next_level: u32,
    pieces_placed: u32,
    combo_count: u32,
    back_to_back_active: bool,
    last_was_special: bool,
    pub line_clear_animation: Option<LineClearAnimation>,
    pub pending_line_clear: bool,
    pub show_help: bool,
}

pub struct LineClearAnimation {
    pub cleared_rows: Vec<usize>,
    pub start_time: Instant,
    pub total_lines: u32,
}

impl GameState {
    pub fn new(config: GameConfig) -> Self {
        let starting_level = config.starting_level;
        let lines_until_next_level = config.lines_per_level;

        let mut game_state = Self {
            board: Board::new(config.board_width, config.board_height),
            current_piece: None,
            held_piece: None,
            next_pieces: Vec::new(),
            score: 0,
            level: starting_level,
            lines_cleared: 0,
            game_over: false,
            config,
            bag: Vec::new(),
            lines_until_next_level,
            pieces_placed: 0,
            combo_count: 0,
            back_to_back_active: false,
            last_was_special: false,
            line_clear_animation: None,
            pending_line_clear: false,
            show_help: false,
        };

        // Initialize the first bag and next pieces
        game_state.refill_bag();
        game_state.populate_next_pieces();
        game_state
    }

    pub fn spawn_piece(&mut self) {
        // Get the next piece from the queue
        if let Some(piece_type) = self.next_pieces.first() {
            let piece = Tetrimino::new(*piece_type);
            self.current_piece = Some(piece);

            // Remove the piece from next_pieces and refill if needed
            self.next_pieces.remove(0);
            self.populate_next_pieces();

            // Check if the spawned piece can be placed
            if let Some(ref current) = self.current_piece
                && !self.board.is_valid_position(current)
            {
                self.game_over = true;
            }
        }
    }

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

    pub fn rotate_piece(&mut self, clockwise: bool) {
        if self.current_piece.as_ref().map(|p| p.kind).is_some() {
            let old_rotation;
            let old_x;
            let old_y;
            let piece_type;

            {
                let piece = self.current_piece.as_mut().unwrap();
                old_rotation = piece.rotation;
                old_x = piece.x;
                old_y = piece.y;
                piece_type = piece.kind;
            }

            let new_rotation = if clockwise {
                old_rotation + 1
            } else {
                old_rotation.wrapping_sub(1)
            };

            // First try the basic rotation
            {
                let piece = self.current_piece.as_mut().unwrap();
                piece.rotation = new_rotation;
            }

            // Check if basic rotation works
            if !self
                .board
                .is_valid_position(self.current_piece.as_ref().unwrap())
            {
                let kicks = self.get_wall_kicks(piece_type, old_rotation, new_rotation);
                let mut kicked = false;

                for (dx, dy) in kicks {
                    {
                        let piece = self.current_piece.as_mut().unwrap();
                        piece.x = old_x + dx;
                        piece.y = old_y + dy;
                    }

                    if self
                        .board
                        .is_valid_position(self.current_piece.as_ref().unwrap())
                    {
                        kicked = true;
                        break;
                    }
                }

                // If no wall kick worked, revert to original position
                if !kicked {
                    let piece = self.current_piece.as_mut().unwrap();
                    piece.rotation = old_rotation;
                    piece.x = old_x;
                    piece.y = old_y;
                }
            }
        }
    }

    pub fn hard_drop(&mut self) {
        while self.move_piece(0, 1) {}
        self.lock_current_piece();
    }

    pub fn lock_current_piece(&mut self) {
        if let Some(piece) = self.current_piece.take() {
            self.board.lock_tetromino(&piece);
            self.pieces_placed += 1;

            let cleared_rows = self.board.get_full_lines();
            let lines = cleared_rows.len() as u32;

            if lines > 0 {
                self.line_clear_animation = Some(LineClearAnimation {
                    cleared_rows,
                    start_time: Instant::now(),
                    total_lines: lines,
                });
                self.pending_line_clear = true;
            } else {
                self.update_score(0, false);
                self.spawn_piece();
            }
        }
    }

    pub fn complete_line_clear(&mut self) {
        if !self.pending_line_clear {
            return;
        }
        self.pending_line_clear = false;

        let lines = self.board.clear_lines();
        self.lines_cleared += lines;

        if lines > 0 {
            self.update_score(lines, true);
        }

        self.line_clear_animation = None;
        self.spawn_piece();
    }

    fn update_score(&mut self, lines: u32, _lines_cleared: bool) {
        if lines == 0 {
            self.combo_count = 0;
            self.last_was_special = false;
            return;
        }

        // Check for T-Spin detection
        let is_tspin = self.check_tspin();

        // Calculate awarded line clears
        let awarded_lines = self.compute_awarded_lines(lines, is_tspin);

        // Apply score based on awarded lines and T-Spin
        let base_score: u64 = match awarded_lines {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };

        // Apply T-Spin bonus multiplier
        let tspin_bonus: u64 = if is_tspin {
            match awarded_lines {
                1 => 800,
                2 => 1200,
                3 => 1600,
                4 => 2000,
                _ => 0,
            }
        } else {
            0
        };

        // Calculate combo bonus
        let combo_bonus: u64 = if self.combo_count > 0 {
            (self.combo_count * 50) as u64
        } else {
            0
        };

        // Calculate back-to-back bonus
        let is_special = awarded_lines == 4 || is_tspin;
        let back_to_back_bonus: u64 = if self.back_to_back_active && is_special {
            (base_score + tspin_bonus) / 2
        } else {
            0
        };

        self.score +=
            (base_score + tspin_bonus + combo_bonus + back_to_back_bonus) * self.level as u64;

        // Update back-to-back state
        self.back_to_back_active = is_special;
        self.last_was_special = is_special;

        // Increment combo if lines were cleared
        if lines > 0 {
            self.combo_count += 1;
        }

        // Update level based on selected goal system
        if self.config.enable_variable_goal {
            self.update_level_variable_goal(lines, is_tspin);
        } else {
            self.update_level_fixed_goal(lines);
        }
    }

    fn compute_awarded_lines(&self, cleared_lines: u32, is_tspin: bool) -> u32 {
        if is_tspin {
            match cleared_lines {
                0 => 0,
                1 => 1,
                2 => 2,
                3 => 3,
                _ => cleared_lines,
            }
        } else {
            cleared_lines
        }
    }

    fn check_tspin(&self) -> bool {
        // Placeholder for T-Spin detection
        // This will be implemented to detect:
        // - Mini T-Spins (1-2 lines)
        // - Regular T-Spins (3 lines)
        // - T-Spin Doubles (2 lines with proper rotation)
        // - T-Spin Triples (3 lines with proper rotation)
        //
        // Detection will check:
        // - Last move was a rotation
        // - T piece is the current piece
        // - T piece has 3 corners occupied by blocks
        // - T-Spin zone is filled appropriately
        false
    }

    fn update_level_fixed_goal(&mut self, lines_cleared: u32) {
        // Fixed Goal System: Advance level after clearing static number of lines
        let lines_required = self.config.lines_per_level;

        if lines_cleared >= self.lines_until_next_level {
            let overflow = lines_cleared - self.lines_until_next_level;
            self.lines_until_next_level = lines_required - overflow;
            self.level += 1;
        } else {
            self.lines_until_next_level -= lines_cleared;
        }
    }

    fn update_level_variable_goal(&mut self, lines_cleared: u32, is_tspin: bool) {
        // Variable Goal System based on Tetris Design Guidelines
        // Adjusts level requirements based on:
        // - Number of pieces placed (performance metric)
        // - Back-to-back bonuses (Tetrises, T-Spins)
        // - Combo multipliers

        let base_lines_per_level = self.config.lines_per_level;
        let is_special_clear = lines_cleared == 4 || is_tspin;

        // Calculate goal adjustment based on pieces placed efficiency
        // More pieces per line cleared = faster level progression
        let efficiency_factor = if self.pieces_placed > 0 {
            (self.pieces_placed as f64 / lines_cleared.max(1) as f64).min(2.0)
        } else {
            1.0
        };

        // Calculate back-to-back bonus (reduces lines needed for next level)
        let back_to_back_reduction = if self.back_to_back_active && is_special_clear {
            (self.level as f64 * 0.1).max(0.5)
        } else {
            0.0
        };

        // Calculate combo bonus (reduces lines needed for next level)
        let combo_reduction = (self.combo_count as f64 * 0.05).min(0.5);

        // Calculate adjusted lines needed for this level
        let adjusted_lines_needed = (base_lines_per_level as f64 * efficiency_factor
            - back_to_back_reduction
            - combo_reduction)
            .max(1.0) as u32;

        if lines_cleared >= self.lines_until_next_level {
            // Level up
            let overflow = lines_cleared.saturating_sub(self.lines_until_next_level);

            // Set new goal for next level with slight increase
            let level_multiplier = 1.0 + (self.level as f64 * 0.02);
            self.lines_until_next_level =
                (adjusted_lines_needed as f64 * level_multiplier).max(1.0) as u32;

            // Apply overflow from this clear to next level
            if overflow > 0 {
                self.lines_until_next_level = self.lines_until_next_level.saturating_sub(overflow);
            }

            self.level += 1;
        } else {
            self.lines_until_next_level -= lines_cleared;
        }
    }

    fn refill_bag(&mut self) {
        // Create a new bag with all 7 tetrimino types
        let mut new_bag = vec![
            TetriminoType::I,
            TetriminoType::O,
            TetriminoType::T,
            TetriminoType::S,
            TetriminoType::Z,
            TetriminoType::J,
            TetriminoType::L,
        ];

        // Shuffle the bag randomly
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
            } else {
                self.refill_bag();
                if let Some(piece) = self.bag.pop() {
                    self.next_pieces.push(piece);
                }
            }
        }
    }

    fn get_wall_kicks(
        &self,
        piece_type: TetriminoType,
        from_rotation: usize,
        to_rotation: usize,
    ) -> Vec<(i32, i32)> {
        // Super Rotation System wall kick tables
        // Format: (dx, dy) offsets to try
        match piece_type {
            TetriminoType::I => {
                // I piece has special wall kick data
                match (from_rotation % 4, to_rotation % 4) {
                    (0, 1) => vec![(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)],
                    (1, 0) => vec![(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
                    (1, 2) => vec![(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
                    (2, 1) => vec![(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
                    (2, 3) => vec![(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
                    (3, 2) => vec![(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)],
                    (3, 0) => vec![(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
                    (0, 3) => vec![(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
                    _ => vec![(0, 0)],
                }
            }
            TetriminoType::O => {
                // O piece doesn't rotate, but include for completeness
                vec![(0, 0)]
            }
            _ => {
                // Basic kicks for most pieces (J, L, S, Z, T)
                match (from_rotation % 4, to_rotation % 4) {
                    (0, 1) => vec![(0, 0), (0, -1), (-1, 0), (-1, -1)],
                    (1, 0) => vec![(0, 0), (0, 1), (1, 0), (1, 1)],
                    (1, 2) => vec![(0, 0), (0, -1), (1, 0), (1, -1)],
                    (2, 1) => vec![(0, 0), (0, 1), (-1, 0), (-1, 1)],
                    (2, 3) => vec![(0, 0), (0, -1), (-1, 0), (-1, -1)],
                    (3, 2) => vec![(0, 0), (0, 1), (1, 0), (1, 1)],
                    (3, 0) => vec![(0, 0), (0, -1), (1, 0), (1, -1)],
                    (0, 3) => vec![(0, 0), (0, 1), (-1, 0), (-1, 1)],
                    _ => vec![(0, 0)],
                }
            }
        }
    }

    pub fn hold_piece(&mut self) {
        if !self.config.enable_hold {
            return;
        }

        if let Some(current) = self.current_piece.take() {
            if let Some(held) = self.held_piece {
                let new_piece = Tetrimino::new(held);
                self.current_piece = Some(new_piece);
            } else {
                self.spawn_piece();
            }
            self.held_piece = Some(current.kind);
        }
    }

    pub fn is_line_clear_animation_active(&self) -> bool {
        if let Some(ref anim) = self.line_clear_animation {
            let elapsed = anim.start_time.elapsed().as_millis() as u64;
            let duration_ms = anim.total_lines as u64 * 500;
            elapsed < duration_ms
        } else {
            false
        }
    }

    pub fn should_show_cleared_rows(&self) -> bool {
        if let Some(ref anim) = self.line_clear_animation {
            let elapsed = anim.start_time.elapsed().as_millis() as u64;
            let blink_interval = 250;
            let blink_num = elapsed / blink_interval;
            blink_num.is_multiple_of(2)
        } else {
            false
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}

#[cfg(test)]
mod tests {
    use crate::config::GameConfig;
    use crate::tetrimino::{Tetrimino, TetriminoType};

    fn make_test_config(enable_hold: bool) -> GameConfig {
        GameConfig {
            board_width: 10,
            board_height: 20,
            starting_level: 1,
            lines_per_level: 10,
            enable_ghost_piece: false,
            enable_hold,
            enable_variable_goal: false,
            enable_sound: false,
            preview_count: 3,
            das_delay: 250,
            das_repeat: 50,
        }
    }

    #[test]
    fn test_hold_piece_disabled() {
        let config = make_test_config(false);
        let mut state = super::GameState::new(config);

        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.held_piece = Some(TetriminoType::I);

        state.hold_piece();

        assert_eq!(state.current_piece.unwrap().kind, TetriminoType::T);
        assert_eq!(state.held_piece, Some(TetriminoType::I));
    }

    #[test]
    fn test_hold_piece_no_current_piece() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = None;
        state.held_piece = Some(TetriminoType::T);

        state.hold_piece();

        assert_eq!(state.current_piece, None);
        assert_eq!(state.held_piece, Some(TetriminoType::T));
    }

    #[test]
    fn test_hold_piece_no_held_spawns() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        // Force the next piece to be different from T by manipulating next_pieces
        state.next_pieces.clear();
        state.next_pieces.push(TetriminoType::I); // Ensure first is not T
        for _ in 0..4 {
            state.next_pieces.push(TetriminoType::O);
        }

        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.held_piece = None;

        state.hold_piece();

        assert_eq!(state.held_piece, Some(TetriminoType::T));
        assert!(state.current_piece.is_some());
        // The new piece should be from next_pieces (I, not T)
        assert_eq!(state.current_piece.unwrap().kind, TetriminoType::I);
    }

    #[test]
    fn test_hold_piece_with_held_swaps() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.held_piece = Some(TetriminoType::I);
        let original_next_count = state.next_pieces.len();

        state.hold_piece();

        assert_eq!(state.held_piece, Some(TetriminoType::T));
        assert_eq!(state.current_piece.unwrap().kind, TetriminoType::I);
        assert_eq!(state.next_pieces.len(), original_next_count);
    }

    #[test]
    fn test_hold_piece_preserves_board() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.held_piece = Some(TetriminoType::I);
        let original_cells = state.board.cells().clone();

        state.hold_piece();

        assert_eq!(state.board.cells(), &original_cells);
    }

    #[test]
    fn test_hold_piece_next_queue_unchanged_on_swap() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.held_piece = Some(TetriminoType::I);
        let original_next = state.next_pieces.clone();

        state.hold_piece();

        assert_eq!(state.next_pieces, original_next);
    }

    #[test]
    fn test_hold_piece_updates_next_queue_on_spawn() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.held_piece = None;
        let first_next = state.next_pieces[0];

        state.hold_piece();

        assert_ne!(state.next_pieces[0], first_next);
    }

    #[test]
    fn test_hold_piece_resets_position_on_swap() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        let mut piece = Tetrimino::new(TetriminoType::T);
        piece.x = 5;
        piece.y = 10;
        piece.rotation = 2;
        state.current_piece = Some(piece);
        state.held_piece = Some(TetriminoType::I);

        state.hold_piece();

        let new_piece = state.current_piece.unwrap();
        assert_eq!(new_piece.x, 0);
        assert_eq!(new_piece.y, 0);
        assert_eq!(new_piece.rotation, 0);
    }

    #[test]
    fn test_hold_piece_all_types() {
        for held_type in [
            TetriminoType::I,
            TetriminoType::O,
            TetriminoType::T,
            TetriminoType::S,
            TetriminoType::Z,
            TetriminoType::J,
            TetriminoType::L,
        ] {
            for current_type in [
                TetriminoType::I,
                TetriminoType::O,
                TetriminoType::T,
                TetriminoType::S,
                TetriminoType::Z,
                TetriminoType::J,
                TetriminoType::L,
            ] {
                let config = make_test_config(true);
                let mut state = super::GameState::new(config);

                state.current_piece = Some(Tetrimino::new(current_type));
                state.held_piece = Some(held_type);

                state.hold_piece();

                assert_eq!(state.held_piece, Some(current_type));
                assert_eq!(state.current_piece.unwrap().kind, held_type);
            }
        }
    }

    #[test]
    fn test_hold_piece_same_type() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.held_piece = Some(TetriminoType::T);

        state.hold_piece();

        assert_eq!(state.held_piece, Some(TetriminoType::T));
        assert_eq!(state.current_piece.unwrap().kind, TetriminoType::T);
    }

    #[test]
    fn test_game_state_new() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert_eq!(state.board.get_width(), 10);
        assert_eq!(state.board.get_height(), 20);
        assert_eq!(state.score, 0);
        assert_eq!(state.level, 1);
        assert_eq!(state.lines_cleared, 0);
        assert!(!state.game_over);
        assert!(state.current_piece.is_none());
        assert_eq!(state.next_pieces.len(), 3);
        assert!(state.held_piece.is_none());
    }

    #[test]
    fn test_game_state_new_custom_config() {
        let config = GameConfig {
            board_width: 15,
            board_height: 25,
            starting_level: 5,
            lines_per_level: 15,
            enable_ghost_piece: true,
            enable_hold: true,
            enable_variable_goal: true,
            enable_sound: true,
            preview_count: 5,
            das_delay: 200,
            das_repeat: 30,
        };
        let state = super::GameState::new(config);

        assert_eq!(state.board.get_width(), 15);
        assert_eq!(state.board.get_height(), 25);
        assert_eq!(state.level, 5);
        assert_eq!(state.lines_until_next_level, 15);
    }

    #[test]
    fn test_spawn_piece() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        assert!(state.current_piece.is_none());

        state.spawn_piece();

        assert!(state.current_piece.is_some());
        assert_eq!(state.next_pieces.len(), 3);
    }

    #[test]
    fn test_spawn_piece_game_over() {
        let config = make_test_config(true);
        let mut new_state = super::GameState::new(config);

        for y in 0..20 {
            for x in 0..10 {
                new_state.board.cells_mut()[y][x] = Some(TetriminoType::I);
            }
        }

        new_state.spawn_piece();

        assert!(new_state.game_over);
    }

    #[test]
    fn test_move_piece_success() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.spawn_piece();
        let piece_x = state.current_piece.unwrap().x;
        let moved = state.move_piece(1, 0);

        assert!(moved);
        assert_eq!(state.current_piece.unwrap().x, piece_x + 1);
    }

    #[test]
    fn test_move_piece_invalid_position() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = Some(Tetrimino::new(TetriminoType::I));
        state.current_piece.as_mut().unwrap().x = -5;

        let moved = state.move_piece(0, 1);

        assert!(!moved);
    }

    #[test]
    fn test_move_piece_no_current_piece() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = None;
        let moved = state.move_piece(1, 0);

        assert!(!moved);
    }

    #[test]
    fn test_move_piece_collision_with_board() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        let piece = Tetrimino::new(TetriminoType::O);
        state.board.lock_tetromino(&piece);

        state.spawn_piece();
        if state.game_over {
            return;
        }
        if state.current_piece.is_none() {
            return;
        }
        state.current_piece.as_mut().unwrap().x = 5;
        state.current_piece.as_mut().unwrap().y = 0;

        let moved = state.move_piece(0, 1);

        assert!(!moved);
    }

    #[test]
    fn test_rotate_piece_clockwise() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.spawn_piece();
        state.current_piece.as_mut().unwrap().rotation = 0;
        state.rotate_piece(true);

        assert_eq!(state.current_piece.unwrap().rotation, 1);
    }

    #[test]
    fn test_rotate_piece_counter_clockwise() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.spawn_piece();
        state.current_piece.as_mut().unwrap().rotation = 2;
        state.rotate_piece(false);

        assert_eq!(state.current_piece.unwrap().rotation, 1);
    }

    #[test]
    fn test_rotate_piece_no_current_piece() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.current_piece = None;
        state.rotate_piece(true);

        assert_eq!(state.current_piece, None);
    }

    #[test]
    fn test_hard_drop() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.spawn_piece();

        state.hard_drop();

        assert!(state.current_piece.is_some());
    }

    #[test]
    fn test_lock_current_piece() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.spawn_piece();
        state.current_piece.as_mut().unwrap().x = 0;
        state.current_piece.as_mut().unwrap().y = 18;
        let blocks_before = state.current_piece.unwrap().get_blocks();

        state.lock_current_piece();

        assert!(state.current_piece.is_some());
        for (dx, dy) in blocks_before {
            let x = (0 + dx) as usize;
            let y = (18 + dy) as usize;
            assert!(
                state.board.get_cell(x, y).is_some(),
                "Cell ({}, {}) should be filled",
                x,
                y
            );
        }
    }

    #[test]
    fn test_lock_current_piece_triggers_animation() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        for x in 0..10 {
            state.board.cells_mut()[19][x] = Some(TetriminoType::I);
        }

        state.spawn_piece();
        state.current_piece.as_mut().unwrap().x = 0;
        state.current_piece.as_mut().unwrap().y = 18;

        state.lock_current_piece();

        assert!(state.pending_line_clear);
        assert!(state.line_clear_animation.is_some());
        assert_eq!(state.line_clear_animation.as_ref().unwrap().total_lines, 1);
    }

    #[test]
    fn test_complete_line_clear() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        let mut bottom_piece = Tetrimino::new(TetriminoType::I);
        bottom_piece.x = 0;
        bottom_piece.y = 19;
        bottom_piece.rotation = 0;
        state.board.lock_tetromino(&bottom_piece);

        state.pending_line_clear = true;
        state.line_clear_animation = Some(super::LineClearAnimation {
            cleared_rows: vec![19],
            start_time: std::time::Instant::now(),
            total_lines: 1,
        });

        state.complete_line_clear();

        assert!(!state.pending_line_clear);
        assert!(state.line_clear_animation.is_none());
        assert!(state.board.get_full_lines().is_empty());
    }

    #[test]
    fn test_complete_line_clear_no_pending() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.complete_line_clear();

        assert!(!state.pending_line_clear);
    }

    #[test]
    fn test_update_score_zero_lines() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.score = 100;
        state.combo_count = 5;

        state.update_score(0, false);

        assert_eq!(state.score, 100);
        assert_eq!(state.combo_count, 0);
    }

    #[test]
    fn test_compute_awarded_lines_no_tspin() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert_eq!(state.compute_awarded_lines(1, false), 1);
        assert_eq!(state.compute_awarded_lines(2, false), 2);
        assert_eq!(state.compute_awarded_lines(3, false), 3);
        assert_eq!(state.compute_awarded_lines(4, false), 4);
    }

    #[test]
    fn test_compute_awarded_lines_with_tspin() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert_eq!(state.compute_awarded_lines(1, true), 1);
        assert_eq!(state.compute_awarded_lines(2, true), 2);
        assert_eq!(state.compute_awarded_lines(3, true), 3);
    }

    #[test]
    fn test_check_tspin_always_false() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert!(!state.check_tspin());
    }

    #[test]
    fn test_update_level_fixed_goal() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.update_level_fixed_goal(5);

        assert_eq!(state.lines_until_next_level, 5);
        assert_eq!(state.level, 1);
    }

    #[test]
    fn test_update_level_fixed_goal_level_up() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.update_level_fixed_goal(15);

        assert_eq!(state.level, 2);
        assert_eq!(state.lines_until_next_level, 5);
    }

    #[test]
    fn test_update_level_fixed_goal_exact_level_up() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.update_level_fixed_goal(10);

        assert_eq!(state.level, 2);
        assert_eq!(state.lines_until_next_level, 10);
    }

    #[test]
    fn test_update_level_variable_goal() {
        let config = GameConfig {
            enable_variable_goal: true,
            ..make_test_config(true)
        };
        let mut state = super::GameState::new(config);

        state.pieces_placed = 10;
        state.update_level_variable_goal(5, false);

        assert_eq!(state.level, 1);
    }

    #[test]
    fn test_update_level_variable_goal_level_up() {
        let config = GameConfig {
            enable_variable_goal: true,
            ..make_test_config(true)
        };
        let mut state = super::GameState::new(config);

        state.pieces_placed = 10;
        state.update_level_variable_goal(15, false);

        assert_eq!(state.level, 2);
    }

    #[test]
    fn test_get_wall_kicks_i_piece() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        let kicks_0_1 = state.get_wall_kicks(TetriminoType::I, 0, 1);
        assert!(!kicks_0_1.is_empty());
        assert_eq!(kicks_0_1[0], (0, 0));

        let kicks_1_0 = state.get_wall_kicks(TetriminoType::I, 1, 0);
        assert!(!kicks_1_0.is_empty());
    }

    #[test]
    fn test_get_wall_kicks_o_piece() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        let kicks = state.get_wall_kicks(TetriminoType::O, 0, 1);
        assert_eq!(kicks, vec![(0, 0)]);
    }

    #[test]
    fn test_get_wall_kicks_t_piece() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        let kicks_0_1 = state.get_wall_kicks(TetriminoType::T, 0, 1);
        assert_eq!(kicks_0_1, vec![(0, 0), (0, -1), (-1, 0), (-1, -1)]);

        let kicks_1_2 = state.get_wall_kicks(TetriminoType::T, 1, 2);
        assert_eq!(kicks_1_2, vec![(0, 0), (0, -1), (1, 0), (1, -1)]);
    }

    #[test]
    fn test_get_wall_kicks_default() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        let kicks = state.get_wall_kicks(TetriminoType::Z, 2, 2);
        assert_eq!(kicks, vec![(0, 0)]);
    }

    #[test]
    fn test_is_line_clear_animation_active_no_animation() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert!(!state.is_line_clear_animation_active());
    }

    #[test]
    fn test_is_line_clear_animation_active_with_animation() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.line_clear_animation = Some(super::LineClearAnimation {
            cleared_rows: vec![19],
            start_time: std::time::Instant::now(),
            total_lines: 1,
        });

        assert!(state.is_line_clear_animation_active());
    }

    #[test]
    fn test_should_show_cleared_rows_no_animation() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert!(!state.should_show_cleared_rows());
    }

    #[test]
    fn test_should_show_cleared_rows_with_animation() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.line_clear_animation = Some(super::LineClearAnimation {
            cleared_rows: vec![19],
            start_time: std::time::Instant::now(),
            total_lines: 2,
        });

        assert!(state.should_show_cleared_rows());
    }

    #[test]
    fn test_line_clear_animation_struct() {
        let anim = super::LineClearAnimation {
            cleared_rows: vec![18, 19],
            start_time: std::time::Instant::now(),
            total_lines: 2,
        };

        assert_eq!(anim.cleared_rows.len(), 2);
        assert_eq!(anim.total_lines, 2);
    }

    #[test]
    fn test_score_accumulates() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.score = 100;
        state.update_score(1, true);

        assert!(state.score > 100);
    }

    #[test]
    fn test_lines_cleared_accumulates() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.lines_cleared = 5;
        for x in 0..10 {
            state.board.cells_mut()[19][x] = Some(TetriminoType::I);
        }
        state.pending_line_clear = true;
        state.line_clear_animation = Some(super::LineClearAnimation {
            cleared_rows: vec![19],
            start_time: std::time::Instant::now(),
            total_lines: 1,
        });

        state.complete_line_clear();

        assert_eq!(state.lines_cleared, 6);
    }

    #[test]
    fn test_back_to_back_state() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.back_to_back_active = true;
        state.update_score(4, true);

        assert!(state.back_to_back_active);
    }

    #[test]
    fn test_combo_count() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.combo_count = 2;
        state.update_score(2, true);

        assert_eq!(state.combo_count, 3);
    }

    #[test]
    fn test_pieces_placed() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        assert_eq!(state.pieces_placed, 0);

        state.spawn_piece();
        state.lock_current_piece();

        assert_eq!(state.pieces_placed, 1);
    }

    #[test]
    fn test_hold_disabled_returns_early() {
        let config = make_test_config(false);
        let mut state = super::GameState::new(config);

        state.current_piece = Some(Tetrimino::new(TetriminoType::T));
        state.hold_piece();

        assert_eq!(state.current_piece.unwrap().kind, TetriminoType::T);
        assert_eq!(state.held_piece, None);
    }

    #[test]
    fn test_all_tetrimino_types_in_bag() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert!(state.bag.len() < 7);
        assert_eq!(state.next_pieces.len(), 3);
    }

    #[test]
    fn test_next_pieces_populated() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert_eq!(state.next_pieces.len(), 3);
        assert!(!state.current_piece.is_some());
    }

    #[test]
    fn test_spawn_piece_refills_next_pieces() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        let initial_next_len = state.next_pieces.len();
        state.spawn_piece();

        assert_eq!(state.next_pieces.len(), initial_next_len);
    }

    #[test]
    fn test_game_over_flag() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        assert!(!state.game_over);

        state.game_over = true;

        assert!(state.game_over);
    }

    #[test]
    fn test_level_increases_with_lines() {
        let config = make_test_config(true);
        let mut state = super::GameState::new(config);

        state.lines_until_next_level = 2;
        state.update_level_fixed_goal(5);

        assert_eq!(state.level, 2);
    }

    #[test]
    fn test_config_stored_in_state() {
        let config = make_test_config(true);
        let state = super::GameState::new(config);

        assert_eq!(state.config.board_width, 10);
        assert_eq!(state.config.board_height, 20);
        assert_eq!(state.config.preview_count, 3);
    }
}
