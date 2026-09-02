//! The display driver's fire animator: cumulative masked-noise XOR.
//!
//! # Provenance
//!
//! `systems/animation.md §12.4` ("The fire fixtures: cumulative masked-noise
//! XOR"), published on `cleak/u5-spec` HEAD. This is the third stage of the
//! same driver pass [`crate::water_scroll`] implements the first two of
//! (`§12.2` rotation, `§12.3` composites), and it shares that pass's gating
//! exactly — `§12.1`: "That call has no gate of its own; whenever the
//! animation step runs at all, this runs too (Section 13)."
//!
//! ## The two parts of one step
//!
//! `§12.4`: "First the driver refreshes four actor-half 'field' tiles —
//! atlas ids `0x1E8` (a poison field), `0x1E9` (a sleep field), `0x1EA` and
//! `0x1EB` (a force field) — with fresh pseudo-random pixel bits from a
//! generator the driver owns. These are the combat field-effect tiles, so
//! those fields are themselves re-randomised on every animation step. Then
//! it uses one of the refreshed tiles as a noise source and, for each fire
//! fixture, over the whole 16x16 tile:"
//!
//! ```text
//! fixture ^= (noise AND mask)
//! ```
//!
//! The fixture / mask / noise pairing is [`FIRE_FIXTURES`], transcribed from
//! that section's table. "Each mask is a small shape sitting exactly over
//! its fixture's flame, so only pixels inside the flame silhouette are ever
//! touched" — the silhouette is the shipped mask tile's own artwork, so
//! nothing about the flame geometry is hardcoded here.
//!
//! **Every id in `§12` is a tile-atlas index, not an actor byte.** The masks
//! `0xC0..0xC3` and `0xCC..0xCF` are terrain-half entries; the identical
//! numerals in the bestiary are Orc and Ettin *actor bytes*, whose atlas
//! entries are `0x1C0..0x1C3` and `0x1CC..0x1CF`. The four field tiles are
//! written `0x1E8..0x1EB` because they are actor-half indices.
//!
//! ## There is no frame set
//!
//! `§12.4`: "**There is no frame set to enumerate.** Any small-N frame-loop
//! model is wrong: a capture of ~1,900 sampled updates produced ~1,900
//! distinct frames, and a separate 5,000-sample run produced 2,755 distinct
//! states." So this stage cannot be a phase counter the way
//! [`crate::WaterScrollClock`] is; it needs real accumulated state, which is
//! [`FireFlickerClock`].
//!
//! ## Cumulative, and how this engine stores it
//!
//! `§12.4`: "**This XOR is cumulative and is never undone.** After the first
//! step the original artwork inside the masked region is gone. What renders
//! from then on is the shipped art XORed with every noise pattern
//! accumulated since the program started. An engine that re-derives each
//! frame from pristine artwork is statistically equivalent and visually
//! indistinguishable, but it is **not** bit-identical to the original."
//!
//! This engine keeps pristine artwork and composes at blit time, as it does
//! for the rotation and the composites — but it *does* reproduce the
//! accumulation, because the accumulation is what makes the effect unbounded
//! rather than a frame loop. The trick is that the whole history collapses
//! into one bit per pixel. Step *k* applies
//! `fixture[i] ^= bit_k[i] * (mask[i] AND planes)`, and XOR of a repeated
//! value is its parity, so after *n* steps
//!
//! ```text
//! fixture[i] = shipped[i] ^ (parity_n[i] * (mask[i] AND planes))
//! ```
//!
//! where `parity_n[i]` is the running XOR of every noise bit written at that
//! pixel. [`FireFlickerClock`] therefore stores one parity plane per field
//! tile — bit-exact with mutating the atlas in place, at 256 bytes per noise
//! tile instead of a mutable atlas.
//!
//! ## Which colour bits move
//!
//! `§12.4`, flagged there as "static derivation from shipped data, not
//! observed. Treat any capture as authoritative over this paragraph": "The
//! generator writes the same random byte into both of the two colour planes
//! that noise tile `0x1EA` occupies and leaves that tile's other two planes
//! at zero, which is also their shipped state. The net rule is that a masked
//! pixel's colour is XORed with `random_bit x (mask_pixel_colour AND 12)`,
//! and for `0xDE` with its different noise tile, `AND 9`."
//!
//! Those two constants are [`FIRE_NOISE_PLANES`] and
//! [`FIRE_SHRINE_NOISE_PLANES`]. Because the plane restriction is folded
//! into the noise tile's pixel values, the published net rule falls straight
//! out of `fixture ^= (noise AND mask)` — see [`FireFlickerClock::
//! fixture_frame`]. `§12.4` also warns: "do not infer flicker breadth from a
//! mask's colour without intersecting it with the noise tile's planes",
//! which is exactly what that intersection does.
//!
//! The plane occupancy of the other two field tiles, `0x1E8` and `0x1E9`, is
//! not published. [`fire_noise_tile_planes`] returns `None` for them and the
//! renderer falls back to their own shipped plane occupancy, which is the
//! conservative reading of "which is also their shipped state".
//!
//! ## The generator
//!
//! `§12.4` says only "a generator the driver owns". Its identity, seeding
//! and period are unpublished, and it is **not** the gameplay PRNG of
//! `systems/prng.md` — nothing published connects the two. [`FireFlickerClock`]
//! therefore uses a documented stand-in producing uniform independent bits,
//! which is all the published behaviour depends on: the measured
//! "~1,900 distinct frames from ~1,900 samples" and "about 12.8 of those 26
//! pixels change per update" are properties of a uniform bit source, not of
//! any particular generator. Nothing here should be read as a claim about
//! the original's generator.
//!
//! ## Not implemented, deliberately
//!
//! * **Bit parity with the original.** `§12.4` states outright that a
//!   pristine-art engine cannot have it, because the original's accumulation
//!   starts from whatever noise the process has seen since launch. Do not
//!   write a pixel-parity test against the original for these ids.
//! * **`§12.5`, the banner and sail row-pair swap.** Published as *probable*
//!   and explicitly awaiting a capture; still absent.

