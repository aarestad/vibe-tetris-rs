use crate::tetrimino::{Tetrimino, TetriminoType};
use std::time::Duration;

/// How long the Minos of a Tetrimino flash after it Locks Down (Guideline 13.1.3). Shorter
/// than the Generation delay, so the flash ends before the next Tetrimino appears.
pub const LOCK_FLASH_TIME: Duration = Duration::from_millis(150);
/// How long a Hard Drop trail takes to fade (Guideline 13.1.2).
pub const HARD_DROP_TRAIL_TIME: Duration = Duration::from_millis(200);

/// Short effects that follow a Lock Down: the locked Minos flash, and a Hard Drop leaves a
/// trail that shrinks down toward where the Tetrimino landed. They only change how cells are
/// drawn, never the game.
#[derive(Debug, Clone, Default)]
pub struct VisualEffects {
    lock_flash: Option<LockFlash>,
    trail: Option<Trail>,
}

#[derive(Debug, Clone)]
struct LockFlash {
    cells: [(i32, i32); 4],
    remaining: Duration,
}

#[derive(Debug, Clone)]
struct Trail {
    kind: TetriminoType,
    /// Cells the Tetrimino passed through, as (x, y, rows above where it landed).
    cells: Vec<(i32, i32, u32)>,
    rows: u32,
    remaining: Duration,
}

impl VisualEffects {
    /// Starts the flash for a Tetrimino that just Locked Down.
    pub fn lock_down(&mut self, piece: &Tetrimino) {
        self.lock_flash = Some(LockFlash {
            cells: piece
                .get_blocks()
                .map(|(dx, dy)| (piece.x + dx, piece.y + dy)),
            remaining: LOCK_FLASH_TIME,
        });
    }

    /// Starts the trail for a Tetrimino that was Hard Dropped `rows` rows to `landed`. Each
    /// column's trail rises from the top Mino in that column, so it covers exactly the cells
    /// the Tetrimino passed through.
    pub fn hard_drop(&mut self, landed: &Tetrimino, rows: u32) {
        if rows == 0 {
            self.trail = None;
            return;
        }
        let blocks = landed
            .get_blocks()
            .map(|(dx, dy)| (landed.x + dx, landed.y + dy));
        let mut columns: Vec<(i32, i32)> = Vec::new();
        for (x, y) in blocks {
            match columns.iter_mut().find(|(column, _)| *column == x) {
                Some((_, top)) => *top = (*top).min(y),
                None => columns.push((x, y)),
            }
        }
        let cells = columns
            .into_iter()
            .flat_map(|(x, top)| {
                (1..=rows).map(move |distance| (x, top - distance as i32, distance))
            })
            .collect();
        self.trail = Some(Trail {
            kind: landed.kind,
            cells,
            rows,
            remaining: HARD_DROP_TRAIL_TIME,
        });
    }

    /// Advances the effects, ending any whose time is up.
    pub fn tick(&mut self, dt: Duration) {
        if let Some(flash) = &mut self.lock_flash {
            flash.remaining = flash.remaining.saturating_sub(dt);
            if flash.remaining.is_zero() {
                self.lock_flash = None;
            }
        }
        if let Some(trail) = &mut self.trail {
            trail.remaining = trail.remaining.saturating_sub(dt);
            if trail.remaining.is_zero() {
                self.trail = None;
            }
        }
    }

    /// Ends every effect, for when the cells they cover move, as after a Line Clear.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether a cell is a Mino still flashing from its Lock Down.
    pub fn is_flashing(&self, x: i32, y: i32) -> bool {
        self.lock_flash
            .as_ref()
            .is_some_and(|flash| flash.cells.contains(&(x, y)))
    }

    /// The Tetrimino whose Hard Drop trail shows in a cell, if any. The trail shrinks from
    /// the top as it fades, so the cells nearest the landed Tetrimino last longest.
    pub fn trail_at(&self, x: i32, y: i32) -> Option<TetriminoType> {
        let trail = self.trail.as_ref()?;
        let shown = (trail.rows as u128 * trail.remaining.as_nanos())
            .div_ceil(HARD_DROP_TRAIL_TIME.as_nanos());
        trail
            .cells
            .iter()
            .any(|&(cx, cy, distance)| cx == x && cy == y && distance as u128 <= shown)
            .then_some(trail.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tetrimino::Facing;

    /// A T pointing up, landed with its flat side on row 19.
    fn landed_t() -> Tetrimino {
        Tetrimino {
            kind: TetriminoType::T,
            x: 3,
            y: 18,
            facing: Facing::North,
        }
    }

    fn trail_cells(effects: &VisualEffects) -> Vec<(i32, i32)> {
        let mut cells: Vec<_> = (0..10)
            .flat_map(|x| (-2..20).map(move |y| (x, y)))
            .filter(|&(x, y)| effects.trail_at(x, y).is_some())
            .collect();
        cells.sort();
        cells
    }

    #[test]
    fn test_lock_flash_covers_the_locked_minos_until_its_time_is_up() {
        let mut effects = VisualEffects::default();
        let piece = landed_t();
        effects.lock_down(&piece);
        for (dx, dy) in piece.get_blocks() {
            assert!(effects.is_flashing(piece.x + dx, piece.y + dy));
        }
        assert!(!effects.is_flashing(0, 0));

        effects.tick(LOCK_FLASH_TIME - Duration::from_millis(1));
        assert!(effects.is_flashing(4, 18));
        effects.tick(Duration::from_millis(1));
        assert!(!effects.is_flashing(4, 18));
    }

    #[test]
    fn test_trail_covers_the_cells_passed_through_and_none_of_the_landed_minos() {
        let mut effects = VisualEffects::default();
        // The T's top Mino is at (4, 18) and its flat side at (3..=5, 19).
        effects.hard_drop(&landed_t(), 3);
        assert_eq!(
            trail_cells(&effects),
            vec![
                (3, 16),
                (3, 17),
                (3, 18),
                (4, 15),
                (4, 16),
                (4, 17),
                (5, 16),
                (5, 17),
                (5, 18)
            ]
        );
        assert_eq!(effects.trail_at(4, 17), Some(TetriminoType::T));
    }

    #[test]
    fn test_trail_shrinks_toward_the_landed_tetrimino_then_ends() {
        let mut effects = VisualEffects::default();
        effects.hard_drop(&landed_t(), 4);
        let top = |effects: &VisualEffects| (0..20).find(|&y| effects.trail_at(4, y).is_some());
        assert_eq!(top(&effects), Some(14));
        effects.tick(HARD_DROP_TRAIL_TIME / 2);
        assert_eq!(top(&effects), Some(16));
        effects.tick(HARD_DROP_TRAIL_TIME / 2 - Duration::from_millis(1));
        assert_eq!(top(&effects), Some(17));
        effects.tick(Duration::from_millis(1));
        assert_eq!(top(&effects), None);
    }

    #[test]
    fn test_a_drop_of_no_rows_leaves_no_trail_and_clear_ends_everything() {
        let mut effects = VisualEffects::default();
        effects.hard_drop(&landed_t(), 0);
        assert!(trail_cells(&effects).is_empty());

        effects.hard_drop(&landed_t(), 2);
        effects.lock_down(&landed_t());
        effects.clear();
        assert!(trail_cells(&effects).is_empty());
        assert!(!effects.is_flashing(4, 18));
    }
}
