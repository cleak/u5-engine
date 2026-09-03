//! `combat.md §8.2` — the `A`-Attack attempt walker and its interactive
//! arena targeting cursor.
//!
//! "After `A` is accepted, does the engine enter a distinct direction read?
//! Yes - accepting Attack opens a second, separate input read, and it is not
//! a one-shot direction key but an **interactive targeting cursor**. It is
//! entered once per readied weapon, not once per Attack command, and it is
//! not reached unconditionally."
//!
//! This module owns the pure half of that contract: which attempts one
//! Attack produces, the per-attempt maximum range, which items run the
//! adjacent-attacker interference abort, the cursor's key vocabulary, and
//! the truncated-Euclidean move test. `combat_frame.rs` owns the stateful
//! half (opening the cursor, resolving a confirmation, spending the turn).

use crate::combat_actor::{
    CombatActorDescriptor, combat_actor_is_present_not_dead, combat_arena_range,
};
use crate::combat_arena::COMBAT_ARENA_SIDE;
use crate::constants::EQUIPMENT_SLOT_COUNT;
use crate::equipment::{combat_armament_item_ids, equipment_name, equipment_weapon_range_cap};
use crate::input_codes::{InputDirection, input_code_direction};

/// `combat.md §8.2`: "Five items - **sling, flaming oil, bow, crossbow and
/// magic bow** - run an interference test before the cursor."
///
/// `catalogs/item-list.md §5.3` names the same five as "the five true
/// missile items - Sling, Flaming Oil, Bow, Crossbow and Magic Bow" and
/// publishes their equipment ids in its range-cap table (17, 19, 26, 28 and
/// 36). "The other reach-bearing items - dagger, spear, throwing axe,
/// morning star, halberd, magic axe - do **not** run this test, and neither
/// does any zero-reach melee attempt or a bare-handed attempt."
pub const COMBAT_MISSILE_INTERFERENCE_ITEM_IDS: [usize; 5] = [17, 19, 26, 28, 36];

/// `combat.md §8.2`: "On abort the engine prints a newline, the interfering
/// actor's name, and ` interferes!`".
pub const COMBAT_INTERFERES_TAIL: &str = " interferes!";

/// `combat.md §8.2`: "On cancel the engine prints `Nothing!` (melee arm) or
/// returns silently (ranged arm)." The same string is printed on a
/// confirmation that finds no eligible occupant.
pub const COMBAT_TARGETING_NOTHING_LINE: &str = "Nothing!";

/// `combat.md §8.2` separator between an item-name line and its colon, for
/// the two- or three-qualifying-item case: "each attempt additionally prints
/// a newline, that item's name, and a colon on its own line before its
/// `Attack-`".
pub const COMBAT_ATTACK_ITEM_LINE_TERMINATOR: &str = ":";

/// `combat.md §12` stage-one damage, party row: "Values `0` and `1` pass
/// through unchanged, and **bare hands are a flat `1`**."
pub const COMBAT_BARE_HANDED_ATTACK_MAX: u8 = 1;

/// `combat.md §8.2`: a bare-handed attempt "behaves as melee with range
/// one", which is the zero-reach arm - "reach zero opens the cursor with
/// maximum range one".
pub const COMBAT_BARE_HANDED_RANGE_CAP: u8 = 0;

/// A bare-handed attempt has no readied item and therefore no per-item
/// ranged effect row. Nothing published gives it one, and with a range cap
/// of zero the resolver never reaches the ranged arm to read it.
pub const COMBAT_BARE_HANDED_EFFECT_CODE: u8 = 0;