use crate::TILE_ATLAS_TILE_PIXELS;

/// `animation.md §12.4`: the four actor-half field tiles the step
/// re-randomises, in the published order — poison, sleep, and the two force
/// field tiles.
pub const FIRE_FIELD_TILES: [usize; FIRE_FIELD_TILE_COUNT] = [0x1E8, 0x1E9, 0x1EA, 0x1EB];

/// How many field tiles the step refreshes.
pub const FIRE_FIELD_TILE_COUNT: usize = 4;

/// The noise source every fixture but the shrine flame reads.
pub const FIRE_NOISE_TILE: usize = 0x1EA;

/// The noise source the shrine flame `0xDE` reads instead.
pub const FIRE_SHRINE_NOISE_TILE: usize = 0x1EB;

/// `animation.md §12.4`: the colour planes noise tile `0x1EA` occupies, so
/// a masked pixel is XORed with `random_bit x (mask_pixel_colour AND 12)`.
pub const FIRE_NOISE_PLANES: u8 = 12;

/// `animation.md §12.4`: the same quantity for `0x1EB`, "`AND 9`".
pub const FIRE_SHRINE_NOISE_PLANES: u8 = 9;

/// Pixels one planar byte covers. The generator "writes the same random
/// byte into both of the two colour planes", and a byte of planar artwork
/// is eight horizontally adjacent pixels, so bits are drawn eight at a time.
pub const FIRE_NOISE_PIXELS_PER_BYTE: usize = 8;

/// One row of the `animation.md §12.4` fixture table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireFixtureSpec {
    /// The terrain-half atlas id of the fixture whose flame flickers.
    pub tile: u8,
    /// The terrain-half atlas id whose artwork is the flame stencil.
    pub mask: u8,
    /// The actor-half atlas id of the field tile used as the noise source.
    pub noise: usize,
}

