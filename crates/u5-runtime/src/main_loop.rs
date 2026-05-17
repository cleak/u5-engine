//! Outer-dispatch routing per `main-loop.md` §3-§4.

use crate::InputDirection;

/// `main-loop.md §11` Q-save scene-byte normalisation. Combat is not
/// saved; if the active scene byte is the temporary combat marker
/// (`0xFF`) when the save handler runs, the writer substitutes the
/// post-combat home scene byte the framer would have restored to.
/// Returns the byte that should land in the saved image.
pub const fn save_scene_byte_normalised(scene_byte: u8, post_combat_home: u8) -> u8 {
    if scene_byte == SCENE_COMBAT_TEMPORARY {
        post_combat_home
    } else {
        scene_byte
    }
}

/// `main-loop.md §3` scene-byte ranges: well-known sentinels for the
/// intro sub-states and the temporary combat marker.
/// `main-loop.md §9` world-tick branch the redraw orchestrator
/// dispatches to. The orchestrator runs between keystrokes from
/// inside the input pipeline's idle wait; it does not run while
/// prompt mode is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldTickPath {
    /// Combat scene — blat-copy the precomputed combat terrain
    /// grid into the viewport scratch grid; the orchestrator does
    /// not run the producer.
    CombatBlatCopy,
    /// 2D scene with the visibility-dirty flag set — run the full
    /// centre-out visibility producer against the active map.
    ProducerFullRebuild,
    /// 2D scene with the dirty flag clear — lazy refill only the
    /// cells whose current value is the post-render zero sentinel.
    LazyRefill,
}

/// `main-loop.md §9`: classify the world-tick branch from the
/// scene byte and the visibility-dirty flag. Combat-class scenes
/// use the blat-copy path regardless of the dirty flag; 2D scenes
/// branch on the dirty flag.
pub const fn world_tick_path(scene_byte: u8, visibility_dirty: bool) -> WorldTickPath {
    match scene_route(scene_byte) {
        SceneRoute::CombatTemporary => WorldTickPath::CombatBlatCopy,
        _ => {
            if visibility_dirty {
                WorldTickPath::ProducerFullRebuild
            } else {
                WorldTickPath::LazyRefill
            }
        }
    }
}

/// `main-loop.md §4` outer-loop bookkeeping flags. The router keeps
/// two single-bit flags between iterations:
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OuterLoopFlags {
    /// Set when the overworld branch returns. Prevents a tight
    /// overworld -> overworld spin when the scene byte is still
    /// zero (e.g. a no-op cancellation produced no scene change).
    pub exit_pending: bool,
    /// Tracks whether the previous iteration ran the dungeon
    /// branch. The dungeon dispatch consults this to know whether
    /// the player is entering fresh or returning from combat.
    pub previous_was_dungeon: bool,
}

impl OuterLoopFlags {
    /// `main-loop.md §4`: returns `true` when the outer loop should
    /// skip the overworld branch this iteration because the
    /// previous iteration already returned with the scene byte
    /// still at zero. Caller clears the flag after honoring the
    /// skip.
    pub const fn should_skip_overworld(self, scene_byte: u8) -> bool {
        self.exit_pending && scene_byte == SCENE_OVERWORLD
    }
}

/// `main-loop.md §7` shared command-dispatcher status word. Returned
/// by per-letter handler blocks so the calling mode loop knows
/// whether to run the per-turn epilogue, suppress the redraw, or
/// treat the keystroke as a meta toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandDispatchStatus {
    /// Action consumed a turn — run the per-turn epilogue and
    /// advance the world clock.
    ConsumesTurn,
    /// Action did not consume a turn — leave the world clock
    /// unchanged and skip the epilogue.
    NoTurn,
    /// Buffer toggle (a non-game key like Ctrl-S that should not
    /// advance the clock).
    BufferToggle,
    /// Town command produced a message but should not redraw.
    /// Mode loop keeps polling without repainting.
    RepollNoRedraw,
}

impl CommandDispatchStatus {
    /// `main-loop.md §6,§7`: returns `true` when the mode-loop should
    /// run its per-turn epilogue (cleanup, NPC schedule, encounter
    /// rolls, etc.). Only `ConsumesTurn` triggers the epilogue.
    pub const fn runs_per_turn_epilogue(self) -> bool {
        matches!(self, Self::ConsumesTurn)
    }

    /// `main-loop.md §7`: returns `true` when the mode-loop should
    /// repaint the viewport after dispatch. `RepollNoRedraw`
    /// suppresses the redraw; the others let it run normally.
    pub const fn requests_redraw(self) -> bool {
        !matches!(self, Self::RepollNoRedraw)
    }
}

