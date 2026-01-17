use crate::board::Board;
use crate::config::GameConfig;
use crate::tetrimino::{Tetrimino, TetriminoType};
use rand::rng;
use rand::seq::SliceRandom;

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

            let lines = self.board.clear_lines();
            self.lines_cleared += lines;
            self.update_score(lines, lines > 0);
            self.spawn_piece();
        }
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
        // Ensure we have enough next pieces (typically 3-5 pieces shown)
        let target_count = 5;

        while self.next_pieces.len() < target_count {
            if self.bag.is_empty() {
                self.refill_bag();
            }

            if let Some(piece) = self.bag.pop() {
                self.next_pieces.push(piece);
            } else {
                // This should never happen, but handle it gracefully
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
}