/// `animation.md §12.4`, verbatim from its table:
///
/// | Fixture id | Shipped name | Mask id | Noise source |
/// |---|---|---|---|
/// | `0xB0`, `0xB1` | A flickering torch | `0xC0`, `0xC1` | `0x1EA` |
/// | `0xB2` | A hot brazier | `0xC2` | `0x1EA` |
/// | `0xB3` | Meat roasting on a spit | `0xC3` | `0x1EA` |
/// | `0xBC` | A fireplace | `0xCC` | `0x1EA` |
/// | `0xBD` | A street lamp | `0xCD` | `0x1EA` |
/// | `0xBE` | A candelabrum | `0xCE` | `0x1EA` |
/// | `0xBF` | A hot stove | `0xCF` | `0x1EA` |
/// | `0xDE` | The shrine flame | `0xC2` | `0x1EB` |
///
/// Eight rows, nine ids: the torch row covers `0xB0` and `0xB1` against
/// `0xC0` and `0xC1` index for index. The shrine flame shares the brazier's
/// mask but not its noise tile.
///
/// No-fallback policy: every id below is published spec text, and
/// [`fire_fixture_spec`] has no catch-all for ids the section does not list.
pub const FIRE_FIXTURES: [FireFixtureSpec; 9] = [
    FireFixtureSpec {
        tile: 0xB0,
        mask: 0xC0,
        noise: FIRE_NOISE_TILE,
    },
    FireFixtureSpec {
        tile: 0xB1,
        mask: 0xC1,
        noise: FIRE_NOISE_TILE,
    },
    FireFixtureSpec {
        tile: 0xB2,
        mask: 0xC2,
        noise: FIRE_NOISE_TILE,
    },
    FireFixtureSpec {
        tile: 0xB3,
        mask: 0xC3,
        noise: FIRE_NOISE_TILE,
    },
    FireFixtureSpec {
        tile: 0xBC,
        mask: 0xCC,
        noise: FIRE_NOISE_TILE,
    },
    FireFixtureSpec {
        tile: 0xBD,
        mask: 0xCD,
        noise: FIRE_NOISE_TILE,
    },
    FireFixtureSpec {
        tile: 0xBE,
        mask: 0xCE,
        noise: FIRE_NOISE_TILE,
    },
    FireFixtureSpec {
        tile: 0xBF,
        mask: 0xCF,
        noise: FIRE_NOISE_TILE,
    },
    FireFixtureSpec {
        tile: 0xDE,
        mask: 0xC2,
        noise: FIRE_SHRINE_NOISE_TILE,
    },
];

/// The published fixture row for a terrain id, or `None` if the fire stage
/// does not touch it.
pub fn fire_fixture_spec(tile: u8) -> Option<FireFixtureSpec> {
    FIRE_FIXTURES
        .iter()
        .copied()
        .find(|fixture| fixture.tile == tile)
}

/// Does the fire stage rewrite this terrain id's pixels?
pub fn fire_pass_animates_tile(tile: u8) -> bool {
    fire_fixture_spec(tile).is_some()
}

/// Slot of an atlas id inside [`FIRE_FIELD_TILES`], or `None` for anything
/// that is not one of the four re-randomised field tiles.
pub fn fire_field_tile_slot(atlas_id: usize) -> Option<usize> {
    FIRE_FIELD_TILES.iter().position(|id| *id == atlas_id)
}

/// Is this atlas id one of the four field tiles the step re-randomises?
pub fn fire_pass_refreshes_field_tile(atlas_id: usize) -> bool {
    fire_field_tile_slot(atlas_id).is_some()
}

/// `animation.md §12.4`: the published colour planes a noise tile occupies.
///
/// Only `0x1EA` and `0x1EB` are published (`AND 12` and `AND 9`). The
/// section is silent about `0x1E8` and `0x1E9`, so this returns `None` for
/// them rather than guessing; callers fall back to
/// [`noise_tile_plane_mask`] over the tile's own shipped artwork, which is
/// the conservative reading of "which is also their shipped state".
pub const fn fire_noise_tile_planes(atlas_id: usize) -> Option<u8> {
    match atlas_id {
        FIRE_NOISE_TILE => Some(FIRE_NOISE_PLANES),
        FIRE_SHRINE_NOISE_TILE => Some(FIRE_SHRINE_NOISE_PLANES),
        _ => None,
    }
}

/// The colour planes a tile's shipped artwork occupies: the OR of every
/// pixel it holds.
///
/// `animation.md §12.4` describes the noise tiles' unwritten planes as
/// being "at zero, which is also their shipped state", so for a noise tile
/// this is exactly the set of planes the generator writes.
pub fn noise_tile_plane_mask(pristine: &[u8]) -> u8 {
    pristine.iter().fold(0u8, |planes, pixel| planes | *pixel)
}