/// `main-loop.md §3` scene-byte zero is the overworld; the town
/// family picks up immediately after at scene byte 1 and runs
/// through scene byte 32. Anchor SCENE_TOWN_FAMILY_FIRST to
/// SCENE_OVERWORLD + 1 so the overworld→town adjacency has one
/// source of truth.
pub const SCENE_OVERWORLD: u8 = 0;
pub const SCENE_TOWN_FAMILY_FIRST: u8 = SCENE_OVERWORLD + 1;
pub const SCENE_TOWN_FAMILY_LAST: u8 = 32;
/// `main-loop.md §3` dungeon-class scene-byte range. The full
/// classification accepts `33..=127`; the stock named dungeons
/// (Deceit through Doom) live in the sub-range
/// `SCENE_DUNGEON_NAMED_FIRST..=SCENE_DUNGEON_NAMED_LAST`, which is
/// the same boundary `FIRST_DUNGEON_SCENE_BYTE..=LAST_DUNGEON_SCENE_BYTE`
/// promoted in `scene.rs`.
pub const SCENE_DUNGEON_FAMILY_FIRST: u8 = crate::FIRST_DUNGEON_SCENE_BYTE;
pub const SCENE_DUNGEON_FAMILY_LAST: u8 = 127;
pub const SCENE_DUNGEON_NAMED_FIRST: u8 = crate::FIRST_DUNGEON_SCENE_BYTE;
pub const SCENE_DUNGEON_NAMED_LAST: u8 = crate::LAST_DUNGEON_SCENE_BYTE;
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

/// `catalogs/quest-graph.md §4` Word of Power for one of the eight
/// stock dungeons indexed by scene byte `33..=40`. The Yell
/// Word-of-Power handler matches these strings (uppercased) against
/// the typed input and dispatches by dungeon. Doom's `VERAMOCOR`
/// opens the chamber seal once the party is already inside Doom;
/// it does not open Doom's exterior entrance.
pub const fn dungeon_word_of_power(scene_byte: u8) -> Option<&'static str> {
    Some(match scene_byte {
        33 => "FALLAX",
        34 => "VILIS",
        35 => "INOPIA",
        36 => "MALUM",
        37 => "AVIDUS",
        38 => "INFAMA",
        39 => "IGNAVUS",
        40 => "VERAMOCOR",
        _ => return None,
    })
}

