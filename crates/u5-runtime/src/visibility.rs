//! Visibility-grid byte-marker classifier per `visibility.md` §2 and the
//! producer's light-radius branch per §4. The producer writes one byte
//! per active cell; the renderer dispatches on these markers to choose
//! between fully-obscured, dim-periphery, clear-visible, companion-buffer,
//! "already rendered", or direct-tile-id presentation.

/// `visibility.md §2`: side length of the active visibility window
/// (eleven by eleven cells around the player).
pub const VIEWPORT_SIDE: usize = 11;
/// `visibility.md §2`: row stride of the visibility grid in bytes
/// (eleven active cells plus twenty-one trailing scratch bytes).
/// Same 32-byte stride the combat arena row uses (11 visible
/// terrain + 21 metadata bytes per formats/cbt.md §3). Anchored
/// to [`crate::COMBAT_ARENA_ROW_STRIDE`] so the two 11-cell-
/// viewport-with-scratch row strides share one source of truth.
pub const VIEWPORT_ROW_STRIDE: usize = crate::COMBAT_ARENA_ROW_STRIDE;
/// `visibility.md §2`: row stride of the terrain band in bytes.
pub const TERRAIN_BAND_ROW_STRIDE: usize = 16;
/// `visibility.md §2`: total visibility-grid backing bytes. Each row has
/// eleven active viewport bytes and trailing scratch bytes.
pub const VISIBILITY_GRID_LEN: usize = VIEWPORT_ROW_STRIDE * VIEWPORT_SIDE;
/// `visibility.md §2`: total companion terrain-band bytes.
pub const TERRAIN_BAND_LEN: usize = TERRAIN_BAND_ROW_STRIDE * VIEWPORT_SIDE;
/// `visibility.md §2`: zero-based player position inside the active
/// window (centre row, centre column). Anchored to
/// `(VIEWPORT_SIDE - 1) / 2` so the centre position derives from
/// the viewport side dimension; changing the viewport size
/// automatically re-centres the player.
pub const VIEWPORT_PLAYER_ROW: usize = (VIEWPORT_SIDE - 1) / 2;
pub const VIEWPORT_PLAYER_COL: usize = (VIEWPORT_SIDE - 1) / 2;

/// `visibility.md §2`: well-known visibility-grid byte markers.
/// The fully-obscured marker is the largest representable byte
/// value (`0xFF`) — the renderer treats this as "leave the
/// previous-frame pixels untouched". Anchored to [`u8::MAX`] so
/// the hidden sentinel derives from the byte width rather than
/// restating `0xFF` as a bare literal.
pub const VISIBILITY_HIDDEN: u8 = u8::MAX;
pub const VISIBILITY_USE_COMPANION: u8 = 0x00;
pub const VISIBILITY_CLEAR: u8 = 0xDD;
pub const VISIBILITY_DIM_PERIPHERY: u8 = 0x1C;
pub const VISIBILITY_ALREADY_RENDERED: u8 = 0x87;

/// `visibility.md §2`: index into the active eleven-cell portion of the
/// 32-byte-stride visibility grid.
pub const fn visibility_grid_active_index(row: usize, col: usize) -> Option<usize> {
    if row < VIEWPORT_SIDE && col < VIEWPORT_SIDE {
        Some(row * VIEWPORT_ROW_STRIDE + col)
    } else {
        None
    }
}

/// `visibility.md §2`: index into the active eleven-cell portion of the
/// 16-byte-stride terrain companion band.
pub const fn terrain_band_active_index(row: usize, col: usize) -> Option<usize> {
    if row < VIEWPORT_SIDE && col < VIEWPORT_SIDE {
        Some(row * TERRAIN_BAND_ROW_STRIDE + col)
    } else {
        None
    }
}

/// `visibility.md §10` cheap-path lazy-refill predicate. On a clean
/// frame, the redraw orchestrator walks the 11x11 active window and
/// requests a fresh world tile for any cell whose current byte is
/// zero; cells with any other value are left alone (they retain
/// their fog markers and active-object stamps from the previous
/// expensive recompute). Zero is the active-object compositor's
/// "needs a fresh tile this frame" hint.
pub const fn visibility_cheap_path_needs_refill(grid_byte: u8) -> bool {
    grid_byte == VISIBILITY_USE_COMPANION
}

