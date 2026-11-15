//! The command-prompt planner, the stop order and the observer (design
//! 6.3.3, 6.2.5, 6.6.1 (a), (b), (d)-(h); build plan M7-02).

use super::*;
use crate::combat_actor::{
    COMBAT_ACTOR_FLAG_CONTROLLED, COMBAT_ACTOR_FLAG_DRAGGED_UNDER,
    COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER, COMBAT_ACTOR_FLAG_SELECTABLE_40,
    COMBAT_ACTOR_FLAG_SELECTABLE_80, COMBAT_ACTOR_FLAG_STATUS_DISABLED,
};
use crate::combat_arena::COMBAT_ARENA_SIDE;
use crate::combat_scenario::{CombatScenarioInput, run_combat_scenario};
use crate::constants::{EQUIP_SLOT_WEAPON, EQUIPMENT_EMPTY, EQUIPMENT_SLOT_COUNT};
use crate::input_codes::input_direction_code;
use crate::{ActiveObject, test_fixtures};

const SKELETON_CLASS: u8 = 33;

/// Every roster position handed over.
fn whole_party() -> AutoEngagement {
    AutoEngagement {
        members: [true; 6],
        until_done: false,
    }
}

/// A fight under way on open ground: member 0 (roster 0) prompted at
/// (2, 5), no foes yet. Add them with [`add_monster`].
fn arena() -> PlayState {
    let mut state = test_fixtures::test_state(test_fixtures::open_grid(), 5, 5);
    state.combat_active = true;
    state.combat_round_loop_prologue_ran = true;
    state.time_stop_counter = u8::MAX;
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
    for member in state.party.iter_mut() {
        member.status = b'G';
        member.hp = 20;
        member.max_hp = 20;
    }
    add_party_member(&mut state, 0, 2, 5, 0);
    state.pending_combat_actor_slot = Some(0);
    state.prng_state = 0x3270;
    state
}

fn add_party_member(state: &mut PlayState, slot: usize, x: u8, y: u8, extra_flags: u8) {
    state.active_objects[slot] = ActiveObject {
        type_byte: 0x80,
        tile: 0x80,
        x: usize::from(x),
        y: usize::from(y),
        ..ActiveObject::empty()
    };
    state.combat_actors[slot] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80 | extra_flags,
        slot as u8,
        slot as u8,
        0,
        x,
        y,
    ]);
}

fn add_monster(state: &mut PlayState, slot: usize, x: u8, y: u8, extra_flags: u8) {
    state.active_objects[slot] = ActiveObject {
        type_byte: 0x90,
        tile: 0x90,
        x: usize::from(x),
        y: usize::from(y),
        ..ActiveObject::empty()
    };
    state.combat_actors[slot] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40 | extra_flags,
        SKELETON_CLASS,
        slot as u8,
        0,
        x,
        y,
    ]);
}

fn plan(state: &PlayState, fight: &AutoFightRuntime) -> AutoPlan {
    plan_next_auto_key(state, &CombatSettings::default(), &whole_party(), fight)
}

fn plan_fresh(state: &PlayState) -> AutoPlan {
    plan(state, &AutoFightRuntime::default())
}

fn scenario_input(key: AutoKey) -> CombatScenarioInput {
    match key {
        AutoKey::Attack => CombatScenarioInput::Attack,
        AutoKey::Pass => CombatScenarioInput::Pass,
        AutoKey::Step(direction) => CombatScenarioInput::Move(input_direction_code(direction)),
        AutoKey::Cursor(_) | AutoKey::Confirm | AutoKey::Cancel => {
            CombatScenarioInput::CursorKey(key.input_char())
        }
    }
}

/// The fixture helper over `run_combat_scenario`: plan, take the print,
/// apply the key as the scripted combat input, observe; repeat until the
/// planner stops answering or `limit` keys. Every planned key is
/// returned, with its prompt kind, for the (h) checks.
fn drive(
    state: &mut PlayState,
    settings: &CombatSettings,
    fight: &mut AutoFightRuntime,
    limit: usize,
) -> (Vec<(AutoPrompt, AutoKey)>, AutoPlan) {
    let mut keys = Vec::new();
    for _ in 0..limit {
        let prompt = auto_prompt(state);
        let planned = plan_next_auto_key(state, settings, &whole_party(), fight);
        let AutoPlan::Key(key) = planned else {
            return (keys, planned);
        };
        keys.push((prompt, key));
        record_print_before_auto_key(state, fight);
        run_combat_scenario(state, &[scenario_input(key)]);
        observe_after_auto_key(state, key, fight);
    }
    (keys, AutoPlan::NotOurs)
}

