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