/// `visibility.md §2` semantic classification of a visibility-grid byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibilityMarker {
    /// `0xFF` — fully obscured; renderer leaves previous-frame pixels.
    Hidden,
    /// `0x00` — visible, paint from the terrain band companion buffer.
    UseCompanion,
    /// `0xDD` — clear visible, full brightness.
    ClearVisible,
    /// `0x1C` — dim periphery on the visibility-radius boundary.
    DimPeriphery,
    /// `0x87` — already-rendered guard for the active-object compositor.
    AlreadyRendered,
    /// Direct tile id or renderer-specific marker (any other byte).
    DirectTile(u8),
}

/// `visibility.md §2`: classify one visibility-grid byte.
pub const fn visibility_marker(byte: u8) -> VisibilityMarker {
    match byte {
        VISIBILITY_HIDDEN => VisibilityMarker::Hidden,
        VISIBILITY_USE_COMPANION => VisibilityMarker::UseCompanion,
        VISIBILITY_CLEAR => VisibilityMarker::ClearVisible,
        VISIBILITY_DIM_PERIPHERY => VisibilityMarker::DimPeriphery,
        VISIBILITY_ALREADY_RENDERED => VisibilityMarker::AlreadyRendered,
        other => VisibilityMarker::DirectTile(other),
    }
}

/// `visibility.md §12` local-light mask side dimension. The mask
/// covers the full 32x32 active map window (not the 11x11 viewport)
/// because local lights placed outside the viewport can still reach
/// in. Anchored to [`crate::TOWN_GRID_SIDE`] so the local-light
/// mask and the active map window share one source of truth.
pub const LOCAL_LIGHT_MASK_SIDE: usize = crate::TOWN_GRID_SIDE;

/// `visibility.md §12.2`: the fixed per-source local-light range. This
/// is a **squared-distance threshold**, not a cell radius: a cell is
/// inside a source's light when `dx*dx + dy*dy <= 10`.
///
/// That is a Euclidean disc of radius sqrt(10) (about 3.16) covering
/// **37 cells** — every offset with `|dx| <= 3` and `|dy| <= 3` *except*
/// the twelve offsets `(±3, ±3)`, `(±3, ±2)` and `(±2, ±3)`. The
/// original looks the distance up from a small folded table, but the
/// table is exactly `dx*dx + dy*dy`.
///
/// An earlier revision of this engine modelled the range as Chebyshev
/// distance 3 (a 7x7 square, 49 cells); `cleak/u5-spec#42` retracted
/// that reading against re-verified binary traces. Corner cells such as
/// `(±3, ±3)` are *not* lit.
pub const LOCAL_LIGHT_SOURCE_SQUARED_THRESHOLD: u32 = 10;

/// `visibility.md §12.2`: number of cells one unobstructed local-light
/// source covers — the 37 offsets satisfying
/// [`LOCAL_LIGHT_SOURCE_SQUARED_THRESHOLD`]. Kept beside the threshold so
/// a change to one is caught against the other.
pub const LOCAL_LIGHT_SOURCE_CELL_COUNT: usize = 37;

/// `visibility.md §12`: returns `true` for tile ids the resident
/// local-light refresh recognises as local-light source candidates.
/// The shipped lookup is `0xB0..=0xB3`, `0xBC..=0xBF`, `0xDC`, and
/// `0xDE`. Other tiles are not treated as light sources.
pub const fn is_local_light_source_tile(tile: u8) -> bool {
    matches!(tile, 0xB0..=0xB3 | 0xBC..=0xBF | 0xDC | 0xDE)
}

