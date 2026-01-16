#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetriminoType {
    I, O, T, S, Z, J, L
}

#[derive(Debug, Clone)]
pub struct Tetromino {
    pub kind: TetriminoType,
    pub x: i32,
    pub y: i32,
    pub rotation: usize,
}

impl Tetromino {
    pub fn new(kind: TetriminoType) -> Self {
        Self {
            kind,
            x: 0,
            y: 0,
            rotation: 0,
        }
    }

    pub fn get_blocks(&self) -> Vec<(i32, i32)> {
        // Returns relative block positions for the current rotation
        // TODO: Implement rotation matrices for each tetrimino type
        match (self.kind, self.rotation % 4) {
            (TetriminoType::I, 0) => vec![(0, 0), (1, 0), (2, 0), (3, 0)],
            (TetriminoType::O, _) => vec![(0, 0), (1, 0), (0, 1), (1, 1)],
            (TetriminoType::T, 0) => vec![(1, 0), (0, 1), (1, 1), (2, 1)],
            // Add all rotation states for all pieces
            _ => vec![(0, 0), (1, 0), (2, 0), (1, 1)], // Placeholder
        }
    }
}
