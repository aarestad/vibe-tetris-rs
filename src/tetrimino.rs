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

/// The direction a Tetrimino points, turned by rotating (Guideline A1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    /// All facings in clockwise order.
    pub const ALL: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

    pub fn clockwise(self) -> Self {
        match self {
            Facing::North => Facing::East,
            Facing::East => Facing::South,
            Facing::South => Facing::West,
            Facing::West => Facing::North,
        }
    }

    pub fn counterclockwise(self) -> Self {
        match self {
            Facing::North => Facing::West,
            Facing::East => Facing::North,
            Facing::South => Facing::East,
            Facing::West => Facing::South,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tetrimino {
    pub kind: TetriminoType,
    pub x: i32,
    pub y: i32,
    pub facing: Facing,
}

impl Tetrimino {
    pub fn new(kind: TetriminoType) -> Self {
        Self {
            kind,
            x: 0,
            y: 0,
            facing: Facing::North,
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
            facing: Facing::North,
        }
    }

    /// Returns block positions relative to (x, y) for the current facing.
    pub fn get_blocks(&self) -> [(i32, i32); 4] {
        use Facing::*;
        use TetriminoType as T;

        match (self.kind, self.facing) {
            (T::I, North) => [(0, 1), (1, 1), (2, 1), (3, 1)],
            (T::I, East) => [(2, 0), (2, 1), (2, 2), (2, 3)],
            (T::I, South) => [(0, 2), (1, 2), (2, 2), (3, 2)],
            (T::I, West) => [(1, 0), (1, 1), (1, 2), (1, 3)],

            // All O-Tetrimino facings occupy the same cells.
            (T::O, _) => [(0, 0), (1, 0), (0, 1), (1, 1)],

            (T::T, North) => [(1, 0), (0, 1), (1, 1), (2, 1)],
            (T::T, East) => [(1, 0), (1, 1), (2, 1), (1, 2)],
            (T::T, South) => [(0, 1), (1, 1), (2, 1), (1, 2)],
            (T::T, West) => [(1, 0), (0, 1), (1, 1), (1, 2)],

            (T::S, North) => [(1, 0), (2, 0), (0, 1), (1, 1)],
            (T::S, East) => [(1, 0), (1, 1), (2, 1), (2, 2)],
            (T::S, South) => [(1, 1), (2, 1), (0, 2), (1, 2)],
            (T::S, West) => [(0, 0), (0, 1), (1, 1), (1, 2)],

            (T::Z, North) => [(0, 0), (1, 0), (1, 1), (2, 1)],
            (T::Z, East) => [(2, 0), (1, 1), (2, 1), (1, 2)],
            (T::Z, South) => [(0, 1), (1, 1), (1, 2), (2, 2)],
            (T::Z, West) => [(1, 0), (0, 1), (1, 1), (0, 2)],

            (T::J, North) => [(0, 0), (0, 1), (1, 1), (2, 1)],
            (T::J, East) => [(1, 0), (2, 0), (1, 1), (1, 2)],
            (T::J, South) => [(0, 1), (1, 1), (2, 1), (2, 2)],
            (T::J, West) => [(1, 0), (1, 1), (0, 2), (1, 2)],

            (T::L, North) => [(2, 0), (0, 1), (1, 1), (2, 1)],
            (T::L, East) => [(1, 0), (1, 1), (1, 2), (2, 2)],
            (T::L, South) => [(0, 1), (1, 1), (2, 1), (0, 2)],
            (T::L, West) => [(0, 0), (1, 0), (1, 1), (1, 2)],
        }
    }
}

/// Super Rotation System offsets to try when rotating from a facing (Guideline A1.4), as
/// (rotation point, (dx, dy)) in the order tried. A positive dy moves down, so the sign is the
/// reverse of a y-up table.
pub fn rotation_points(
    kind: TetriminoType,
    from: Facing,
    clockwise: bool,
) -> Vec<(u8, (i32, i32))> {
    use Facing::*;

    let offsets: [(i32, i32); 5] = match kind {
        TetriminoType::O => return vec![(1, (0, 0))],
        TetriminoType::I => match (from, clockwise) {
            (North, true) | (West, false) => [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
            (East, false) | (South, true) => [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
            (East, true) | (North, false) => [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
            (South, false) | (West, true) => [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
        },
        _ => match (from, clockwise) {
            (North, true) | (South, false) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
            (East, _) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
            (South, true) | (North, false) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            (West, _) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        },
    };

    (1u8..)
        .zip(offsets)
        .filter(|(point, _)| !is_unused_point(kind, from, *point))
        .collect()
}

/// The Guideline marks these T-Tetrimino points as "Not Used" (pages 49 and 51).
fn is_unused_point(kind: TetriminoType, from: Facing, point: u8) -> bool {
    kind == TetriminoType::T && matches!((from, point), (Facing::North, 4) | (Facing::South, 3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Facing::*;
    use TetriminoType as T;

    const ALL_TYPES: [T; 7] = [T::I, T::O, T::T, T::S, T::Z, T::J, T::L];
    const THREE_WIDE: [T; 5] = [T::T, T::S, T::Z, T::J, T::L];

    fn offsets(kind: T, from: Facing, clockwise: bool) -> Vec<(i32, i32)> {
        rotation_points(kind, from, clockwise)
            .into_iter()
            .map(|(_, offset)| offset)
            .collect()
    }

    fn blocks(kind: T, facing: Facing) -> Vec<(i32, i32)> {
        let mut blocks = Tetrimino {
            kind,
            x: 0,
            y: 0,
            facing,
        }
        .get_blocks()
        .to_vec();
        blocks.sort();
        blocks
    }

    #[test]
    fn test_north_shapes_match_guideline() {
        // Guideline A1.3, read from the top row of each Tetrimino's box.
        let cases = [
            (T::I, vec![(0, 1), (1, 1), (2, 1), (3, 1)]),
            (T::O, vec![(0, 0), (0, 1), (1, 0), (1, 1)]),
            (T::T, vec![(0, 1), (1, 0), (1, 1), (2, 1)]),
            (T::S, vec![(0, 1), (1, 0), (1, 1), (2, 0)]),
            (T::Z, vec![(0, 0), (1, 0), (1, 1), (2, 1)]),
            (T::J, vec![(0, 0), (0, 1), (1, 1), (2, 1)]),
            (T::L, vec![(0, 1), (1, 1), (2, 0), (2, 1)]),
        ];
        for (kind, expected) in cases {
            assert_eq!(blocks(kind, North), expected, "{kind:?}");
        }
    }

    #[test]
    fn test_each_facing_is_the_last_one_turned_clockwise() {
        // The T, S, Z, J and L turn within a 3 x 3 box and the I within a 4 x 4 box.
        for kind in THREE_WIDE.into_iter().chain([T::I]) {
            let last = if kind == T::I { 3 } else { 2 };
            for facing in Facing::ALL {
                let mut turned: Vec<_> = blocks(kind, facing)
                    .into_iter()
                    .map(|(x, y)| (last - y, x))
                    .collect();
                turned.sort();
                assert_eq!(
                    turned,
                    blocks(kind, facing.clockwise()),
                    "{kind:?} {facing:?}"
                );
            }
        }
    }

    #[test]
    fn test_o_is_the_same_in_every_facing() {
        for facing in Facing::ALL {
            assert_eq!(blocks(T::O, facing), blocks(T::O, North));
        }
    }

    #[test]
    fn test_clockwise_and_counterclockwise_undo_each_other() {
        for facing in Facing::ALL {
            assert_eq!(facing.clockwise().counterclockwise(), facing);
            assert_eq!(facing.counterclockwise().clockwise(), facing);
            assert_eq!(
                facing.clockwise().clockwise().clockwise().clockwise(),
                facing
            );
        }
    }

    #[test]
    fn test_i_rotation_points_match_guideline_pages_41_to_44() {
        // Each diagram read as: right is +x and down is +y.
        let cases = [
            (North, true, [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)]),
            (North, false, [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)]),
            (East, false, [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)]),
            (East, true, [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)]),
            (South, false, [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)]),
            (South, true, [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)]),
            (West, false, [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)]),
            (West, true, [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)]),
        ];
        for (from, clockwise, expected) in cases {
            assert_eq!(
                offsets(T::I, from, clockwise),
                expected,
                "{from:?} {clockwise}"
            );
        }
    }

    #[test]
    fn test_l_j_s_z_rotation_points_match_guideline_pages_55_to_76() {
        // All four share one pattern. Checked against all 16 of their pages.
        let cases = [
            (North, true, [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)]),
            (North, false, [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)]),
            (East, false, [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)]),
            (East, true, [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)]),
            (South, false, [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)]),
            (South, true, [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)]),
            (West, false, [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)]),
            (West, true, [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)]),
        ];
        for kind in [T::L, T::J, T::S, T::Z] {
            for (from, clockwise, expected) in cases {
                assert_eq!(
                    offsets(kind, from, clockwise),
                    expected,
                    "{kind:?} {from:?} {clockwise}"
                );
            }
        }
    }

    #[test]
    fn test_t_rotation_points_match_guideline_pages_49_to_52() {
        let t = T::T;
        assert_eq!(
            offsets(t, North, true),
            [(0, 0), (-1, 0), (-1, -1), (-1, 2)]
        );
        assert_eq!(offsets(t, North, false), [(0, 0), (1, 0), (1, -1), (1, 2)]);
        assert_eq!(
            offsets(t, East, false),
            [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)]
        );
        assert_eq!(
            offsets(t, East, true),
            [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)]
        );
        assert_eq!(offsets(t, South, false), [(0, 0), (-1, 0), (0, 2), (-1, 2)]);
        assert_eq!(offsets(t, South, true), [(0, 0), (1, 0), (0, 2), (1, 2)]);
        assert_eq!(
            offsets(t, West, false),
            [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)]
        );
        assert_eq!(
            offsets(t, West, true),
            [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)]
        );
    }

    #[test]
    fn test_t_point_numbers_skip_unused_points() {
        let numbers = |from, clockwise| -> Vec<u8> {
            rotation_points(T::T, from, clockwise)
                .into_iter()
                .map(|(point, _)| point)
                .collect()
        };
        assert_eq!(numbers(North, true), [1, 2, 3, 5]);
        assert_eq!(numbers(South, true), [1, 2, 4, 5]);
    }

    #[test]
    fn test_first_point_is_always_visual_rotation() {
        for kind in ALL_TYPES {
            for from in Facing::ALL {
                for clockwise in [true, false] {
                    assert_eq!(rotation_points(kind, from, clockwise)[0], (1, (0, 0)));
                }
            }
        }
    }

    #[test]
    fn test_o_has_one_rotation_point() {
        assert_eq!(rotation_points(T::O, North, true).len(), 1);
    }

    #[test]
    fn test_generation_x_is_centered_for_each_width() {
        assert_eq!(Tetrimino::at_generation(T::I, 10).x, 3);
        assert_eq!(Tetrimino::at_generation(T::O, 10).x, 4);
        for kind in THREE_WIDE {
            assert_eq!(Tetrimino::at_generation(kind, 10).x, 3, "{kind:?}");
        }
    }
}
