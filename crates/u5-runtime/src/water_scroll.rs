//! Water-surface scroll — a display-driver pixel treatment.
//!
//! # Provenance
//!
//! **Runtime observation, `cleak/u5-spec#179`.** Nothing in the published
//! spec set describes this effect; `systems/animation.md §6` explicitly
//! excludes water from the five global tile-animation families
//! (`RETRACTIONS.md` R148: "no water, lava, brazier or torch tile animates
//! through this pass at all"), and the shipped location maps author
//! `0x01`, `0x02` and `0x03` independently in the tens of thousands, so
//! water cannot be a tile-id frame family. It nevertheless animates.
//!
//! Black-box capture of the shipped build (DOSBox Staging, `machine=ega`)
//! measured, on a Britannia overworld river:
//!
//! * exactly **16 distinct phases**, in strict repeating order
//!   `0 1 2 … 15 0 1 …`, over 1638 transitions with no deviation;
//! * **one phase advance per BIOS user tick** (54.913 ms measured), so a
//!   full cycle is 878.6 ms;
//! * the surface pattern **scrolls straight down, one pixel row per
//!   tick**, with no horizontal component at all, wrapping after 16 rows —
//!   the tile is 16 rows tall, which is why 16 phases close the cycle;
//! * **one global counter**: five water regions of interest spread across
//!   the viewport transitioned within 0.07–0.41 ms of each other, so there
//!   is no per-tile phase and no offset seeded from map position. This is
//!   the opposite of the `§6` families, whose ids are deliberately a
//!   quarter-cycle apart so a wall of them does not flicker in lockstep.
//!
//! The model is therefore, for a tile of pure water:
//!
//! ```text
//! phase_k[row] = authored[(row - k) mod 16]
//! ```
//!
//! This is a **display-driver pixel treatment**, not a tile-id selector:
//! the map byte and the resolved tile id are untouched, and only the
//! pixels handed to the blit are rolled. It runs beside
//! [`crate::static_tile_animation_pass`], never inside it.
//!
//! # The direction was measured twice, and the first reading was wrong
//!
//! An earlier revision of this module implemented a *horizontal* roll,
//! from an initial report describing the effect as "a horizontal scroll of
//! the water surface pattern". That reading is **withdrawn upstream**, and
//! it is worth recording why, because the wrong answer is what a naive
//! analysis produces: successive rows of the wave texture are themselves
//! horizontally offset from one another, so scrolling a diagonal texture
//! vertically reads as sideways motion to a row-wise cross-correlator. It
//! reports a confident ~4 px per tick leftward — and never matches a row
//! exactly. The vertical roll matches bit for bit.
//!
//! Checked against the captured 16-phase art at true EGA resolution, over
//! every consecutive phase pair: rows 5..=10 of the tile satisfy
//! `phase_k[row] == phase_{k-1}[row - 1]` in **15 of 15** pairs, the
//! upward test matches **0** times anywhere, and rows 0..=1 and 13..=15
//! are bit-identical in all 16 phases. Those static rows are the river
//! tile's bank, composited over the moving water after the roll; the five
//! remaining rows are the band edges where bank pixels partly mask it.
//!
//! # What is still unverified
//!
//! `cleak/u5-spec#179` mechanism question 1 is still open as a *spec*
//! answer — this is measurement, not published contract.
//!
//! * **Which tile ids.** The capture is black-box and cannot name ids. It
//!   saw 23 animated water cells resolving to 14 distinct appearances —
//!   river runs, bends and shore corners — so more ids animate than the
//!   three treated here. This module deliberately covers only open water
//!   `0x01..=0x03`, whose art is water edge to edge and needs no mask. The
//!   river and shore ids are a known gap: animating them correctly needs
//!   the bank/water split the capture describes, which would be invented
//!   here rather than derived.
//! * **Open sea.** The capture never reached it; everything measured is
//!   river and shore. All three water ids are treated alike, which is the
//!   conservative reading of "all water tiles are in lockstep".
//! * **The tile-edge wrap.** No two vertically adjacent tiles of identical
//!   water art were on screen, so per-tile wrapping is inferred from the
//!   16-row period rather than directly observed. It is indistinguishable
//!   from a continuous scroll while the field is 16-row periodic and every
//!   tile shares the counter, which is exactly the case here.

use crate::{TILE_ATLAS_SIDE, TILE_ATLAS_TILE_PIXELS};

