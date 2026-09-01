//! `town-mode.md §13` Lord British's Castle harpsichord puzzle — the gameplay
//! half.
//!
//! [`crate::audio`] already owns the audible half: the ten key pitches, the
//! thirteen-note tune [`crate::audio::HARPSICHORD_TUNE`], and the wrong-note
//! re-sync rule [`crate::audio::harpsichord_progress_after`]. Nothing here
//! duplicates them.
//!
//! What this module adds is the three published gameplay predicates —
//! which tile is the instrument, when the party is seated at it, and which
//! wall cell a finished tune opens — plus the digit handler that the town
//! turn loop routes `0`..`9` into. `commands.md §3` names that handler the
//! sole producer of command status `3`, "re-prompt immediately, without
//! advancing the world", so the handler advances no clock, ticks no NPC
//! schedule, and requests no redraw of its own.

use crate::audio::{HARPSICHORD_TUNE, harpsichord_progress_after};
use crate::{Area, Direction, PlayState, SCENE_LORD_BRITISHS_CASTLE, SoundEffect};

/// `town-mode.md §13` harpsichord tile. Its `LOOK2.DAT` description names it
/// as an instrument with ten keys numbered zero through nine.
pub const HARPSICHORD_TILE: u8 = 0x8D;

/// `town-mode.md §13` floor byte the instrument and its passage live on: two
/// storeys above the castle's entry floor.
pub const HARPSICHORD_FLOOR: i8 = 2;

/// `town-mode.md §13`: the completed tune opens the wall cell five squares
/// north of the harpsichord, in the same column.
pub const HARPSICHORD_PASSAGE_CELLS_NORTH: usize = 5;

/// `town-mode.md §13`: that wall cell is rewritten to ordinary cobble floor.
/// Anchored to [`crate::TOWN_DOOR_CLEARED_TILE`] so every "rewritten to
/// ordinary cobble" site in the engine shares one byte;
/// `catalogs/tile-catalog.md` names `0x44` cobble.
pub const HARPSICHORD_PASSAGE_CLEARED_TILE: u8 = crate::TOWN_DOOR_CLEARED_TILE;

/// `town-mode.md §13`: the wall cell a finished tune opens, given the
/// harpsichord's own cell. `None` when the column runs off the top of the
/// thirty-two-by-thirty-two floor grid.
pub const fn harpsichord_passage_cell(x: usize, y: usize) -> Option<(usize, usize)> {
    match y.checked_sub(HARPSICHORD_PASSAGE_CELLS_NORTH) {
        Some(passage_y) => Some((x, passage_y)),
        None => None,
    }
}

impl PlayState {
    /// `town-mode.md §13`: the harpsichord's cell when the party is seated at
    /// it, otherwise `None`.
    ///
    /// The instrument is armed by position alone — no flag, latch, or prior
    /// event participates — and the test is exactly "the tile one cell south
    /// of the party is the harpsichord tile". This is the same four-neighbour
    /// probe the Fire and Yell commands use, so it reuses
    /// [`PlayState::adjacent_position`]. Note that the arming test carries no
    /// scene or floor gate of its own; only completion is gated (see
    /// [`PlayState::open_harpsichord_passage`]).
    pub fn seated_harpsichord_cell(&self) -> Option<(usize, usize)> {
        if !matches!(self.area, Area::Town { .. }) {
            return None;
        }
        let (x, y) = self.adjacent_position(Direction::South)?;
        (self.grid[y * 32 + x] == HARPSICHORD_TILE).then_some((x, y))
    }

    /// `town-mode.md §13`: whether the party is seated at the harpsichord.
    pub fn seated_at_harpsichord(&self) -> bool {
        self.seated_harpsichord_cell().is_some()
    }

    /// `town-mode.md §13`: how many leading notes of the thirteen-note tune
    /// have been played. Not cleared by leaving the chair — only by a wrong
    /// note or by completing the tune.
    pub fn harpsichord_progress(&self) -> usize {
        self.harpsichord_progress
    }

