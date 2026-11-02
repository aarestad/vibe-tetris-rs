use crate::tetrimino::{Tetrimino, TetriminoType};

/// Rows above the Skyline, used for generation and play above the Matrix (Guideline 10).
pub const BUFFER_ZONE_HEIGHT: usize = 20;

/// The Matrix and the Buffer Zone above it.
///
/// Row 0 is the top row of the Matrix, so the Skyline sits just above it. Negative rows are
/// in the Buffer Zone, and y increases downward.
pub struct Board {
    width: usize,
    height: usize,
    cells: Vec<Vec<Option<TetriminoType>>>,
}

impl Board {
    /// Creates an empty board. `height` counts Matrix rows only.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![vec![None; width]; height + BUFFER_ZONE_HEIGHT],
        }
    }

    /// Storage index of a row, or None if it is outside the board.
    fn row_index(&self, y: i32) -> Option<usize> {
        let index = y + BUFFER_ZONE_HEIGHT as i32;
        (index >= 0 && (index as usize) < self.cells.len()).then_some(index as usize)
    }

    pub fn is_valid_position(&self, tetrimino: &Tetrimino) -> bool {
        tetrimino
            .get_blocks()
            .iter()
            .all(|(dx, dy)| !self.is_occupied(tetrimino.x + dx, tetrimino.y + dy))
    }

    /// True for a cell holding a Block or lying outside the board, such as a wall or the floor.
    pub fn is_occupied(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= self.width as i32 {
            return true;
        }
        match self.row_index(y) {
            Some(row) => self.cells[row][x as usize].is_some(),
            None => true,
        }
    }

    pub fn lock_tetrimino(&mut self, tetrimino: &Tetrimino) {
        for (dx, dy) in tetrimino.get_blocks() {
            let x = tetrimino.x + dx;
            if let Some(row) = self.row_index(tetrimino.y + dy)
                && x >= 0
                && (x as usize) < self.width
            {
                self.cells[row][x as usize] = Some(tetrimino.kind);
            }
        }
    }

    /// Rows that are completely filled, from top to bottom. Buffer Zone rows are negative.
    pub fn get_full_lines(&self) -> Vec<i32> {
        (0..self.cells.len())
            .filter(|&row| self.cells[row].iter().all(|cell| cell.is_some()))
            .map(|row| row as i32 - BUFFER_ZONE_HEIGHT as i32)
            .collect()
    }

    /// Removes filled rows, collapses the rows above and returns how many were removed.
    pub fn clear_lines(&mut self) -> u32 {
        let total_rows = self.cells.len();
        self.cells
            .retain(|row| row.iter().any(|cell| cell.is_none()));

        let cleared = total_rows - self.cells.len();
        for _ in 0..cleared {
            self.cells.insert(0, vec![None; self.width]);
        }
        cleared as u32
    }

    /// The cell at a position. Anything outside the board reads as empty.
    pub fn get_cell(&self, x: usize, y: i32) -> Option<TetriminoType> {
        if x >= self.width {
            return None;
        }
        self.row_index(y).and_then(|row| self.cells[row][x])
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    /// Matrix height, not counting the Buffer Zone.
    pub fn get_height(&self) -> usize {
        self.height
    }

    #[cfg(test)]
    pub fn cells(&self) -> &Vec<Vec<Option<TetriminoType>>> {
        &self.cells
    }

    #[cfg(test)]
    pub fn set_cell(&mut self, x: usize, y: i32, value: Option<TetriminoType>) {
        let row = self.row_index(y).unwrap();
        self.cells[row][x] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tetrimino::{Facing, Tetrimino, TetriminoType};

    fn create_test_piece(kind: TetriminoType, x: i32, y: i32, facing: Facing) -> Tetrimino {
        Tetrimino { kind, x, y, facing }
    }

    #[test]
    fn test_new_standard_size() {
        let board = Board::new(10, 20);
        assert_eq!(board.get_width(), 10);
        assert_eq!(board.get_height(), 20);
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
        let piece = create_test_piece(TetriminoType::I, 0, 0, Facing::North);
        assert!(board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_left_boundary() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, -1, 0, Facing::North);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_right_boundary() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 7, 0, Facing::North);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_top_boundary() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 0, -22, Facing::North);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_bottom_boundary() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 0, 19, Facing::North);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_bottom_boundary_i_piece() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 0, 18, Facing::West);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn test_is_valid_position_collision_with_block() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, Facing::North);
        board.lock_tetrimino(&piece);

        let overlapping_piece = create_test_piece(TetriminoType::O, 5, 10, Facing::North);
        assert!(!board.is_valid_position(&overlapping_piece));
    }

    #[test]
    fn test_is_valid_position_adjacent_no_collision() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, Facing::North);
        board.lock_tetrimino(&piece);

        let adjacent_piece = create_test_piece(TetriminoType::O, 6, 10, Facing::North);
        assert!(board.is_valid_position(&adjacent_piece));
    }

    #[test]
    fn test_is_valid_position_piece_above_block() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, Facing::North);
        board.lock_tetrimino(&piece);

        let above_piece = create_test_piece(TetriminoType::O, 4, 8, Facing::North);
        assert!(board.is_valid_position(&above_piece));
    }

    #[test]
    fn test_lock_tetrimino_o_piece() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, 10, Facing::North);
        board.lock_tetrimino(&piece);

        assert_eq!(board.get_cell(4, 10), Some(TetriminoType::O));
        assert_eq!(board.get_cell(5, 10), Some(TetriminoType::O));
        assert_eq!(board.get_cell(4, 11), Some(TetriminoType::O));
        assert_eq!(board.get_cell(5, 11), Some(TetriminoType::O));
    }

    #[test]
    fn test_lock_tetrimino_i_piece_vertical() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::I, 0, 0, Facing::East);
        board.lock_tetrimino(&piece);

        assert_eq!(board.get_cell(2, 0), Some(TetriminoType::I));
        assert_eq!(board.get_cell(2, 1), Some(TetriminoType::I));
        assert_eq!(board.get_cell(2, 2), Some(TetriminoType::I));
        assert_eq!(board.get_cell(2, 3), Some(TetriminoType::I));
    }

    #[test]
    fn test_lock_tetrimino_l_piece() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::L, 0, 5, Facing::North);
        board.lock_tetrimino(&piece);

        assert_eq!(board.get_cell(2, 5), Some(TetriminoType::L));
        assert_eq!(board.get_cell(0, 6), Some(TetriminoType::L));
        assert_eq!(board.get_cell(1, 6), Some(TetriminoType::L));
        assert_eq!(board.get_cell(2, 6), Some(TetriminoType::L));
    }

    #[test]
    fn test_get_full_lines_no_full_lines() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 0, 0, Facing::North);
        board.lock_tetrimino(&piece);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_get_full_lines_one_full_line() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, 10, Some(TetriminoType::I));
        }
        let full_lines = board.get_full_lines();
        assert_eq!(full_lines.len(), 1);
        assert!(full_lines.contains(&10));
    }

    #[test]
    fn test_get_full_lines_multiple_full_lines() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, 5, Some(TetriminoType::I));
            board.set_cell(x, 10, Some(TetriminoType::O));
            board.set_cell(x, 15, Some(TetriminoType::T));
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
            board.set_cell(x, 10, Some(TetriminoType::I));
        }
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_clear_lines_no_full_lines() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 0, 0, Facing::North);
        board.lock_tetrimino(&piece);
        assert_eq!(board.clear_lines(), 0);
    }

    #[test]
    fn test_clear_lines_one_line() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, 10, Some(TetriminoType::I));
        }
        assert_eq!(board.clear_lines(), 1);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_clear_lines_multiple_lines() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, 5, Some(TetriminoType::I));
            board.set_cell(x, 10, Some(TetriminoType::O));
            board.set_cell(x, 15, Some(TetriminoType::T));
        }
        assert_eq!(board.clear_lines(), 3);
    }

    #[test]
    fn test_clear_lines_blocks_above_move_down() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, 19, Some(TetriminoType::I));
        }
        for x in 0..10 {
            board.set_cell(x, 15, Some(TetriminoType::O));
        }
        assert_eq!(board.clear_lines(), 2);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_clear_lines_non_adjacent() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, 5, Some(TetriminoType::I));
            board.set_cell(x, 19, Some(TetriminoType::O));
        }
        assert_eq!(board.clear_lines(), 2);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_get_cell_in_bounds() {
        let mut board = Board::new(10, 20);
        board.set_cell(3, 5, Some(TetriminoType::T));
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
    fn test_lock_multiple_pieces() {
        let mut board = Board::new(10, 20);
        let piece1 = create_test_piece(TetriminoType::O, 0, 0, Facing::North);
        board.lock_tetrimino(&piece1);
        let piece2 = create_test_piece(TetriminoType::I, 5, 0, Facing::North);
        board.lock_tetrimino(&piece2);

        assert_eq!(board.get_cell(0, 0), Some(TetriminoType::O));
        assert_eq!(board.get_cell(5, 1), Some(TetriminoType::I));
        assert_eq!(board.get_cell(6, 1), Some(TetriminoType::I));
        assert_eq!(board.get_cell(7, 1), Some(TetriminoType::I));
        assert_eq!(board.get_cell(8, 1), Some(TetriminoType::I));
    }

    #[test]
    fn test_clear_lines_after_locking() {
        let mut board = Board::new(10, 20);
        let piece1 = create_test_piece(TetriminoType::O, 0, 17, Facing::North);
        board.lock_tetrimino(&piece1);
        let piece2 = create_test_piece(TetriminoType::O, 5, 17, Facing::North);
        board.lock_tetrimino(&piece2);

        for x in 0..10 {
            board.set_cell(x, 19, Some(TetriminoType::I));
        }

        assert_eq!(board.clear_lines(), 1);
    }

    #[test]
    fn test_is_valid_position_with_rotation() {
        let board = Board::new(10, 20);
        for facing in Facing::ALL {
            let piece = create_test_piece(TetriminoType::T, 4, 5, facing);
            assert!(board.is_valid_position(&piece), "Failed for {:?}", facing);
        }
    }

    #[test]
    fn test_board_dimensions_consistency() {
        let board = Board::new(10, 20);
        let cells = board.cells();
        assert_eq!(cells.len(), board.get_height() + BUFFER_ZONE_HEIGHT);
        for row in cells.iter() {
            assert_eq!(row.len(), board.get_width());
        }
    }

    #[test]
    fn test_clear_lines_skips_non_full_lines() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, 5, Some(TetriminoType::I));
        }
        board.set_cell(9, 10, None);
        assert_eq!(board.clear_lines(), 1);
    }

    #[test]
    fn test_top_matrix_row_clears() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, 0, Some(TetriminoType::I));
        }
        assert_eq!(board.get_full_lines(), vec![0]);
        assert_eq!(board.clear_lines(), 1);
        assert!(board.get_full_lines().is_empty());
    }

    #[test]
    fn test_full_row_in_buffer_zone_clears() {
        let mut board = Board::new(10, 20);
        for x in 0..10 {
            board.set_cell(x, -1, Some(TetriminoType::I));
        }
        assert_eq!(board.get_full_lines(), vec![-1]);
        assert_eq!(board.clear_lines(), 1);
    }

    #[test]
    fn test_clear_collapses_rows_by_number_cleared() {
        let mut board = Board::new(10, 20);
        board.set_cell(3, 10, Some(TetriminoType::T));
        for x in 0..10 {
            board.set_cell(x, 11, Some(TetriminoType::I));
            board.set_cell(x, 13, Some(TetriminoType::I));
        }
        board.set_cell(7, 12, Some(TetriminoType::S));
        assert_eq!(board.clear_lines(), 2);
        assert_eq!(board.get_cell(3, 12), Some(TetriminoType::T));
        assert_eq!(board.get_cell(7, 13), Some(TetriminoType::S));
        assert_eq!(board.cells().len(), 40);
    }

    #[test]
    fn test_pieces_are_valid_in_buffer_zone() {
        let board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::T, 3, -20, Facing::North);
        assert!(board.is_valid_position(&piece));
    }

    #[test]
    fn test_blocks_can_lock_in_buffer_zone() {
        let mut board = Board::new(10, 20);
        let piece = create_test_piece(TetriminoType::O, 4, -2, Facing::North);
        board.lock_tetrimino(&piece);
        assert_eq!(board.get_cell(4, -2), Some(TetriminoType::O));
        assert!(board.is_occupied(4, -1));
    }

    #[test]
    fn test_is_occupied_treats_walls_and_floor_as_occupied() {
        let board = Board::new(10, 20);
        assert!(board.is_occupied(-1, 5));
        assert!(board.is_occupied(10, 5));
        assert!(board.is_occupied(5, 20));
        assert!(board.is_occupied(5, -21));
        assert!(!board.is_occupied(5, 5));
        assert!(!board.is_occupied(5, -20));
    }
}
