//! Spec-conformance pins for the `chunk07-town` gap batch.
//!
//! Every test names the published sentence it pins. Read the spec text in the
//! doc comment before changing an assertion.

use std::fs;

use u5_runtime::test_fixtures::{debug_game_dir, open_grid, test_state};
use u5_runtime::*;

/// Append `count` living companions behind the Avatar, keeping every parallel
/// party vector in step so the roster sync sees a well-formed party.
fn push_companions(state: &mut PlayState, count: usize) {
    for _ in 0..count {
        let slot = state.party.len() as u8;
        state.party.push(PartyMember {
            slot,
            class_byte: b'F',
            status: b'G',
            climb_stat: 20,
            mana: 0,
            hp: 40,
            max_hp: 40,
            level: 3,
        });
        let mut name = [0u8; SAVE_CHARACTER_NAME_LEN];
        name[0] = b'C';
        name[1] = b'0' + slot;
        state.party_names.push(name);
        state.party_experience.push(100 + u16::from(slot));
        state.party_stay_counters.push(0);
        state.party_strengths.push(20);
        state.party_intelligence.push(20);
        state
            .party_equipment
            .push([EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]);
    }
    state.party_roster = state.synced_party_roster();
}

/// `blackthorn.md §5`: "**The punishment is an execution, and it is durable.**
/// ... the routine:
///
/// - erases that companion's on-screen actor;
/// - lifts their roster record out of the party, compacts the remaining records
///   up, and decrements the party count;
/// - parks the lifted record in the last roster slot with a **whereabouts value
///   that matches no location**."
///
/// The withdrawn reading of §4 substituted a per-member jail flag for the
/// death, so the roster never shrank; the engine's own earlier shape did a bare
/// party removal with no roster park and no whereabouts write at all.
#[test]
fn blackthorn_execution_lifts_the_record_and_parks_it_with_an_unmatchable_whereabouts() {
    let mut state = test_state(open_grid(), 5, 5);
    push_companions(&mut state, 2);
    // The cinematic holds the victim in the `SecondPartyMember` actor slot.
    let actor_slot = BlackthornCutsceneActor::SecondPartyMember.slot_index() as usize;
    if state.active_objects.len() <= actor_slot {
        state
            .active_objects
            .resize(actor_slot + 1, ActiveObject::empty());
    }
    state.active_objects[actor_slot] = ActiveObject {
        type_byte: 0x21,
        tile: 0x21,
        x: 4,
        y: 8,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 1,
        aux3: BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER,
    };
    let victim_name = state.party_names[1];
    let victim_experience = state.party_experience[1];
    let trailing_name = state.party_names[2];

    let report = state
        .execute_blackthorn_companion(1)
        .expect("slot 1 is a companion behind the Avatar");
    assert!(report.contains("party count 2"), "{report}");

    // "erases that companion's on-screen actor"
    assert!(
        state.active_objects[actor_slot].is_empty(),
        "the victim's cinematic actor is erased",
    );

    // "lifts their roster record out of the party, compacts the remaining
    // records up, and decrements the party count"
    assert_eq!(state.party.len(), 2);
    assert_eq!(state.party_names.len(), 2);
    assert_eq!(
        state.party_names[1], trailing_name,
        "the record behind the victim compacts up into slot 1",
    );
    assert_eq!(
        state.party.iter().map(|m| m.slot).collect::<Vec<_>>(),
        vec![0, 1],
        "the surviving party slots are renumbered",
    );

    // "parks the lifted record in the last roster slot"
    let parked = state
        .party_roster
        .last()
        .expect("the roster keeps the lifted record");
    assert_eq!(parked.name, victim_name);
    assert_eq!(parked.experience, victim_experience);

    // "...with a whereabouts value that matches no location. That whereabouts
    // field is the same one the innkeeper uses when a companion is left at an
    // inn; the value written here matches no inn and no scene, so no inn can
    // ever retrieve them".
    let executed = state
        .inn_registry
        .iter()
        .find(|guest| guest.name == victim_name)
        .expect("the executed record carries a whereabouts value");
    assert_eq!(
        executed.scene_marker,
        PlayState::BLACKTHORN_EXECUTED_WHEREABOUTS,
    );
    assert!(
        Scene::new(PlayState::BLACKTHORN_EXECUTED_WHEREABOUTS).is_err(),
        "the parked whereabouts value matches no scene",
    );
    for marker in 1u8..=0xfeu8 {
        assert!(
            inn_guest_indices_for_scene(&state.inn_registry, marker).is_empty(),
            "no inn scene marker {marker:#04x} can retrieve the executed companion",
        );
    }
}