/// One `A`-Attack attempt: `combat.md §8.2` "walks the acting character's
/// three readied equipment slots in order - helm, weapon hand, shield hand.
/// Each slot holding an item with a non-zero weapon-capability entry
/// produces **one attack attempt** ... A character with no qualifying item
/// makes a single bare-handed attempt, which behaves as melee with range
/// one."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatAttackAttempt {
    /// The readied item this attempt swings, or `None` for the published
    /// bare-handed attempt.
    pub item_id: Option<usize>,
    /// `§8.2`: "reach zero opens the cursor with maximum range one; a
    /// non-zero reach opens it with that reach as the maximum range."
    pub max_range: u8,
    /// `§8.2` melee arm — the zero-reach and bare-handed attempts. Only this
    /// arm prints `Nothing!` on cancel; "a non-zero reach" is the ranged arm
    /// and "returns silently".
    pub melee_arm: bool,
    /// `§8.2`: whether this attempt runs the adjacent-attacker interference
    /// abort before the cursor.
    pub runs_interference: bool,
    /// `combat.md §11`, the spell/weapon dispatcher's non-party-side arm -
    /// "the arm a controlled monster acting at the player's prompt reaches".
    /// A class reach selector above `1` "selects the **cast/effect arm**
    /// unconditionally, at every distance including one". `§11.1`: such a
    /// class "takes the cast/effect arm instead and prints no `Aim! `", opens
    /// no cursor, "and reaches neither the melee miss line nor any of the
    /// three `Nothing!` routes" (`RETRACTIONS.md` R382).
    ///
    /// What that arm *does* is not published - `§11.1`'s residue names it as
    /// still unread - so this engine stops after `Attack-` and spends the
    /// turn.
    pub class_effect_arm: bool,
}

impl CombatAttackAttempt {
    /// `combat.md §8.2` bare-handed attempt: "which behaves as melee with
    /// range one".
    pub const fn bare_handed() -> Self {
        Self {
            item_id: None,
            max_range: 1,
            melee_arm: true,
            runs_interference: false,
            class_effect_arm: false,
        }
    }

    pub fn for_item(item_id: usize) -> Self {
        let reach = equipment_weapon_range_cap(item_id).unwrap_or(0);
        Self {
            item_id: Some(item_id),
            max_range: if reach == 0 { 1 } else { reach },
            melee_arm: reach == 0,
            runs_interference: COMBAT_MISSILE_INTERFERENCE_ITEM_IDS.contains(&item_id),
            class_effect_arm: false,
        }
    }

    /// `combat.md §8.2`: "For a **monster-side** actor under player control
    /// there is no equipment to walk and the walker is skipped outright: `A`
    /// makes **exactly one attempt**, unconditionally and without a loop,
    /// carrying a fixed pseudo-item that sends the dispatcher to the
    /// monster-side reach and effect rows of that actor's class (Section 11)
    /// instead of to any item row."
    ///
    /// `§11`'s dispatcher row folds selector `1` "to zero, selecting the
    /// **melee / Aim-cursor arm**", which `§8.2` states from the other side -
    /// "a class reach of exactly 1 is normalised to zero, so it takes the
    /// fixed-range-one melee path rather than a one-cell ranged cursor". A
    /// selector above `1` takes the cast/effect arm.
    pub const fn for_monster_class(reach_selector: u8) -> Self {
        let melee_arm = reach_selector <= 1;
        Self {
            item_id: None,
            max_range: if melee_arm { 1 } else { reach_selector },
            melee_arm,
            runs_interference: false,
            class_effect_arm: !melee_arm,
        }
    }
}

/// `combat.md §8.2`: the ordered attempt list one `A` produces for a
/// party-side actor's readied equipment.
pub fn combat_attack_attempts(equipment: &[u8; EQUIPMENT_SLOT_COUNT]) -> Vec<CombatAttackAttempt> {
    let items = combat_armament_item_ids(equipment);
    if items.is_empty() {
        return vec![CombatAttackAttempt::bare_handed()];
    }
    items
        .into_iter()
        .map(CombatAttackAttempt::for_item)
        .collect()
}

/// `combat.md §8.2`: the item-name line is printed only "when two or three
/// items qualify"; "with exactly one qualifying item, or none, no item-name
/// line is printed".
pub const COMBAT_ATTACK_ITEM_NAME_LINE_MIN_ATTEMPTS: usize = 2;

