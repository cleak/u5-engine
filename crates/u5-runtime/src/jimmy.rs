//! Lockpicking helpers per `doors-and-z-transitions.md` §3. Jimmy uses
//! three different roll formulas:
//!   - Doors, visible chests, and NPC pockets: roll `1..=29` strictly
//!     less than the picker's class byte.
//!   - Per-map object chests: `(object_difficulty - member_class + 30)/2`
//!     threshold against a `1..=30` roll, requiring the stat high bit.
//!   - Dungeon chests: `(2*depth - member_class + 30)/2` threshold
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

/// `doors-and-z-transitions.md §3`: door / visible-chest / NPC pickpocket
/// success predicate. Roll is uniform `[1, 29]`; success when the
/// picker's class byte is strictly greater than the roll.
pub const JIMMY_DOOR_DIE_LOW: u8 = 1;
pub const JIMMY_DOOR_DIE_HIGH: u8 = 29;
pub const fn jimmy_door_succeeds(member_class: u8, roll_1_to_29: u8) -> bool {
    member_class > roll_1_to_29
}

/// `doors-and-z-transitions.md §3` shared moral-standing reward for
/// a successful NPC pickpocket. The shared selector is raised by
/// this many units on success, then clamped at the published
/// 99 cap. Failure does not advance the picked/thanked state and
/// does not apply this increase.
pub const JIMMY_NPC_PICKPOCKET_KARMA_REWARD: u8 = 2;

/// `doors-and-z-transitions.md §3` shared `+30` bias applied to the
/// object-chest and dungeon-chest pick thresholds before halving.
/// Both formulas are `(difficulty - member_class + JIMMY_CHEST_THRESHOLD_BIAS) / 2`,
/// so the bias is shared spec data rather than a per-formula constant.
pub const JIMMY_CHEST_THRESHOLD_BIAS: i16 = 30;

/// `doors-and-z-transitions.md §3`: per-map object chest pick. Returns
/// `None` when the high bit of the object stat is clear (the chest is in
/// the broken-lock state and no real pick can occur). Otherwise computes
/// the threshold using the original unsigned word halving and tests the
/// `1..=30` roll.
pub const JIMMY_OBJECT_DIE_LOW: u8 = 1;
pub const JIMMY_OBJECT_DIE_HIGH: u8 = 30;
pub const fn object_chest_jimmy_threshold(object_stat: u8, member_class: u8) -> Option<u8> {
    if object_stat & 0x80 == 0 {
        return None;
    }
    let difficulty = (object_stat & 0x7f) as i16;
    let raw = difficulty - member_class as i16 + JIMMY_CHEST_THRESHOLD_BIAS;
    if raw < 0 {
        Some(0)
    } else {
        Some((raw as u16 / JIMMY_CHEST_THRESHOLD_DIVISOR) as u8)
    }
}
pub const fn object_chest_jimmy_succeeds(threshold: u8, roll_1_to_30: u8) -> bool {
    roll_1_to_30 <= threshold
}

/// `doors-and-z-transitions.md §3` dungeon-chest pick depth
/// multiplier. The threshold formula
/// `(2 * dungeon_depth - member_class + 30) / 2` uses this factor on
/// the depth term; promoting it lets the helper name the depth
/// weight rather than encoding `2` as a bare literal.
pub const JIMMY_DUNGEON_CHEST_DEPTH_MULTIPLIER: i16 = 2;
/// `doors-and-z-transitions.md §3` divisor applied to both chest-pick
/// thresholds (object and dungeon) before the `1..=30` roll compare.
/// The original formulas halve the bias-adjusted difficulty before
/// comparing against the die.
pub const JIMMY_CHEST_THRESHOLD_DIVISOR: u16 = 2;

/// `doors-and-z-transitions.md §3`: dungeon chest pick. Threshold is
/// `(2*depth - member_class + JIMMY_CHEST_THRESHOLD_BIAS) / 2`; roll
/// is `1..=30` and success occurs when `roll <= threshold`.
pub const fn dungeon_chest_jimmy_threshold(depth: u8, member_class: u8) -> u8 {
    let raw = JIMMY_DUNGEON_CHEST_DEPTH_MULTIPLIER * (depth as i16)
        - member_class as i16
        + JIMMY_CHEST_THRESHOLD_BIAS;
    if raw < 0 {
        0
    } else {
        (raw as u16 / JIMMY_CHEST_THRESHOLD_DIVISOR) as u8
    }
}
pub const fn dungeon_chest_jimmy_succeeds(threshold: u8, roll_1_to_30: u8) -> bool {
    roll_1_to_30 <= threshold
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
pub const MAGIC_UNLOCK_CLOSED_WOODEN_A: u8 = 0x97;
pub const MAGIC_UNLOCK_OPEN_WOODEN_A: u8 = 0xB8;
pub const MAGIC_UNLOCK_CLOSED_WOODEN_B: u8 = 0x98;
pub const MAGIC_UNLOCK_OPEN_WOODEN_B: u8 = 0xBA;
pub const fn magic_unlock_door_rewrite(tile: u8) -> Option<u8> {
    match tile {
        MAGIC_UNLOCK_CLOSED_WOODEN_A => Some(MAGIC_UNLOCK_OPEN_WOODEN_A),
        MAGIC_UNLOCK_CLOSED_WOODEN_B => Some(MAGIC_UNLOCK_OPEN_WOODEN_B),
        _ => None,
    }
}
