use crate::{TRAP_ACID_DAMAGE_MAX, TRAP_BOMB_DAMAGE_MAX, TRAP_NON_COMBAT_EFFECT_TABLE};

pub fn shared_trap_effect_id_from_index(index: u8, combat_active: bool) -> u8 {
    if combat_active {
        index & 1
    } else {
        TRAP_NON_COMBAT_EFFECT_TABLE[usize::from(index & 7)]
    }
}

/// `traps.md §3`: classify a raw `index` byte into a [`TrapEffect`]
/// family using the same combat/non-combat split as
/// [`shared_trap_effect_id_from_index`]. Combat scenes always land on
/// Acid or Poison; non-combat scenes consult the published 8-slot
/// outcome table, so every byte resolves to one of the four families.
pub fn shared_trap_effect_family_from_index(index: u8, combat_active: bool) -> TrapEffect {
    let id = shared_trap_effect_id_from_index(index, combat_active);
    match id {
        0 => TrapEffect::Acid,
        1 => TrapEffect::Poison,
        2 => TrapEffect::Bomb,
        _ => TrapEffect::Gas,
    }
}

pub fn shared_trap_damage_from_index(index: u8, max_damage: u8) -> u8 {
    1 + (index % max_damage)
}

/// `traps.md §3` shared trap-effect family for one resolved id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrapEffect {
    /// Effect id 0 — acid sting; rolls `1..=30` damage on the triggering
    /// slot only.
    Acid,
    /// Effect id 1 — poison label; runs the narrow revive helper for the
    /// triggering slot.
    Poison,
    /// Effect id 2 — bomb; rolls `1..=8` damage independently for each
    /// living member in slots `0..=5`.
    Bomb,
    /// Effect id 3 — gas; runs the narrow revive helper for slots `0..=5`.
    Gas,
}

/// `traps.md §3`: classify a resolver effect id into its family.
pub const fn trap_effect_for_id(effect_id: u8) -> Option<TrapEffect> {
    Some(match effect_id {
        0 => TrapEffect::Acid,
        1 => TrapEffect::Poison,
        2 => TrapEffect::Bomb,
        3 => TrapEffect::Gas,
        _ => return None,
    })
}

/// `traps.md §3`: per-family damage upper bound for the families that
/// roll damage. Returns `None` for revive-helper families.
pub const fn trap_effect_damage_max(effect: TrapEffect) -> Option<u8> {
    match effect {
        TrapEffect::Acid => Some(TRAP_ACID_DAMAGE_MAX),
        TrapEffect::Bomb => Some(TRAP_BOMB_DAMAGE_MAX),
        TrapEffect::Poison | TrapEffect::Gas => None,
    }
}

/// `traps.md §3`: predicate marking effects that target every living
/// party slot (rather than just the triggering slot).
pub const fn trap_effect_targets_whole_party(effect: TrapEffect) -> bool {
    matches!(effect, TrapEffect::Bomb | TrapEffect::Gas)
}

/// `traps.md §3`: predicate marking effects that route through the
/// narrow revive helper (Poison and Gas). Acid and Bomb roll damage
/// instead.
pub const fn trap_effect_uses_revive_helper(effect: TrapEffect) -> bool {
    matches!(effect, TrapEffect::Poison | TrapEffect::Gas)
}

/// `traps.md §3` non-combat lookup-table outcome count for the given
/// family. Acid maps to 3 of the 8 equiprobable rolls, Poison and
/// Bomb to 2 each, Gas to 1. Sum equals the table size of 8.
pub const fn trap_non_combat_outcomes(effect: TrapEffect) -> u8 {
    match effect {
        TrapEffect::Acid => 3,
        TrapEffect::Poison => 2,
        TrapEffect::Bomb => 2,
        TrapEffect::Gas => 1,
    }
}

/// `traps.md §3` combat-class scenes only roll between effect ids
/// `0` (Acid) and `1` (Poison). Returns `true` for those families;
/// `false` for Bomb and Gas, which never appear in combat traps.
pub const fn trap_effect_appears_in_combat(effect: TrapEffect) -> bool {
    matches!(effect, TrapEffect::Acid | TrapEffect::Poison)
}

/// `traps.md §3` raw status byte written by the trap-effect
/// resolver's revive helper. The helper rewrites Dead slots to the
/// `'P'` status byte and refreshes the stats panel, without
/// running the broader resurrection spell contract.
pub const TRAP_REVIVE_STATUS_BYTE: u8 = b'P';

/// `traps.md §3`: returns `true` when the trap-effect resolver's
/// revive helper accepts the supplied slot status. Only Dead slots
/// are revived; every other status is left unchanged.
pub const fn trap_revive_accepts(status: crate::CharacterStatus) -> bool {
    matches!(status, crate::CharacterStatus::Dead)
}