/// `combat.md §8.2`: "When two or three items qualify, each attempt
/// additionally prints a newline, that item's name, and a colon **on its own
/// line before its `Attack-`**; with exactly one qualifying item, or none, no
/// item-name line is printed."
///
/// `§8.2` now settles the terminator directly: "**'On its own line' is
/// literal: the colon carries its own trailing newline**, so each attempt's
/// `Attack-` starts the row below its item-name line. This is a different
/// mechanism from the turn banner's line break - there the colon literal
/// carries no line feed and the turn handler supplies one (Section 8.1) -
/// but the visible result is the same on both lines, and the two colons are
/// two separate strings that cannot be served by one shared 'print a colon'
/// helper." `RETRACTIONS.md` R356 carries the consumer consequence.
pub fn combat_attack_item_line(attempts: &[CombatAttackAttempt], index: usize) -> Option<String> {
    if attempts.len() < COMBAT_ATTACK_ITEM_NAME_LINE_MIN_ATTEMPTS {
        return None;
    }
    let item_id = attempts.get(index)?.item_id?;
    Some(format!(
        "\n{}{COMBAT_ATTACK_ITEM_LINE_TERMINATOR}\n",
        equipment_name(item_id)
    ))
}

/// `combat.md §8.2` cursor key vocabulary. Every other byte is
/// "Discarded; the loop reads again."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatTargetingCursorInput {
    /// One of the eight direction codes. `§8.2` accepts the four internal
    /// cardinal codes "delivered by the arrow keys, or by a typed digit the
    /// shared reader has already remapped (Section 8.3)" and "the four
    /// corner keys - Home, End, PgUp, PgDn".
    Move(InputDirection),
    /// "Enter, or the letter `A` (either case)".
    Confirm,
    /// Space — cancels on the attacker's own cell, confirms anywhere else.
    Space,
    /// Escape — cancels.
    Escape,
    /// "Anything else | Discarded; the loop reads again."
    Ignored,
}

/// `combat.md §8.2`/`§8.3`: classify one keystroke inside the cursor loop.
///
/// The typed characters `1`-`9` are deliberately **not** read as directions
/// here. `§8.2` says the internal direction codes are "never by the
/// characters `1`-`4` reaching the loop unremapped", and `§8.3` puts the
/// remap in the shared input routine ahead of this loop: with the numpad
/// flag clear typed digits "are inert inside the targeting cursor", and with
/// it set they arrive already translated into direction codes. `0` and `5`
/// "are unconditionally inert inside the cursor".
pub fn combat_targeting_cursor_input(key: char) -> CombatTargetingCursorInput {
    let scalar = key as u32;
    if scalar <= u8::MAX as u32 {
        let byte = scalar as u8;
        if let Some(direction) = input_code_direction(byte) {
            return CombatTargetingCursorInput::Move(direction);
        }
        match byte {
            0x0D => return CombatTargetingCursorInput::Confirm,
            0x1B => return CombatTargetingCursorInput::Escape,
            b' ' => return CombatTargetingCursorInput::Space,
            b'A' | b'a' => return CombatTargetingCursorInput::Confirm,
            _ => {}
        }
    }
    CombatTargetingCursorInput::Ignored
}

/// `combat.md §8.2` one cursor step. "A move is applied only if the
/// destination stays inside the eleven-by-eleven arena **and** its distance
/// from the attacker does not exceed the maximum range. If either test fails
/// the cursor simply does not move: no message, no beep, no turn consumed,
/// and the loop reads another key."
///
/// "Because the range test is the truncated Euclidean distance, all eight
/// neighbours are within range one, so **a melee attack can target
/// diagonals**."
pub fn combat_targeting_cursor_step(
    cursor: (u8, u8),
    attacker: (u8, u8),
    max_range: u8,
    direction: InputDirection,
) -> Option<(u8, u8)> {
    let (dx, dy) = combat_targeting_direction_delta(direction);
    let x = i32::from(cursor.0) + dx;
    let y = i32::from(cursor.1) + dy;
    if x < 0 || y < 0 || x >= COMBAT_ARENA_SIDE as i32 || y >= COMBAT_ARENA_SIDE as i32 {
        return None;
    }
    let (x, y) = (x as u8, y as u8);
    (combat_arena_range(attacker.0, attacker.1, x, y) <= max_range).then_some((x, y))
}

