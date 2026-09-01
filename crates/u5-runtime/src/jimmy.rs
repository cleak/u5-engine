//! Lockpicking helpers per `doors-and-z-transitions.md` §3. Jimmy uses
//! two different roll formulas:
//!   - Locked doors and restraints: roll `0..=29` strictly less than
//!     the picker's Dexterity.
//!   - Per-map object chests: `(object_difficulty - Dexterity + 30)/2`
//!     threshold against a `1..=30` roll, requiring the lock/trap high bit.
//!   - Dungeon chests: `(2*depth - Dexterity + 30)/2` threshold
//!     against a `1..=30` roll.
//! These helpers implement only the success/failure decision; key counter
//! mutation, narration, and tile rewrites are caller responsibilities.

/// `doors-and-z-transitions.md §9` outdoor Klimb fall risk roll
/// die. Each living party member rolls `1..=30` against their
/// Dexterity byte; a roll above the Dex byte is a fall.
pub const OUTDOOR_KLIMB_FALL_DIE_LOW: u8 = 1;
pub const OUTDOOR_KLIMB_FALL_DIE_HIGH: u8 = 30;

/// `doors-and-z-transitions.md §9` outdoor K-Klimb entry-gate
/// outcome. The handler refuses before probing the target cell when
/// the party lacks the Grapple quest flag or is in a vehicle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverworldKlimbEntryGate {
    /// Party lacks the Grapple flag — handler prints "With what?"
    /// and exits.
    NoGrapple,
    /// Party is in a vehicle state (not on foot) — handler prints
    /// "On foot!" and exits.
    NotOnFoot,
    /// Both gates passed — handler proceeds to the target-tile probe.
    Proceed,
}

/// `doors-and-z-transitions.md §9`: classify the outdoor K-Klimb
/// entry gate result. The Grapple check is consulted first; the
/// vehicle check is only reached when the Grapple flag is set.
pub const fn overworld_klimb_entry_gate(
    has_grapple: bool,
    on_foot: bool,
) -> OverworldKlimbEntryGate {
    if !has_grapple {
        return OverworldKlimbEntryGate::NoGrapple;
    }
    if !on_foot {
        return OverworldKlimbEntryGate::NotOnFoot;
    }
    OverworldKlimbEntryGate::Proceed
}

/// `doors-and-z-transitions.md §9`: returns `true` when the
/// supplied member falls during the outdoor Klimb risk roll. The
/// fall fires when the rolled value is strictly greater than the
/// member's Dexterity byte.
pub const fn outdoor_klimb_member_falls(dexterity: u8, roll_1_to_30: u8) -> bool {
    roll_1_to_30 > dexterity
}

/// `doors-and-z-transitions.md §9` fall damage roll bounds applied
/// to a member that fell during the outdoor Klimb pass.
pub const OUTDOOR_KLIMB_FALL_DAMAGE_MIN: u8 = 1;
pub const OUTDOOR_KLIMB_FALL_DAMAGE_MAX: u8 = 5;

/// `doors-and-z-transitions.md §3`: locked-door/restraint success
/// predicate. Roll is uniform `[0, 29]`; success when the picker's
/// Dexterity is strictly greater than the roll.
pub const JIMMY_DOOR_DIE_LOW: u8 = 0;
pub const JIMMY_DOOR_DIE_HIGH: u8 = 29;
pub const fn jimmy_door_succeeds(dexterity: u8, roll_0_to_29: u8) -> bool {
    dexterity > roll_0_to_29
}

/// `doors-and-z-transitions.md §3` shared moral-standing reward for
/// a successful prisoner release. The shared selector is raised by
/// this many units on success, then clamped at the published
/// 99 cap. Failure does not advance the freed/thanked state and
/// does not apply this increase.
pub const JIMMY_PRISONER_RELEASE_KARMA_REWARD: u8 = 2;