/// `visibility.md §8` active-object compositor branch the helper
/// dispatches an active-object slot through. The default branch is
/// the catch-all that passes through the terrain-aware stamp helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveObjectCompositorBranch {
    /// `0xE8..=0xEB` or exact `0x1E`/`0x1F` — water-bound
    /// companion-band class. Stamps the slot's frame byte into the
    /// terrain band and writes the use-companion marker into the
    /// visibility grid.
    WaterBoundCompanion,
    /// Frame byte exactly `0x1D` or `0x1E` — water-creature
    /// companion-band class. Same companion-band stamp.
    WaterCreatureCompanion,
    /// Type byte exactly `0x5C` — the **single-sprite-family seated
    /// branch**. The caller still has to check that the terrain byte
    /// standing in the visibility-grid cell is the chair id `0x92`
    /// before stamping the companion path.
    ///
    /// `RETRACTIONS.md` R330 withdraws the earlier "vehicle/avatar-family
    /// companion branch" reading in full: "`0x5C` is **one ordinary NPC
    /// sprite family** (the tile-name table calls it a minstrel), not a
    /// vehicle or avatar marker, and the party's own type byte is the
    /// party sprite marker, which is **never `0x5C` outside combat** — so
    /// no vehicle and no avatar ever reaches this arm in a world scene.
    /// The `0x92` it tests is the **chair terrain id** standing in the
    /// grid cell, not a marker. The arm's meaning is that an actor of that
    /// one family seated on a chair of that facing keeps its own sprite
    /// instead of merging into an occupied-chair tile; off that terrain
    /// the slot falls through to the default helper with its frame byte
    /// **reduced by eight**, a remap recorded as observed and not
    /// explained."
    SingleSpriteFamilySeated,
    /// Anything else — falls through to the default tile compositor
    /// helper.
    DefaultHelper,
}

/// `visibility.md §8`: classify a slot by its `(type, frame)` byte
/// pair. The water-bound branch is keyed by the type byte; the
/// water-creature branch is keyed by the frame byte; the
/// single-sprite-family seated branch is keyed by the type byte alone
/// (caller applies the chair-terrain check).
pub const fn active_object_compositor_branch(
    type_byte: u8,
    frame_byte: u8,
) -> ActiveObjectCompositorBranch {
    if (type_byte >= 0xE8 && type_byte <= 0xEB) || type_byte == 0x1E || type_byte == 0x1F {
        return ActiveObjectCompositorBranch::WaterBoundCompanion;
    }
    if frame_byte == 0x1D || frame_byte == 0x1E {
        return ActiveObjectCompositorBranch::WaterCreatureCompanion;
    }
    if type_byte == 0x5C {
        return ActiveObjectCompositorBranch::SingleSpriteFamilySeated;
    }
    ActiveObjectCompositorBranch::DefaultHelper
}

/// `visibility.md §8`: the chair terrain id the single-sprite-family
/// seated branch tests. The branch "stamps the slot's frame byte into the
/// terrain band" only when "the terrain byte standing in the
/// visibility-grid cell is the chair id `0x92`".
///
/// `RETRACTIONS.md` R330 renamed the value: it is the **chair terrain
/// id**, not a vehicle/avatar underlay marker.
pub const SINGLE_SPRITE_FAMILY_SEATED_CHAIR_TERRAIN: u8 = 0x92;

/// `visibility.md §8` / `RETRACTIONS.md` R330: "When the type byte is
/// `0x5C` but the terrain is anything else, the slot goes to the default
/// helper with its frame byte reduced by eight, remapping it to a
/// different sprite family; what that remap is *for* has not been
/// established, only that it happens."
pub const SINGLE_SPRITE_FAMILY_SEATED_FRAME_FALLTHROUGH_DECREMENT: u8 = 8;

/// `visibility.md §8` result of stamping one active-object slot into the
/// visibility/terrain-band pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveObjectCompositeResult {
    /// Leave the existing grid and terrain-band cells unchanged.
    Suppress,
    /// Write the tile to the terrain band and set the visibility grid to
    /// `VISIBILITY_USE_COMPANION`.
    Companion(u8),
    /// Write a direct visibility-grid marker/tile and leave the terrain band
    /// untouched.
    Direct(u8),
    /// Write a direct marker into the previous viewport row, then companion-
    /// stamp this cell.
    PreviousRowDirectAndCompanion { previous_marker: u8, tile: u8 },
}

/// `visibility.md §8`: the small set of effective tile bytes that the default
/// compositor compares against the current terrain before stamping.
pub const fn active_object_default_tile_is_terrain_aware(tile: u8) -> bool {
    tile == 0x1C || (tile >= 0x12 && tile <= 0x15) || (tile >= 0x28 && tile <= 0x2B) || tile >= 0x40
}

