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
    /// Type byte exactly `0x5C` — vehicle/avatar-family companion
    /// branch. Caller still needs to check the visibility-grid
    /// underlay marker `0x92` before stamping the companion path.
    VehicleAvatarCompanion,
    /// Anything else — falls through to the default tile compositor
    /// helper.
    DefaultHelper,
}

/// `visibility.md §8`: classify a slot by its `(type, frame)` byte
/// pair. The water-bound branch is keyed by the type byte; the
/// water-creature branch is keyed by the frame byte; the
/// vehicle/avatar branch is keyed by the type byte alone (caller
/// applies the underlay-marker check).
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
        return ActiveObjectCompositorBranch::VehicleAvatarCompanion;
    }
    ActiveObjectCompositorBranch::DefaultHelper
}

/// `visibility.md §8` vehicle/avatar underlay marker. The
/// `VehicleAvatarCompanion` branch stamps through the companion
/// path only when the visibility-grid cell currently holds this
/// marker; otherwise it falls through to the default helper.
pub const VEHICLE_AVATAR_UNDERLAY_MARKER: u8 = 0x92;

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

/// `visibility.md §8`: four-entry selector for terrain edge variants. Tinkers
/// force the first entry; other active characters use the caller-supplied
/// low-two-bit selector.
pub const fn active_object_compositor_variant(
    active_character_is_tinker: bool,
    selector: u8,
) -> u8 {
    if active_character_is_tinker {
        0
    } else {
        selector & 0x03
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
        0x84 => ActiveObjectCompositeResult::Companion(0x60 + (variant & 0x03)),
        0x85 => ActiveObjectCompositeResult::Companion(0x64 + (variant & 0x03)),
        0x90 => {
            if matches!(previous_row_terrain, Some(0x9B | 0x9C)) {
                ActiveObjectCompositeResult::Companion(0x38 + (variant & 0x03))
            } else {
                ActiveObjectCompositeResult::Companion(0x30)
            }
        }
        0x91 => ActiveObjectCompositeResult::Companion(0x31),
        0x92 => {
            if matches!(next_row_terrain, Some(0x9A | 0x9C)) {
                ActiveObjectCompositeResult::Companion(0x34 + (variant & 0x03))
            } else {
                ActiveObjectCompositeResult::Companion(0x32)
            }
        }
        0x93 => ActiveObjectCompositeResult::Companion(0x33),
        0x9D | 0x9E => ActiveObjectCompositeResult::Companion(0x3C + (variant & 0x03)),
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
    if current_grid_byte == VISIBILITY_HIDDEN || current_grid_byte == VISIBILITY_ALREADY_RENDERED {
        return ActiveObjectCompositeResult::Suppress;
    }

    match active_object_compositor_branch(type_byte, frame_byte) {
        ActiveObjectCompositorBranch::WaterBoundCompanion => {
            if current_grid_byte == VISIBILITY_USE_COMPANION {
                ActiveObjectCompositeResult::Suppress
            } else {
                ActiveObjectCompositeResult::Companion(frame_byte)
            }
        }
        ActiveObjectCompositorBranch::WaterCreatureCompanion => {
            ActiveObjectCompositeResult::Companion(frame_byte)
        }
        ActiveObjectCompositorBranch::VehicleAvatarCompanion => {
            if current_grid_byte == VEHICLE_AVATAR_UNDERLAY_MARKER {
                ActiveObjectCompositeResult::Companion(frame_byte)
            } else {
                active_object_default_composite(
                    frame_byte,
                    current_terrain,
                    previous_row_terrain,
                    next_row_terrain,
                    viewport_row,
                    variant,
                )
            }
        }
        ActiveObjectCompositorBranch::DefaultHelper => active_object_default_composite(
            frame_byte,
            current_terrain,
            previous_row_terrain,
            next_row_terrain,
            viewport_row,
            variant,
        ),
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