/// Native top-down door/restraint bytes used by Open and Jimmy.
pub const TOWN_DOOR_PLAIN_UNLOCKED_TILE: u8 = 0xB8;
pub const TOWN_DOOR_PLAIN_LOCKED_TILE: u8 = 0xB9;
pub const TOWN_DOOR_WINDOWED_UNLOCKED_TILE: u8 = 0xBA;
pub const TOWN_DOOR_WINDOWED_LOCKED_TILE: u8 = 0xBB;
pub const TOWN_DOOR_MAGIC_PLAIN_TILE: u8 = 0x97;
pub const TOWN_DOOR_MAGIC_WINDOWED_TILE: u8 = 0x98;
pub const TOWN_OPEN_ALREADY_OPEN_TILE: u8 = 0xAF;
/// `doors-and-z-transitions.md §2.2`: this exact non-dungeon tile selects
/// Open's fixed "Too heavy!" branch. Neighboring high-range tiles do not.
pub const TOWN_OPEN_TOO_HEAVY_TILE: u8 = 0x99;
pub const TOWN_DOOR_CLEARED_TILE: u8 = 0x44;
pub const JIMMY_STOCKS_TILE: u8 = 0x84;
pub const JIMMY_MANACLES_TILE: u8 = 0x85;
pub const JIMMY_RELEASE_AI_MODE: u8 = 5;

/// `dungeon-mode.md` / `containers.md`: opening a dungeon chest replaces
/// its class nibble, clears the mutable subtype/trap bits, and preserves only
/// the visit-local marker bit.
pub const DUNGEON_OPEN_CHEST_CLASS: u8 = 0x70;
pub const DUNGEON_VISIT_MARKER_BIT: u8 = 0x08;
pub const fn dungeon_open_chest_rewrite(tile: u8) -> u8 {
    DUNGEON_OPEN_CHEST_CLASS | (tile & DUNGEON_VISIT_MARKER_BIT)
}

pub const fn jimmy_locked_door_rewrite(tile: u8) -> Option<u8> {
    match tile {
        TOWN_DOOR_PLAIN_LOCKED_TILE => Some(TOWN_DOOR_PLAIN_UNLOCKED_TILE),
        TOWN_DOOR_WINDOWED_LOCKED_TILE => Some(TOWN_DOOR_WINDOWED_UNLOCKED_TILE),
        _ => None,
    }
}

pub const fn jimmy_magic_locked_door(tile: u8) -> bool {
    matches!(
        tile,
        TOWN_DOOR_MAGIC_PLAIN_TILE | TOWN_DOOR_MAGIC_WINDOWED_TILE
    )
}

pub const fn jimmy_restraint_tile(tile: u8) -> bool {
    matches!(tile, JIMMY_STOCKS_TILE | JIMMY_MANACLES_TILE)
}

pub const fn openable_town_door(tile: u8) -> bool {
    matches!(
        tile,
        TOWN_DOOR_PLAIN_UNLOCKED_TILE | TOWN_DOOR_WINDOWED_UNLOCKED_TILE
    )
}

pub const fn town_command_door_tile(tile: u8) -> bool {
    openable_town_door(tile)
        || jimmy_locked_door_rewrite(tile).is_some()
        || jimmy_magic_locked_door(tile)
}

/// `doors-and-z-transitions.md §3` shared `+30` bias applied to the
/// object-chest and dungeon-chest pick thresholds before halving.
/// Both formulas are `(difficulty - Dexterity + JIMMY_CHEST_THRESHOLD_BIAS) / 2`,
/// so the bias is shared spec data rather than a per-formula constant.
pub const JIMMY_CHEST_THRESHOLD_BIAS: i16 = 30;

/// `doors-and-z-transitions.md §3`: per-map object chest pick. Returns
/// `None` when the high bit of the object stat is clear (the container is
/// already unlocked/disarmed and takes the wasteful broken-key short circuit).
/// Otherwise computes the threshold with the original wrapping unsigned-word
/// arithmetic and tests the `1..=30` roll.
pub const JIMMY_OBJECT_DIE_LOW: u8 = 1;
pub const JIMMY_OBJECT_DIE_HIGH: u8 = 30;
pub const fn object_chest_jimmy_threshold(object_stat: u8, dexterity: u8) -> Option<u16> {
    if object_stat & 0x80 == 0 {
        return None;
    }
    let difficulty = (object_stat & 0x7f) as u16;
    let raw = difficulty
        .wrapping_sub(dexterity as u16)
        .wrapping_add(JIMMY_CHEST_THRESHOLD_BIAS as u16);
    Some(raw / JIMMY_CHEST_THRESHOLD_DIVISOR)
}
pub const fn object_chest_jimmy_succeeds(threshold: u16, roll_1_to_30: u8) -> bool {
    (roll_1_to_30 as u16) > threshold
}

