//! The display driver's water animator: a rotation pass and a composite pass.
//!
//! # Provenance
//!
//! The mechanism is published on `cleak/u5-spec#179` (comments of
//! 2026-09-01 01:48 and the 02:07 correction), as **interim contract pending
//! the spec commit** — it is still going through an adversarial verification
//! pass upstream. The cadence and the visible result are independently
//! measured, black-box, from the shipped build (same issue).
//!
//! This is not the `animation.md §6` tile-id selector pass.
//! `RETRACTIONS.md` R148 keeps water out of the five published families —
//! "no water, lava, brazier or torch tile animates through this pass at
//! all" — and the shipped maps author `0x01`, `0x02` and `0x03`
//! independently in the tens of thousands, so water was never a frame
//! family. It animates through the display driver instead, in two stages
//! that run one after the other on the same counter.
//!
//! ## Stage one: rotation
//!
//! A whole-tile vertical rotation, one pixel row per step, wrapping inside
//! the 16-row tile:
//!
//! ```text
//! phase_k[row] = authored[(row - k) mod 16]
//! ```
//!
//! The rotated set is exactly [`WATER_ROTATED_TILES`] — the three water ids
//! **and lava `0x8F`**, which takes literally the same code path rather
//! than an analogue. Period 16, one global counter, no horizontal
//! component anywhere.
//!
//! Measured to match: 16 phases in strict order over 1638 transitions, one
//! phase per BIOS user tick (54.913 ms), an 878.6 ms cycle. On the clean
//! open-sea art the model holds for **256 of 256** (phase, row) pairs, with
//! the downward test matching 240/240 rows and the upward test 0.
//!
//! ## Stage two: composite
//!
//! Several ids animate without being rotated themselves. Each step they are
//! rebuilt from the *shoals* tile [`WATER_COMPOSITE_SOURCE_TILE`] through a
//! mask tile, bitwise across the colour planes:
//!
//! ```text
//! dest = (dest & !mask) | (rotated_shoals & mask)
//! ```
//!
//! The destination sets and their masks are [`WATER_COMPOSITE_SETS`], and
//! the masks are **tiles in the shipped atlas the engine already parses** —
//! nothing about the geometry is hardcoded here. The rotated source frame
//! is **not** advanced between destinations, so every composited id shows
//! the same phase as the rotated tiles, which is why separate water regions
//! were measured transitioning within 0.07–0.41 ms of each other.
//!
//! ### The mask is one boolean per pixel, broadcast to every plane
//!
//! `mask` above is **not** four per-plane bytes. The compositor reads the
//! mask tile's *intensity-plane* byte only, and ANDs all four source-plane
//! reads against that same byte before advancing the mask pointer to the
//! next pixel group. So `m` is a single bit per pixel — the intensity bit
//! of the mask pixel — used as a boolean and applied identically to every
//! plane: where it is set the source is taken whole, where it is clear the
//! destination is kept whole.
//!
//! An earlier revision of this module implemented the mask **per plane**,
//! which is a different rule wherever a mask pixel has its intensity bit
//! set but some other bit clear. The river masks `0x70..=0x7F` hold colour
//! `13` (`0b1101`) — intensity set, green clear — so per-plane would keep
//! the destination's green plane over the flowing water. Applied to shipped
//! art the two models differ on 738 of 65536 river pixels (1.1%), always
//! the same substitution: broadcast draws **light cyan** where per-plane
//! draws **light blue**, i.e. the water's bright highlight pixels. The
//! coast sets cannot tell the two apart at all — `0xD0..=0xD3` hold only
//! `0` and `15`, and `15` has every bit set, so both models are
//! byte-identical there (0 of 16384 pixels differ).
//!
//! ### The third set is composited through the complement
//!
//! `0xE4..=0xE7` use the *same* mask tiles as `0x34..=0x37`, but a
//! standalone inversion pass runs between the two composites and flips
//! those mask tiles' plane-3 bytes — which, the mask being that plane, is
//! exactly an inversion of the boolean. The third set therefore sees the
//! complement of the mask the second set used. Applying one uniform rule
//! puts that set inside-out — water where bank belongs. This engine models
//! it as [`WaterCompositeSet::mask_inverted`] and never mutates shared
//! state to get there.
//!
//! ## Why one counter is enough
//!
//! In the original, both stages mutate the **shared tile asset** rather
//! than per-instance screen pixels, so every on-screen instance of a tile
//! changes identically and simultaneously by construction. This engine
//! re-derives each frame from pristine art instead, which is equivalent for
//! these two stages because both are idempotent functions of the phase.
//! (It would *not* be equivalent for the fire XOR, which is cumulative and
//! never restored — see "Not implemented" below.)
//!
//! # Emergence check
//!
//! The masks are not asserted here; they fall out of the shipped art. With
//! the rule above and the shipped atlas:
//!
//! * `0x34..=0x37` through `0xD0..=0xD3` move exactly 136 pixels each, in
//!   the four distinct 45-degree half-tiles — the same three shapes and the
//!   same 136-pixel count that were measured on seven coast cells, plus the
//!   fourth that was never on screen;
//! * `0xE4..=0xE7` through the complement move exactly the 120-pixel
//!   remainder;
//! * `0x60..=0x6F` through `0x70..=0x7F` move horizontal bands with static
//!   top and bottom rows — the river shape that was measured separately;
//! * composing shipped art per this rule reproduces the captured coast
//!   frames in **540 of 546** frames whose phase could be read (the six
//!   misses are torn captures, in the same cells the observation session
//!   reported tearing). Note this result does **not** discriminate between
//!   the broadcast and per-plane mask readings; see above.
//!
//! The broadcast reading is **not yet confirmed against a capture**. No
//! nearest-neighbour capture taken so far contains a river tile — the only
//! clean water capture holds open sea and the four diagonal-coast ids,
//! where the two readings are provably identical — and the filtered
//! captures blend roughly a third of their interior pixels off-palette, so
//! they cannot settle a single-bit question. The observable that would
//! settle it in one clean frame: **in a river tile, are the bright water
//! highlight pixels light cyan (broadcast) or light blue (per-plane)?**
//!
//! # Not implemented, deliberately
//!
//! * **Two further composite destinations**, "two gem ids", are named
//!   upstream but not yet identified. Pending `#179`.
//! * **Fire.** Each step XORs fresh pseudo-random noise from a dedicated
//!   noise tile through a per-fixture mask, admitting noise only on certain
//!   colour planes. The noise-tile id and the per-fixture plane rules are
//!   unpublished until the upstream adversarial pass completes. Note for
//!   whoever implements it: the XOR is cumulative and never restored, so a
//!   pristine-art engine will be statistically equivalent but **not**
//!   bit-identical — do not write a pixel-parity test against the original.
//! * **Banner and sail row-pair swaps** under per-bit pseudo-random gates,
//!   a third mechanism again, covering keep/towne/castle banners and four
//!   ship ids. Unpublished.
//!
//! # One caveat on the period
//!
//! What the code establishes upstream is 16 invocations of the driver
//! animator per full cycle. That equals 16 ticks only if there is exactly
//! one invocation per tick, and a third caller into the animator exists on
//! a path where that has not been checked. On the measured path the
//! 878.6 ms / 16 phases figure is authoritative, and it is what this engine
//! implements.

