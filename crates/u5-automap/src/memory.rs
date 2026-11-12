//! Remembered dungeon cells, one bit per cell (design 4.3.2, 10.3.2).
//!
//! The automap's `ExplorationMemory` and the gem map's `GemKnowledge`
//! both keep the eight dungeons as `[[u8; 8]; 64]`: the outer index is
//! `record * 8 + level`, the inner one the row `y`, and bit `x` of a row
//! byte (least significant bit first) is the cell `(x, y)`. The same
//! eight row bytes are a dungeon section's payload (`AMAP` tag 3, `GEMK`
//! tag 9).
//!
//! `Debug` prints counts only, never which cells are set, so a log line
//! cannot leak a map.

use std::fmt;

/// Dungeon records (`DUNGEON.DAT` holds eight).
pub const DUNGEON_RECORDS: u8 = 8;
/// Levels per dungeon record.
pub const DUNGEON_LEVEL_COUNT: u8 = 8;
/// Cells per row and rows per level.
pub const DUNGEON_SIDE_CELLS: u8 = 8;

const SLOTS: usize = DUNGEON_RECORDS as usize * DUNGEON_LEVEL_COUNT as usize;

/// One bit per remembered dungeon cell, for all eight dungeons.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DungeonBits {
    levels: [[u8; 8]; SLOTS],
}

impl Default for DungeonBits {
    fn default() -> Self {
        Self::new()
    }
}

impl DungeonBits {
    pub const fn new() -> Self {
        Self {
            levels: [[0; 8]; SLOTS],
        }
    }

    /// The `record * 8 + level` slot, or `None` when either is out of
    /// range.
    pub const fn slot(record: u8, level: u8) -> Option<usize> {
        if record < DUNGEON_RECORDS && level < DUNGEON_LEVEL_COUNT {
            Some(record as usize * DUNGEON_LEVEL_COUNT as usize + level as usize)
        } else {
            None
        }
    }

    /// Whether `(x, y)` on `record`, `level` is remembered. Out-of-range
    /// arguments read as not remembered.
    pub fn is_set(&self, record: u8, level: u8, x: u8, y: u8) -> bool {
        match (Self::slot(record, level), x < DUNGEON_SIDE_CELLS) {
            (Some(slot), true) if y < DUNGEON_SIDE_CELLS => {
                self.levels[slot][y as usize] & (1 << x) != 0
            }
            _ => false,
        }
    }

    /// Remembers `(x, y)`. Returns true when the bit was not set before.
    /// Out-of-range arguments change nothing.
    pub fn set(&mut self, record: u8, level: u8, x: u8, y: u8) -> bool {
        let Some(slot) = Self::slot(record, level) else {
            return false;
        };
        if x >= DUNGEON_SIDE_CELLS || y >= DUNGEON_SIDE_CELLS {
            return false;
        }
        let row = &mut self.levels[slot][y as usize];
        let before = *row;
        *row |= 1 << x;
        *row != before
    }

    /// The eight row bytes of one level (all zero when out of range).
    pub fn level_rows(&self, record: u8, level: u8) -> [u8; 8] {
        Self::slot(record, level).map_or([0; 8], |slot| self.levels[slot])
    }

    /// ORs eight row bytes into one level, as a section decoder does.
    /// Returns true when any bit changed.
    pub fn merge_level_rows(&mut self, record: u8, level: u8, rows: [u8; 8]) -> bool {
        let Some(slot) = Self::slot(record, level) else {
            return false;
        };
        let mut changed = false;
        for (row, add) in self.levels[slot].iter_mut().zip(rows) {
            let before = *row;
            *row |= add;
            changed |= *row != before;
        }
        changed
    }

    /// Forgets one level. Returns true when any bit was set.
    pub fn clear_level(&mut self, record: u8, level: u8) -> bool {
        let Some(slot) = Self::slot(record, level) else {
            return false;
        };
        let had = self.levels[slot] != [0; 8];
        self.levels[slot] = [0; 8];
        had
    }

    /// Forgets everything.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Remembered cells on one level.
    pub fn level_count(&self, record: u8, level: u8) -> u32 {
        self.level_rows(record, level)
            .iter()
            .map(|row| row.count_ones())
            .sum()
    }

