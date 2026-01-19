use crate::tetrimino::{Tetrimino, TetriminoType};

pub struct Board {
    width: usize,
    height: usize,
    cells: Vec<Vec<Option<TetriminoType>>>,
}

impl Board {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![vec![None; width]; height],
        }
    }

    pub fn is_valid_position(&self, tetromino: &Tetrimino) -> bool {
        for (dx, dy) in tetromino.get_blocks() {
            let x = tetromino.x + dx;
            let y = tetromino.y + dy;

            if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
                return false;
            }

            if self.cells[y as usize][x as usize].is_some() {
                return false;
            }
        }
        true
    }

    pub fn lock_tetromino(&mut self, tetromino: &Tetrimino) {
        for (dx, dy) in tetromino.get_blocks() {
            let x = (tetromino.x + dx) as usize;
            let y = (tetromino.y + dy) as usize;
            self.cells[y][x] = Some(tetromino.kind);
        }
    }

    pub fn get_full_lines(&self) -> Vec<usize> {
        let mut full_lines = Vec::new();
        for y in 0..self.height {
            if self.cells[y].iter().all(|cell| cell.is_some()) {
                full_lines.push(y);
            }
        }
        full_lines
    }

    pub fn clear_lines(&mut self) -> u32 {
        let mut lines_cleared = 0;
        let mut y = self.height - 1;

        while y > 0 {
            if self.cells[y].iter().all(|cell| cell.is_some()) {
                self.cells.remove(y);
                self.cells.insert(0, vec![None; self.width]);
                lines_cleared += 1;
            } else {
                y -= 1;
            }
        }

        lines_cleared
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<TetriminoType> {
        if y < self.height && x < self.width {
            self.cells[y][x]
        } else {
            None
        }
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    #[cfg(test)]
    pub fn cells(&self) -> &Vec<Vec<Option<TetriminoType>>> {
        &self.cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tetrimino::{Tetrimino, TetriminoType};

    fn create_test_piece(kind: TetriminoType, x: i32, y: i32, rotation: usize) -> Tetrimino {
        Tetrimino {
            kind,
            x,
            y,
            rotation,
        }
    }

    #[test]
    fn test_new_standard_size() {
        let board = Board::new(10, 20);
        assert_eq!(board.get_width(), 10);
        assert_eq!(board.get_height(), 20);
    }

    #[test]
    fn test_new_small_board() {
        let board = Board::new(4, 4);
        assert_eq!(board.get_width(), 4);
        assert_eq!(board.get_height(), 4);
    }

    #[test]
    fn test_new_large_board() {
        let board = Board::new(30, 40);
        assert_eq!(board.get_width(), 30);
        assert_eq!(board.get_height(), 40);
    }

    #[test]
    fn test_new_all_cells_empty() {
        let board = Board::new(10, 20);
        for y in 0..20 {
            for x in 0..10 {
                assert_eq!(board.get_cell(x, y), None);
            }
        }
    }

    #[test]
    fn test_is_valid_position_empty_board() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 0, 0, 0);
        assert!(board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_o_piece() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, 0);
        assert!(board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_t_piece() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::T, 5, 5, 0);
        assert!(board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_left_boundary() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, -1, 0, 0);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_right_boundary() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 7, 0, 0);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_top_boundary() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 0, -1, 0);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_bottom_boundary() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 0, 19, 0);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_bottom_boundary_i_piece() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 0, 18, 3);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_collision_with_block() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, 0);
        board.lock_tetromino(&piece);

        let overlapping_piece = create_test_piece(TetriminoType::O, 5, 10, 0);
        assert!(!board.is_valid_position(&overlapping_piece));
    }

    #[test]
    fn test_is_valid_position_adjacent_no_collision() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, 0);
        board.lock_tetromino(&piece);

        let adjacent_piece = create_test_piece(TetriminoType::O, 6, 10, 0);
        assert!(board.is_valid_position(&adjacent_piece));
    }

    #[test]
    fn test_is_valid_position_piece_above_block() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, 0);
        board.lock_tetromino(&piece);

        let above_piece = create_test_piece(TetriminoType::O, 4, 8, 0);
        assert!(board.is_valid_position(&above_piece));
    }

    #[test]
    fn test_lock_tetromino_o_piece() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, 0);
        board.lock_tetromino(&piece);

        assert_eq!(board.get_cell(4, 10), Some(TetriminoType::O));
        assert_eq!(board.get_cell(5, 10), Some(TetriminoType::O));
        assert_eq!(board.get_cell(4, 11), Some(TetriminoType::O));
        assert_eq!(board.get_cell(5, 11), Some(TetriminoType::O));
    }

    #[test]
    fn test_lock_tetromino_i_piece_vertical() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 0, 0, 1);
        board.lock_tetromino(&piece);

        assert_eq!(board.get_cell(2, 0), Some(TetriminoType::I));
        assert_eq!(board.get_cell(2, 1), Some(TetriminoType::I));
        assert_eq!(board.get_cell(2, 2), Some(TetriminoType::I));
        assert_eq!(board.get_cell(2, 3), Some(TetriminoType::I));
    }

    #[test]
    fn test_lock_tetromino_l_piece() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::L, 0, 5, 0);
        board.lock_tetromino(&piece);

        assert_eq!(board.get_cell(2, 5), Some(TetriminoType::L));
        assert_eq!(board.get_cell(0, 6), Some(TetriminoType::L));
        assert_eq!(board.get_cell(1, 6), Some(TetriminoType::L));
        assert_eq!(board.get_cell(2, 6), Some(TetriminoType::L));
    }

    #[test]
    fn test_get_full_lines_empty_board() {
        let board = Board::new(10, 20);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_get_full_lines_no_full_lines() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 0, 0, 0);
        board.lock_tetromino(&piece);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_get_full_lines_one_full_line() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.cells[10][x] = Some(TetriminoType::I);
        }
        let full_lines = board.get_full_lines();
        assert_eq!(full_lines.len(), 1);
        assert!(full_lines.contains(&10));
    }

    #[test]
    fn test_get_full_lines_multiple_full_lines() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.cells[5][x] = Some(TetriminoType::I);
            board.cells[10][x] = Some(TetriminoType::O);
            board.cells[15][x] = Some(TetriminoType::T);
        }
        let full_lines = board.get_full_lines();
        assert_eq!(full_lines.len(), 3);
        assert!(full_lines.contains(&5));
        assert!(full_lines.contains(&10));
        assert!(full_lines.contains(&15));
    }

    #[test]
    fn test_get_full_lines_partial_line() {
        let mut board = Board::new(10, 20);
        for x in 0..9 {
            board.cells[10][x] = Some(TetriminoType::I);
        }
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_clear_lines_empty_board() {
        let mut board = Board::new(10, 20);
        assert_eq!(board.clear_lines(), 0);
    }

    #[test]
    fn test_clear_lines_no_full_lines() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 0, 0, 0);
        board.lock_tetromino(&piece);
        assert_eq!(board.clear_lines(), 0);
    }

    #[test]
    fn test_clear_lines_one_line() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.cells[10][x] = Some(TetriminoType::I);
        }
        assert_eq!(board.clear_lines(), 1);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_clear_lines_multiple_lines() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.cells[5][x] = Some(TetriminoType::I);
            board.cells[10][x] = Some(TetriminoType::O);
            board.cells[15][x] = Some(TetriminoType::T);
        }
        assert_eq!(board.clear_lines(), 3);
    }

    #[test]
    fn test_clear_lines_blocks_above_move_down() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.cells[19][x] = Some(TetriminoType::I);
        }
        for x in 0..10 {
            board.cells[15][x] = Some(TetriminoType::O);
        }
        assert_eq!(board.clear_lines(), 2);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_clear_lines_non_adjacent() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.cells[5][x] = Some(TetriminoType::I);
            board.cells[19][x] = Some(TetriminoType::O);
        }
        assert_eq!(board.clear_lines(), 2);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_get_cell_in_bounds() {
        let mut board = Board::new(10, 20);
        board.cells[5][3] = Some(TetriminoType::T);
        assert_eq!(board.get_cell(3, 5), Some(TetriminoType::T));
    }

    #[test]
    fn test_get_cell_out_of_bounds_x() {
        let board = Board::new(10, 20);
        assert_eq!(board.get_cell(10, 5), None);
        assert_eq!(board.get_cell(100, 5), None);
    }

    #[test]
    fn test_get_cell_out_of_bounds_y() {
        let board = Board::new(10, 20);
        assert_eq!(board.get_cell(5, 20), None);
        assert_eq!(board.get_cell(5, 100), None);
    }

    #[test]
    fn test_get_cell_out_of_bounds_both() {
        let board = Board::new(10, 20);
        assert_eq!(board.get_cell(100, 100), None);
    }

    #[test]
    fn test_get_cell_corner() {
        let board = Board::new(10, 20);
        assert_eq!(board.get_cell(0, 0), None);
        assert_eq!(board.get_cell(9, 19), None);
    }

    #[test]
    fn test_get_width() {
        let board = Board::new(15, 25);
        assert_eq!(board.get_width(), 15);
    }

    #[test]
    fn test_get_height() {
        let board = Board::new(15, 25);
        assert_eq!(board.get_height(), 25);
    }

    #[test]
    fn test_cells_method() {
        let board = Board::new(10, 20);
        let cells = board.cells();
        assert_eq!(cells.len(), 20);
        assert_eq!(cells[0].len(), 10);
    }

    #[test]
    fn test_lock_multiple_pieces() {
        let mut board = Board::new(10, 20);
        let piece1 = create_test_piece(TetriminoType::O, 0, 0, 0);
        board.lock_tetromino(&piece1);
        let piece2 = create_test_piece(TetriminoType::I, 5, 0, 0);
        board.lock_tetromino(&piece2);

        assert_eq!(board.get_cell(0, 0), Some(TetriminoType::O));
        assert_eq!(board.get_cell(5, 0), Some(TetriminoType::I));
        assert_eq!(board.get_cell(6, 0), Some(TetriminoType::I));
        assert_eq!(board.get_cell(7, 0), Some(TetriminoType::I));
        assert_eq!(board.get_cell(8, 0), Some(TetriminoType::I));
    }

    #[test]
    fn test_clear_lines_after_locking() {
        let mut board = Board::new(10, 20);
        let piece1 = create_test_piece(TetriminoType::O, 0, 17, 0);
        board.lock_tetromino(&piece1);
        let piece2 = create_test_piece(TetriminoType::O, 5, 17, 0);
        board.lock_tetromino(&piece2);

        for x in 0..10 {
            board.cells[19][x] = Some(TetriminoType::I);
        }

        assert_eq!(board.clear_lines(), 1);
    }

    #[test]
    fn test_is_valid_position_with_rotation() {
        let board = Board::new(10, 20);
        for rotation in 0..4 {
            let piece = create_test_piece(TetriminoType::T, 4, 5, rotation);
            assert!(
                board.is_valid_position(&piece),
                "Failed for rotation {}",
                rotation
            );
        }
    }

    #[test]
    fn test_lock_tetromino_all_types() {
        let mut board = Board::new(10, 20);
        for kind in [
            TetriminoType::I,
            TetriminoType::O,
            TetriminoType::T,
            TetriminoType::S,
            TetriminoType::Z,
            TetriminoType::J,
            TetriminoType::L,
        ] {
            let piece = create_test_piece(kind, 0, 0, 0);
            board.lock_tetromino(&piece);
        }
    }

    #[test]
    fn test_board_dimensions_consistency() {
        let board = Board::new(10, 20);
        let cells = board.cells();
        assert_eq!(cells.len(), board.get_height());
        for row in cells.iter() {
            assert_eq!(row.len(), board.get_width());
        }
    }

    #[test]
    fn test_clear_lines_skips_non_full_lines() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.cells[5][x] = Some(TetriminoType::I);
        }
        board.cells[10][9] = None;
        assert_eq!(board.clear_lines(), 1);
    }
}