/// Measured phase count of the water scroll: sixteen, the tile height.
///
/// Runtime observation, `cleak/u5-spec#179`.
pub const WATER_SCROLL_PHASE_COUNT: u8 = 16;

/// Measured advance per world tick: one pixel row.
///
/// Runtime observation, `cleak/u5-spec#179`. A single bright highlight
/// pixel tracked across the whole cycle moves down exactly one row per
/// phase and never changes column.
pub const WATER_SCROLL_ROWS_PER_PHASE: usize = 1;

/// Scroll direction: `true` means the surface pattern travels toward
/// higher y, i.e. **downward** on screen.
///
/// Runtime observation, `cleak/u5-spec#179`: the downward test matches
/// rows bit for bit and the upward test matches nowhere. This is the
/// single place the direction is encoded, so flipping this constant flips
/// the effect.
pub const WATER_SCROLL_TOWARD_HIGHER_Y: bool = true;

/// The terrain ids the scroll applies to: open water `0x01..=0x03`.
///
/// Deliberately the same set as [`crate::is_water_tile`] and deliberately
/// **not** swamp `0x04`, which is walkable terrain rather than water. See
/// the module docs for why the river and shore ids are left out.
pub fn tile_uses_water_scroll(tile: u8) -> bool {
    crate::is_water_tile(tile)
}

/// The one global water-scroll counter.
///
/// Every water tile on screen reads this same value — the measured
/// lockstep — so it lives once on [`crate::PlayState`] rather than per
/// cell. It is **not** a member of [`crate::AnimationClock`]: that clock
/// is the `animation.md §6` shared phase counter with its own published
/// period and nested gates, and this counter has neither.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WaterScrollClock {
    /// Current phase, always `< WATER_SCROLL_PHASE_COUNT`.
    pub phase: u8,
}

impl WaterScrollClock {
    /// A clock that has advanced `phase` times since phase zero.
    pub const fn at_phase(phase: u8) -> Self {
        Self {
            phase: phase % WATER_SCROLL_PHASE_COUNT,
        }
    }

    /// One world tick: exactly one phase, in strict order.
    pub fn tick(&mut self) {
        self.phase = self.phase.wrapping_add(1) % WATER_SCROLL_PHASE_COUNT;
    }

    /// How many pixel rows the surface pattern is displaced by right now.
    pub const fn row_shift(self) -> usize {
        (self.phase as usize * WATER_SCROLL_ROWS_PER_PHASE) % TILE_ATLAS_SIDE
    }
}

/// Source row that supplies destination row `y` at `shift`.
///
/// Downward motion means the drawn row takes the content of the row
/// *above* it, so the source index runs backwards: `(y - shift) mod 16`.
const fn source_row(y: usize, shift: usize) -> usize {
    if WATER_SCROLL_TOWARD_HIGHER_Y {
        (y + TILE_ATLAS_SIDE - shift % TILE_ATLAS_SIDE) % TILE_ATLAS_SIDE
    } else {
        (y + shift) % TILE_ATLAS_SIDE
    }
}

