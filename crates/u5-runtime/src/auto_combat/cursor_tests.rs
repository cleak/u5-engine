//! The targeting-cursor planner (design 6.3.3, 6.2.4, 6.6.1 (a), (c), (h),
//! (i); build plan M7-03).

use super::planner_tests::{add_monster, add_party_member, arena, plan, whole_party};
use super::*;
use crate::combat_actor::COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER;
use crate::constants::{
    EQUIP_SLOT_HELM, EQUIP_SLOT_OFFHAND, EQUIP_SLOT_WEAPON, EQUIPMENT_EMPTY, EQUIPMENT_SLOT_COUNT,
};
use crate::enhancements::digest::trajectory_digest;
use crate::handle_play_key_input;

const SPIKED_HELM: u8 = 3;
const SPIKED_SHIELD: u8 = 6;
const DAGGER: u8 = 16;
const BOW: u8 = 26;

fn ready(state: &mut PlayState, items: &[(usize, u8)]) {
    state.party_equipment[0] = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
    for (slot, item) in items {
        state.party_equipment[0][*slot] = *item;
    }
}

/// The runtime as the observer leaves it right after an auto `A` opened a
/// cursor for member 0.
fn owned() -> AutoFightRuntime {
    AutoFightRuntime {
        turn_slot: Some(0),
        owns_open_attempt: true,
        ..AutoFightRuntime::default()
    }
}

/// Opens member 0's `A` walk directly, as an accepted `A` does.
fn open_walk(state: &mut PlayState) {
    state.pending_combat_actor_slot = Some(0);
    assert!(state.begin_combat_attack_walk(0, true).cursor_open);
}

fn session(state: &PlayState) -> &crate::combat_targeting::CombatTargetingCursorSession {
    state.active_combat_targeting.as_ref().expect("cursor open")
}

/// Plans and applies cursor keys inside the open attempt until the planner
/// answers Confirm or Cancel (not applied), or `limit` moves. Every move
/// must be accepted by the engine and stay inside the range clamp.
fn walk_cursor(state: &mut PlayState, limit: usize) -> (Vec<AutoKey>, AutoKey) {
    let mut moves = Vec::new();
    for _ in 0..limit {
        let AutoPlan::Key(key) = plan(state, &owned()) else {
            panic!("an owned cursor always plans a key");
        };
        assert_cursor_key(state, key);
        if !matches!(key, AutoKey::Cursor(_)) {
            return (moves, key);
        }
        let before = session(state).cursor;
        let walk = state
            .apply_combat_targeting_cursor_key(key.input_char())
            .expect("cursor open");
        assert!(walk.cursor_open);
        let open = session(state);
        assert_ne!(open.cursor, before, "{key:?} was refused by the engine");
        assert!(
            combat_arena_range(
                open.attacker.0,
                open.attacker.1,
                open.cursor.0,
                open.cursor.1
            ) <= open.max_range
        );
        moves.push(key);
    }
    panic!("no Confirm or Cancel within {limit} moves");
}

/// (h) inside the cursor: moves, Confirm or Cancel; never Space, and never
/// Confirm on the attacker's own cell or on anything but an occupant.
fn assert_cursor_key(state: &PlayState, key: AutoKey) {
    assert!(
        matches!(key, AutoKey::Cursor(_) | AutoKey::Confirm | AutoKey::Cancel),
        "{key:?} in the cursor"
    );
    assert_ne!(key.input_char(), ' ', "Space in the cursor");
    if key == AutoKey::Confirm {
        let open = session(state);
        assert_ne!(open.cursor, open.attacker, "Enter on the attacker's cell");
        let occupant = state.combat_targeting_occupant_at(open.cursor);
        assert!(occupant.is_some_and(|slot| !state.combat_actors[slot].is_party_side()));
    }
}

fn chebyshev(a: (u8, u8), b: (u8, u8)) -> usize {
    usize::from(a.0.abs_diff(b.0).max(a.1.abs_diff(b.1)))
}

// (a) cursor paths, edge, clamp, re-target, Escape.