/// Default state for the driver's own generator. Its real identity, seed
/// and period are unpublished (see the module docs); this value only makes
/// the stand-in deterministic across runs and tests.
pub const FIRE_FLICKER_DEFAULT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// The driver-side fire animator's accumulated state.
///
/// One entry per field tile of [`FIRE_FIELD_TILES`]:
///
/// * `current` is that tile's live per-pixel noise bit, which is what the
///   field tile itself draws as (`§12.4`: the combat field-effect tiles "are
///   themselves re-randomised on every animation step");
/// * `accumulated` is the running XOR parity of every noise bit ever written
///   at that pixel, which is the whole cumulative history of `§12.4`'s XOR
///   in one bit per pixel (see the module docs for why that is exact).
///
/// `§12.1`: "**The mutation persists.** It is not an overlay applied at draw
/// time and undone afterwards. It survives scene changes, save loads, and
/// everything else short of reloading the asset." Nothing on this type
/// resets it, and no caller should; this engine's remaining divergence is
/// that a fresh [`crate::PlayState`] starts a fresh clock, the same
/// divergence [`crate::WaterScrollClock`] already has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FireFlickerClock {
    generator: u64,
    current: [[u8; TILE_ATLAS_TILE_PIXELS]; FIRE_FIELD_TILE_COUNT],
    accumulated: [[u8; TILE_ATLAS_TILE_PIXELS]; FIRE_FIELD_TILE_COUNT],
    steps: u32,
}

impl Default for FireFlickerClock {
    fn default() -> Self {
        Self::seeded(FIRE_FLICKER_DEFAULT_SEED)
    }
}

impl FireFlickerClock {
    /// A clock whose stand-in generator starts from `seed`, with no noise
    /// drawn yet: every field tile still shows its shipped artwork and no
    /// fixture has been XORed.
    pub fn seeded(seed: u64) -> Self {
        Self {
            generator: seed,
            current: [[0u8; TILE_ATLAS_TILE_PIXELS]; FIRE_FIELD_TILES.len()],
            accumulated: [[0u8; TILE_ATLAS_TILE_PIXELS]; FIRE_FIELD_TILES.len()],
            steps: 0,
        }
    }

    /// How many animation steps this clock has run.
    pub const fn steps(&self) -> u32 {
        self.steps
    }

    /// One driver animation step.
    ///
    /// `§12.4` part one: all four field tiles take "fresh pseudo-random
    /// pixel bits". Part two — the per-fixture XOR — needs no work here,
    /// because folding this step's bits into the parity plane is exactly
    /// what `fixture ^= (noise AND mask)` does to every fixture reading that
    /// tile at once. `§12.3`'s "the source frame is not advanced between
    /// destinations" has the same shape here: every fixture on one noise
    /// tile sees the same draw.
    pub fn tick(&mut self) {
        for slot in 0..FIRE_FIELD_TILE_COUNT {
            for group in 0..(TILE_ATLAS_TILE_PIXELS / FIRE_NOISE_PIXELS_PER_BYTE) {
                let byte = self.next_noise_byte();
                for bit in 0..FIRE_NOISE_PIXELS_PER_BYTE {
                    // Planar artwork stores the leftmost pixel in the most
                    // significant bit.
                    let pixel = group * FIRE_NOISE_PIXELS_PER_BYTE + bit;
                    let value = (byte >> (FIRE_NOISE_PIXELS_PER_BYTE - 1 - bit)) & 1;
                    self.current[slot][pixel] = value;
                    self.accumulated[slot][pixel] ^= value;
                }
            }
        }
        self.steps = self.steps.wrapping_add(1);
    }

    /// The live per-pixel noise bit of one field tile, or `None` for an id
    /// that is not one of the four.
    pub fn field_tile_noise_bits(&self, atlas_id: usize) -> Option<&[u8; TILE_ATLAS_TILE_PIXELS]> {
        fire_field_tile_slot(atlas_id).map(|slot| &self.current[slot])
    }