/// Roll one tile's pixels vertically by `shift` rows, wrapping at the tile
/// edge.
///
/// Every row is displaced by the same amount and no row moves sideways —
/// a uniform whole-tile scroll, which is what the phase art shows for the
/// rows that are water edge to edge. Tiles that mix water with a static
/// bank need that bank composited back over the result; this engine only
/// scrolls the open-water ids, which have no bank, so no mask is applied
/// and none is invented.
///
/// Returns `None` when `source` is not exactly one tile.
pub fn scroll_tile_pixels(source: &[u8], shift: usize) -> Option<Vec<u8>> {
    if source.len() != TILE_ATLAS_TILE_PIXELS {
        return None;
    }
    let shift = shift % TILE_ATLAS_SIDE;
    if shift == 0 {
        return Some(source.to_vec());
    }
    let mut rolled = vec![0u8; TILE_ATLAS_TILE_PIXELS];
    for y in 0..TILE_ATLAS_SIDE {
        let src = source_row(y, shift) * TILE_ATLAS_SIDE;
        let dst = y * TILE_ATLAS_SIDE;
        rolled[dst..dst + TILE_ATLAS_SIDE].copy_from_slice(&source[src..src + TILE_ATLAS_SIDE]);
    }
    Some(rolled)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The measured phase structure: sixteen phases, strict order, one
    /// pixel row each, closing the cycle exactly at the tile height.
    #[test]
    fn water_scroll_cycle_is_sixteen_single_row_phases() {
        assert_eq!(usize::from(WATER_SCROLL_PHASE_COUNT), TILE_ATLAS_SIDE);
        let mut clock = WaterScrollClock::default();
        let mut seen = Vec::new();
        for _ in 0..WATER_SCROLL_PHASE_COUNT {
            seen.push(clock.row_shift());
            clock.tick();
        }
        assert_eq!(clock.phase, 0, "sixteen ticks close the cycle");
        assert_eq!(seen, (0..TILE_ATLAS_SIDE).collect::<Vec<_>>());
    }

    /// `phase_k[row] == phase_{k-1}[row - 1]`: content moves **down** one
    /// row per tick, and the upward reading matches nowhere. This is the
    /// exact test the capture was scored against.
    #[test]
    fn each_phase_moves_the_pattern_down_exactly_one_row() {
        // Rows carrying distinct content, so a displacement is visible.
        let source: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i / TILE_ATLAS_SIDE) as u8)
            .collect();
        let row =
            |tile: &[u8], y: usize| tile[y * TILE_ATLAS_SIDE..(y + 1) * TILE_ATLAS_SIDE].to_vec();

        let phases: Vec<Vec<u8>> = (0..TILE_ATLAS_SIDE)
            .map(|k| scroll_tile_pixels(&source, k).expect("one tile"))
            .collect();

        let mut down = 0;
        let mut up = 0;
        for k in 1..TILE_ATLAS_SIDE {
            for y in 1..TILE_ATLAS_SIDE {
                down += usize::from(row(&phases[k], y) == row(&phases[k - 1], y - 1));
            }
            for y in 0..TILE_ATLAS_SIDE - 1 {
                up += usize::from(row(&phases[k], y) == row(&phases[k - 1], y + 1));
            }
        }
        assert_eq!(
            down,
            (TILE_ATLAS_SIDE - 1) * (TILE_ATLAS_SIDE - 1),
            "every interior row must take the row above it"
        );
        assert_eq!(up, 0, "nothing moves upward");
    }

    /// No horizontal component whatsoever: a column keeps its pixels, only
    /// reordered. The withdrawn first reading of this effect was a
    /// horizontal roll, so this is pinned explicitly.
    #[test]
    fn water_scroll_has_no_horizontal_component() {
        let source: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS).map(|i| i as u8).collect();
        let column = |tile: &[u8], x: usize| {
            (0..TILE_ATLAS_SIDE)
                .map(|y| tile[y * TILE_ATLAS_SIDE + x])
                .collect::<Vec<u8>>()
        };

        for shift in 0..TILE_ATLAS_SIDE {
            let rolled = scroll_tile_pixels(&source, shift).expect("one tile");
            for x in 0..TILE_ATLAS_SIDE {
                let mut before = column(&source, x);
                let mut after = column(&rolled, x);
                before.sort_unstable();
                after.sort_unstable();
                assert_eq!(before, after, "column {x} keeps its own pixels at {shift}");
            }
        }
    }

    /// Phase zero draws the authored tile, and a whole cycle returns to
    /// it. Every phase is a pure row permutation — never new pixel data.
    #[test]
    fn a_whole_cycle_returns_the_authored_tile() {
        let source: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i % 251) as u8)
            .collect();

        assert_eq!(scroll_tile_pixels(&source, 0).expect("one tile"), source);
        assert_eq!(
            scroll_tile_pixels(&source, TILE_ATLAS_SIDE).expect("one tile"),
            source
        );

        for shift in 0..TILE_ATLAS_SIDE {
            let rolled = scroll_tile_pixels(&source, shift).expect("one tile");
            assert_eq!(rolled.len(), TILE_ATLAS_TILE_PIXELS);
            let mut before = source.clone();
            let mut after = rolled;
            before.sort_unstable();
            after.sort_unstable();
            assert_eq!(before, after, "shift {shift} is a permutation");
        }
    }

    /// Only open water scrolls. Swamp is walkable terrain, and the five
    /// `animation.md §6` families own a different mechanism entirely.
    #[test]
    fn only_open_water_ids_scroll() {
        for tile in 0x01..=0x03u8 {
            assert!(tile_uses_water_scroll(tile), "0x{tile:02x} is open water");
        }
        for tile in [
            0x00u8, 0x04, 0x05, 0x0A, 0x8F, 0xB0, 0xD4, 0xD8, 0xE4, 0xEC, 0xFA,
        ] {
            assert!(
                !tile_uses_water_scroll(tile),
                "0x{tile:02x} must not scroll"
            );
        }
    }
}
