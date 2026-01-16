use crate::board::Board;
use crate::config::GameConfig;
use crate::tetrimino::{Tetrimino, TetriminoType};

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
}

impl GameState {
    pub fn new(config: GameConfig) -> Self {
        Self {
            board: Board::new(config.board_width, config.board_height),
            current_piece: None,
            held_piece: None,
            next_pieces: Vec::new(),
            score: 0,
            level: config.starting_level,
            lines_cleared: 0,
            game_over: false,
            config,
        }
    }

    pub fn spawn_piece(&mut self) {
        // TODO: Implement piece bag randomization
        let piece = Tetrimino::new(TetriminoType::T);
        self.current_piece = Some(piece);
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
        if let Some(ref mut piece) = self.current_piece {
            let old_rotation = piece.rotation;
            piece.rotation = if clockwise {
                piece.rotation + 1
            } else {
                piece.rotation.wrapping_sub(1)
            };

            if !self.board.is_valid_position(piece) {
                // TODO: Implement wall kick system
                piece.rotation = old_rotation;
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