    /// The accumulated XOR parity of one field tile, or `None` for an id
    /// that is not one of the four. This is the whole cumulative history of
    /// `§12.4`'s XOR for every fixture reading that noise tile.
    pub fn accumulated_noise_bits(&self, atlas_id: usize) -> Option<&[u8; TILE_ATLAS_TILE_PIXELS]> {
        fire_field_tile_slot(atlas_id).map(|slot| &self.accumulated[slot])
    }

    /// `§12.4` part one: what a field tile draws as right now.
    ///
    /// The tile's whole content is the fresh noise — the occupied planes
    /// take the random byte and the others stay at zero — so `planes` is the
    /// tile's plane occupancy, from [`fire_noise_tile_planes`] where
    /// published and [`noise_tile_plane_mask`] over the shipped art
    /// otherwise.
    ///
    /// Returns `None` for an id that is not a field tile.
    pub fn field_tile_frame(&self, atlas_id: usize, planes: u8) -> Option<Vec<u8>> {
        let bits = self.field_tile_noise_bits(atlas_id)?;
        Some(
            bits.iter()
                .map(|bit| if *bit != 0 { planes } else { 0 })
                .collect(),
        )
    }

    /// `§12.4` part two: what a fire fixture draws as right now.
    ///
    /// `pristine` is the fixture's shipped artwork, `mask` its shipped mask
    /// tile's artwork, and `planes` the noise tile's colour planes. The
    /// published net rule — "a masked pixel's colour is XORed with
    /// `random_bit x (mask_pixel_colour AND 12)`" — is this expression with
    /// `random_bit` replaced by the accumulated parity, which is what makes
    /// the XOR cumulative rather than a two-state blink.
    ///
    /// Pixels the mask leaves clear are copied through untouched: "Each mask
    /// is a small shape sitting exactly over its fixture's flame, so only
    /// pixels inside the flame silhouette are ever touched."
    ///
    /// Returns `None` unless `tile` is a published fixture and both inputs
    /// are exactly one tile.
    pub fn fixture_frame(
        &self,
        tile: u8,
        pristine: &[u8],
        mask: &[u8],
        planes: u8,
    ) -> Option<Vec<u8>> {
        let fixture = fire_fixture_spec(tile)?;
        if pristine.len() != TILE_ATLAS_TILE_PIXELS || mask.len() != TILE_ATLAS_TILE_PIXELS {
            return None;
        }
        let parity = self.accumulated_noise_bits(fixture.noise)?;
        Some(
            (0..TILE_ATLAS_TILE_PIXELS)
                .map(|index| {
                    let noise = if parity[index] != 0 { planes } else { 0 };
                    pristine[index] ^ (noise & mask[index])
                })
                .collect(),
        )
    }

    /// One draw from the stand-in generator described in the module docs.
    fn next_noise_byte(&mut self) -> u8 {
        self.generator = self.generator.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.generator;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 24) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TILE_ATLAS_SIDE;

    /// A stand-in for a shipped mask tile: a small flame silhouette in one
    /// colour, everything else clear. `§12.4` measured the brazier's
    /// animated region at "26 pixels of the tile's 256, confined to rows 2
    /// through 6 at four to six pixels per row", so this is that shape.
    fn brazier_shaped_mask(colour: u8) -> Vec<u8> {
        let mut mask = vec![0u8; TILE_ATLAS_TILE_PIXELS];
        for (row, width) in [(2usize, 4usize), (3, 5), (4, 6), (5, 6), (6, 5)] {
            for x in 0..width {
                mask[row * TILE_ATLAS_SIDE + 5 + x] = colour;
            }
        }
        mask
    }

    fn masked_pixel_indices(mask: &[u8], planes: u8) -> Vec<usize> {
        (0..TILE_ATLAS_TILE_PIXELS)
            .filter(|index| (mask[*index] & planes) != 0)
            .collect()
    }

