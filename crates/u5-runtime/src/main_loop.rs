//! Outer-dispatch routing per `main-loop.md` §3-§4.

use crate::{
    CHUNK_SIDE, InputDirection, PartyMember, SAVE_QUEST_TILE_FLAG_HIGH_BIT,
    SAVE_SHRINE_RUIN_FLAG_COUNT, SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT, WORLD_CELLS, WORLD_SIDE,
    WorldPlane,
};

/// `main-loop.md §6` shared exploration-loop roster result.
///
/// The scan is status-byte-only: Good and Poisoned can act, Sleeping selects
/// the automatic sleep pass only when no earlier member can act, and every
/// other status contributes to total-party defeat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyCapability {
    CanAct { member_index: usize },
    Sleeping,
    Defeated,
}

/// Exact line printed by the no-input sleeping-party loop branch.
pub const PARTY_SLEEP_LINE: &str = "Zzzzzz...";

/// `town-mode.md §7`: inclusive PRNG range for each sleeping member's
/// independent 1-in-16 wake roll. Roll zero wakes the member to Good.
pub const TOWN_SLEEP_WAKE_ROLL_MAX: u8 = 15;

pub fn party_capability(party: &[PartyMember]) -> PartyCapability {
    let mut sleeping = false;
    for (member_index, member) in party.iter().enumerate() {
        match member.status {
            b'G' | b'P' => return PartyCapability::CanAct { member_index },
            b'S' => sleeping = true,
            _ => {}
        }
    }
    if sleeping {
        PartyCapability::Sleeping
    } else {
        PartyCapability::Defeated
    }
}

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
/// dispatches to.
///
/// "The input pipeline's idle wait is where the world tick runs most
/// often, but it is not the only caller: presentations, cutscene beats
/// and paced turn loops call it directly, dozens of call sites across
/// the resident image and the overlays." What the idle wait owns is the
/// *idle* pump, and that pump does not run while prompt mode is active.
///
/// **R319.** `main-loop.md §9` previously said "the world tick is **only**
/// called from inside the input pipeline's idle wait". That is withdrawn:
/// "an engine that can reach its world tick only from an idle poll cannot
/// implement the paced presentations at all" — see
/// [`PlayState::advance_presentation_frame`] and `animation.md §13`.
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

/// `timing.md §8.2`: the contiguous scene-value band whose idle pass
/// performs **no world step**.
///
/// "**The world step is suppressed for a contiguous band of scene
/// values.** The shared wait tests the current scene value and performs no
/// world step for values `0x21` through `0x7F` **inclusive**; both the
/// bound and its inclusiveness are exact."
///
/// These are the published literals. They happen to coincide with
/// [`SCENE_DUNGEON_FAMILY_FIRST`] / [`SCENE_DUNGEON_FAMILY_LAST`], but the
/// two bands are reached different ways - `main-loop.md §3`'s dungeon
/// *classification* versus `timing.md §8.2`'s *suppression* - so they are
/// stated independently here and cross-checked below rather than one being
/// defined in terms of the other.
pub const IDLE_WORLD_STEP_SUPPRESSED_FIRST_SCENE: u8 = 0x21;
/// Upper bound of the suppressed band, inclusive. See
/// [`IDLE_WORLD_STEP_SUPPRESSED_FIRST_SCENE`].
pub const IDLE_WORLD_STEP_SUPPRESSED_LAST_SCENE: u8 = 0x7F;

/// Alias for [`IDLE_WORLD_STEP_SUPPRESSED_FIRST_SCENE`].
pub const IDLE_WORLD_STEP_SUPPRESSED_FIRST: u8 = IDLE_WORLD_STEP_SUPPRESSED_FIRST_SCENE;
/// Alias for [`IDLE_WORLD_STEP_SUPPRESSED_LAST_SCENE`].
pub const IDLE_WORLD_STEP_SUPPRESSED_LAST: u8 = IDLE_WORLD_STEP_SUPPRESSED_LAST_SCENE;

// The suppression band and the dungeon-family band are the same numbers
// reached two different ways; if either published boundary ever moves this
// stops compiling rather than drifting silently.
const _: () = {
    assert!(IDLE_WORLD_STEP_SUPPRESSED_FIRST_SCENE == SCENE_DUNGEON_FAMILY_FIRST);
    assert!(IDLE_WORLD_STEP_SUPPRESSED_LAST_SCENE == SCENE_DUNGEON_FAMILY_LAST);
};