/// `visibility.md §8`: four-entry selector for terrain edge variants —
/// "**unless the Negate Time timed effect is active, select a uniform random
/// entry from the four-value range; while it is active, the selector
/// short-circuits and returns the first entry for every actor.**"
///
/// The short-circuit input is the single **global timed-magic-effect code**
/// byte, not anything about a party member. `visibility.md §8` retracts the
/// earlier revision that said the first entry is selected "when the current
/// active character's class letter is Tinker": "There is no character-class
/// input to this selector. The byte it tests is the single global
/// timed-magic-effect code, and the value that short-circuits it is the one
/// Negate Time writes; the resemblance is that both are stored as a letter.
/// An implementation that wired this to the party's classes will pick variant
/// 0 for the wrong reason and will animate through Negate Time."
///
/// That byte has **two** producers (`RETRACTIONS.md` R333): "the **Negate Time
/// spell** handler, which writes the code as an immediate together with the
/// effect's ten-turn duration; the shared **timed-effect setter**, which
/// writes the code from its argument and is passed this code by exactly one of
/// its call sites, the **Negate Time scroll**, with a twenty-turn duration" —
/// and one further writer installs "a different timed-effect code into the
/// same byte — this is a shared timed-effect register that other effects also
/// write, not a Negate Time flag".
///
/// `visibility.md §8.2`: "The composite still runs while Negate Time is
/// active; it just draws variant 0 every time." — so this is a selector
/// short-circuit, not a suppression of the stamp. `§8.1` adds that the
/// generator advance is "unconditional whenever the selector is entered on its
/// random arm", so the short-circuit arm takes no draw at all.
pub const fn active_object_compositor_variant(negate_time_active: bool, selector: u8) -> u8 {
    if negate_time_active {
        0
    } else {
        selector & 0x03
    }
}

/// `visibility.md §8`: the inclusive bounds of the shared variant
/// selector's draw. `§8.1` calls it "a value in `0..3`", and `§8.3`
/// derives the observable rate from the fact that "the requested span of four
/// divides its output range exactly so the four outcomes are equally likely"
/// - so the draw is `random(0, 3)` over the shared gameplay stream, not a
/// modulo of a wider draw.
pub const ACTIVE_OBJECT_VARIANT_RANGE_LOW: u8 = 0;
/// See [`ACTIVE_OBJECT_VARIANT_RANGE_LOW`].
pub const ACTIVE_OBJECT_VARIANT_RANGE_HIGH: u8 = 3;

/// `visibility.md §8.3`: "the probability that two draws separated by one
/// to five steps differ is 0.7508" - so a qualifying seat repaints a
/// *different* variant on about three idle passes in four. The named-cell
/// recapture table on `cleak/u5-spec#182` lists **five** qualifying seats and
/// three fall-throughs. Four of the five carry a rate — 0.695, 0.709, 0.742
/// and 0.753 transitions per tick; the fifth, Britain (3,8), is recorded with
/// four distinct states and 204 transitions but **no tick count**, so it has
/// no rate. All three fall-throughs measured 0.000.
pub const ACTIVE_OBJECT_VARIANT_TRANSITION_PROBABILITY: f64 = 0.7508;

/// `visibility.md §8`'s **five selecting rows**, and only those: the base of
/// the four-entry range a default-helper composite selects among, or `None`
/// when the composite lands on any other row.
///
/// > **Normative, and the single most-misread line in this section.** Those
/// > five rows are the **only** rows that reach the selector. **Every other
/// > row of the table above — including both chair fall-throughs, the bed, the
/// > two ladders, the two facing-only chairs, and the plain pass-through —
/// > makes no selection at all.** An engine that draws from the shared stream
/// > on any other row advances the single global generator when the original
/// > does not, and its stream position diverges permanently from the
/// > original's.
///
/// The two chair rows are gated on a *neighbouring-row* terrain byte, and
/// "**the accepted set differs per facing, asymmetrically.** The `0x92` chair
/// accepts `0x9A` or `0x9C` on the row below it and rejects `0x9B`; the `0x90`
/// chair accepts `0x9B` or `0x9C` on the row above it and rejects `0x9A`. The
/// two sets are not the same set, and neither is 'any laden table'." The plain
/// tables `0x94..0x96` and every other neighbour fall through to a single
/// fixed occupied-chair tile with no draw.
///
/// [`active_object_default_composite`] is written on top of this function so
/// the "does this row draw?" question and the stamped tile can never drift
/// apart.
pub const fn active_object_default_variant_base(
    effective_tile: u8,
    current_terrain: u8,
    previous_row_terrain: Option<u8>,
    next_row_terrain: Option<u8>,
) -> Option<u8> {
    if !active_object_default_tile_is_terrain_aware(effective_tile) {
        return None;
    }
    match current_terrain {
        0xEC | 0x0A | 0x57 | 0x6A | 0x6B => None,
        _ if effective_tile >= 0x80 => None,
        // Stocks — an unconditional row, no neighbour predicate.
        0x84 => Some(0x60),
        // Manacles — an unconditional row, no neighbour predicate.
        0x85 => Some(0x64),
        0x90 => {
            if matches!(previous_row_terrain, Some(0x9B | 0x9C)) {
                Some(0x38)
            } else {
                None
            }
        }
        0x92 => {
            if matches!(next_row_terrain, Some(0x9A | 0x9C)) {
                Some(0x34)
            } else {
                None
            }
        }
        // `§8.4`: "**Terrain `0x9E` never appears as map terrain and its row
        // is dead in the shipped game.** Only `0x9D` reaches the trapped-soul
        // selection." The published table still lists both, so both are kept.
        0x9D | 0x9E => Some(0x3C),
        _ => None,
    }
}

