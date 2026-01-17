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