    /// Remembered cells in one dungeon, all levels.
    pub fn record_count(&self, record: u8) -> u32 {
        (0..DUNGEON_LEVEL_COUNT)
            .map(|level| self.level_count(record, level))
            .sum()
    }

    /// Remembered cells in all eight dungeons.
    pub fn total(&self) -> u32 {
        self.levels
            .iter()
            .flatten()
            .map(|row| row.count_ones())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.levels.iter().all(|level| *level == [0; 8])
    }

    /// Every `(record, level)` with at least one remembered cell, in
    /// slot order. Encoders write one section per item.
    pub fn nonempty_levels(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.levels
            .iter()
            .enumerate()
            .filter(|(_, rows)| **rows != [0; 8])
            .map(|(slot, _)| {
                (
                    (slot / DUNGEON_LEVEL_COUNT as usize) as u8,
                    (slot % DUNGEON_LEVEL_COUNT as usize) as u8,
                )
            })
    }
}

impl fmt::Debug for DungeonBits {
    /// Counts per dungeon only, never cells.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let per_record: Vec<u32> = (0..DUNGEON_RECORDS)
            .map(|record| self.record_count(record))
            .collect();
        f.debug_struct("DungeonBits")
            .field("cells", &self.total())
            .field("levels", &self.nonempty_levels().count())
            .field("per_record", &per_record)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addressing_is_record_times_eight_plus_level_row_y_bit_x() {
        let mut bits = DungeonBits::new();
        assert!(bits.set(3, 5, 6, 2));
        assert_eq!(DungeonBits::slot(3, 5), Some(29));
        assert_eq!(bits.levels[29][2], 1 << 6);
        assert_eq!(bits.level_rows(3, 5), [0, 0, 1 << 6, 0, 0, 0, 0, 0]);
        assert!(bits.is_set(3, 5, 6, 2));
        assert!(!bits.is_set(3, 5, 2, 6));
        assert!(!bits.is_set(5, 3, 6, 2));
    }

    #[test]
    fn set_reports_only_a_new_bit() {
        let mut bits = DungeonBits::new();
        assert!(bits.set(0, 0, 0, 0));
        assert!(!bits.set(0, 0, 0, 0));
        assert!(bits.set(7, 7, 7, 7));
        assert_eq!(bits.total(), 2);
    }

    #[test]
    fn out_of_range_is_ignored_and_reads_unset() {
        let mut bits = DungeonBits::new();
        assert!(!bits.set(8, 0, 0, 0));
        assert!(!bits.set(0, 8, 0, 0));
        assert!(!bits.set(0, 0, 8, 0));
        assert!(!bits.set(0, 0, 0, 8));
        assert!(!bits.is_set(8, 0, 0, 0));
        assert!(!bits.merge_level_rows(0, 8, [0xFF; 8]));
        assert!(!bits.clear_level(9, 0));
        assert_eq!(bits.level_rows(8, 0), [0; 8]);
        assert!(bits.is_empty());
    }

    #[test]
    fn rows_round_trip_through_merge_and_clear() {
        let mut bits = DungeonBits::new();
        let rows = [1, 2, 4, 8, 16, 32, 64, 128];
        assert!(bits.merge_level_rows(2, 4, rows));
        assert!(!bits.merge_level_rows(2, 4, rows));
        assert_eq!(bits.level_rows(2, 4), rows);
        assert_eq!(bits.level_count(2, 4), 8);
        assert_eq!(bits.record_count(2), 8);
        assert_eq!(bits.nonempty_levels().collect::<Vec<_>>(), [(2, 4)]);
        assert!(bits.clear_level(2, 4));
        assert!(!bits.clear_level(2, 4));
        assert!(bits.is_empty());

        bits.set(1, 1, 1, 1);
        bits.clear();
        assert_eq!(bits, DungeonBits::default());
    }

    #[test]
    fn debug_prints_counts_never_cells() {
        let mut bits = DungeonBits::new();
        bits.set(4, 6, 3, 5);
        bits.set(4, 6, 7, 1);
        let text = format!("{bits:?}");
        assert_eq!(
            text,
            "DungeonBits { cells: 2, levels: 1, per_record: [0, 0, 0, 0, 2, 0, 0, 0] }"
        );
        // Small: no row bytes and no coordinates.
        assert!(text.len() < 100, "{text}");
        assert!(!text.contains("[[") && !text.contains("128"), "{text}");
    }
}