fn step_destination(from: (u8, u8), direction: InputDirection) -> (i16, i16) {
    let (dx, dy) = cardinal_delta(direction);
    (i16::from(from.0) + dx, i16::from(from.1) + dy)
}

/// (h) at the command prompt: only `A`, Space and cardinal steps, so never
/// a digit, a Ctrl chord or Escape.
fn assert_command_key(key: AutoKey) {
    assert!(
        matches!(key, AutoKey::Attack | AutoKey::Pass)
            || matches!(key, AutoKey::Step(direction) if direction.is_cardinal()),
        "{key:?} at the command prompt"
    );
    let byte = key.input_char();
    assert!(!byte.is_ascii_digit());
    assert_ne!(byte, '\u{1b}');
    assert_ne!(byte, '\u{13}', "Ctrl+S");
    if byte.is_ascii_control() {
        assert!(
            matches!(key, AutoKey::Step(_)),
            "only arrows are control bytes"
        );
    }
}

#[test]
fn the_fixture_is_an_owned_command_prompt() {
    let mut state = arena();
    add_monster(&mut state, 8, 8, 5, 0);
    assert_eq!(
        auto_prompt(&state),
        AutoPrompt::Command {
            slot: 0,
            roster: Some(0)
        }
    );
    state.combat_active = false;
    assert_eq!(auto_prompt(&state), AutoPrompt::None);
}

// (a) target choice, step legality, bounds, all-interfered.

#[test]
fn the_nearest_visible_hostile_is_the_target() {
    let mut state = arena();
    add_monster(&mut state, 8, 9, 5, 0);
    add_monster(&mut state, 9, 2, 8, 0);
    assert_eq!(auto_target(&state, 0), Some(9));
    // Adjacent: the bare-handed attempt reaches, so Attack.
    add_monster(&mut state, 10, 3, 6, 0);
    assert_eq!(auto_target(&state, 0), Some(10));
    assert_eq!(plan_fresh(&state), AutoPlan::Key(AutoKey::Attack));
}

#[test]
fn out_of_reach_steps_on_the_larger_axis_first() {
    let mut state = arena();
    add_monster(&mut state, 8, 6, 7, 0);
    assert_eq!(
        plan_fresh(&state),
        AutoPlan::Key(AutoKey::Step(InputDirection::East))
    );
    let mut state = arena();
    add_monster(&mut state, 8, 3, 1, 0);
    assert_eq!(
        plan_fresh(&state),
        AutoPlan::Key(AutoKey::Step(InputDirection::North))
    );
}

#[test]
fn a_held_cell_a_field_or_a_refused_direction_falls_back_to_the_other_axis() {
    let mut state = arena();
    add_monster(&mut state, 8, 6, 7, 0);
    // A visible party member holds the east cell.
    add_party_member(&mut state, 1, 3, 5, 0);
    assert_eq!(
        plan_fresh(&state),
        AutoPlan::Key(AutoKey::Step(InputDirection::South))
    );

    let mut state = arena();
    add_monster(&mut state, 8, 6, 7, 0);
    state.active_objects[20] = ActiveObject {
        type_byte: 0xEA,
        tile: 0xEA,
        x: 3,
        y: 5,
        ..ActiveObject::empty()
    };
    assert_eq!(
        plan_fresh(&state),
        AutoPlan::Key(AutoKey::Step(InputDirection::South))
    );

    let mut state = arena();
    add_monster(&mut state, 8, 6, 7, 0);
    let fight = AutoFightRuntime {
        turn_slot: Some(0),
        no_progress: 1,
        blocked_dirs: blocked_dir_bit(InputDirection::East),
        ..AutoFightRuntime::default()
    };
    assert_eq!(
        plan(&state, &fight),
        AutoPlan::Key(AutoKey::Step(InputDirection::South))
    );
    // Both axes refused: Pass.
    let fight = AutoFightRuntime {
        blocked_dirs: blocked_dir_bit(InputDirection::East)
            | blocked_dir_bit(InputDirection::South),
        ..fight
    };
    assert_eq!(plan(&state, &fight), AutoPlan::Key(AutoKey::Pass));
    // A refusal recorded for another member's turn does not carry over.
    let fight = AutoFightRuntime {
        turn_slot: Some(1),
        ..fight
    };
    assert_eq!(
        plan(&state, &fight),
        AutoPlan::Key(AutoKey::Step(InputDirection::East))
    );
}

