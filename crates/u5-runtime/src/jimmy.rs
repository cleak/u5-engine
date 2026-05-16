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
    let raw = difficulty - member_class as i16 + 30;
    if raw < 0 {
        Some(0)
    } else {
        Some((raw as u16 / 2) as u8)
    }
}
pub const fn object_chest_jimmy_succeeds(threshold: u8, roll_1_to_30: u8) -> bool {
    roll_1_to_30 <= threshold
}

/// `doors-and-z-transitions.md §3`: dungeon chest pick. Threshold is
/// `(2*depth - member_class + 30) / 2`; roll is `1..=30` and success
/// occurs when `roll <= threshold`.
pub const fn dungeon_chest_jimmy_threshold(depth: u8, member_class: u8) -> u8 {
    let raw = (2 * depth as i16) - member_class as i16 + 30;
    if raw < 0 {
        0
    } else {
        (raw as u16 / 2) as u8
    }
}
pub const fn dungeon_chest_jimmy_succeeds(threshold: u8, roll_1_to_30: u8) -> bool {
    roll_1_to_30 <= threshold
}

/// `doors-and-z-transitions.md §5`: O-Open initialises the door
/// auto-close countdown to four turns; each turn-consuming pass
/// decrements it.
pub const DOOR_AUTO_CLOSE_TURNS: u8 = 4;

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