    /// The published fixture table, id for id, including the two details a
    /// reader gets wrong most easily: the torch row is two ids against two
    /// masks, and the shrine flame reuses the brazier's mask with a
    /// *different* noise tile.
    #[test]
    fn the_fixture_table_matches_the_published_rows() {
        for (tile, mask, noise) in [
            (0xB0u8, 0xC0u8, FIRE_NOISE_TILE),
            (0xB1, 0xC1, FIRE_NOISE_TILE),
            (0xB2, 0xC2, FIRE_NOISE_TILE),
            (0xB3, 0xC3, FIRE_NOISE_TILE),
            (0xBC, 0xCC, FIRE_NOISE_TILE),
            (0xBD, 0xCD, FIRE_NOISE_TILE),
            (0xBE, 0xCE, FIRE_NOISE_TILE),
            (0xBF, 0xCF, FIRE_NOISE_TILE),
            (0xDE, 0xC2, FIRE_SHRINE_NOISE_TILE),
        ] {
            assert_eq!(
                fire_fixture_spec(tile),
                Some(FireFixtureSpec { tile, mask, noise }),
                "0x{tile:02x}"
            );
        }
        assert_eq!(FIRE_FIXTURES.len(), 9);
        assert_eq!(FIRE_FIELD_TILES, [0x1E8, 0x1E9, 0x1EA, 0x1EB]);
        assert_eq!(fire_noise_tile_planes(FIRE_NOISE_TILE), Some(12));
        assert_eq!(fire_noise_tile_planes(FIRE_SHRINE_NOISE_TILE), Some(9));
        // `§12.4` publishes no plane occupancy for the poison and sleep
        // field tiles, so nothing is asserted for them.
        assert_eq!(fire_noise_tile_planes(0x1E8), None);
        assert_eq!(fire_noise_tile_planes(0x1E9), None);
    }

    /// The masks are terrain-half ids, and the fire stage must not claim the
    /// actor-half sprite runs that share their numerals — `§12` preamble:
    /// "An engine that runs the two together XOR-flickers its monsters and
    /// draws stencils where creatures should be."
    #[test]
    fn no_non_fixture_terrain_id_is_animated_by_the_fire_stage() {
        for tile in 0x00u8..=0xFF {
            let animated = fire_pass_animates_tile(tile);
            let published = matches!(tile, 0xB0..=0xB3 | 0xBC..=0xBF | 0xDE);
            assert_eq!(animated, published, "0x{tile:02x}");
        }
        // The mask ids themselves are stencils, not fixtures.
        for mask in (0xC0u8..=0xC3).chain(0xCC..=0xCF) {
            assert!(!fire_pass_animates_tile(mask), "mask 0x{mask:02x}");
        }
    }

    /// `§12.4`: the four field tiles are "themselves re-randomised on every
    /// animation step", and nothing else is a field tile.
    #[test]
    fn exactly_the_four_field_tiles_are_refreshed() {
        for atlas_id in 0x1E0usize..=0x1EF {
            assert_eq!(
                fire_pass_refreshes_field_tile(atlas_id),
                (0x1E8..=0x1EB).contains(&atlas_id),
                "0x{atlas_id:03X}"
            );
        }
        // Terrain-half `0xE8..0xEB` is the hourglass, which `§6.2` says is
        // genuinely static; the field tiles are the actor-half ids.
        for terrain in 0xE8usize..=0xEB {
            assert!(!fire_pass_refreshes_field_tile(terrain));
        }
    }

    /// A field tile's own pixels change on every step: `§12.4` part one.
    #[test]
    fn every_step_re_randomises_all_four_field_tiles() {
        let mut clock = FireFlickerClock::default();
        let mut previous: Vec<Vec<u8>> = FIRE_FIELD_TILES
            .iter()
            .map(|id| {
                clock
                    .field_tile_frame(*id, 12)
                    .expect("a published field tile")
            })
            .collect();

        let mut changed = [0usize; 4];
        for _ in 0..64 {
            clock.tick();
            for (slot, id) in FIRE_FIELD_TILES.iter().enumerate() {
                let now = clock.field_tile_frame(*id, 12).expect("a field tile");
                if now != previous[slot] {
                    changed[slot] += 1;
                }
                previous[slot] = now;
            }
        }
        for (slot, count) in changed.iter().enumerate() {
            assert_eq!(
                *count, 64,
                "field tile 0x{:03X} must change on every step",
                FIRE_FIELD_TILES[slot]
            );
        }
        assert_eq!(clock.steps(), 64);
    }

