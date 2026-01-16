#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetriminoType {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

#[derive(Debug, Clone)]
pub struct Tetrimino {
    pub kind: TetriminoType,
    pub x: i32,
    pub y: i32,
    pub rotation: usize,
}

impl Tetrimino {
    pub fn new(kind: TetriminoType) -> Self {
        Self {
            kind,
            x: 0,
            y: 0,
            rotation: 0,
        }
    }

    /// Returns relative block positions for the current rotation
    pub fn get_blocks(&self) -> Vec<(i32, i32)> {
        match (self.kind, self.rotation % 4) {
            // I piece - 4 blocks in a line
            (TetriminoType::I, 0) => vec![(0, 0), (1, 0), (2, 0), (3, 0)],
            (TetriminoType::I, 1) => vec![(2, 0), (2, 1), (2, 2), (2, 3)],
            (TetriminoType::I, 2) => vec![(0, 2), (1, 2), (2, 2), (3, 2)],
            (TetriminoType::I, 3) => vec![(1, 0), (1, 1), (1, 2), (1, 3)],

            // O piece - 2x2 square (no rotation needed)
            (TetriminoType::O, _) => vec![(0, 0), (1, 0), (0, 1), (1, 1)],

            // T piece - T-shape
            (TetriminoType::T, 0) => vec![(1, 0), (0, 1), (1, 1), (2, 1)],
            (TetriminoType::T, 1) => vec![(1, 0), (1, 1), (2, 1), (1, 2)],
            (TetriminoType::T, 2) => vec![(0, 1), (1, 1), (2, 1), (1, 2)],
            (TetriminoType::T, 3) => vec![(1, 0), (0, 1), (1, 1), (1, 2)],

            // S piece - Z-shape (mirrored)
            (TetriminoType::S, 0) => vec![(1, 0), (2, 0), (0, 1), (1, 1)],
            (TetriminoType::S, 1) => vec![(1, 0), (1, 1), (2, 1), (2, 2)],
            (TetriminoType::S, 2) => vec![(1, 1), (2, 1), (0, 2), (1, 2)],
            (TetriminoType::S, 3) => vec![(0, 0), (0, 1), (1, 1), (1, 2)],

            // Z piece - S-shape
            (TetriminoType::Z, 0) => vec![(0, 0), (1, 0), (1, 1), (2, 1)],
            (TetriminoType::Z, 1) => vec![(2, 0), (1, 1), (2, 1), (1, 2)],
            (TetriminoType::Z, 2) => vec![(0, 1), (1, 1), (1, 2), (2, 2)],
            (TetriminoType::Z, 3) => vec![(1, 0), (0, 1), (1, 1), (0, 2)],

            // J piece - L-shape (mirrored)
            (TetriminoType::J, 0) => vec![(0, 0), (0, 1), (1, 1), (2, 1)],
            (TetriminoType::J, 1) => vec![(1, 0), (2, 0), (1, 1), (1, 2)],
            (TetriminoType::J, 2) => vec![(0, 1), (1, 1), (2, 1), (2, 2)],
            (TetriminoType::J, 3) => vec![(1, 0), (1, 1), (0, 2), (1, 2)],

            // L piece - L-shape
            (TetriminoType::L, 0) => vec![(2, 0), (0, 1), (1, 1), (2, 1)],
            (TetriminoType::L, 1) => vec![(1, 0), (1, 1), (1, 2), (2, 2)],
            (TetriminoType::L, 2) => vec![(0, 1), (1, 1), (2, 1), (0, 2)],
            (TetriminoType::L, 3) => vec![(0, 0), (1, 0), (1, 1), (1, 2)],

            _ => unreachable!(),
        }
    }
}