#[test]
fn a_step_never_leaves_the_arena() {
    let side = COMBAT_ARENA_SIDE as u8;
    for (ax, ay) in (0..side).flat_map(|x| (0..side).map(move |y| (x, y))) {
        let edge = ax == 0 || ay == 0 || ax == side - 1 || ay == side - 1;
        if !edge {
            continue;
        }
        for (tx, ty) in (0..side).flat_map(|x| (0..side).map(move |y| (x, y))) {
            if (tx, ty) == (ax, ay) {
                continue;
            }
            let mut state = arena();
            add_party_member(&mut state, 0, ax, ay, 0);
            add_monster(&mut state, 8, tx, ty, 0);
            if let AutoPlan::Key(AutoKey::Step(direction)) = plan_fresh(&state) {
                let (x, y) = step_destination((ax, ay), direction);
                assert!(
                    (0..i16::from(side)).contains(&x) && (0..i16::from(side)).contains(&y),
                    "({ax},{ay}) toward ({tx},{ty}) stepped {direction:?}"
                );
            }
        }
    }
}

#[test]
fn every_readied_attempt_interfered_passes() {
    let mut state = arena();
    add_monster(&mut state, 8, 3, 5, 0);
    state.party_equipment[0] = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
    state.party_equipment[0][EQUIP_SLOT_WEAPON] = 26; // a bow, a missile item
    // Not yet struck: the bow reaches and fires.
    assert_eq!(plan_fresh(&state), AutoPlan::Key(AutoKey::Attack));
    // The adjacent skeleton last struck member 0: every attempt aborts.
    state.combat_interference_sources[0] = 8;
    assert!(
        state
            .combat_attack_interference_source_for_slot(0)
            .is_some()
    );
    assert_eq!(plan_fresh(&state), AutoPlan::Key(AutoKey::Pass));
}

// (b) inertness.

#[test]
fn planning_never_changes_state() {
    let mut fixtures = Vec::new();
    let mut state = arena();
    add_monster(&mut state, 8, 3, 5, 0);
    fixtures.push(state.clone());
    let mut state = arena();
    add_monster(&mut state, 8, 8, 8, 0);
    fixtures.push(state.clone());
    add_monster(&mut state, 9, 4, 5, COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER);
    fixtures.push(state.clone());
    let mut cursor = arena();
    add_monster(&mut cursor, 8, 3, 5, 0);
    run_combat_scenario(&mut cursor, &[CombatScenarioInput::Attack]);
    assert!(matches!(
        auto_prompt(&cursor),
        AutoPrompt::Cursor { slot: 0 }
    ));
    fixtures.push(cursor);
    let mut over = arena();
    over.combat_active = false;
    fixtures.push(over);

    for state in fixtures {
        let before = format!("{state:?}");
        let prng = state.prng_state;
        for fight in [
            AutoFightRuntime::default(),
            AutoFightRuntime {
                turn_slot: Some(0),
                no_progress: NO_PROGRESS_BUDGET,
                ..AutoFightRuntime::default()
            },
        ] {
            let _ = plan(&state, &fight);
            let _ = auto_prompt(&state);
            let _ = progress_print(&state);
        }
        assert_eq!(state.prng_state, prng);
        assert_eq!(format!("{state:?}"), before);
    }
}

// (d) livelock, cap, no target.

