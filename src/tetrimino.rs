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

#[derive(Debug, Clone, Copy, PartialEq)]
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

    /// A Tetrimino at its generation position: North Facing, on the two rows just above the
    /// Skyline (y = -2 and -1), and centered (Guideline 3.4).
    pub fn at_generation(kind: TetriminoType, board_width: usize) -> Self {
        let piece_width = match kind {
            TetriminoType::I => 4,
            TetriminoType::O => 2,
            _ => 3,
        };
        Self {
            kind,
            x: (board_width as i32 - piece_width) / 2,
            y: -2,
            rotation: 0,
        }
    }

    /// Returns relative block positions for the current rotation
    pub fn get_blocks(&self) -> Vec<(i32, i32)> {
        match (self.kind, self.rotation % 4) {
            // I piece - 4 blocks in a line
            (TetriminoType::I, 0) => vec![(0, 1), (1, 1), (2, 1), (3, 1)],
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

/// Super Rotation System offsets to try for a rotation between two facings (Guideline A1.4),
/// as (rotation point, (dx, dy)) in the order tried. Facings are 0 to 3 for North, East, South
/// and West. A positive dy moves down, so the sign is the reverse of a y-up table.
pub fn rotation_points(kind: TetriminoType, from: usize, to: usize) -> Vec<(u8, (i32, i32))> {
    let offsets: [(i32, i32); 5] = match kind {
        TetriminoType::O => return vec![(1, (0, 0))],
        TetriminoType::I => match (from, to) {
            (0, 1) | (3, 2) => [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
            (1, 0) | (2, 3) => [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
            (1, 2) | (0, 3) => [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
            (2, 1) | (3, 0) => [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
            _ => return vec![(1, (0, 0))],
        },
        _ => match (from, to) {
            (0, 1) | (2, 1) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
            (1, 0) | (1, 2) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
            (2, 3) | (0, 3) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            (3, 2) | (3, 0) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
            _ => return vec![(1, (0, 0))],
        },
    };

    (1u8..)
        .zip(offsets)
        .filter(|(point, _)| !is_unused_point(kind, from, *point))
        .collect()
}

/// The Guideline marks these T-Tetrimino points as "Not Used" (pages 49 and 51).
fn is_unused_point(kind: TetriminoType, from: usize, point: u8) -> bool {
    kind == TetriminoType::T && matches!((from, point), (0, 4) | (2, 3))
}

#[cfg(test)]
mod tests {
    use super::*;

    const THREE_WIDE: [TetriminoType; 5] = [
        TetriminoType::T,
        TetriminoType::S,
        TetriminoType::Z,
        TetriminoType::J,
        TetriminoType::L,
    ];
    const ROTATIONS: [(usize, usize); 8] = [
        (0, 1),
        (1, 0),
        (1, 2),
        (2, 1),
        (2, 3),
        (3, 2),
        (3, 0),
        (0, 3),
    ];

    fn offsets(kind: TetriminoType, from: usize, to: usize) -> Vec<(i32, i32)> {
        rotation_points(kind, from, to)
            .into_iter()
            .map(|(_, offset)| offset)
            .collect()
    }

    #[test]
    fn test_i_rotation_points_match_guideline_pages_41_to_44() {
        // Each diagram read as: right is +x and down is +y.
        assert_eq!(
            offsets(TetriminoType::I, 0, 1),
            [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)]
        );
        assert_eq!(
            offsets(TetriminoType::I, 0, 3),
            [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)]
        );
        assert_eq!(
            offsets(TetriminoType::I, 1, 0),
            [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)]
        );
        assert_eq!(
            offsets(TetriminoType::I, 1, 2),
            [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)]
        );
        assert_eq!(
            offsets(TetriminoType::I, 2, 1),
            [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)]
        );
        assert_eq!(
            offsets(TetriminoType::I, 2, 3),
            [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)]
        );
        assert_eq!(
            offsets(TetriminoType::I, 3, 2),
            [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)]
        );
        assert_eq!(
            offsets(TetriminoType::I, 3, 0),
            [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)]
        );
    }

    #[test]
    fn test_t_rotation_points_match_guideline_pages_49_to_52() {
        let t = TetriminoType::T;
        assert_eq!(offsets(t, 0, 1), [(0, 0), (-1, 0), (-1, -1), (-1, 2)]);
        assert_eq!(offsets(t, 0, 3), [(0, 0), (1, 0), (1, -1), (1, 2)]);
        assert_eq!(offsets(t, 1, 0), [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)]);
        assert_eq!(offsets(t, 1, 2), [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)]);
        assert_eq!(offsets(t, 2, 1), [(0, 0), (-1, 0), (0, 2), (-1, 2)]);
        assert_eq!(offsets(t, 2, 3), [(0, 0), (1, 0), (0, 2), (1, 2)]);
        assert_eq!(
            offsets(t, 3, 2),
            [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)]
        );
        assert_eq!(
            offsets(t, 3, 0),
            [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)]
        );
    }

    #[test]
    fn test_t_point_numbers_skip_unused_points() {
        let north: Vec<u8> = rotation_points(TetriminoType::T, 0, 1)
            .into_iter()
            .map(|(point, _)| point)
            .collect();
        assert_eq!(north, [1, 2, 3, 5]);
        let south: Vec<u8> = rotation_points(TetriminoType::T, 2, 3)
            .into_iter()
            .map(|(point, _)| point)
            .collect();
        assert_eq!(south, [1, 2, 4, 5]);
    }

    #[test]
    fn test_non_t_three_wide_pieces_use_all_five_points() {
        for kind in THREE_WIDE.into_iter().filter(|k| *k != TetriminoType::T) {
            for (from, to) in ROTATIONS {
                assert_eq!(rotation_points(kind, from, to).len(), 5, "{kind:?}");
            }
        }
    }

    #[test]
    fn test_first_point_is_always_visual_rotation() {
        for kind in THREE_WIDE
            .into_iter()
            .chain([TetriminoType::I, TetriminoType::O])
        {
            for (from, to) in ROTATIONS {
                assert_eq!(rotation_points(kind, from, to)[0], (1, (0, 0)), "{kind:?}");
            }
        }
    }

    #[test]
    fn test_o_has_one_rotation_point() {
        assert_eq!(rotation_points(TetriminoType::O, 0, 1).len(), 1);
    }

    #[test]
    fn test_i_rotates_within_its_4x4_box() {
        // Guideline A1.3: the I occupies row 1, column 2, row 2 and column 1 in turn.
        let blocks = |rotation| {
            Tetrimino {
                kind: TetriminoType::I,
                x: 0,
                y: 0,
                rotation,
            }
            .get_blocks()
        };
        assert!(blocks(0).iter().all(|(_, y)| *y == 1));
        assert!(blocks(1).iter().all(|(x, _)| *x == 2));
        assert!(blocks(2).iter().all(|(_, y)| *y == 2));
        assert!(blocks(3).iter().all(|(x, _)| *x == 1));
    }

    #[test]
    fn test_generation_x_is_centered_for_each_width() {
        assert_eq!(Tetrimino::at_generation(TetriminoType::I, 10).x, 3);
        assert_eq!(Tetrimino::at_generation(TetriminoType::O, 10).x, 4);
        for kind in THREE_WIDE {
            assert_eq!(Tetrimino::at_generation(kind, 10).x, 3, "{kind:?}");
        }
    }

    #[test]
    fn test_tetrimino_type_variants() {
        assert_eq!(TetriminoType::I, TetriminoType::I);
        assert_eq!(TetriminoType::O, TetriminoType::O);
        assert_eq!(TetriminoType::T, TetriminoType::T);
        assert_eq!(TetriminoType::S, TetriminoType::S);
        assert_eq!(TetriminoType::Z, TetriminoType::Z);
        assert_eq!(TetriminoType::J, TetriminoType::J);
        assert_eq!(TetriminoType::L, TetriminoType::L);
    }

    #[test]
    fn test_tetrimino_type_inequality() {
        assert_ne!(TetriminoType::I, TetriminoType::O);
        assert_ne!(TetriminoType::T, TetriminoType::S);
        assert_ne!(TetriminoType::Z, TetriminoType::J);
        assert_ne!(TetriminoType::L, TetriminoType::I);
    }

    #[test]
    fn test_tetrimino_new_i() {
        let piece = Tetrimino::new(TetriminoType::I);
        assert_eq!(piece.kind, TetriminoType::I);
        assert_eq!(piece.x, 0);
        assert_eq!(piece.y, 0);
        assert_eq!(piece.rotation, 0);
    }

    #[test]
    fn test_tetrimino_new_o() {
        let piece = Tetrimino::new(TetriminoType::O);
        assert_eq!(piece.kind, TetriminoType::O);
        assert_eq!(piece.x, 0);
        assert_eq!(piece.y, 0);
        assert_eq!(piece.rotation, 0);
    }

    #[test]
    fn test_tetrimino_new_t() {
        let piece = Tetrimino::new(TetriminoType::T);
        assert_eq!(piece.kind, TetriminoType::T);
        assert_eq!(piece.x, 0);
        assert_eq!(piece.y, 0);
        assert_eq!(piece.rotation, 0);
    }

    #[test]
    fn test_tetrimino_new_s() {
        let piece = Tetrimino::new(TetriminoType::S);
        assert_eq!(piece.kind, TetriminoType::S);
        assert_eq!(piece.x, 0);
        assert_eq!(piece.y, 0);
        assert_eq!(piece.rotation, 0);
    }

    #[test]
    fn test_tetrimino_new_z() {
        let piece = Tetrimino::new(TetriminoType::Z);
        assert_eq!(piece.kind, TetriminoType::Z);
        assert_eq!(piece.x, 0);
        assert_eq!(piece.y, 0);
        assert_eq!(piece.rotation, 0);
    }

    #[test]
    fn test_tetrimino_new_j() {
        let piece = Tetrimino::new(TetriminoType::J);
        assert_eq!(piece.kind, TetriminoType::J);
        assert_eq!(piece.x, 0);
        assert_eq!(piece.y, 0);
        assert_eq!(piece.rotation, 0);
    }

    #[test]
    fn test_tetrimino_new_l() {
        let piece = Tetrimino::new(TetriminoType::L);
        assert_eq!(piece.kind, TetriminoType::L);
        assert_eq!(piece.x, 0);
        assert_eq!(piece.y, 0);
        assert_eq!(piece.rotation, 0);
    }

    #[test]
    fn test_tetrimino_get_blocks_i_rotation_0() {
        let piece = Tetrimino::new(TetriminoType::I);
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 1), (1, 1), (2, 1), (3, 1)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_i_rotation_1() {
        let mut piece = Tetrimino::new(TetriminoType::I);
        piece.rotation = 1;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(2, 0), (2, 1), (2, 2), (2, 3)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_i_rotation_2() {
        let mut piece = Tetrimino::new(TetriminoType::I);
        piece.rotation = 2;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 2), (1, 2), (2, 2), (3, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_i_rotation_3() {
        let mut piece = Tetrimino::new(TetriminoType::I);
        piece.rotation = 3;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (1, 1), (1, 2), (1, 3)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_o_all_rotations() {
        let rotations = [0, 1, 2, 3, 4, 5, 6, 7];
        let expected = vec![(0, 0), (1, 0), (0, 1), (1, 1)];
        for r in rotations {
            let mut piece = Tetrimino::new(TetriminoType::O);
            piece.rotation = r;
            let blocks = piece.get_blocks();
            assert_eq!(blocks, expected, "Failed for rotation {}", r);
        }
    }

    #[test]
    fn test_tetrimino_get_blocks_t_rotation_0() {
        let piece = Tetrimino::new(TetriminoType::T);
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_t_rotation_1() {
        let mut piece = Tetrimino::new(TetriminoType::T);
        piece.rotation = 1;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (1, 1), (2, 1), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_t_rotation_2() {
        let mut piece = Tetrimino::new(TetriminoType::T);
        piece.rotation = 2;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 1), (1, 1), (2, 1), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_t_rotation_3() {
        let mut piece = Tetrimino::new(TetriminoType::T);
        piece.rotation = 3;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (0, 1), (1, 1), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_s_rotation_0() {
        let mut piece = Tetrimino::new(TetriminoType::S);
        piece.rotation = 0;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (2, 0), (0, 1), (1, 1)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_s_rotation_1() {
        let mut piece = Tetrimino::new(TetriminoType::S);
        piece.rotation = 1;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (1, 1), (2, 1), (2, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_s_rotation_2() {
        let mut piece = Tetrimino::new(TetriminoType::S);
        piece.rotation = 2;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 1), (2, 1), (0, 2), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_s_rotation_3() {
        let mut piece = Tetrimino::new(TetriminoType::S);
        piece.rotation = 3;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 0), (0, 1), (1, 1), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_z_rotation_0() {
        let mut piece = Tetrimino::new(TetriminoType::Z);
        piece.rotation = 0;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 0), (1, 0), (1, 1), (2, 1)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_z_rotation_1() {
        let mut piece = Tetrimino::new(TetriminoType::Z);
        piece.rotation = 1;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(2, 0), (1, 1), (2, 1), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_z_rotation_2() {
        let mut piece = Tetrimino::new(TetriminoType::Z);
        piece.rotation = 2;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 1), (1, 1), (1, 2), (2, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_z_rotation_3() {
        let mut piece = Tetrimino::new(TetriminoType::Z);
        piece.rotation = 3;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (0, 1), (1, 1), (0, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_j_rotation_0() {
        let mut piece = Tetrimino::new(TetriminoType::J);
        piece.rotation = 0;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 0), (0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_j_rotation_1() {
        let mut piece = Tetrimino::new(TetriminoType::J);
        piece.rotation = 1;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (2, 0), (1, 1), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_j_rotation_2() {
        let mut piece = Tetrimino::new(TetriminoType::J);
        piece.rotation = 2;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 1), (1, 1), (2, 1), (2, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_j_rotation_3() {
        let mut piece = Tetrimino::new(TetriminoType::J);
        piece.rotation = 3;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (1, 1), (0, 2), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_l_rotation_0() {
        let mut piece = Tetrimino::new(TetriminoType::L);
        piece.rotation = 0;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(2, 0), (0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_l_rotation_1() {
        let mut piece = Tetrimino::new(TetriminoType::L);
        piece.rotation = 1;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(1, 0), (1, 1), (1, 2), (2, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_l_rotation_2() {
        let mut piece = Tetrimino::new(TetriminoType::L);
        piece.rotation = 2;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 1), (1, 1), (2, 1), (0, 2)]);
    }

    #[test]
    fn test_tetrimino_get_blocks_l_rotation_3() {
        let mut piece = Tetrimino::new(TetriminoType::L);
        piece.rotation = 3;
        let blocks = piece.get_blocks();
        assert_eq!(blocks, vec![(0, 0), (1, 0), (1, 1), (1, 2)]);
    }

    #[test]
    fn test_tetrimino_rotation_wraps_at_4() {
        let mut piece = Tetrimino::new(TetriminoType::T);
        piece.rotation = 4;
        let blocks_0 = piece.get_blocks();
        piece.rotation = 0;
        let blocks_4 = piece.get_blocks();
        assert_eq!(blocks_0, blocks_4);
    }

    #[test]
    fn test_tetrimino_rotation_wraps_at_5() {
        let mut piece = Tetrimino::new(TetriminoType::I);
        piece.rotation = 5;
        let blocks_1 = piece.get_blocks();
        piece.rotation = 1;
        let blocks_5 = piece.get_blocks();
        assert_eq!(blocks_1, blocks_5);
    }

    #[test]
    fn test_tetrimino_rotation_wraps_at_6() {
        let mut piece = Tetrimino::new(TetriminoType::S);
        piece.rotation = 6;
        let blocks_2 = piece.get_blocks();
        piece.rotation = 2;
        let blocks_6 = piece.get_blocks();
        assert_eq!(blocks_2, blocks_6);
    }

    #[test]
    fn test_tetrimino_rotation_wraps_at_7() {
        let mut piece = Tetrimino::new(TetriminoType::Z);
        piece.rotation = 7;
        let blocks_3 = piece.get_blocks();
        piece.rotation = 3;
        let blocks_7 = piece.get_blocks();
        assert_eq!(blocks_3, blocks_7);
    }

    #[test]
    fn test_tetrimino_all_pieces_have_4_blocks() {
        for kind in [
            TetriminoType::I,
            TetriminoType::O,
            TetriminoType::T,
            TetriminoType::S,
            TetriminoType::Z,
            TetriminoType::J,
            TetriminoType::L,
        ] {
            let piece = Tetrimino::new(kind);
            let blocks = piece.get_blocks();
            assert_eq!(blocks.len(), 4, "Failed for {:?}", kind);
        }
    }

    #[test]
    fn test_tetrimino_copy() {
        let piece = Tetrimino::new(TetriminoType::L);
        let copied = piece;
        assert_eq!(piece.kind, copied.kind);
        assert_eq!(piece.x, copied.x);
        assert_eq!(piece.y, copied.y);
        assert_eq!(piece.rotation, copied.rotation);
    }

    #[test]
    fn test_tetrimino_partial_eq() {
        let piece1 = Tetrimino::new(TetriminoType::J);
        let piece2 = Tetrimino::new(TetriminoType::J);
        assert_eq!(piece1, piece2);

        let mut piece3 = Tetrimino::new(TetriminoType::J);
        piece3.rotation = 2;
        assert_ne!(piece1, piece3);
    }
}