#[test]
fn a_bow_cursor_walks_a_shortest_path_and_confirms_on_the_target() {
    for target in [(6, 5), (2, 1), (5, 8), (0, 9), (6, 2)] {
        let mut state = arena();
        ready(&mut state, &[(EQUIP_SLOT_WEAPON, BOW)]);
        add_monster(&mut state, 8, target.0, target.1, 0);
        open_walk(&mut state);
        assert_eq!(session(&state).cursor, (2, 5), "no remembered target");
        let (moves, last) = walk_cursor(&mut state, 20);
        assert_eq!(last, AutoKey::Confirm, "{target:?}");
        assert_eq!(session(&state).cursor, target);
        assert_eq!(moves.len(), chebyshev((2, 5), target), "{target:?}");
        assert!(
            moves
                .iter()
                .all(|key| matches!(key, AutoKey::Cursor(_)) && key.is_well_formed())
        );
    }
}

#[test]
fn a_melee_cursor_uses_a_diagonal_and_confirms() {
    let mut state = arena();
    add_monster(&mut state, 8, 3, 6, 0);
    open_walk(&mut state);
    assert_eq!(session(&state).max_range, 1);
    let (moves, last) = walk_cursor(&mut state, 4);
    assert_eq!(moves, [AutoKey::Cursor(InputDirection::Southeast)]);
    assert_eq!(last, AutoKey::Confirm);
}

#[test]
fn the_path_stays_on_the_arena_at_its_edge() {
    let mut state = arena();
    ready(&mut state, &[(EQUIP_SLOT_WEAPON, BOW)]);
    add_party_member(&mut state, 0, 0, 0, 0);
    add_monster(&mut state, 8, 0, 6, 0);
    open_walk(&mut state);
    let (moves, last) = walk_cursor(&mut state, 20);
    assert_eq!(last, AutoKey::Confirm);
    assert_eq!(moves, [AutoKey::Cursor(InputDirection::South); 6]);

    let mut state = arena();
    ready(&mut state, &[(EQUIP_SLOT_WEAPON, BOW)]);
    let side = COMBAT_ARENA_SIDE as u8;
    add_party_member(&mut state, 0, side - 1, side - 1, 0);
    add_monster(&mut state, 8, side - 6, side - 1, 0);
    open_walk(&mut state);
    let (moves, last) = walk_cursor(&mut state, 20);
    assert_eq!(last, AutoKey::Confirm);
    assert_eq!(moves.len(), 5);
}

#[test]
fn the_range_clamp_picks_only_a_reachable_target() {
    // The Spiked Helm's range-one arm: a skeleton two cells away is out of
    // reach, so the attempt is given up rather than aimed at it.
    let mut state = arena();
    ready(&mut state, &[(EQUIP_SLOT_HELM, SPIKED_HELM)]);
    add_monster(&mut state, 8, 4, 5, 0);
    open_walk(&mut state);
    assert_eq!(session(&state).max_range, 1);
    assert_eq!(plan(&state, &owned()), AutoPlan::Key(AutoKey::Cancel));
    // A second skeleton in reach is the pick; the first is never chosen.
    add_monster(&mut state, 9, 2, 4, 0);
    let (moves, last) = walk_cursor(&mut state, 4);
    assert_eq!(moves, [AutoKey::Cursor(InputDirection::North)]);
    assert_eq!(last, AutoKey::Confirm);
}

#[test]
fn escape_when_nothing_is_in_reach_spends_the_attempt() {
    let mut state = arena();
    add_monster(&mut state, 8, 5, 5, 0);
    open_walk(&mut state);
    assert_eq!(plan(&state, &owned()), AutoPlan::Key(AutoKey::Cancel));
    let walk = state
        .apply_combat_targeting_cursor_key(AutoKey::Cancel.input_char())
        .unwrap();
    assert!(!walk.cursor_open);
    assert!(state.active_combat_targeting.is_none());

    // An invisible foe in reach is not in reach for auto either.
    let mut state = arena();
    add_monster(&mut state, 8, 3, 5, COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER);
    add_monster(&mut state, 9, 8, 8, 0);
    open_walk(&mut state);
    assert_eq!(plan(&state, &owned()), AutoPlan::Key(AutoKey::Cancel));
}