/// `combat.md §8.2` cursor step vector. "Move the cursor one cell west,
/// east, north, south" for the cardinals, and "one cell diagonally:
/// Home/`7` north-west, End/`1` south-west, PgUp/`9` north-east, PgDn/`3`
/// south-east" for the corners.
pub const fn combat_targeting_direction_delta(direction: InputDirection) -> (i32, i32) {
    match direction {
        InputDirection::West => (-1, 0),
        InputDirection::East => (1, 0),
        InputDirection::North => (0, -1),
        InputDirection::South => (0, 1),
        InputDirection::Northwest => (-1, -1),
        InputDirection::Northeast => (1, -1),
        InputDirection::Southwest => (-1, 1),
        InputDirection::Southeast => (1, 1),
    }
}

/// `combat.md §8.2` cursor start cell: "It starts on the attacker's
/// remembered previous target when that target is still a valid, live,
/// visible actor within the maximum range, and on the attacker's own cell
/// otherwise."
///
/// `§8.2` states the seed's five-part validity gate outright: "the remembered
/// value must name a real slot, that slot must be neither **dead-marked nor
/// blink-hidden**, it must not be an empty slot, its linked presentation
/// record must be displayed, and its distance from the attacker must not
/// exceed this attempt's maximum range."
///
/// The blink-hidden term is bit `0x10`, which `§6.1` now publishes as the
/// invisibility bit (`RETRACTIONS.md` R380); the dragged-under bit `0x04` is
/// **not** on this list - it belongs to the occupancy lookup below. The
/// asleep/magically-disabled bit is on neither, and `§7.1` keeps a non-acting
/// actor fully targetable, so no status term enters this test.
///
/// `presentation_displayed` carries the fourth term - the linked
/// presentation record's displayed state - which lives outside the descriptor.
pub fn combat_targeting_cursor_start(
    attacker: (u8, u8),
    remembered: Option<CombatActorDescriptor>,
    presentation_displayed: bool,
    max_range: u8,
) -> (u8, u8) {
    if let Some(target) = remembered
        && combat_actor_is_present_not_dead(target)
        && !target.is_phase_suppressed()
        && presentation_displayed
        && usize::from(target.x) < COMBAT_ARENA_SIDE
        && usize::from(target.y) < COMBAT_ARENA_SIDE
        && combat_arena_range(attacker.0, attacker.1, target.x, target.y) <= max_range
    {
        return (target.x, target.y);
    }
    attacker
}

/// `combat.md §8.2` interference gate. The attempt "is aborted if **all** of
/// the following hold":
///
/// - "that recorded actor exists, and its slot is not empty";
/// - "it is on the automatic-driver side - a monster, or a party member
///   acting under Sword-of-Chaos / charm control. **An adjacent ordinary
///   party member never interferes;**"
/// - "it is neither invisible nor asleep";
/// - "the Negate Time effect is not currently active";
/// - "its distance from the attacker is exactly one. Distance uses the same
///   truncated Euclidean metric as the cursor, so 'exactly one' means any of
///   the eight surrounding cells, diagonals included."
pub fn combat_attack_interference_aborts(
    attacker: CombatActorDescriptor,
    source: Option<CombatActorDescriptor>,
    source_on_automatic_driver_side: bool,
    negate_time_active: bool,
) -> bool {
    let Some(source) = source else {
        return false;
    };
    if source.is_empty() || !source_on_automatic_driver_side || negate_time_active {
        return false;
    }
    // "it is neither invisible nor asleep" - and `§6.1`/`RETRACTIONS.md` R380
    // put invisibility on the phase/blink bit `0x10`, not on `0x04`.
    if source.is_phase_suppressed() || source.is_status_disabled() {
        return false;
    }
    attacker.range_to(source) == 1
}

/// `combat.md §8.2`: what one cursor keystroke resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatTargetingCursorAction {
    /// The cursor moved to this cell.
    Moved((u8, u8)),
    /// "If either test fails the cursor simply does not move: no message, no
    /// beep, no turn consumed, and the loop reads another key." Also the
    /// `Ignored` key arm, and Enter/`A` on the attacker's own cell:
    /// "nothing happens and the loop reads another key".
    Held,
    /// Confirm at the cursor cell.
    Confirmed((u8, u8)),
    /// Escape, or Space on the attacker's own cell.
    Cancelled,
}