/// `blackthorn.md §4`: "**A wrong answer otherwise escalates.** The first wrong
/// answer produces a threat naming the companion at risk. Later wrong answers
/// stamp a tile into the cutscene map, and the fourth wrong answer **kills**
/// the named companion with the pendulum-blade narration."
///
/// The engine previously ran the captive-cell handoff on the *first* wrong
/// answer, so the interrogation ended immediately and no later ordinal was ever
/// reached.
#[test]
fn blackthorn_first_wrong_answer_threatens_and_re_asks_instead_of_ending() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    push_companions(&mut state, 2);
    let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap();
    state.area = Area::Town { scene, floor: 0 };
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    assert!(state.active_blackthorn.is_some());

    // Shrine zero is Honesty, whose accepted answer is `Ahm`.
    let outcome = state
        .submit_blackthorn_audience_answer("nonsense", &dir)
        .unwrap();

    assert_eq!(outcome, MoveOutcome::PromptDeclined);
    assert!(
        state.active_blackthorn.is_some(),
        "the interrogation is still open after the first wrong answer",
    );
    assert_eq!(
        state.party.len(),
        3,
        "the first wrong answer threatens; it does not kill",
    );
    assert!(
        state.message.contains("threatens"),
        "the first wrong answer names the companion at risk: {}",
        state.message,
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn blackthorn_middle_wrong_answers_stamp_props_without_erasing_the_victim() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    push_companions(&mut state, 2);
    state.area = Area::Town {
        scene: Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap(),
        floor: 0,
    };
    state.begin_blackthorn_audience_capture(&dir).unwrap();

    state
        .submit_blackthorn_audience_answer("nonsense", &dir)
        .unwrap();
    state
        .submit_blackthorn_audience_answer("nonsense", &dir)
        .unwrap();
    assert_eq!(
        state
            .blackthorn_audience_map
            .as_ref()
            .and_then(|map| map.tile(5, 7)),
        Some(BLACKTHORN_PENDULUM_TILE)
    );
    assert!(!state.active_objects[1].is_empty());

    state
        .submit_blackthorn_audience_answer("nonsense", &dir)
        .unwrap();
    assert_eq!(
        state
            .blackthorn_audience_map
            .as_ref()
            .and_then(|map| map.tile(5, 9)),
        Some(BLACKTHORN_HOURGLASS_TILE)
    );
    assert!(!state.active_objects[1].is_empty());
    assert_eq!(state.party.len(), 3);
    let _ = fs::remove_dir_all(dir);
}

/// `blackthorn.md §4`: "... and the fourth wrong answer **kills** the named
/// companion with the pendulum-blade narration", executed through the durable
/// §5 routine.
#[test]
fn blackthorn_fourth_wrong_answer_executes_the_named_companion() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    push_companions(&mut state, 2);
    let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap();
    state.area = Area::Town { scene, floor: 0 };
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    let victim_name = state.party_names[1];

    for ordinal in 1..=3 {
        assert_eq!(
            state
                .submit_blackthorn_audience_answer("nonsense", &dir)
                .unwrap(),
            MoveOutcome::PromptDeclined,
            "wrong answer {ordinal} re-asks rather than resolving",
        );
        assert!(state.active_blackthorn.is_some());
        assert_eq!(state.party.len(), 3);
    }

    state
        .submit_blackthorn_audience_answer("nonsense", &dir)
        .unwrap();

    assert!(
        state.active_blackthorn.is_none(),
        "the fourth wrong answer resolves the interrogation",
    );
    assert_eq!(
        state.party.len(),
        2,
        "the fourth wrong answer kills the named companion",
    );
    assert!(
        state
            .inn_registry
            .iter()
            .any(|guest| guest.name == victim_name
                && guest.scene_marker == PlayState::BLACKTHORN_EXECUTED_WHEREABOUTS),
        "the killed companion is parked with an unmatchable whereabouts",
    );
    let _ = fs::remove_dir_all(dir);
}

/// `blackthorn.md §5`: "The same execution runs on the *correct*-answer branch
/// whenever more than one companion is alive, under a different message -
/// Blackthorn thanking the player for their honesty and granting the companion
/// 'a merciful death'." The merciful-death branch previously did a bare party
/// removal with no roster park and no whereabouts write.
#[test]
fn blackthorn_merciful_death_runs_the_same_durable_execution() {
    let mut state = test_state(open_grid(), 5, 5);
    push_companions(&mut state, 2);
    let victim_name = state.party_names[1];

    let fate = state.apply_blackthorn_correct_answer_companion_fate();

    assert!(fate.contains("merciful death"), "{fate}");
    assert_eq!(state.party.len(), 2);
    assert_eq!(
        state
            .party_roster
            .last()
            .expect("the roster keeps the lifted record")
            .name,
        victim_name,
    );
    assert!(
        state
            .inn_registry
            .iter()
            .any(|guest| guest.name == victim_name
                && guest.scene_marker == PlayState::BLACKTHORN_EXECUTED_WHEREABOUTS),
        "the merciful death parks the record with an unmatchable whereabouts",
    );
}