#[test]
fn three_no_progress_keys_pass_and_a_still_pass_is_stuck() {
    let mut state = arena();
    add_monster(&mut state, 8, 3, 5, 0);
    let mut fight = AutoFightRuntime::default();
    // Three keys that change nothing (observed on unchanged state).
    for _ in 0..NO_PROGRESS_BUDGET {
        assert_eq!(plan(&state, &fight), AutoPlan::Key(AutoKey::Attack));
        let key = AutoKey::Attack;
        record_print_before_auto_key(&state, &mut fight);
        observe_after_auto_key(&state, key, &mut fight);
    }
    assert_eq!(fight.no_progress, NO_PROGRESS_BUDGET);
    assert_eq!(fight.turn_slot, Some(0));
    assert_eq!(plan(&state, &fight), AutoPlan::Key(AutoKey::Pass));
    record_print_before_auto_key(&state, &mut fight);
    observe_after_auto_key(&state, AutoKey::Pass, &mut fight);
    assert_eq!(plan(&state, &fight), AutoPlan::HandBack(HandBack::Stuck));
    assert_eq!(fight.actions, u16::from(NO_PROGRESS_BUDGET) + 1);

    // Refused steps on both axes exhaust the step candidates sooner: Pass.
    let mut state = arena();
    add_monster(&mut state, 8, 8, 8, 0);
    let mut fight = AutoFightRuntime::default();
    for expected in [InputDirection::East, InputDirection::South] {
        assert_eq!(plan(&state, &fight), AutoPlan::Key(AutoKey::Step(expected)));
        record_print_before_auto_key(&state, &mut fight);
        observe_after_auto_key(&state, AutoKey::Step(expected), &mut fight);
    }
    assert_eq!(plan(&state, &fight), AutoPlan::Key(AutoKey::Pass));
}

#[test]
fn a_step_into_an_unseen_monster_is_learned_only_by_its_refusal() {
    let mut state = arena();
    add_monster(&mut state, 8, 8, 5, 0);
    // An invisible hostile on the east cell: the plan still steps east.
    add_monster(&mut state, 9, 3, 5, COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER);
    let mut fight = AutoFightRuntime::default();
    let (keys, _) = drive(&mut state, &CombatSettings::default(), &mut fight, 1);
    assert_eq!(keys[0].1, AutoKey::Step(InputDirection::East));
    let progressed = fight.no_progress == 0;
    if !progressed {
        // The game refused it: east is now skipped and the fallback passes.
        assert_ne!(
            fight.blocked_dirs & blocked_dir_bit(InputDirection::East),
            0
        );
        assert_eq!(plan(&state, &fight), AutoPlan::Key(AutoKey::Pass));
    }
}

#[test]
fn a_progressing_key_resets_the_turn_counters() {
    let mut state = arena();
    add_monster(&mut state, 8, 8, 5, 0);
    let mut fight = AutoFightRuntime {
        turn_slot: Some(0),
        no_progress: 2,
        blocked_dirs: blocked_dir_bit(InputDirection::North),
        ..AutoFightRuntime::default()
    };
    record_print_before_auto_key(&state, &mut fight);
    run_combat_scenario(&mut state, &[CombatScenarioInput::Pass]);
    observe_after_auto_key(&state, AutoKey::Pass, &mut fight);
    assert_eq!((fight.no_progress, fight.blocked_dirs), (0, 0));
    assert_eq!(fight.actions, 1);
}

#[test]
fn the_action_cap_hands_back() {
    let mut state = arena();
    add_monster(&mut state, 8, 8, 5, 0);
    let fight = AutoFightRuntime {
        actions: ACTION_CAP_DEFAULT,
        ..AutoFightRuntime::default()
    };
    assert_eq!(
        plan(&state, &fight),
        AutoPlan::HandBack(HandBack::ActionCap)
    );

    // A driven fight stops within the cap.
    let settings = CombatSettings {
        action_cap: ACTION_CAP_MIN,
        ..CombatSettings::default()
    };
    let mut fight = AutoFightRuntime::default();
    let (keys, last) = drive(&mut state, &settings, &mut fight, 500);
    assert!(keys.len() <= usize::from(ACTION_CAP_MIN));
    assert!(
        matches!(last, AutoPlan::HandBack(_) | AutoPlan::NotOurs),
        "{last:?}"
    );
    for (prompt, key) in keys {
        if matches!(prompt, AutoPrompt::Command { .. }) {
            assert_command_key(key);
        }
    }
}