/// `visibility.md §8`: default terrain-aware compositor helper.
pub const fn active_object_default_composite(
    effective_tile: u8,
    current_terrain: u8,
    previous_row_terrain: Option<u8>,
    next_row_terrain: Option<u8>,
    viewport_row: usize,
    variant: u8,
) -> ActiveObjectCompositeResult {
    if !active_object_default_tile_is_terrain_aware(effective_tile) {
        return ActiveObjectCompositeResult::Companion(effective_tile);
    }

    match active_object_default_variant_base(
        effective_tile,
        current_terrain,
        previous_row_terrain,
        next_row_terrain,
    ) {
        Some(base) => return ActiveObjectCompositeResult::Companion(base + (variant & 0x03)),
        None => {}
    }

    match current_terrain {
        0xEC | 0x0A => ActiveObjectCompositeResult::Suppress,
        0x57 => ActiveObjectCompositeResult::Direct(0x38),
        0x6A | 0x6B => {
            if (effective_tile >= 0x80 && effective_tile <= 0x8F)
                || (effective_tile >= 0x28 && effective_tile <= 0x2B)
            {
                ActiveObjectCompositeResult::Suppress
            } else {
                ActiveObjectCompositeResult::Companion(effective_tile)
            }
        }
        _ if effective_tile >= 0x80 => ActiveObjectCompositeResult::Companion(effective_tile),
        // `§8`: "Current terrain `0x90`, without that previous-row match —
        // Stamp `0x30`. **No variant is selected on this row.**"
        0x90 => ActiveObjectCompositeResult::Companion(0x30),
        0x91 => ActiveObjectCompositeResult::Companion(0x31),
        // `§8`: "Current terrain `0x92`, without that next-row match — Stamp
        // `0x32`. **No variant is selected on this row.**"
        0x92 => ActiveObjectCompositeResult::Companion(0x32),
        0x93 => ActiveObjectCompositeResult::Companion(0x33),
        // `§8`: "Current terrain `0xAB` — Stamp `0x1A`. **A single fixed tile
        // — not a variant, and no selection is made.**"
        0xAB => ActiveObjectCompositeResult::Companion(0x1A),
        0xC8 => ActiveObjectCompositeResult::Companion(0x17),
        0xC9 => ActiveObjectCompositeResult::Companion(0x18),
        _ if viewport_row > 0 && matches!(previous_row_terrain, Some(0x9D)) => {
            ActiveObjectCompositeResult::PreviousRowDirectAndCompanion {
                previous_marker: 0x9E,
                tile: effective_tile,
            }
        }
        _ => ActiveObjectCompositeResult::Companion(effective_tile),
    }
}

/// `visibility.md §8`/`§8.1`: which arm one active-object slot takes before
/// the terrain-aware helper is consulted.
///
/// `§8.1`: "The three direct-stamp branches (the two water/companion classes
/// and the single-sprite-family seated branch of Section 8) bypass the
/// compositor entirely and therefore never draw, whatever terrain they are
/// on." Splitting the dispatch out this way lets a caller answer "does this
/// slot take a draw?" without stamping and without duplicating the arm list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveObjectCompositeStep {
    /// A cell guard or one of the three direct-stamp branches settled the
    /// slot. **No draw is taken.**
    Resolved(ActiveObjectCompositeResult),
    /// The slot is handed to the default terrain-aware helper with this
    /// effective tile byte. A draw is taken only when the terrain under it is
    /// one of the five selecting rows
    /// ([`active_object_default_variant_base`]).
    DefaultHelper { effective_tile: u8 },
}