use crate::{TILE_ATLAS_SIDE, TILE_ATLAS_TILE_PIXELS};

/// Phases in one full cycle: sixteen, the tile height.
pub const WATER_SCROLL_PHASE_COUNT: u8 = 16;

/// Rows the rotation advances per step.
pub const WATER_SCROLL_ROWS_PER_PHASE: usize = 1;

/// Rotation direction: `true` means the pattern travels toward higher y,
/// i.e. **downward** on screen. The single place the direction is encoded.
pub const WATER_SCROLL_TOWARD_HIGHER_Y: bool = true;

/// The ids stage one rotates: the three water ids and lava.
///
/// `cleak/u5-spec#179`: "The rotated set is exactly `0x01`, `0x02`, `0x03`
/// and `0x8F`", all period 16 on one counter, and lava "is literally the
/// same code path, not an analogue". Tile `0x00` is not in it.
pub const WATER_ROTATED_TILES: [u8; 4] = [0x01, 0x02, 0x03, 0x8F];

/// The tile stage two takes every composited id's water pixels from: the
/// shoals tile, rotated to the current phase.
pub const WATER_COMPOSITE_SOURCE_TILE: u8 = 0x03;

/// One contiguous run of composite destinations and the mask run it pairs
/// with, index for index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaterCompositeSet {
    /// First destination id.
    pub first_dest: u8,
    /// How many adjacent ids the set covers.
    pub count: u8,
    /// First mask id; destination `first_dest + n` uses `first_mask + n`.
    pub first_mask: u8,
    /// Whether the standalone inversion pass has already flipped these mask
    /// tiles by the time this set is composited.
    pub mask_inverted: bool,
}

