//! Outer-dispatch routing per `main-loop.md` §3-§4.

/// `main-loop.md §3` scene-byte ranges: well-known sentinels for the
/// intro sub-states and the temporary combat marker.
pub const SCENE_OVERWORLD: u8 = 0;
pub const SCENE_TOWN_FAMILY_FIRST: u8 = 1;
pub const SCENE_TOWN_FAMILY_LAST: u8 = 32;
pub const SCENE_DUNGEON_FAMILY_FIRST: u8 = 33;
pub const SCENE_DUNGEON_FAMILY_LAST: u8 = 127;
pub const SCENE_DUNGEON_NAMED_FIRST: u8 = 33;
pub const SCENE_DUNGEON_NAMED_LAST: u8 = 40;
pub const SCENE_INTRO_FIRST: u8 = 0x40;
pub const SCENE_INTRO_LAST: u8 = 0x42;
pub const SCENE_COMBAT_TEMPORARY: u8 = 0xFF;

/// `main-loop.md §3,§4`: route the scene byte to the mode-loop branch
/// the outer dispatch should run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneRoute {
    /// Scene `0` — overworld / underworld travel mode.
    Overworld,
    /// Scenes `1..=32` — town-family entry pass + town turn loop.
    TownFamily,
    /// Scenes `33..=127` — dungeon dispatch path (DNGLOOK + DUNGEON
    /// turn loop). Normal play only writes `33..=40`; higher values are
    /// non-stock dungeon-class targets.
    Dungeon,
    /// `0x40..=0x42` — intro and Return-to-View preview states. Outer
    /// dispatch consumes them only on the boot pass.
    IntroOrPreview,
    /// `0xFF` — temporary combat-class marker; the outer loop never
    /// sees this routinely (combat returns through its framer).
    CombatTemporary,
}

/// `main-loop.md §3,§4`: classify the scene byte for outer dispatch.
pub const fn scene_route(scene_byte: u8) -> SceneRoute {
    match scene_byte {
        SCENE_OVERWORLD => SceneRoute::Overworld,
        SCENE_TOWN_FAMILY_FIRST..=SCENE_TOWN_FAMILY_LAST => SceneRoute::TownFamily,
        SCENE_INTRO_FIRST..=SCENE_INTRO_LAST => SceneRoute::IntroOrPreview,
        SCENE_COMBAT_TEMPORARY => SceneRoute::CombatTemporary,
        SCENE_DUNGEON_FAMILY_FIRST..=SCENE_DUNGEON_FAMILY_LAST => SceneRoute::Dungeon,
        // `0x80..0xFE` is treated as combat-class by several readers; the
        // only observed writer is `0xFF`. Route them as combat-temporary
        // so they are not silently misrouted to the dungeon dispatch.
        _ => SceneRoute::CombatTemporary,
    }
}

/// `catalogs/gazetteer.md §6` resident name for one of the eight
/// stock dungeons indexed by scene byte `33..=40`.
pub const fn dungeon_resident_name(scene_byte: u8) -> Option<&'static str> {
    Some(match scene_byte {
        33 => "Deceit",
        34 => "Despise",
        35 => "Destard",
        36 => "Wrong",
        37 => "Covetous",
        38 => "Shame",
        39 => "Hythloth",
        40 => "Doom",
        _ => return None,
    })
}

/// `catalogs/gazetteer.md §6`: dungeon-mode entry seed coordinates.
/// Britannia surface entry uses `(Z=0, X=1, Y=1)` facing east; the
/// underworld entry into non-Doom dungeons uses `(Z=7, X=7, Y=7)`
/// facing west; Doom always uses the surface seed even when reached
/// from the underworld.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonEntrySeed {
    /// Z (level), 0..=7.
    pub z: u8,
    /// X coordinate, 0..=7.
    pub x: u8,
    /// Y coordinate, 0..=7.
    pub y: u8,
    /// Cardinal facing — 0 north, 1 east, 2 south, 3 west.
    pub facing: u8,
}

pub const DUNGEON_FACING_NORTH: u8 = 0;
pub const DUNGEON_FACING_EAST: u8 = 1;
pub const DUNGEON_FACING_SOUTH: u8 = 2;
pub const DUNGEON_FACING_WEST: u8 = 3;

/// `catalogs/gazetteer.md §6`: pick the entry seed for the given
/// dungeon scene byte and origin plane. Doom uses the surface seed
/// even when reached from the underworld.
pub const fn dungeon_entry_seed(scene_byte: u8, from_underworld: bool) -> Option<DungeonEntrySeed> {
    if dungeon_resident_name(scene_byte).is_none() {
        return None;
    }
    let surface_seed = DungeonEntrySeed {
        z: 0,
        x: 1,
        y: 1,
        facing: DUNGEON_FACING_EAST,
    };
    let underworld_seed = DungeonEntrySeed {
        z: 7,
        x: 7,
        y: 7,
        facing: DUNGEON_FACING_WEST,
    };
    if !from_underworld {
        return Some(surface_seed);
    }
    // Doom (scene 40) uses the surface seed regardless of origin.
    if scene_byte == 40 {
        return Some(surface_seed);
    }
    Some(underworld_seed)
}

/// `main-loop.md §3`: zero-based `DUNGEON.DAT` record index for a
/// stock-named dungeon scene (`33..=40`). Returns `None` for any value
/// outside that named-dungeon range.
pub const fn dungeon_record_index(scene_byte: u8) -> Option<u8> {
    if scene_byte >= SCENE_DUNGEON_NAMED_FIRST && scene_byte <= SCENE_DUNGEON_NAMED_LAST {
        Some(scene_byte - SCENE_DUNGEON_NAMED_FIRST)
    } else {
        None
    }
}

/// `main-loop.md §6` per-mode minute increment passed to the per-turn
/// cleanup helper. Returns `None` for combat (combat does not advance
/// the world clock per action; the round-end cadence belongs to the
/// combat framer instead).
pub const fn mode_minute_increment(route: SceneRoute) -> Option<u8> {
    match route {
        SceneRoute::Overworld => Some(2),
        SceneRoute::TownFamily | SceneRoute::Dungeon => Some(1),
        SceneRoute::IntroOrPreview | SceneRoute::CombatTemporary => None,
    }
}