/// `combat.md §8.2` cursor loop body for one keystroke.
pub fn resolve_combat_targeting_cursor_key(
    input: CombatTargetingCursorInput,
    cursor: (u8, u8),
    attacker: (u8, u8),
    max_range: u8,
) -> CombatTargetingCursorAction {
    match input {
        CombatTargetingCursorInput::Move(direction) => {
            match combat_targeting_cursor_step(cursor, attacker, max_range, direction) {
                Some(cell) => CombatTargetingCursorAction::Moved(cell),
                None => CombatTargetingCursorAction::Held,
            }
        }
        // "Confirm at the cursor cell - **unless** the cursor is on the
        // attacker's own cell, in which case nothing happens and the loop
        // reads another key."
        CombatTargetingCursorInput::Confirm => {
            if cursor == attacker {
                CombatTargetingCursorAction::Held
            } else {
                CombatTargetingCursorAction::Confirmed(cursor)
            }
        }
        // "Cancels if the cursor is on the attacker's own cell; anywhere
        // else it confirms exactly like Enter."
        CombatTargetingCursorInput::Space => {
            if cursor == attacker {
                CombatTargetingCursorAction::Cancelled
            } else {
                CombatTargetingCursorAction::Confirmed(cursor)
            }
        }
        CombatTargetingCursorInput::Escape => CombatTargetingCursorAction::Cancelled,
        CombatTargetingCursorInput::Ignored => CombatTargetingCursorAction::Held,
    }
}

/// The live `A`-Attack walk for one keyboard-driven combatant.
///
/// `combat.md §8.2`: the cursor "is entered once per readied weapon, not
/// once per Attack command", so the whole attempt list plus the index of the
/// attempt whose cursor is currently open is the session state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatTargetingCursorSession {
    pub actor_slot: usize,
    pub attempts: Vec<CombatAttackAttempt>,
    /// Index of the attempt whose cursor is open.
    pub index: usize,
    pub attacker: (u8, u8),
    pub cursor: (u8, u8),
    pub max_range: u8,
    pub melee_arm: bool,
    /// Whether any non-party actor was still active when this `A` walk
    /// began.
    ///
    /// `combat.md §7`: "If party actors remain and foes do not, it prints
    /// `VICTORY!` once and continues" (`RETRACTIONS.md` R289). One `A`
    /// produces one attempt per readied item, so a kill on a non-final
    /// attempt is followed by another cursor and another keystroke; asking
    /// "were there foes?" again at that later keystroke answers `false` and
    /// the announcement is lost. The walk therefore carries the answer it
    /// had when the turn's Attack was accepted.
    pub foes_present_at_walk_start: bool,
}

impl CombatTargetingCursorSession {
    pub fn attempt(&self) -> Option<CombatAttackAttempt> {
        self.attempts.get(self.index).copied()
    }
}

