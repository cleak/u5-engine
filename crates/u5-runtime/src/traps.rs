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

/// `traps.md §3` uniform trap damage draw. Used by the Bomb family,
/// which rolls `1..=8` independently for each living member. Acid does
/// **not** use this shape — see [`shared_trap_acid_damage_from_index`].
pub fn shared_trap_damage_from_index(index: u8, max_damage: u8) -> u8 {
    1 + (index % max_damage)
}

/// `traps.md §3` inclusive raw-roll ceiling the Acid family draws
/// before halving: "The roll is an inclusive `0..60` roll halved with
/// truncation and floored to one".
pub const TRAP_ACID_RAW_ROLL_MAX: u8 = 60;

/// `traps.md §3` effect id 0 (Acid) damage draw.
///
/// "The roll is an inclusive `0..60` roll halved with truncation and
/// floored to one - the same shape `systems/magic.md` publishes for
/// Mani - so it is **not** uniform over `1..30`: low values are
/// markedly more likely."
///
/// The halve-and-floor step is [`crate::combat_skewed_roll_1_to_30`]
/// (`combat.md §9.1`), reused rather than duplicated so the two shapes
/// cannot drift. The caller-supplied `index` is folded into the
/// published inclusive `0..=60` input domain first; the result is
/// always inside the `1..=30` bound the same paragraph states.
pub fn shared_trap_acid_damage_from_index(index: u8) -> u8 {
    let raw = index % (TRAP_ACID_RAW_ROLL_MAX + 1);
    let damage = crate::combat_skewed_roll_1_to_30(raw);
    debug_assert!(damage >= 1 && damage <= TRAP_ACID_DAMAGE_MAX);
    damage
}

/// `traps.md §3` shared trap-effect family for one resolved id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrapEffect {
    /// Effect id 0 — acid sting; rolls damage on the triggering slot
    /// only. The draw is bounded by `1..30` but is the skewed
    /// halve-and-floor shape, not a uniform draw — see
    /// [`shared_trap_acid_damage_from_index`].
    Acid,
    /// Effect id 1 — poison label; runs the shared poison-status helper
    /// for the triggering slot.
    Poison,
    /// Effect id 2 — bomb; rolls `1..=8` damage independently for each
    /// living member in slots `0..=5`.
    Bomb,
    /// Effect id 3 — gas; runs the shared poison-status helper for slots
    /// `0..=5`.
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
/// roll damage. Returns `None` for the poison-helper families.
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
/// shared poison-status helper (Poison and Gas). Acid and Bomb roll
/// damage instead.
pub const fn trap_effect_uses_poison_helper(effect: TrapEffect) -> bool {
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
/// resolver's poison helper. The helper rewrites an accepted slot to
/// the `'P'` status byte and refreshes the stats panel. It touches no
/// hit points, magic points, or maxima, and has no relation to the
/// resurrection spell path.
pub const TRAP_POISON_STATUS_BYTE: u8 = b'P';

/// `traps.md §3`: returns `true` when the trap-effect resolver's
/// poison helper accepts the supplied slot status. A member already
/// marked Dead is skipped and left Dead; every other status is
/// rewritten to Poisoned.
///
/// The helper tests **exactly two** things: the slot index against the
/// live party count, and the status byte against `'D'` alone. If both
/// pass it writes `'P'`. Nothing else is touched — no HP, no maximum
/// HP, no mana. Confirmed against the shipped routine in answer to
/// `cleak/u5-spec#89`, and stated as a positive because the negative
/// ("it does not test Ashes") is the form a partial reading gets wrong.
///
/// Ashes is `'A'`, a distinct value from `'D'`, so an Ashes member
/// fails the Dead test and **is** written to Poisoned. The asymmetry is
/// real and deliberate: an Ashes character cannot be resurrected by the
/// ordinary path, which treats only Dead as a valid target, yet can be
/// converted to Poisoned by a gas trap. Poisoning a pile of ashes is
/// odd, which is exactly why it is spelled out here — a reasonable
/// implementer assumes Ashes shares the Dead exclusion and fixes a
/// divergence into existence.
pub const fn trap_poison_accepts(status: crate::CharacterStatus) -> bool {
    trap_poison_accepts_status_byte(status.save_byte())
}

/// `traps.md §3`: raw-status-byte form of [`trap_poison_accepts`], for
/// the resolver's party slots, which hold the published status letter
/// rather than a decoded [`crate::CharacterStatus`].
pub const fn trap_poison_accepts_status_byte(status: u8) -> bool {
    status != crate::CharacterStatus::Dead.save_byte()
}

/// `traps.md §3` effect id 2 (Bomb): "rolls an inclusive `1..8` damage
/// separately for each in-party member of the six-slot band that is not
/// marked Dead - the only status excluded is Dead… The sweep applies the
/// same two gates the status helper applies: an unsigned party-count check,
/// then a Dead skip."
///
/// Named separately from [`trap_poison_accepts_status_byte`] because the
/// bomb sweep deals damage rather than poison, but the predicate is
/// deliberately the same single-value test: Ashes (`'A'`) is a distinct
/// status from Dead (`'D'`) and must fall through into the sweep.
pub const fn trap_party_sweep_accepts_status_byte(status: u8) -> bool {
    trap_poison_accepts_status_byte(status)
}

/// `traps.md §2.1` outcome of the shared acting-member selection — the
/// selection that decides who performs Search, Jimmy, Get, Open, Look and
/// Cast, and which both container call sites consult **before** they test
/// whether the container is trapped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActingMemberSelection {
    /// A slot was chosen silently: no prompt, and no echo of the chosen
    /// member's name. §2.1 is explicit that the single-qualifier case does
    /// *not* echo the name, where a prompted pick does, so an engine that
    /// echoes here prints a line the original does not.
    Selected(usize),
    /// Two or more members qualify, so the interactive picker runs.
    Prompt,
    /// Nobody qualifies. The command reports it and aborts — before the
    /// trap can fire.
    NoneAble,
}

/// `traps.md §2.1` branch 3 status gate: the scan considers members whose
/// status is Good or Poisoned. Every other status — Dead, Ashes, Asleep,
/// Charmed — is ineligible, and a confirmed pick that is ineligible is
/// rejected with the short "disabled" notice rather than accepted.
pub const fn acting_member_status_eligible(status: u8) -> bool {
    matches!(status, b'G' | b'P')
}

/// `traps.md §2.1` branch 3: scan the roster positions inside the current
/// party count for Good-or-Poisoned members, **keeping the last match**.
///
/// Three outcomes, and the middle one is the easy one to get wrong: zero
/// matches aborts the command, **exactly one match is auto-selected
/// silently with no prompt at all**, and two or more prompt. `statuses`
/// is the status byte of each roster position already bounded by the
/// party count.
pub fn acting_member_scan(statuses: &[u8]) -> ActingMemberSelection {
    let mut last = None;
    let mut matches = 0usize;
    for (slot, status) in statuses.iter().copied().enumerate() {
        if acting_member_status_eligible(status) {
            last = Some(slot);
            matches += 1;
        }
    }
    match (matches, last) {
        (1, Some(slot)) => ActingMemberSelection::Selected(slot),
        (0, _) => ActingMemberSelection::NoneAble,
        _ => ActingMemberSelection::Prompt,
    }
}