#[test]
fn no_visible_hostile_hands_back_and_no_foe_is_victory() {
    let mut state = arena();
    add_monster(&mut state, 8, 8, 5, COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER);
    assert_eq!(plan_fresh(&state), AutoPlan::HandBack(HandBack::NoTarget));
    let state = arena();
    assert_eq!(plan_fresh(&state), AutoPlan::HandBack(HandBack::Victory));
}

#[test]
fn the_stop_order_is_victory_cap_hp_budget() {
    let mut state = arena();
    state.party[0].hp = 4; // 20 % of 20, under the default 25 %
    let exhausted = AutoFightRuntime {
        actions: ACTION_CAP_DEFAULT,
        turn_slot: Some(0),
        no_progress: NO_PROGRESS_BUDGET + 1,
        ..AutoFightRuntime::default()
    };
    assert_eq!(
        plan(&state, &exhausted),
        AutoPlan::HandBack(HandBack::Victory)
    );
    add_monster(&mut state, 8, 8, 5, 0);
    assert_eq!(
        plan(&state, &exhausted),
        AutoPlan::HandBack(HandBack::ActionCap)
    );
    let budget_only = AutoFightRuntime {
        actions: 0,
        ..exhausted
    };
    assert_eq!(
        plan(&state, &budget_only),
        AutoPlan::HandBack(HandBack::Hurt(0))
    );
    state.party[0].hp = 5; // exactly 25 %: not below
    assert_eq!(
        plan(&state, &budget_only),
        AutoPlan::HandBack(HandBack::Stuck)
    );
    // 0 disables the HP limit; a dead member never counts.
    state.party[0].hp = 4;
    let off = CombatSettings {
        pause_below_hp_percent: 0,
        ..CombatSettings::default()
    };
    assert_eq!(
        plan_next_auto_key(&state, &off, &whole_party(), &budget_only),
        AutoPlan::HandBack(HandBack::Stuck)
    );
    state.party[0].hp = 20;
    let mut fallen = state.party[0];
    fallen.status = b'D';
    fallen.hp = 0;
    state.party.push(fallen);
    assert_eq!(
        plan(&state, &budget_only),
        AutoPlan::HandBack(HandBack::Stuck)
    );
}

// (e) never answered.

#[test]
fn prompts_auto_never_answers() {
    let base = || {
        let mut state = arena();
        add_monster(&mut state, 8, 8, 5, 0);
        state
    };
    assert!(matches!(plan_fresh(&base()), AutoPlan::Key(_)));

    for flag in [
        COMBAT_ACTOR_FLAG_CONTROLLED,
        COMBAT_ACTOR_FLAG_STATUS_DISABLED,
        COMBAT_ACTOR_FLAG_DRAGGED_UNDER,
    ] {
        let mut state = base();
        add_party_member(&mut state, 0, 2, 5, flag);
        assert_eq!(plan_fresh(&state), AutoPlan::NotOurs, "flag {flag:#04x}");
    }

    // Solo mode on roster 1 denies roster 0's turn.
    let mut state = base();
    state.active_player = Some(1);
    assert_eq!(plan_fresh(&state), AutoPlan::NotOurs);

    // A charmed monster prompted on the party's side, allies off.
    let mut state = base();
    add_monster(&mut state, 9, 5, 2, COMBAT_ACTOR_FLAG_CONTROLLED);
    state.pending_combat_actor_slot = Some(9);
    assert_eq!(plan_fresh(&state), AutoPlan::NotOurs);

    // A roster position not handed over.
    let state = base();
    let mut engaged = whole_party();
    engaged.members[0] = false;
    assert_eq!(
        plan_next_auto_key(
            &state,
            &CombatSettings::default(),
            &engaged,
            &AutoFightRuntime::default()
        ),
        AutoPlan::NotOurs
    );

    // Not at the bare prompt (no pending slot).
    let mut state = base();
    state.pending_combat_actor_slot = None;
    assert_eq!(plan_fresh(&state), AutoPlan::NotOurs);
}

// (f) never targeted.