/// The published composite destination sets.
///
/// `cleak/u5-spec#179` (01:48, as corrected at 02:07). Order matters only
/// as documentation of the original's pass order — the sets are disjoint,
/// so this engine can resolve any one of them on its own.
///
/// `0x70..=0x7F` are **also real drawable terrain** ("strange walls"); the
/// driver reuses them as masks. Anything enumerating drawable tiles must
/// not exclude that range on the strength of their mask role.
pub const WATER_COMPOSITE_SETS: [WaterCompositeSet; 3] = [
    // Rivers and bridges.
    WaterCompositeSet {
        first_dest: 0x60,
        count: 16,
        first_mask: 0x70,
        mask_inverted: false,
    },
    // Diagonal coast.
    WaterCompositeSet {
        first_dest: 0x34,
        count: 4,
        first_mask: 0xD0,
        mask_inverted: false,
    },
    // The same mask tiles, after the standalone inversion pass.
    WaterCompositeSet {
        first_dest: 0xE4,
        count: 4,
        first_mask: 0xD0,
        mask_inverted: true,
    },
];

/// Does stage one rotate this tile?
pub fn water_pass_rotates_tile(tile: u8) -> bool {
    WATER_ROTATED_TILES.contains(&tile)
}

/// The mask tile and polarity stage two rebuilds `tile` through, or `None`
/// if `tile` is not a composite destination.
pub fn water_composite_mask(tile: u8) -> Option<(u8, bool)> {
    WATER_COMPOSITE_SETS.iter().find_map(|set| {
        let offset = tile.checked_sub(set.first_dest)?;
        (offset < set.count).then(|| (set.first_mask + offset, set.mask_inverted))
    })
}

/// Does the water animator touch this tile at all, by either stage?
pub fn water_pass_animates_tile(tile: u8) -> bool {
    water_pass_rotates_tile(tile) || water_composite_mask(tile).is_some()
}

/// The one global counter both stages read.
///
/// Every animated tile on screen reads this same value — the measured
/// lockstep, which in the original falls out of both stages mutating the
/// shared tile asset. It is **not** a member of [`crate::AnimationClock`]:
/// that clock is the `animation.md §6` shared phase counter with its own
/// published period and nested gates, and this counter has neither.
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

    /// How many pixel rows the rotation is displaced by right now.
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

