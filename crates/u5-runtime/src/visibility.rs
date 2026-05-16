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
pub const VIEWPORT_ROW_STRIDE: usize = 32;
/// `visibility.md §2`: row stride of the terrain band in bytes.
pub const TERRAIN_BAND_ROW_STRIDE: usize = 16;
/// `visibility.md §2`: zero-based player position inside the active
/// window (centre row, centre column).
pub const VIEWPORT_PLAYER_ROW: usize = 5;
pub const VIEWPORT_PLAYER_COL: usize = 5;

/// `visibility.md §2`: well-known visibility-grid byte markers.
pub const VISIBILITY_HIDDEN: u8 = 0xFF;
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
/// in.
pub const LOCAL_LIGHT_MASK_SIDE: usize = 32;

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

/// `visibility.md §7` fog-edge refinement squared-distance threshold.
/// The post-pass folds each viewport coordinate around the centre
/// `(5, 5)`, computes `(5 - folded_x)^2 + (5 - folded_y)^2`, and
/// compares it against `5`. Cells with `squared > 5` carrying the
/// clear marker are downgraded to dim; cells with `squared <= 5`
/// carrying the dim marker are upgraded to clear.
pub const FOG_REFINE_SQUARED_THRESHOLD: u32 = 5;

/// `visibility.md §7`: fold a viewport coordinate `0..=10` around
/// the centre `5` so the squared-distance lookup can be computed
/// as a 6x6 table (`folded = min(coord, 10 - coord)`).
pub const fn fog_refine_folded_coord(coord: u8) -> u8 {
    let mirrored = 10u8.saturating_sub(coord);
    if coord < mirrored { coord } else { mirrored }
}

/// `visibility.md §7`: squared centre-relative distance the fog
/// post-pass uses, computed from a viewport `(col, row)`. Returns
/// `(5 - folded_col)^2 + (5 - folded_row)^2`.
pub const fn fog_refine_squared_distance(col: u8, row: u8) -> u32 {
    let dx = 5u8.saturating_sub(fog_refine_folded_coord(col));
    let dy = 5u8.saturating_sub(fog_refine_folded_coord(row));
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