#[test]
fn each_attempt_of_a_three_item_attack_re_picks_after_a_kill() {
    let mut state = arena();
    ready(
        &mut state,
        &[
            (EQUIP_SLOT_HELM, SPIKED_HELM),
            (EQUIP_SLOT_WEAPON, DAGGER),
            (EQUIP_SLOT_OFFHAND, SPIKED_SHIELD),
        ],
    );
    add_monster(&mut state, 8, 3, 5, 0);
    add_monster(&mut state, 9, 2, 4, 0);
    open_walk(&mut state);
    assert_eq!(session(&state).attempts.len(), 3);

    let mut struck = Vec::new();
    for attempt in 0..3 {
        assert_eq!(session(&state).index, attempt);
        let (_, last) = walk_cursor(&mut state, 8);
        if attempt == 2 {
            // Both are gone: nothing in reach.
            assert_eq!(last, AutoKey::Cancel);
            break;
        }
        assert_eq!(last, AutoKey::Confirm);
        let target = state
            .combat_targeting_occupant_at(session(&state).cursor)
            .unwrap();
        struck.push(target);
        state
            .apply_combat_targeting_cursor_key(AutoKey::Confirm.input_char())
            .unwrap();
        // The fixture's kill: whatever the draws did, the target is gone
        // before the next attempt is planned.
        state.combat_actors[target] = CombatActorDescriptor::empty();
        state.active_objects[target] = crate::ActiveObject::empty();
        assert!(state.active_combat_targeting.is_some());
    }
    struck.sort_unstable();
    assert_eq!(struck, [8, 9], "the second attempt re-targets");
}

// Ownership, deferred stopping and unexpected states.

#[test]
fn a_cursor_the_player_opened_is_not_ours() {
    let mut state = arena();
    add_monster(&mut state, 8, 3, 5, 0);
    open_walk(&mut state);
    assert_eq!(auto_prompt(&state), AutoPrompt::Cursor { slot: 0 });
    assert_eq!(
        plan(&state, &AutoFightRuntime::default()),
        AutoPlan::NotOurs
    );
}

#[test]
fn stopping_waits_for_the_next_command_prompt() {
    let mut state = arena();
    add_monster(&mut state, 8, 3, 6, 0);
    let stopping = AutoFightRuntime {
        stopping: Some(HandBack::Interrupted),
        ..owned()
    };
    assert_eq!(
        plan(&state, &stopping),
        AutoPlan::HandBack(HandBack::Interrupted)
    );
    open_walk(&mut state);
    assert_eq!(
        plan(&state, &stopping),
        AutoPlan::Key(AutoKey::Cursor(InputDirection::Southeast))
    );
}

#[test]
fn an_unexpected_cursor_state_hands_back() {
    let mut state = arena();
    add_monster(&mut state, 8, 3, 6, 0);
    open_walk(&mut state);
    let other_turn = AutoFightRuntime {
        turn_slot: Some(1),
        ..owned()
    };
    assert_eq!(
        plan(&state, &other_turn),
        AutoPlan::HandBack(HandBack::UnexpectedPrompt)
    );
    let mut broken = state.clone();
    broken.active_combat_targeting.as_mut().unwrap().cursor = (7, 7);
    assert_eq!(
        plan(&broken, &owned()),
        AutoPlan::HandBack(HandBack::UnexpectedPrompt)
    );
    let mut broken = state.clone();
    broken.active_combat_targeting.as_mut().unwrap().index = 9;
    assert_eq!(
        plan(&broken, &owned()),
        AutoPlan::HandBack(HandBack::UnexpectedPrompt)
    );
    // Cursor keys that keep changing nothing give the attempt up.
    let stuck = AutoFightRuntime {
        no_progress: NO_PROGRESS_BUDGET,
        ..owned()
    };
    assert_eq!(plan(&state, &stuck), AutoPlan::Key(AutoKey::Cancel));
}

// (c) same-as-human.

/// One shell step: the idle pass both shells run before reading a key.
fn settle(state: &mut PlayState) {
    let _ = state.ensure_pending_combat_player_turn();
}

fn human_fixture() -> PlayState {
    let mut state = arena();
    ready(&mut state, &[(EQUIP_SLOT_WEAPON, BOW)]);
    add_monster(&mut state, 8, 6, 8, 0);
    add_monster(&mut state, 9, 9, 2, 0);
    state
}