/// `combat.md §8.2` result of advancing the `A`-Attack walk — either from
/// the accepted `A` itself or from one keystroke fed to a live cursor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CombatAttackWalkApplication {
    /// Transcript this advance produced: the per-item name lines, `Attack-`,
    /// `Aim! `, any interference abort line, and `Nothing!`.
    pub text: String,
    /// Whether a targeting cursor is now open and owns the next keystroke.
    /// When this is `false` the whole `A` walk is finished and the caller
    /// spends the acting combatant's turn — `§8.2`: "The acting character's
    /// turn is consumed either way".
    pub cursor_open: bool,
    /// A confirmed attempt's resolved attack, for the caller's narration.
    pub attack: Option<(usize, crate::combat_frame::CombatWeaponAttackApplication)>,
    /// The same, for a **monster-side** actor acting at the player's prompt.
    /// `combat.md §8.2`'s fixed pseudo-item "sends the dispatcher to the
    /// monster-side reach and effect rows of that actor's class", which is the
    /// shared monster attack primitive, not the party weapon cascade.
    pub monster_attack: Option<(usize, crate::combat_frame::CombatMonsterAttackApplication)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        EQUIP_SLOT_HELM, EQUIP_SLOT_OFFHAND, EQUIP_SLOT_WEAPON, EQUIPMENT_EMPTY,
    };

    fn empty_equipment() -> [u8; EQUIPMENT_SLOT_COUNT] {
        [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]
    }

    /// `combat.md §8.2`: "A character with no qualifying item makes a single
    /// bare-handed attempt, which behaves as melee with range one."
    #[test]
    fn no_qualifying_item_makes_one_bare_handed_melee_attempt() {
        let attempts = combat_attack_attempts(&empty_equipment());
        assert_eq!(attempts, vec![CombatAttackAttempt::bare_handed()]);
        assert_eq!(attempts[0].max_range, 1);
        assert!(attempts[0].melee_arm);
        assert!(!attempts[0].runs_interference);
    }

    /// `combat.md §8.2`: "The engine walks the acting character's three
    /// readied equipment slots in order - helm, weapon hand, shield hand."
    /// "A non-zero reach does **not** mean 'missile weapon': the morning
    /// star and the halberd are in-hand melee weapons with reach 2".
    #[test]
    fn each_qualifying_slot_produces_one_attempt_in_published_order() {
        let mut equipment = empty_equipment();
        equipment[EQUIP_SLOT_HELM] = 3; // Spiked Helm
        equipment[EQUIP_SLOT_WEAPON] = 25; // Morning Star, reach 2
        equipment[EQUIP_SLOT_OFFHAND] = 6; // Spiked Shield

        let attempts = combat_attack_attempts(&equipment);

        assert_eq!(
            attempts
                .iter()
                .map(|attempt| attempt.item_id)
                .collect::<Vec<_>>(),
            vec![Some(3), Some(25), Some(6)]
        );
        assert_eq!(attempts[1].max_range, 2);
        assert!(!attempts[1].melee_arm);
        assert!(attempts[0].melee_arm);
    }

    /// `combat.md §8.2`: "Five items - **sling, flaming oil, bow, crossbow
    /// and magic bow** - run an interference test before the cursor. ... The
    /// other reach-bearing items - dagger, spear, throwing axe, morning
    /// star, halberd, magic axe - do **not** run this test".
    #[test]
    fn only_the_five_missile_items_run_the_interference_test() {
        for item in COMBAT_MISSILE_INTERFERENCE_ITEM_IDS {
            assert!(
                CombatAttackAttempt::for_item(item).runs_interference,
                "item {item} must run the interference abort"
            );
        }
        for item in [16usize, 21, 22, 25, 34, 38] {
            assert!(
                !CombatAttackAttempt::for_item(item).runs_interference,
                "item {item} must not run the interference abort"
            );
        }
    }

    /// `catalogs/item-list.md §5.3` names the five missile items and gives
    /// their ids in the range-cap table.
    #[test]
    fn the_five_missile_item_ids_are_the_published_rows() {
        assert_eq!(
            COMBAT_MISSILE_INTERFERENCE_ITEM_IDS
                .map(|item| equipment_name(item))
                .to_vec(),
            vec!["Sling", "Flaming Oil", "Bow", "Crossbow", "Magic Bow"]
        );
    }

    /// `combat.md §8.2`: "When two or three items qualify, each attempt
    /// additionally prints a newline, that item's name, and a colon **on its
    /// own line before its `Attack-`**; with exactly one qualifying item, or
    /// none, no item-name line is printed." `§8.1` calls the same emission
    /// "a per-item name **line**", and `§11.1`'s announcement table
    /// republishes the sibling turn banner as ending "a colon, **newline**",
    /// so the line is terminated and `Attack-` starts the next one.
    #[test]
    fn item_name_lines_appear_only_when_two_or_three_items_qualify() {
        let mut equipment = empty_equipment();
        equipment[EQUIP_SLOT_WEAPON] = 26; // Bow
        let single = combat_attack_attempts(&equipment);
        assert_eq!(combat_attack_item_line(&single, 0), None);

        equipment[EQUIP_SLOT_HELM] = 3; // Spiked Helm
        let pair = combat_attack_attempts(&equipment);
        assert_eq!(
            combat_attack_item_line(&pair, 0).as_deref(),
            Some("\nSpiked Helm:\n")
        );
        assert_eq!(
            combat_attack_item_line(&pair, 1).as_deref(),
            Some("\nBow:\n")
        );
    }

    /// `combat.md §8.2`: "all eight neighbours are within range one, so **a
    /// melee attack can target diagonals**", and a move outside the arena or
    /// beyond the range cap leaves the cursor where it is.
    #[test]
    fn cursor_moves_only_inside_the_arena_and_within_range() {
        assert_eq!(
            combat_targeting_cursor_step((5, 5), (5, 5), 1, InputDirection::Northwest),
            Some((4, 4))
        );
        assert_eq!(
            combat_targeting_cursor_step((4, 4), (5, 5), 1, InputDirection::West),
            None,
            "range two is outside a range-one cursor"
        );
        assert_eq!(
            combat_targeting_cursor_step((0, 0), (0, 0), 4, InputDirection::West),
            None,
            "the arena edge stops the cursor"
        );
    }

    /// `combat.md §8.2`: Enter/`A` on the attacker's own cell does nothing,
    /// Space there cancels, Escape cancels, and anything else is discarded.
    #[test]
    fn cursor_key_table_matches_the_published_rows() {
        let attacker = (5, 5);
        assert_eq!(
            resolve_combat_targeting_cursor_key(
                CombatTargetingCursorInput::Confirm,
                attacker,
                attacker,
                1
            ),
            CombatTargetingCursorAction::Held
        );
        assert_eq!(
            resolve_combat_targeting_cursor_key(
                CombatTargetingCursorInput::Space,
                attacker,
                attacker,
                1
            ),
            CombatTargetingCursorAction::Cancelled
        );
        assert_eq!(
            resolve_combat_targeting_cursor_key(
                CombatTargetingCursorInput::Space,
                (5, 4),
                attacker,
                1
            ),
            CombatTargetingCursorAction::Confirmed((5, 4))
        );
        assert_eq!(
            resolve_combat_targeting_cursor_key(
                CombatTargetingCursorInput::Escape,
                (5, 4),
                attacker,
                1
            ),
            CombatTargetingCursorAction::Cancelled
        );
        assert_eq!(
            resolve_combat_targeting_cursor_key(
                CombatTargetingCursorInput::Ignored,
                (5, 4),
                attacker,
                1
            ),
            CombatTargetingCursorAction::Held
        );
    }

    /// `combat.md §8.2`/`§8.3`: the internal direction codes reach the loop
    /// from the arrow and corner keys; the typed characters `1`-`9` never do
    /// unremapped, and `0`/`5` are unconditionally inert.
    #[test]
    fn typed_digits_are_inert_inside_the_cursor() {
        for digit in '0'..='9' {
            assert_eq!(
                combat_targeting_cursor_input(digit),
                CombatTargetingCursorInput::Ignored,
                "typed `{digit}` must not steer the cursor"
            );
        }
        assert_eq!(
            combat_targeting_cursor_input(char::from(crate::INPUT_CODE_WEST)),
            CombatTargetingCursorInput::Move(InputDirection::West)
        );
        assert_eq!(
            combat_targeting_cursor_input(char::from(crate::INPUT_CODE_SOUTHWEST)),
            CombatTargetingCursorInput::Move(InputDirection::Southwest)
        );
        assert_eq!(
            combat_targeting_cursor_input('\r'),
            CombatTargetingCursorInput::Confirm
        );
        assert_eq!(
            combat_targeting_cursor_input('a'),
            CombatTargetingCursorInput::Confirm
        );
        assert_eq!(
            combat_targeting_cursor_input('A'),
            CombatTargetingCursorInput::Confirm
        );
        assert_eq!(
            combat_targeting_cursor_input('Q'),
            CombatTargetingCursorInput::Ignored
        );
    }
}