/// `visibility.md §8`: run the cell guards and the branch classifier for one
/// slot, stopping short of the terrain-aware helper.
pub const fn active_object_composite_step(
    type_byte: u8,
    frame_byte: u8,
    current_grid_byte: u8,
) -> ActiveObjectCompositeStep {
    if current_grid_byte == VISIBILITY_HIDDEN || current_grid_byte == VISIBILITY_ALREADY_RENDERED {
        return ActiveObjectCompositeStep::Resolved(ActiveObjectCompositeResult::Suppress);
    }

    match active_object_compositor_branch(type_byte, frame_byte) {
        ActiveObjectCompositorBranch::WaterBoundCompanion => {
            if current_grid_byte == VISIBILITY_USE_COMPANION {
                ActiveObjectCompositeStep::Resolved(ActiveObjectCompositeResult::Suppress)
            } else {
                ActiveObjectCompositeStep::Resolved(ActiveObjectCompositeResult::Companion(
                    frame_byte,
                ))
            }
        }
        ActiveObjectCompositorBranch::WaterCreatureCompanion => {
            ActiveObjectCompositeStep::Resolved(ActiveObjectCompositeResult::Companion(frame_byte))
        }
        ActiveObjectCompositorBranch::SingleSpriteFamilySeated => {
            if current_grid_byte == SINGLE_SPRITE_FAMILY_SEATED_CHAIR_TERRAIN {
                ActiveObjectCompositeStep::Resolved(ActiveObjectCompositeResult::Companion(
                    frame_byte,
                ))
            } else {
                ActiveObjectCompositeStep::DefaultHelper {
                    effective_tile: frame_byte
                        .wrapping_sub(SINGLE_SPRITE_FAMILY_SEATED_FRAME_FALLTHROUGH_DECREMENT),
                }
            }
        }
        ActiveObjectCompositorBranch::DefaultHelper => ActiveObjectCompositeStep::DefaultHelper {
            effective_tile: frame_byte,
        },
    }
}

/// `visibility.md §8.1`, the exact per-pass count: **one draw** for each actor
/// that "(a) survives all of the pass's per-slot skips, (b) is handed to the
/// default helper rather than to one of the three direct-stamp branches, and
/// (c) stands on stocks, manacles, a mirror, or a chair whose neighbouring row
/// on the correct side holds a laden-table id — and zero draws for everything
/// else, including actors on chairs that do not qualify, on beds, on ladders,
/// and on ordinary floor."
///
/// This answers (b) and (c) for a slot that already survived (a). Slot zero
/// takes the same answer: the slot-zero contract of
/// [`active_object_composite_for_player`] only rewrites a `Suppress` result
/// after the fact, and no `Suppress` arm selects.
pub const fn composite_active_object_slot_draws_variant(
    type_byte: u8,
    frame_byte: u8,
    current_grid_byte: u8,
    current_terrain: u8,
    previous_row_terrain: Option<u8>,
    next_row_terrain: Option<u8>,
) -> bool {
    match active_object_composite_step(type_byte, frame_byte, current_grid_byte) {
        ActiveObjectCompositeStep::Resolved(_) => false,
        ActiveObjectCompositeStep::DefaultHelper { effective_tile } => {
            active_object_default_variant_base(
                effective_tile,
                current_terrain,
                previous_row_terrain,
                next_row_terrain,
            )
            .is_some()
        }
    }
}

/// `visibility.md §8`: dispatch one active-object slot through the branch
/// classifier and, when needed, the default terrain-aware helper.
pub const fn active_object_composite(
    type_byte: u8,
    frame_byte: u8,
    current_grid_byte: u8,
    current_terrain: u8,
    previous_row_terrain: Option<u8>,
    next_row_terrain: Option<u8>,
    viewport_row: usize,
    variant: u8,
) -> ActiveObjectCompositeResult {
    match active_object_composite_step(type_byte, frame_byte, current_grid_byte) {
        ActiveObjectCompositeStep::Resolved(result) => result,
        ActiveObjectCompositeStep::DefaultHelper { effective_tile } => {
            active_object_default_composite(
                effective_tile,
                current_terrain,
                previous_row_terrain,
                next_row_terrain,
                viewport_row,
                variant,
            )
        }
    }
}