#[test]
fn auto_keys_replayed_by_hand_give_the_same_run() {
    let game_dir = std::path::Path::new(".");
    let mut auto = human_fixture();
    let mut human = auto.clone();
    let settings = CombatSettings::default();
    let mut fight = AutoFightRuntime::default();
    let mut keys = Vec::new();
    for _ in 0..80 {
        settle(&mut auto);
        let AutoPlan::Key(key) = plan_next_auto_key(&auto, &settings, &whole_party(), &fight)
        else {
            break;
        };
        record_print_before_auto_key(&auto, &mut fight);
        handle_play_key_input(&mut auto, key.input_char(), "", game_dir).unwrap();
        observe_after_auto_key(&auto, key, &mut fight);
        keys.push(key);
    }
    assert!(keys.contains(&AutoKey::Attack));
    assert!(keys.contains(&AutoKey::Confirm));
    assert!(keys.iter().any(|key| matches!(key, AutoKey::Cursor(_))));

    for key in &keys {
        settle(&mut human);
        handle_play_key_input(&mut human, key.input_char(), "", game_dir).unwrap();
    }
    assert_eq!(auto.message_entries(), human.message_entries());
    assert_eq!(auto.prng_state, human.prng_state);
    assert_eq!(trajectory_digest(&auto), trajectory_digest(&human));
}

// (h) never Space in the cursor, over every target cell and arm.

#[test]
fn cursor_keys_are_only_moves_confirm_and_cancel() {
    let side = COMBAT_ARENA_SIDE as u8;
    let mut confirms = 0;
    for items in [&[][..], &[(EQUIP_SLOT_WEAPON, BOW)][..]] {
        for (tx, ty) in (0..side).flat_map(|x| (0..side).map(move |y| (x, y))) {
            if (tx, ty) == (2, 5) {
                continue;
            }
            let mut state = arena();
            ready(&mut state, items);
            add_monster(&mut state, 8, tx, ty, 0);
            open_walk(&mut state);
            let (_, last) = walk_cursor(&mut state, 20);
            confirms += usize::from(last == AutoKey::Confirm);
        }
    }
    assert!(confirms > 8);
}

// (i) the runtime spoiler test.

/// The keys the planner gives from the command prompt through the whole
/// cursor, without letting the game resolve the attack (the first point
/// at which the game itself could react to an unseen monster).
fn planned_until_the_game_reacts(mut state: PlayState) -> Vec<AutoPlan> {
    let mut plans = vec![plan(&state, &AutoFightRuntime::default())];
    if plans[0] != AutoPlan::Key(AutoKey::Attack) {
        return plans;
    }
    open_walk(&mut state);
    loop {
        let planned = plan(&state, &owned());
        plans.push(planned);
        match planned {
            AutoPlan::Key(key @ AutoKey::Cursor(_)) => {
                state.apply_combat_targeting_cursor_key(key.input_char());
            }
            _ => return plans,
        }
    }
}

#[test]
fn an_invisible_hostile_anywhere_does_not_change_the_planned_keys() {
    let side = COMBAT_ARENA_SIDE as u8;
    let fixtures = [
        (&[(EQUIP_SLOT_WEAPON, BOW)][..], (6, 8)),
        (&[][..], (3, 6)),
        (&[][..], (7, 9)),
    ];
    for (items, target) in fixtures {
        let base = || {
            let mut state = arena();
            ready(&mut state, items);
            add_monster(&mut state, 15, target.0, target.1, 0);
            state
        };
        let expected = planned_until_the_game_reacts(base());
        assert!(expected.len() >= 2 || target == (7, 9));
        for (x, y) in (0..side).flat_map(|x| (0..side).map(move |y| (x, y))) {
            if (x, y) == (2, 5) || (x, y) == target {
                continue;
            }
            // In a lower slot than the visible foe too, so it is scanned
            // first.
            for slot in [9, 20] {
                let mut state = base();
                add_monster(&mut state, slot, x, y, COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER);
                assert_eq!(
                    planned_until_the_game_reacts(state),
                    expected,
                    "invisible slot {slot} at {:?}, target {target:?}",
                    (x, y)
                );
            }
        }
    }
    // The planned keys apply to the step cell as well: a step the unseen
    // monster blocks is planned all the same.
    let mut state = arena();
    add_monster(&mut state, 8, 7, 9, 0);
    let step = plan(&state, &AutoFightRuntime::default());
    assert_eq!(step, AutoPlan::Key(AutoKey::Step(InputDirection::East)));
    add_monster(&mut state, 12, 3, 5, COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER);
    assert_eq!(plan(&state, &AutoFightRuntime::default()), step);
}