    /// `town-mode.md §13` + `commands.md §3`: play one digit key on the
    /// instrument.
    ///
    /// Returns `false` when the party is not seated, which is the caller's
    /// signal to forward the digit to the ordinary dispatcher and return its
    /// result. Returns `true` when the instrument consumed the key, which is
    /// command status `3`: one note is sounded, progress advances or
    /// re-syncs, and no turn is consumed.
    pub fn play_harpsichord_digit(&mut self, digit: u8) -> bool {
        let Some((x, y)) = self.seated_harpsichord_cell() else {
            return false;
        };
        // `town-mode.md §13` and `audio.md §8.6`: the digit plays its note
        // only while the global sound setting is on. Unlike the ordinary
        // triggers of `audio.md §3`, this one is gated at the caller, so a
        // muted instrument records no boundary at all. `music_enabled` is the
        // Ctrl-S sound boolean.
        if self.music_enabled {
            self.emit_sound_effect(SoundEffect::HarpsichordNote { digit });
        }
        self.harpsichord_progress = harpsichord_progress_after(self.harpsichord_progress, digit);
        if self.harpsichord_progress == HARPSICHORD_TUNE.len() {
            self.harpsichord_progress = 0;
            self.open_harpsichord_passage(x, y);
        }
        true
    }

    /// `town-mode.md §13`: on the thirteenth correct note, and only while the
    /// scene is Lord British's Castle and the floor byte is `2`, the wall cell
    /// five squares north of the harpsichord is rewritten to ordinary cobble
    /// floor and the view is marked dirty.
    ///
    /// The rewrite is a live tile-buffer edit rather than a saved map change,
    /// so it deliberately records nothing the way an opened door does:
    /// reloading that floor restores the wall.
    pub(crate) fn open_harpsichord_passage(&mut self, x: usize, y: usize) {
        let Area::Town { scene, floor } = self.area else {
            return;
        };
        if scene.byte != SCENE_LORD_BRITISHS_CASTLE || floor != HARPSICHORD_FLOOR {
            return;
        }
        let Some((passage_x, passage_y)) = harpsichord_passage_cell(x, y) else {
            return;
        };
        self.grid[passage_y * 32 + passage_x] = HARPSICHORD_PASSAGE_CLEARED_TILE;
        self.mark_visibility_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{open_grid, test_state};

    #[test]
    fn passage_cell_is_five_squares_north_in_the_same_column() {
        assert_eq!(harpsichord_passage_cell(18, 12), Some((18, 7)));
        assert_eq!(harpsichord_passage_cell(0, 5), Some((0, 0)));
        assert_eq!(harpsichord_passage_cell(4, 4), None);
    }

    #[test]
    fn seated_test_reads_the_south_cell_only() {
        let mut state = test_state(open_grid(), 10, 10);
        assert!(!state.seated_at_harpsichord());

        for (dx, dy) in [(0isize, -1isize), (-1, 0), (1, 0), (-1, -1), (1, 1)] {
            let x = (10 + dx) as usize;
            let y = (10 + dy) as usize;
            state.grid[y * 32 + x] = HARPSICHORD_TILE;
            assert!(
                !state.seated_at_harpsichord(),
                "tile at ({dx}, {dy}) must not arm the instrument"
            );
            state.grid[y * 32 + x] = 0x44;
        }

        state.grid[11 * 32 + 10] = HARPSICHORD_TILE;
        assert_eq!(state.seated_harpsichord_cell(), Some((10, 11)));
    }

    #[test]
    fn seated_test_is_position_alone_on_every_floor_and_scene() {
        let mut state = test_state(open_grid(), 10, 10);
        state.grid[11 * 32 + 10] = HARPSICHORD_TILE;
        let Area::Town { scene, .. } = state.area else {
            unreachable!("town fixture");
        };
        for floor in [0i8, 1, 2, 3] {
            state.area = Area::Town { scene, floor };
            assert!(state.seated_at_harpsichord());
        }
    }

    #[test]
    fn party_on_the_bottom_row_is_never_seated() {
        let state = test_state(open_grid(), 10, 31);
        assert!(!state.seated_at_harpsichord());
    }
}