/// `visibility.md §8` + `active-objects.md §5`: composite **slot zero**, the
/// player, through the same branch classifier as every other slot, with one
/// difference: the terrain-aware suppress arms cannot erase the party sprite.
///
/// `active-objects.md §5` gives slot zero its own contract - "the renderer
/// walks the table from slot thirty-one down so slot zero paints on top",
/// restated by `visibility.md §8` as "slot zero is the player, so the avatar
/// always draws on top". The party sprite is the one stamp the player cannot
/// lose track of, so a `Suppress` from the terrain-aware helper degrades to
/// the helper's ordinary fallback - stamp the effective tile unchanged through
/// the companion band - instead of leaving the bare terrain on screen.
///
/// The two *cell-state* guards still suppress: a player cell carrying the
/// hidden marker (`0xFF`) is in fog and a cell carrying the already-rendered
/// marker (`0x87`) has been claimed, and `visibility.md §8` step 3 skips both
/// before any class branch runs. Only the terrain-aware rows are exempted.
///
/// The party never reaches the single-sprite-family seated branch:
/// `visibility.md §8` says the slot-zero refresh "writes the party sprite
/// marker into **both** the slot's type byte and its sprite byte, which is why
/// the party can never satisfy the type-byte test of the single-sprite-family
/// seated branch above".
///
/// Deviation note: `visibility.md §8`'s terrain-aware table row "current
/// terrain `0xEC` or `0x0A` - suppress the active-object stamp" carries no
/// effective-tile qualifier, unlike the `0x6A`/`0x6B` row beside it, and the
/// spec never carves slot zero out of the table. Taken literally it makes the
/// party vanish on dense-forest terrain `0x0A` (`visibility.md §6` names it
/// "tropical forest"), which the shipped passability bitset marks walkable, so
/// the party walks onto it in ordinary play. The published text does not
/// settle whether the row was meant to reach slot zero; this engine keeps the
/// row for every other slot and exempts the player.
pub const fn active_object_composite_for_player(
    type_byte: u8,
    frame_byte: u8,
    current_grid_byte: u8,
    current_terrain: u8,
    previous_row_terrain: Option<u8>,
    next_row_terrain: Option<u8>,
    viewport_row: usize,
    variant: u8,
) -> ActiveObjectCompositeResult {
    if current_grid_byte == VISIBILITY_HIDDEN || current_grid_byte == VISIBILITY_ALREADY_RENDERED {
        return ActiveObjectCompositeResult::Suppress;
    }
    match active_object_composite(
        type_byte,
        frame_byte,
        current_grid_byte,
        current_terrain,
        previous_row_terrain,
        next_row_terrain,
        viewport_row,
        variant,
    ) {
        ActiveObjectCompositeResult::Suppress => ActiveObjectCompositeResult::Companion(frame_byte),
        other => other,
    }
}

/// `visibility.md §8`: composite one active-object slot, choosing the slot-zero
/// contract when `player_slot` is set. `active-objects.md §5` reserves slot
/// zero for the player and every other slot for NPCs, monsters, vehicles and
/// props, so this is the single place the two contracts part company.
pub const fn composite_active_object_slot(
    player_slot: bool,
    type_byte: u8,
    frame_byte: u8,
    current_grid_byte: u8,
    current_terrain: u8,
    previous_row_terrain: Option<u8>,
    next_row_terrain: Option<u8>,
    viewport_row: usize,
    variant: u8,
) -> ActiveObjectCompositeResult {
    if player_slot {
        active_object_composite_for_player(
            type_byte,
            frame_byte,
            current_grid_byte,
            current_terrain,
            previous_row_terrain,
            next_row_terrain,
            viewport_row,
            variant,
        )
    } else {
        active_object_composite(
            type_byte,
            frame_byte,
            current_grid_byte,
            current_terrain,
            previous_row_terrain,
            next_row_terrain,
            viewport_row,
            variant,
        )
    }
}