    /// A fire tile changes between successive ticks, and about half the
    /// pixels the mask admits change each time. `§12.4` measured "about 12.8
    /// of those 26 pixels change per update" on the brazier — half, which is
    /// what a uniform noise bit gives.
    #[test]
    fn a_fire_tile_changes_about_half_its_masked_pixels_per_tick() {
        let pristine: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i % 16) as u8)
            .collect();
        let mask = brazier_shaped_mask(FIRE_NOISE_PLANES);
        let masked = masked_pixel_indices(&mask, FIRE_NOISE_PLANES);
        assert_eq!(masked.len(), 26, "the measured brazier region");

        let mut clock = FireFlickerClock::default();
        let mut previous = clock
            .fixture_frame(0xB2, &pristine, &mask, FIRE_NOISE_PLANES)
            .expect("a published fixture");

        let steps = 400usize;
        let mut differing_steps = 0usize;
        let mut total_changed = 0usize;
        for _ in 0..steps {
            clock.tick();
            let now = clock
                .fixture_frame(0xB2, &pristine, &mask, FIRE_NOISE_PLANES)
                .expect("a published fixture");
            let changed = masked.iter().filter(|i| now[**i] != previous[**i]).count();
            total_changed += changed;
            if changed > 0 {
                differing_steps += 1;
            }
            previous = now;
        }

        assert_eq!(
            differing_steps, steps,
            "every step must change the fixture: 26 pixels each flipping \
             with probability one half never all stay put"
        );
        let mean = total_changed as f64 / steps as f64;
        let half = masked.len() as f64 / 2.0;
        assert!(
            (mean - half).abs() < 2.0,
            "about half the {} masked pixels should change per step, saw {mean:.2}",
            masked.len()
        );
    }

    /// Pixels outside the mask are never touched, at any step, for any
    /// fixture. `§12.4`: "only pixels inside the flame silhouette are ever
    /// touched."
    #[test]
    fn pixels_outside_the_mask_never_change() {
        let pristine: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i % 16) as u8)
            .collect();
        let mut clock = FireFlickerClock::default();

        for _ in 0..200 {
            clock.tick();
            for fixture in FIRE_FIXTURES {
                let planes =
                    fire_noise_tile_planes(fixture.noise).expect("both noise tiles are published");
                // A mask with every colour bit set everywhere inside the
                // silhouette is the widest a real mask can be, so this is
                // the strongest form of the negative.
                let mask = brazier_shaped_mask(0x0F);
                let frame = clock
                    .fixture_frame(fixture.tile, &pristine, &mask, planes)
                    .expect("a published fixture");
                for index in 0..TILE_ATLAS_TILE_PIXELS {
                    if mask[index] == 0 {
                        assert_eq!(
                            frame[index], pristine[index],
                            "0x{:02x} pixel {index} is outside the mask",
                            fixture.tile
                        );
                    }
                }
            }
        }
    }

    /// Inside the mask, only the bits the noise tile's planes carry ever
    /// move. `§12.4`: "do not infer flicker breadth from a mask's colour
    /// without intersecting it with the noise tile's planes."
    #[test]
    fn only_the_noise_tiles_own_planes_ever_flip() {
        let pristine = vec![0x0Fu8; TILE_ATLAS_TILE_PIXELS];
        let mask = vec![0x0Fu8; TILE_ATLAS_TILE_PIXELS];
        let mut clock = FireFlickerClock::default();

        for _ in 0..64 {
            clock.tick();
            let torch = clock
                .fixture_frame(0xB0, &pristine, &mask, FIRE_NOISE_PLANES)
                .expect("torch");
            let shrine = clock
                .fixture_frame(0xDE, &pristine, &mask, FIRE_SHRINE_NOISE_PLANES)
                .expect("shrine flame");
            for index in 0..TILE_ATLAS_TILE_PIXELS {
                assert_eq!(
                    (torch[index] ^ pristine[index]) & !FIRE_NOISE_PLANES,
                    0,
                    "0x1EA supplies only planes 0b1100"
                );
                assert_eq!(
                    (shrine[index] ^ pristine[index]) & !FIRE_SHRINE_NOISE_PLANES,
                    0,
                    "0x1EB supplies only planes 0b1001"
                );
            }
        }
    }

    /// The XOR is cumulative, not a small-N frame loop. `§12.4`: "a capture
    /// of ~1,900 sampled updates produced ~1,900 distinct frames".
    #[test]
    fn the_accumulated_xor_is_not_a_small_frame_loop() {
        let pristine = vec![0x0Cu8; TILE_ATLAS_TILE_PIXELS];
        let mask = brazier_shaped_mask(FIRE_NOISE_PLANES);
        let mut clock = FireFlickerClock::default();

        let mut seen = std::collections::HashSet::new();
        let samples = 1_000usize;
        for _ in 0..samples {
            clock.tick();
            seen.insert(
                clock
                    .fixture_frame(0xB2, &pristine, &mask, FIRE_NOISE_PLANES)
                    .expect("brazier"),
            );
        }
        // 26 masked pixels give 2^26 states; a frame-loop model would give
        // a handful. Anything short of near-injective sampling is wrong.
        assert!(
            seen.len() > samples * 9 / 10,
            "expected nearly every sample to be a fresh frame, got {} of {samples}",
            seen.len()
        );
    }

    /// Every fixture on one noise tile sees the same draw, so their masked
    /// regions move together — the fire stage's counterpart of `§12.3`'s
    /// "the source frame is not advanced between destinations".
    #[test]
    fn fixtures_sharing_a_noise_tile_share_its_parity() {
        let mut clock = FireFlickerClock::default();
        for _ in 0..8 {
            clock.tick();
        }
        let pristine = vec![0u8; TILE_ATLAS_TILE_PIXELS];
        let mask = vec![0x0Fu8; TILE_ATLAS_TILE_PIXELS];

        let torch = clock
            .fixture_frame(0xB0, &pristine, &mask, FIRE_NOISE_PLANES)
            .expect("torch");
        let stove = clock
            .fixture_frame(0xBF, &pristine, &mask, FIRE_NOISE_PLANES)
            .expect("stove");
        assert_eq!(torch, stove, "both read 0x1EA");

        let shrine = clock
            .fixture_frame(0xDE, &pristine, &mask, FIRE_NOISE_PLANES)
            .expect("shrine flame");
        assert_ne!(
            shrine, torch,
            "the shrine flame reads 0x1EB, an independent draw"
        );
    }

    /// A fresh clock has drawn no noise, so every fixture still shows its
    /// shipped artwork exactly.
    #[test]
    fn a_fresh_clock_shows_the_shipped_artwork() {
        let clock = FireFlickerClock::default();
        let pristine: Vec<u8> = (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| (i % 16) as u8)
            .collect();
        let mask = vec![0x0Fu8; TILE_ATLAS_TILE_PIXELS];
        for fixture in FIRE_FIXTURES {
            let planes = fire_noise_tile_planes(fixture.noise).expect("published");
            assert_eq!(
                clock
                    .fixture_frame(fixture.tile, &pristine, &mask, planes)
                    .expect("a published fixture"),
                pristine,
                "0x{:02x}",
                fixture.tile
            );
        }
        assert_eq!(clock.steps(), 0);
    }

    /// The fire stage refuses ids it does not own and inputs that are not
    /// one tile, rather than guessing.
    #[test]
    fn the_fire_stage_refuses_ids_and_shapes_it_does_not_own() {
        let clock = FireFlickerClock::default();
        let tile = vec![0u8; TILE_ATLAS_TILE_PIXELS];
        assert_eq!(clock.fixture_frame(0xC0, &tile, &tile, 12), None);
        assert_eq!(clock.fixture_frame(0xB4, &tile, &tile, 12), None);
        assert_eq!(clock.fixture_frame(0xB0, &tile[..4], &tile, 12), None);
        assert_eq!(clock.fixture_frame(0xB0, &tile, &tile[..4], 12), None);
        assert_eq!(clock.field_tile_frame(0x1EC, 12), None);
        assert_eq!(clock.field_tile_noise_bits(0x0EA), None);
    }

    /// The plane-occupancy fallback for the two unpublished field tiles is
    /// the OR of the shipped artwork.
    #[test]
    fn plane_occupancy_falls_out_of_the_shipped_artwork() {
        let mut art = vec![0u8; TILE_ATLAS_TILE_PIXELS];
        art[3] = 0b1000;
        art[9] = 0b0100;
        assert_eq!(noise_tile_plane_mask(&art), 0b1100);
        assert_eq!(noise_tile_plane_mask(&[0u8; 4]), 0);
    }
}
