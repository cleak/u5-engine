//! Town-family scene classification per `town-mode.md` §2.

/// `town-mode.md §2` four classes the eight-per-class scene-byte band
/// `1..=32` divides into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownLocationClass {
    Town,
    Dwelling,
    Castle,
    Keep,
}

impl TownLocationClass {
    /// Per-class file-family name used by the four-entry pointer table.
    /// (Returned in canonical lower-case singular form; the loader
    /// composes the actual filename with the per-class extension.)
    pub const fn family_name(self) -> &'static str {
        match self {
            TownLocationClass::Town => "town",
            TownLocationClass::Dwelling => "dwelling",
            TownLocationClass::Castle => "castle",
            TownLocationClass::Keep => "keep",
        }
    }
}

/// `town-mode.md §2`: classify a scene byte (`1..=32`) into its
/// town-family class. Returns `None` for the overworld scene `0` and
/// for any value outside the town-family range.
pub const fn town_location_class(scene_byte: u8) -> Option<TownLocationClass> {
    Some(match scene_byte {
        1..=8 => TownLocationClass::Town,
        9..=16 => TownLocationClass::Dwelling,
        17..=24 => TownLocationClass::Castle,
        25..=32 => TownLocationClass::Keep,
        _ => return None,
    })
}

/// `town-mode.md §4`: zero-based per-class location index used to index
/// the roster and dialogue files. Returns `None` for non-town-family
/// scene bytes.
pub const fn town_per_class_index(scene_byte: u8) -> Option<u8> {
    match scene_byte {
        1..=8 => Some(scene_byte - 1),
        9..=16 => Some(scene_byte - 9),
        17..=24 => Some(scene_byte - 17),
        25..=32 => Some(scene_byte - 25),
        _ => None,
    }
}

/// `town-mode.md §3`: per-location grid dimensions and floor byte size.
pub const TOWN_GRID_SIDE: usize = 32;
pub const TOWN_GRID_BYTES: usize = TOWN_GRID_SIDE * TOWN_GRID_SIDE;

/// `town-mode.md §3`: signed-eight-bit interpretation of the runtime
/// floor byte. Returns the signed offset from the scene's resident base
/// page; values `0..=127` are non-negative floors, values `128..=255`
/// are negative offsets.
pub const fn town_floor_offset(floor_byte: u8) -> i8 {
    floor_byte as i8
}

/// `town-mode.md §4`: NPC roster size — up to 31 active NPC slots
/// (slot zero is a sentinel) with three parallel 16/1/1-byte sub-blocks
/// per slot for a total of 576 bytes per location.
pub const TOWN_NPC_ROSTER_SLOTS: usize = 31;
pub const TOWN_NPC_BLOCK_BYTES: usize = 576;