/// `visibility.md §7` fog-edge refinement squared-distance threshold.
/// The post-pass folds each viewport coordinate around the centre
/// `(5, 5)`, computes `(5 - folded_x)^2 + (5 - folded_y)^2`, and
/// compares it against `5`. Cells with `squared > 5` carrying the
/// clear marker are downgraded to dim; cells with `squared <= 5`
/// carrying the dim marker are upgraded to clear.
pub const FOG_REFINE_SQUARED_THRESHOLD: u32 = 5;

/// `visibility.md §2,§7`: viewport centre column/row index. The grid
/// is `VIEWPORT_SIDE` (11) wide and 11 tall with the player at
/// `(VIEWPORT_CENTER, VIEWPORT_CENTER)` = `(5, 5)`.
pub const VIEWPORT_CENTER: u8 = (VIEWPORT_SIDE / 2) as u8;
/// `visibility.md §2,§7`: highest valid viewport column/row index
/// (`VIEWPORT_SIDE - 1` = 10).
pub const VIEWPORT_MAX_INDEX: u8 = (VIEWPORT_SIDE - 1) as u8;

/// `visibility.md §7`: fold a viewport coordinate `0..=VIEWPORT_MAX_INDEX`
/// around the centre `VIEWPORT_CENTER` so the squared-distance lookup
/// can be computed as a 6x6 table (`folded = min(coord, VIEWPORT_MAX_INDEX - coord)`).
pub const fn fog_refine_folded_coord(coord: u8) -> u8 {
    let mirrored = VIEWPORT_MAX_INDEX.saturating_sub(coord);
    if coord < mirrored { coord } else { mirrored }
}

/// `visibility.md §7`: squared centre-relative distance the fog
/// post-pass uses, computed from a viewport `(col, row)`. Returns
/// `(VIEWPORT_CENTER - folded_col)^2 + (VIEWPORT_CENTER - folded_row)^2`.
pub const fn fog_refine_squared_distance(col: u8, row: u8) -> u32 {
    let dx = VIEWPORT_CENTER.saturating_sub(fog_refine_folded_coord(col));
    let dy = VIEWPORT_CENTER.saturating_sub(fog_refine_folded_coord(row));
    (dx as u32) * (dx as u32) + (dy as u32) * (dy as u32)
}

/// `visibility.md §7`: returns `true` when the supplied viewport
/// `(col, row)` falls inside the clear-marker core (squared
/// distance `<= 5`); cells outside that core are downgraded to dim.
pub const fn fog_refine_inside_clear_core(col: u8, row: u8) -> bool {
    fog_refine_squared_distance(col, row) <= FOG_REFINE_SQUARED_THRESHOLD
}

/// `visibility.md §5` neighbour expansion order for the visibility
/// carve helper. The carve pops a coordinate and examines its eight
/// neighbours in this fixed ring order: West, Southwest, South,
/// Southeast, East, Northeast, North, Northwest. Each entry is the
/// `(dx, dy)` offset from the popped coordinate.
pub const VISIBILITY_CARVE_NEIGHBOR_ORDER: [(i8, i8); 8] = [
    (-1, 0),  // West
    (-1, 1),  // Southwest
    (0, 1),   // South
    (1, 1),   // Southeast
    (1, 0),   // East
    (1, -1),  // Northeast
    (0, -1),  // North
    (-1, -1), // Northwest
];

/// `visibility.md §5`: squared-distance threshold check. A cell is
/// inside the main light radius when its squared centre-relative
/// distance is `<=` the supplied light value.
pub const fn visibility_in_radius(squared_distance: u32, light_value: u32) -> bool {
    squared_distance <= light_value
}

/// `visibility.md §3`: producer behaviour selected by the sign of the
/// (signed) light-radius byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightRadiusBranch {
    /// Positive radius: run the visibility carve helper.
    Carve(u8),
    /// Zero radius: leave the grid fully obscured (pitch dark).
    PitchDark,
    /// Negative radius: full-fill from the world map regardless of the
    /// carve (debug-style branch).
    DebugFullFill,
}

/// `visibility.md §3`: branch selector. Treats the light-radius byte as
/// signed: positive runs the carve, zero is darkness, negative is the
/// full-fill compatibility path.
pub const fn light_radius_branch(radius_byte: u8) -> LightRadiusBranch {
    let signed = radius_byte as i8;
    if signed > 0 {
        LightRadiusBranch::Carve(signed as u8)
    } else if signed == 0 {
        LightRadiusBranch::PitchDark
    } else {
        LightRadiusBranch::DebugFullFill
    }
}