/// Stage one: rotate one tile's rows by `shift`, wrapping at the tile edge.
///
/// Every row is displaced by the same amount and no row moves sideways.
///
/// Returns `None` when `source` is not exactly one tile.
pub fn rotate_tile_rows_down(source: &[u8], shift: usize) -> Option<Vec<u8>> {
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

/// Stage two: `dest = (dest & !m) | (source & m)` on every colour plane,
/// where `m` is one boolean per pixel — the intensity bit of the mask
/// pixel — and `mask_inverted` flips that boolean.
///
/// The mask is read as a single byte per pixel group and broadcast to all
/// four planes, so a pixel is taken from the source or kept from the
/// destination **whole**; it is never blended plane by plane. See the
/// module docs for why the earlier per-plane reading is withdrawn and what
/// distinguishes the two.
///
/// `intensity_bit` is the atlas depth's top plane bit — `0x08` for the
/// sixteen-colour EGA sheet, `0x02` for the four-colour CGA one.
///
/// Returns `None` unless all three inputs are exactly one tile.
pub fn composite_tile_pixels(
    dest: &[u8],
    mask: &[u8],
    source: &[u8],
    mask_inverted: bool,
    intensity_bit: u8,
) -> Option<Vec<u8>> {
    if dest.len() != TILE_ATLAS_TILE_PIXELS
        || mask.len() != TILE_ATLAS_TILE_PIXELS
        || source.len() != TILE_ATLAS_TILE_PIXELS
    {
        return None;
    }
    let mut composed = vec![0u8; TILE_ATLAS_TILE_PIXELS];
    for index in 0..TILE_ATLAS_TILE_PIXELS {
        let take_source = ((mask[index] & intensity_bit) != 0) != mask_inverted;
        composed[index] = if take_source {
            source[index]
        } else {
            dest[index]
        };
    }
    Some(composed)
}

/// The plane bit the composite mask is read from, for an atlas depth.
///
/// The intensity plane is the top one: `0x08` of the sixteen-colour EGA
/// sheet's four planes, `0x02` of the four-colour CGA sheet's two.
pub const fn composite_mask_intensity_bit(pixel_limit: u8) -> u8 {
    pixel_limit >> 1
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
        let source: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i / TILE_ATLAS_SIDE) as u8)
            .collect();
        let row =
            |tile: &[u8], y: usize| tile[y * TILE_ATLAS_SIDE..(y + 1) * TILE_ATLAS_SIDE].to_vec();

        let phases: Vec<Vec<u8>> = (0..TILE_ATLAS_SIDE)
            .map(|k| rotate_tile_rows_down(&source, k).expect("one tile"))
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
    fn the_rotation_has_no_horizontal_component() {
        let source: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS).map(|i| i as u8).collect();
        let column = |tile: &[u8], x: usize| {
            (0..TILE_ATLAS_SIDE)
                .map(|y| tile[y * TILE_ATLAS_SIDE + x])
                .collect::<Vec<u8>>()
        };

        for shift in 0..TILE_ATLAS_SIDE {
            let rolled = rotate_tile_rows_down(&source, shift).expect("one tile");
            for x in 0..TILE_ATLAS_SIDE {
                let mut before = column(&source, x);
                let mut after = column(&rolled, x);
                before.sort_unstable();
                after.sort_unstable();
                assert_eq!(before, after, "column {x} keeps its own pixels at {shift}");
            }
        }
    }

    /// Phase zero draws the authored tile, and a whole cycle returns to it.
    #[test]
    fn a_whole_cycle_returns_the_authored_tile() {
        let source: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i % 251) as u8)
            .collect();

        assert_eq!(rotate_tile_rows_down(&source, 0).expect("one tile"), source);
        assert_eq!(
            rotate_tile_rows_down(&source, TILE_ATLAS_SIDE).expect("one tile"),
            source
        );
    }

    /// The rotated set is the three water ids and lava, and nothing else —
    /// in particular not `0x00`, not swamp, and not the composite
    /// destinations, which animate through the other stage.
    #[test]
    fn the_rotated_set_is_the_three_water_ids_and_lava() {
        assert_eq!(WATER_ROTATED_TILES, [0x01, 0x02, 0x03, 0x8F]);
        for tile in WATER_ROTATED_TILES {
            assert!(water_pass_rotates_tile(tile), "0x{tile:02x} rotates");
            assert_eq!(
                water_composite_mask(tile),
                None,
                "0x{tile:02x} is rotated, not composited"
            );
        }
        for tile in [0x00u8, 0x04, 0x05, 0x0A, 0x8E, 0x90, 0xD4, 0xEC, 0xFA] {
            assert!(
                !water_pass_rotates_tile(tile),
                "0x{tile:02x} must not rotate"
            );
        }
    }

    /// The composite sets, their index-for-index mask pairing, and the one
    /// set that is composited through the complement.
    #[test]
    fn the_composite_sets_pair_destinations_with_masks_index_for_index() {
        assert_eq!(WATER_COMPOSITE_SOURCE_TILE, 0x03);
        assert!(
            water_pass_rotates_tile(WATER_COMPOSITE_SOURCE_TILE),
            "the composite source is itself a rotated tile"
        );

        for (dest, mask, inverted) in [
            (0x60u8, 0x70u8, false),
            (0x6F, 0x7F, false),
            (0x34, 0xD0, false),
            (0x37, 0xD3, false),
            (0xE4, 0xD0, true),
            (0xE7, 0xD3, true),
        ] {
            assert_eq!(
                water_composite_mask(dest),
                Some((mask, inverted)),
                "0x{dest:02x}"
            );
        }

        // The two sets that share mask tiles must disagree about polarity;
        // one uniform rule would render `0xE4..=0xE7` inside-out.
        for offset in 0..4u8 {
            let (coast_mask, coast_inverted) = water_composite_mask(0x34 + offset).expect("coast");
            let (shore_mask, shore_inverted) = water_composite_mask(0xE4 + offset).expect("shore");
            assert_eq!(coast_mask, shore_mask, "the same mask tile");
            assert!(!coast_inverted);
            assert!(shore_inverted);
        }

        // Just outside every run.
        for tile in [0x33u8, 0x38, 0x5F, 0x70, 0xE3, 0xE8] {
            assert_eq!(water_composite_mask(tile), None, "0x{tile:02x}");
        }
    }

    /// The mask is one boolean per pixel, broadcast to every plane — not a
    /// per-plane blend. A mask pixel of `0b1101` has its intensity bit set,
    /// so the source is taken whole, green plane included.
    #[test]
    fn the_composite_mask_is_one_boolean_broadcast_to_every_plane() {
        let bit = composite_mask_intensity_bit(16);
        assert_eq!(bit, 0x08);

        let dest = vec![0b1111u8; TILE_ATLAS_TILE_PIXELS];
        let source = vec![0b0000u8; TILE_ATLAS_TILE_PIXELS];

        // Intensity set, green clear: the whole pixel comes from the source.
        // A per-plane reading would have kept the destination's green bit
        // and produced `0b0010`.
        let river_mask = vec![0b1101u8; TILE_ATLAS_TILE_PIXELS];
        let composed =
            composite_tile_pixels(&dest, &river_mask, &source, false, bit).expect("one tile");
        assert!(
            composed.iter().all(|value| *value == 0b0000),
            "the source is taken whole where the intensity bit is set"
        );

        // Intensity clear, every other bit set: the destination is kept
        // whole, even though three planes of the mask are set.
        let inverse_mask = vec![0b0111u8; TILE_ATLAS_TILE_PIXELS];
        let kept =
            composite_tile_pixels(&dest, &inverse_mask, &source, false, bit).expect("one tile");
        assert!(
            kept.iter().all(|value| *value == 0b1111),
            "only the intensity bit is consulted"
        );

        // The complement inverts that boolean.
        let inverted =
            composite_tile_pixels(&dest, &river_mask, &source, true, bit).expect("one tile");
        assert!(inverted.iter().all(|value| *value == 0b1111));

        // A four-colour sheet reads its own top plane.
        assert_eq!(composite_mask_intensity_bit(4), 0x02);
        let cga_mask = vec![0b10u8; TILE_ATLAS_TILE_PIXELS];
        let cga = composite_tile_pixels(&dest, &cga_mask, &source, false, 0x02).expect("one tile");
        assert!(cga.iter().all(|value| *value == 0b0000));
    }

    /// A solid mask reduces the composite to "take the source there", which
    /// is what the coast stencils do: they hold `0` and `15` only.
    #[test]
    fn a_solid_mask_selects_whole_pixels() {
        let dest = vec![0x0Au8; TILE_ATLAS_TILE_PIXELS];
        let source: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i % 16) as u8)
            .collect();
        // Lower-left half solid, the rest clear.
        let mut mask = vec![0u8; TILE_ATLAS_TILE_PIXELS];
        let mut solid = 0;
        for y in 0..TILE_ATLAS_SIDE {
            for x in 0..TILE_ATLAS_SIDE {
                if x <= y {
                    mask[y * TILE_ATLAS_SIDE + x] = 0x0F;
                    solid += 1;
                }
            }
        }
        assert_eq!(solid, TILE_ATLAS_SIDE * (TILE_ATLAS_SIDE + 1) / 2);

        let composed = composite_tile_pixels(&dest, &mask, &source, false, 0x0F).expect("one tile");
        for y in 0..TILE_ATLAS_SIDE {
            for x in 0..TILE_ATLAS_SIDE {
                let at = y * TILE_ATLAS_SIDE + x;
                if x <= y {
                    assert_eq!(composed[at], source[at], "({x},{y}) takes the source");
                } else {
                    assert_eq!(composed[at], dest[at], "({x},{y}) keeps the destination");
                }
            }
        }
    }

    /// Both stages read one counter and the source frame does not advance
    /// between destinations, so two composited ids are always at the same
    /// phase as each other and as the rotated tiles.
    #[test]
    fn every_destination_shows_the_same_phase_as_the_source() {
        let source: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i / TILE_ATLAS_SIDE) as u8)
            .collect();
        let mask = vec![0x0Fu8; TILE_ATLAS_TILE_PIXELS];
        let dest_a = vec![0x01u8; TILE_ATLAS_TILE_PIXELS];
        let dest_b = vec![0x02u8; TILE_ATLAS_TILE_PIXELS];

        for shift in 0..TILE_ATLAS_SIDE {
            let rotated = rotate_tile_rows_down(&source, shift).expect("one tile");
            let a = composite_tile_pixels(&dest_a, &mask, &rotated, false, 0x08).expect("a");
            let b = composite_tile_pixels(&dest_b, &mask, &rotated, false, 0x08).expect("b");
            assert_eq!(a, b, "shift {shift}: both destinations show one frame");
            assert_eq!(a, rotated, "and that frame is the rotated source");
        }
    }
}
