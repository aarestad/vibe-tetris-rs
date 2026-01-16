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
}

impl GameState {
    pub fn new(config: GameConfig) -> Self {
        let mut game_state = Self {
            board: Board::new(config.board_width, config.board_height),
            current_piece: None,
            held_piece: None,
            next_pieces: Vec::new(),
            score: 0,
            level: config.starting_level,
            lines_cleared: 0,
            game_over: false,
            config,
            bag: Vec::new(),
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
        if let Some(_) = self.current_piece.as_ref().map(|p| p.kind) {
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
                .is_valid_position(&self.current_piece.as_ref().unwrap())
            {
                let kicks = self.get_wall_kicks(piece_type, old_rotation, new_rotation, clockwise);
                let mut kicked = false;

                for (dx, dy) in kicks {
                    {
                        let piece = self.current_piece.as_mut().unwrap();
                        piece.x = old_x + dx;
                        piece.y = old_y + dy;
                    }

                    if self
                        .board
                        .is_valid_position(&self.current_piece.as_ref().unwrap())
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
            let lines = self.board.clear_lines();
            self.lines_cleared += lines;
            self.update_score(lines);
            self.spawn_piece();
        }
    }

    fn update_score(&mut self, lines: u32) {
        // Simple scoring system
        let base_score = match lines {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };
        self.score += base_score * self.level as u64;
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
        clockwise: bool,
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
