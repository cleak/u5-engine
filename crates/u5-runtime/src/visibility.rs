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