/// `timing.md §8.2`: does the idle wait skip its per-pass world step for
/// this scene value?
///
/// "Implement the gate as a numeric range test on the scene value, **not**
/// as an 'is this dungeon mode' test: the band is a strict superset of the
/// dungeon scenes, and the intro, character-creation and Return-to-View
/// animation states (`0x40`, `0x41`, `0x42`) also lie inside it."
///
/// "First-person dungeon scenes occupy `0x21..0x28` and therefore get no
/// idle world step - they run their own loop instead ... Combat sets scene
/// value `0xFF` and does run the world step."
pub const fn idle_world_step_suppressed_for_scene(scene_byte: u8) -> bool {
    scene_byte >= IDLE_WORLD_STEP_SUPPRESSED_FIRST_SCENE
        && scene_byte <= IDLE_WORLD_STEP_SUPPRESSED_LAST_SCENE
}

/// `timing.md §8.2`: what one pass of the input helper's idle wait did.
///
/// "On the overworld the input helper performs one scripted step-and-wait -
/// one world step followed by one one-tick wait - before either entering the
/// command wait or, when sails are set, performing a bare cursor poll
/// instead; so an **under-sail auto-advance pass costs two ticks and one
/// world step and never enters the command wait at all**."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdleWaitPass {
    /// The ordinary route: the scripted step-and-wait ran, and the helper is
    /// now entering the blocking command wait.
    CommandWait,
    /// Under sail, the first half of the auto-advance pass: the scripted
    /// world step.
    UnderSailWorldStep,
    /// Under sail, the second half: the bare cursor poll, which costs a tick
    /// and performs no world step. The helper returns after it instead of
    /// waiting for a command, which is what makes the route auto-advance.
    UnderSailCursorPoll,
}

impl IdleWaitPass {
    /// `timing.md §8.2`: the under-sail route "never enters the command wait
    /// at all", so only the ordinary route hands control to a blocking read.
    pub const fn enters_command_wait(self) -> bool {
        matches!(self, Self::CommandWait)
    }

    /// The pass performed the scripted world step. The bare cursor poll does
    /// not - it is one of the pumps that "share the one-tick wait but not the
    /// world step".
    pub const fn performed_world_step(self) -> bool {
        matches!(self, Self::CommandWait | Self::UnderSailWorldStep)
    }
}

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

/// `commands.md §11.1`: one Word-of-Power seal predicate row. The horizontal
/// coordinate pair is shared by both world surfaces; `plane` documents the
/// dungeon's canonical entrance surface, but does not gate Yell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordOfPowerSeal {
    pub word: &'static str,
    pub dungeon: &'static str,
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub unsealed_tile: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordOfPowerTargetOutcome {
    NoQualifyingNeighbor,
    RuinedShrine { x: usize, y: usize },
    WrongCoordinate { x: usize, y: usize },
    EntranceToggled { x: usize, y: usize, open: bool },
}

pub const WORD_OF_POWER_SEALED_TILE: u8 = 0xDF;
pub const WORLD_SHRINE_TILE: u8 = 0x19;
pub const WORLD_RUINED_SHRINE_TILE: u8 = 0x1A;

/// Public issue #32 byte-level Word-of-Power coordinate and category tables.
pub const WORD_OF_POWER_SEALS: [WordOfPowerSeal; 8] = [
    WordOfPowerSeal {
        word: "FALLAX",
        dungeon: "Deceit",
        plane: WorldPlane::Britannia,
        x: 240,
        y: 73,
        unsealed_tile: 0x18,
    },
    WordOfPowerSeal {
        word: "VILIS",
        dungeon: "Despise",
        plane: WorldPlane::Britannia,
        x: 91,
        y: 67,
        unsealed_tile: 0x16,
    },
    WordOfPowerSeal {
        word: "INOPIA",
        dungeon: "Destard",
        plane: WorldPlane::Britannia,
        x: 72,
        y: 168,
        unsealed_tile: 0x16,
    },
    WordOfPowerSeal {
        word: "MALUM",
        dungeon: "Wrong",
        plane: WorldPlane::Britannia,
        x: 126,
        y: 20,
        unsealed_tile: 0x18,
    },
    WordOfPowerSeal {
        word: "AVIDUS",
        dungeon: "Covetous",
        plane: WorldPlane::Britannia,
        x: 156,
        y: 27,
        unsealed_tile: 0x18,
    },
    WordOfPowerSeal {
        word: "INFAMA",
        dungeon: "Shame",
        plane: WorldPlane::Britannia,
        x: 58,
        y: 102,
        unsealed_tile: 0x17,
    },
    WordOfPowerSeal {
        word: "IGNAVUS",
        dungeon: "Hythloth",
        plane: WorldPlane::Britannia,
        x: 239,
        y: 240,
        unsealed_tile: 0x17,
    },
    WordOfPowerSeal {
        word: "VERAMOCOR",
        dungeon: "Doom",
        plane: WorldPlane::Underworld,
        x: 128,
        y: 128,
        unsealed_tile: 0x16,
    },
];