/// `catalogs/quest-graph.md §4`: inverse of [`dungeon_word_of_power`].
/// Returns the dungeon scene byte (`33..=40`) the typed Word of
/// Power corresponds to, or `None` for any input that is not one of
/// the eight published words. Matching is case-insensitive ASCII to
/// match the Yell input pipeline's uppercase folding.
pub fn dungeon_scene_for_word_of_power(word: &str) -> Option<u8> {
    if word.eq_ignore_ascii_case("FALLAX") {
        Some(33)
    } else if word.eq_ignore_ascii_case("VILIS") {
        Some(34)
    } else if word.eq_ignore_ascii_case("INOPIA") {
        Some(35)
    } else if word.eq_ignore_ascii_case("MALUM") {
        Some(36)
    } else if word.eq_ignore_ascii_case("AVIDUS") {
        Some(37)
    } else if word.eq_ignore_ascii_case("INFAMA") {
        Some(38)
    } else if word.eq_ignore_ascii_case("IGNAVUS") {
        Some(39)
    } else if word.eq_ignore_ascii_case("VERAMOCOR") {
        Some(40)
    } else {
        None
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

/// `dungeon-mode.md §9` per-facing forward step `(dx, dy)` on the
/// 8x8 floor. The party moves into `(x + dx, y + dy)` for the
/// supplied facing, with X west-to-east and Y north-to-south. The
/// floor wraps modulo 8 in the caller; this helper returns the raw
/// signed delta. Returns `None` for facing values outside `0..=3`.
/// `dungeon-mode.md §9` movement-input action. The dungeon command
/// parser intercepts numpad/arrow keys before the A-Z dispatcher
/// sees them; each accepted code maps to one of these published
/// actions. Unrecognized movement subcodes fall through to
/// `TurnAround` (a 180-degree facing rotate, not a step).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonMovementAction {
    /// Step one cell in the current facing direction.
    Forward,
    /// Step one cell opposite the current facing direction.
    Back,
    /// Decrement the facing byte by one (modulo four).
    TurnLeft,
    /// Increment the facing byte by one (modulo four).
    TurnRight,
    /// Rotate the party by 180 degrees (the fall-through for
    /// unrecognized movement subcodes); does not step.
    TurnAround,
}

/// `dungeon-mode.md §9`: classify one input direction code into a
/// dungeon movement action. The published numpad/arrow mapping:
/// `North` = Forward, `South` = Back, `West` = TurnLeft,
/// `East` = TurnRight. Diagonal/Pass and any other value falls
/// through to `TurnAround`.
pub const fn dungeon_movement_action(input: InputDirection) -> DungeonMovementAction {
    match input {
        InputDirection::North => DungeonMovementAction::Forward,
        InputDirection::South => DungeonMovementAction::Back,
        InputDirection::West => DungeonMovementAction::TurnLeft,
        InputDirection::East => DungeonMovementAction::TurnRight,
        _ => DungeonMovementAction::TurnAround,
    }
}

pub const fn dungeon_facing_forward_delta(facing: u8) -> Option<(i8, i8)> {
    match facing {
        DUNGEON_FACING_NORTH => Some((0, -1)),
        DUNGEON_FACING_EAST => Some((1, 0)),
        DUNGEON_FACING_SOUTH => Some((0, 1)),
        DUNGEON_FACING_WEST => Some((-1, 0)),
        _ => None,
    }
}

/// `dungeon-mode.md §9` per-facing back-step `(dx, dy)`: the
/// negation of the forward delta.
pub const fn dungeon_facing_back_delta(facing: u8) -> Option<(i8, i8)> {
    match dungeon_facing_forward_delta(facing) {
        Some((dx, dy)) => Some((-dx, -dy)),
        None => None,
    }
}

/// `dungeon-mode.md §6` per-facing left-cell `(dx, dy)` for the
/// renderer's side-wall mirroring path. Rotating the forward delta
/// 90 degrees counter-clockwise: `(dx, dy)` becomes `(dy, -dx)`.
/// Returns `None` for facing values outside `0..=3`.
pub const fn dungeon_facing_left_delta(facing: u8) -> Option<(i8, i8)> {
    match dungeon_facing_forward_delta(facing) {
        Some((dx, dy)) => Some((dy, -dx)),
        None => None,
    }
}

/// `dungeon-mode.md §6` per-facing right-cell `(dx, dy)` for the
/// renderer's side-wall mirroring path. Rotating the forward delta
/// 90 degrees clockwise: `(dx, dy)` becomes `(-dy, dx)`.
/// Returns `None` for facing values outside `0..=3`.
pub const fn dungeon_facing_right_delta(facing: u8) -> Option<(i8, i8)> {
    match dungeon_facing_forward_delta(facing) {
        Some((dx, dy)) => Some((-dy, dx)),
        None => None,
    }
}

/// `dungeon-mode.md §9` left turn — decrement facing modulo 4.
pub const fn dungeon_facing_turn_left(facing: u8) -> u8 {
    (facing + 3) % 4
}

/// `dungeon-mode.md §9` right turn — increment facing modulo 4.
pub const fn dungeon_facing_turn_right(facing: u8) -> u8 {
    (facing + 1) % 4
}

/// `dungeon-mode.md §9` 180-degree turnaround — facing + 2 mod 4.
/// The unrecognised-movement-subcode fallthrough rotates the party
/// by this amount.
pub const fn dungeon_facing_turn_around(facing: u8) -> u8 {
    (facing + 2) % 4
}

/// `dungeon-mode.md §3` surface-plane dungeon entry seed. Reached
/// from the overworld surface, the party lands on the top floor
/// (`Z=0`) at the published `(X, Y) = (1, 1)` cell, facing east.
pub const DUNGEON_ENTRY_SURFACE_Z: u8 = 0;
pub const DUNGEON_ENTRY_SURFACE_X: u8 = 1;
pub const DUNGEON_ENTRY_SURFACE_Y: u8 = 1;

/// `dungeon-mode.md §3` underworld-plane dungeon entry seed. Reached
/// from the underworld plane (for every dungeon except Doom), the
/// party lands on the deepest floor (`Z=7`) at the published
/// `(X, Y) = (7, 7)` cell, facing west.
pub const DUNGEON_ENTRY_UNDERWORLD_Z: u8 = 7;
pub const DUNGEON_ENTRY_UNDERWORLD_X: u8 = 7;
pub const DUNGEON_ENTRY_UNDERWORLD_Y: u8 = 7;

/// `dungeon-mode.md §3` Doom-exception scene byte. The Doom dungeon
/// uses the surface entry seed even when reached from the
/// underworld plane. Doom is the eighth dungeon record
/// (`DUNGEON.DAT` record 7), so its scene byte is the last value in
/// the dungeon range `FIRST_DUNGEON_SCENE_BYTE..=LAST_DUNGEON_SCENE_BYTE`.
pub const DUNGEON_DOOM_SCENE_BYTE: u8 = crate::LAST_DUNGEON_SCENE_BYTE;

/// `catalogs/gazetteer.md §6`: pick the entry seed for the given
/// dungeon scene byte and origin plane. Doom uses the surface seed
/// even when reached from the underworld.
pub const fn dungeon_entry_seed(scene_byte: u8, from_underworld: bool) -> Option<DungeonEntrySeed> {
    if dungeon_resident_name(scene_byte).is_none() {
        return None;
    }
    let surface_seed = DungeonEntrySeed {
        z: DUNGEON_ENTRY_SURFACE_Z,
        x: DUNGEON_ENTRY_SURFACE_X,
        y: DUNGEON_ENTRY_SURFACE_Y,
        facing: DUNGEON_FACING_EAST,
    };
    let underworld_seed = DungeonEntrySeed {
        z: DUNGEON_ENTRY_UNDERWORLD_Z,
        x: DUNGEON_ENTRY_UNDERWORLD_X,
        y: DUNGEON_ENTRY_UNDERWORLD_Y,
        facing: DUNGEON_FACING_WEST,
    };
    if !from_underworld {
        return Some(surface_seed);
    }
    if scene_byte == DUNGEON_DOOM_SCENE_BYTE {
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