/// `doors-and-z-transitions.md §3` dungeon-chest pick depth
/// multiplier. The threshold formula
/// `(2 * dungeon_depth - Dexterity + 30) / 2` uses this factor on
/// the depth term; promoting it lets the helper name the depth
/// weight rather than encoding `2` as a bare literal.
pub const JIMMY_DUNGEON_CHEST_DEPTH_MULTIPLIER: i16 = 2;
/// `doors-and-z-transitions.md §3` divisor applied to both chest-pick
/// thresholds (object and dungeon) before the `1..=30` roll compare.
/// The original formulas halve the bias-adjusted difficulty before
/// comparing against the die.
pub const JIMMY_CHEST_THRESHOLD_DIVISOR: u16 = 2;

/// `doors-and-z-transitions.md §3`: dungeon chest pick. Threshold is
/// `(2*depth - Dexterity + JIMMY_CHEST_THRESHOLD_BIAS) / 2`; roll
/// is `1..=30` and success occurs when the roll is strictly greater.
pub const fn dungeon_chest_jimmy_threshold(depth: u8, dexterity: u8) -> u16 {
    let weighted_depth = depth as u16 * JIMMY_DUNGEON_CHEST_DEPTH_MULTIPLIER as u16;
    weighted_depth
        .wrapping_sub(dexterity as u16)
        .wrapping_add(JIMMY_CHEST_THRESHOLD_BIAS as u16)
        / JIMMY_CHEST_THRESHOLD_DIVISOR
}
pub const fn dungeon_chest_jimmy_succeeds(threshold: u16, roll_1_to_30: u8) -> bool {
    (roll_1_to_30 as u16) > threshold
}

/// `doors-and-z-transitions.md §5`: O-Open initialises the door
/// auto-close countdown to four turns; each turn-consuming pass
/// decrements it.
pub const DOOR_AUTO_CLOSE_TURNS: u8 = 4;

/// `doors-and-z-transitions.md §5` per-turn outcome for the door
/// auto-close tracker. Each turn-consuming pass decrements the
/// countdown; when it reaches zero the saved cell is rewritten back
/// to the previous (closed but unlocked) tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorAutoCloseTick {
    /// No door is currently being tracked (idle slot).
    Idle,
    /// The countdown still has turns remaining; the helper rewrites
    /// the slot with the decremented countdown and leaves the cell
    /// open. Carries the remaining countdown.
    DecrementInPlace(u8),
    /// The countdown reached zero this turn; the cell is rewritten
    /// to the saved previous tile (closed/unlocked) and the slot
    /// becomes idle.
    CloseAndClear,
}

/// `doors-and-z-transitions.md §5`: drive one turn of the door
/// auto-close tracker. Caller passes `Some(current_countdown)` for
/// an active slot or `None` for an idle slot. Dungeon mode is
/// suppressed at the caller — pass `None` (or skip this call) when
/// the active scene is a dungeon.
pub const fn door_auto_close_tick(slot_countdown: Option<u8>) -> DoorAutoCloseTick {
    match slot_countdown {
        None => DoorAutoCloseTick::Idle,
        Some(0) => DoorAutoCloseTick::CloseAndClear,
        Some(remaining) => {
            let next = remaining - 1;
            if next == 0 {
                DoorAutoCloseTick::CloseAndClear
            } else {
                DoorAutoCloseTick::DecrementInPlace(next)
            }
        }
    }
}

/// `doors-and-z-transitions.md §7` magic Open/Unlock helper. The
/// helper opens only ordinary closed wooden-door variants and uses
/// fixed per-variant rewrites. Returns `None` for any other tile —
/// non-wooden doors, magic-locked variants, chests, NPCs, walls.
pub const MAGIC_UNLOCK_CLOSED_WOODEN_A: u8 = TOWN_DOOR_MAGIC_PLAIN_TILE;
pub const MAGIC_UNLOCK_OPEN_WOODEN_A: u8 = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
pub const MAGIC_UNLOCK_CLOSED_WOODEN_B: u8 = TOWN_DOOR_MAGIC_WINDOWED_TILE;
pub const MAGIC_UNLOCK_OPEN_WOODEN_B: u8 = TOWN_DOOR_WINDOWED_UNLOCKED_TILE;
pub const fn magic_unlock_door_rewrite(tile: u8) -> Option<u8> {
    match tile {
        MAGIC_UNLOCK_CLOSED_WOODEN_A => Some(MAGIC_UNLOCK_OPEN_WOODEN_A),
        MAGIC_UNLOCK_CLOSED_WOODEN_B => Some(MAGIC_UNLOCK_OPEN_WOODEN_B),
        _ => None,
    }
}