pub fn word_of_power_seal_for_word(word: &str) -> Option<WordOfPowerSeal> {
    WORD_OF_POWER_SEALS
        .iter()
        .copied()
        .find(|seal| seal.word.eq_ignore_ascii_case(word))
}

/// `commands.md §11.1`: the scanner walks the fixed word table and accepts
/// the first row whose complete word is a prefix of the normalized input.
pub fn word_of_power_seal_prefix_match(word: &str) -> Option<(usize, WordOfPowerSeal)> {
    WORD_OF_POWER_SEALS
        .iter()
        .copied()
        .enumerate()
        .find(|(_, seal)| word.starts_with(seal.word))
}

/// `catalogs/gazetteer.md §7`: shrine coordinates in the same fixed order as
/// the eight save-backed ruin flags. Spirituality's `(0, 0)` is the published
/// surface-map sentinel.
pub const WORLD_SHRINE_COORDINATES: [(usize, usize); SAVE_SHRINE_RUIN_FLAG_COUNT] = [
    (233, 66),
    (128, 92),
    (36, 229),
    (73, 11),
    (205, 45),
    (81, 207),
    (0, 0),
    (231, 216),
];

const fn same_world_chunk(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 / CHUNK_SIDE == b.0 / CHUNK_SIDE && a.1 / CHUNK_SIDE == b.1 / CHUNK_SIDE
}

pub fn word_of_power_chunk_owner(x: usize, y: usize) -> Option<usize> {
    WORD_OF_POWER_SEALS
        .iter()
        .position(|seal| same_world_chunk((x, y), (seal.x, seal.y)))
}

pub fn shrine_chunk_owner(x: usize, y: usize) -> Option<usize> {
    WORLD_SHRINE_COORDINATES
        .iter()
        .position(|coordinate| same_world_chunk((x, y), *coordinate))
}

/// `formats/brit-dat.md §9.1`: derive quest-gated live tiles from the shipped
/// unsealed/intact world map. This mutates only the in-memory decoded grid.
pub fn apply_world_quest_tile_substitutions(
    grid: &mut [u8],
    word_flags: &[u8; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
    shrine_flags: &[u8; SAVE_SHRINE_RUIN_FLAG_COUNT],
) {
    debug_assert_eq!(grid.len(), WORLD_CELLS);
    for (index, tile) in grid.iter_mut().take(WORLD_CELLS).enumerate() {
        let x = index % WORLD_SIDE;
        let y = index / WORLD_SIDE;
        if matches!(*tile, 0x16..=0x18) {
            let open = word_of_power_chunk_owner(x, y)
                .is_some_and(|owner| word_flags[owner] & SAVE_QUEST_TILE_FLAG_HIGH_BIT != 0);
            if !open {
                *tile = WORD_OF_POWER_SEALED_TILE;
            }
        } else if *tile == WORLD_SHRINE_TILE
            && shrine_chunk_owner(x, y)
                .is_some_and(|owner| shrine_flags[owner] & SAVE_QUEST_TILE_FLAG_HIGH_BIT != 0)
        {
            *tile = WORLD_RUINED_SHRINE_TILE;
        }
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
pub const DUNGEON_FACING_EAST: u8 = DUNGEON_FACING_NORTH + 1;
pub const DUNGEON_FACING_SOUTH: u8 = DUNGEON_FACING_EAST + 1;
pub const DUNGEON_FACING_WEST: u8 = DUNGEON_FACING_SOUTH + 1;

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
/// party lands on the deepest floor (`Z = DUNGEON_DEEPEST_LEVEL`)
/// at the published `(X, Y) = (DUNGEON_SIDE - 1, DUNGEON_SIDE - 1)`
/// south-east corner cell, facing west. Anchor the Z to
/// [`crate::DUNGEON_DEEPEST_LEVEL`] and the X/Y to the
/// [`crate::DUNGEON_SIDE`]-derived corner index so the underworld
/// entry seed derives from the dungeon's published 8x8 floor
/// dimensions.
pub const DUNGEON_ENTRY_UNDERWORLD_Z: u8 = crate::DUNGEON_DEEPEST_LEVEL;
pub const DUNGEON_ENTRY_UNDERWORLD_X: u8 = crate::DUNGEON_SIDE as u8 - 1;
pub const DUNGEON_ENTRY_UNDERWORLD_Y: u8 = crate::DUNGEON_SIDE as u8 - 1;

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