#[test]
fn a_controlled_party_member_is_never_the_target() {
    let mut state = arena();
    // Member 1 under Sword of Chaos or charm, adjacent: the nearest
    // hostile-group actor, but a roster member.
    add_party_member(&mut state, 1, 3, 5, COMBAT_ACTOR_FLAG_CONTROLLED);
    add_monster(&mut state, 8, 2, 9, 0);
    assert!(combat_target_groups_are_hostile(
        state.combat_target_group_for_slot(0),
        state.combat_target_group_for_slot(1)
    ));
    assert_eq!(auto_target(&state, 0), Some(8));
    assert_eq!(
        plan_fresh(&state),
        AutoPlan::Key(AutoKey::Step(InputDirection::South))
    );
    // With no monster left in sight it hands back rather than hit them.
    state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER;
    assert_ne!(plan_fresh(&state), AutoPlan::Key(AutoKey::Attack));
}

// (g) no game text.

/// The string literals of a Rust source, comments stripped.
fn string_literals(source: &str) -> Vec<String> {
    let mut literals = Vec::new();
    for line in source.lines() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        let mut chars = code.chars().peekable();
        // auto_combat.rs has no '"' char literal, so a plain scan suffices.
        while let Some(c) = chars.next() {
            if c == '/' && chars.peek() == Some(&'/') {
                break;
            }
            if c == '"' {
                let mut literal = String::new();
                let mut escaped = false;
                for d in chars.by_ref() {
                    if escaped {
                        literal.push(d);
                        escaped = false;
                    } else if d == '\\' {
                        literal.push(d);
                        escaped = true;
                    } else if d == '"' {
                        break;
                    } else {
                        literal.push(d);
                    }
                }
                literals.push(literal);
            }
        }
    }
    literals
}

#[test]
fn auto_combat_rs_holds_no_game_text() {
    let source = include_str!("../auto_combat.rs");
    let literals = string_literals(source);
    // Only the 1.0.1 script tokens of `AutoKey::script_token` may appear.
    let allowed = [
        "A", "pass", "enter", "\\x1b", "\\x1b[A", "\\x1b[B", "\\x1b[C", "\\x1b[D", "\\x1b[H",
        "\\x1b[5~", "\\x1b[F", "\\x1b[6~",
    ];
    assert!(!literals.is_empty());
    for literal in &literals {
        assert!(
            allowed.contains(&literal.as_str()),
            "string literal {literal:?} in auto_combat.rs"
        );
    }
    for word in [
        "VICTORY", "Blocked", "Nothing", "What?", "Zzzzz", "ARGH", "Aim",
    ] {
        assert!(
            !literals.iter().any(|literal| literal.contains(word)),
            "{word}"
        );
    }
}

// (h) never emitted at the command prompt.

#[test]
fn command_prompt_keys_are_only_attack_pass_and_cardinal_steps() {
    let side = COMBAT_ARENA_SIDE as u8;
    let mut seen = Vec::new();
    for (tx, ty) in (0..side).flat_map(|x| (0..side).map(move |y| (x, y))) {
        if (tx, ty) == (2, 5) {
            continue;
        }
        let mut state = arena();
        add_monster(&mut state, 8, tx, ty, 0);
        for no_progress in 0..=NO_PROGRESS_BUDGET {
            let fight = AutoFightRuntime {
                turn_slot: Some(0),
                no_progress,
                ..AutoFightRuntime::default()
            };
            if let AutoPlan::Key(key) = plan(&state, &fight) {
                assert_command_key(key);
                seen.push(key);
            }
        }
    }
    assert!(seen.contains(&AutoKey::Attack));
    assert!(seen.contains(&AutoKey::Pass));
    assert!(seen.iter().any(|key| matches!(key, AutoKey::Step(_))));

    // And over a driven fight.
    let mut state = arena();
    add_monster(&mut state, 8, 8, 8, 0);
    add_monster(&mut state, 9, 6, 2, 0);
    let mut fight = AutoFightRuntime::default();
    let (keys, _) = drive(&mut state, &CombatSettings::default(), &mut fight, 60);
    assert!(!keys.is_empty());
    for (prompt, key) in keys {
        if matches!(prompt, AutoPrompt::Command { .. }) {
            assert_command_key(key);
        }
    }
}
