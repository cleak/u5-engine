// Spec-conformance regressions for the 2026-10-16 conformance audit, cluster
// g5: `systems/blackthorn.md`, `systems/conversation.md`,
// `formats/signs-dat.md` and `formats/look2-dat.md`.
//
// Each test names the audit item and the published sentence it pins.

/// Audit A038 / A039. `blackthorn.md §7` step 11: the rescue hands control to
/// scene seventeen "on logical floor one at local position `(10, 10)`, with
/// the clock spun forward until the hour reads 06:00". "Logical floor" is the
/// signed floor byte (`formats/location-dat.md` Section 4), so floor `+1`.
#[test]
fn audit_a038_a039_rescue_lands_on_floor_one_at_six_in_the_morning() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.clock = GameClock::new(14, 30).unwrap();
    state.torch_counter = 9;
    state.food = 0;

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    let outcome = state.run_blackthorn_rescue_to_handoff(&dir).unwrap();
    assert!(matches!(
        outcome,
        Some(MoveOutcome::Transition(AreaTransition::EnteredLocation(_)))
    ));

    assert_eq!(state.clock.hour, 6, "§7 step 11: the hour reads 06:00");
    match state.area {
        Area::Town { scene, floor } => {
            assert_eq!(scene.byte, BLACKTHORN_RESCUE_HANDOFF_SCENE);
            assert_eq!(floor, 1, "§7 step 11: logical floor one");
        }
        other => panic!("rescue left the party in {other:?}"),
    }
    // §8.1: the food repair and light clears come after the spin.
    assert_eq!(state.torch_counter, 0);
    assert_eq!(state.food, 63);
    let _ = fs::remove_dir_all(dir);
}

/// Audit A030. `blackthorn.md §3`: inside Lord Blackthorn's Castle, "If the
/// party reaches that helper there with nobody able to act and nobody
/// asleep, the helper returns silently, without the audience or a surrender
/// question." The ordinary-surrender fallback is withdrawn (R442).
#[test]
fn audit_a030_defeated_party_in_the_palace_gets_no_surrender_question() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    assert_eq!(state.party_capability(), PartyCapability::Defeated);
    let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap();
    state.area = Area::Town { scene, floor: 0 };
    state.message.clear();

    let outcome = state
        .open_town_arrest(
            TownArrestPrompt {
                scene_byte: BLACKTHORN_CAPTIVE_CELL_SCENE,
                floor: 0,
                npc_slot: 1,
            },
            Some(&dir),
        )
        .unwrap();

    assert_eq!(outcome, MoveOutcome::Used);
    assert!(state.pending_town_arrest.is_none());
    assert!(state.active_blackthorn.is_none());
    assert!(state.message.is_empty(), "{:?}", state.message);
    let _ = fs::remove_dir_all(dir);
}

fn audit_g5_companion(slot: u8) -> PartyMember {
    PartyMember {
        slot,
        class_byte: b'F',
        status: b'G',
        climb_stat: 10,
        mana: 0,
        hp: 30,
        max_hp: 30,
        level: 2,
    }
}

/// Audit A032. `blackthorn.md §4` (R440): "Avatar plus one living companion
/// is enough for the execution branch; the earlier requirement for more than
/// one living companion is withdrawn."
#[test]
fn audit_a032_avatar_plus_one_companion_is_enough_for_the_merciful_death() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }

    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    assert!(
        state.message.contains("merciful death"),
        "record 5 for two nondead members: {:?}",
        state.message
    );
    // The exit-beat key, then the beat itself.
    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    state.run_blackthorn_audience_exit_to_handoff(&dir).unwrap();
    assert_eq!(state.party.len(), 1, "the second nondead member is executed");
    let _ = fs::remove_dir_all(dir);
}

fn audit_g5_named(state: &mut PlayState, names: &[&str]) {
    state.party_names = names
        .iter()
        .map(|name| {
            let mut bytes = [0u8; SAVE_CHARACTER_NAME_LEN];
            bytes[..name.len()].copy_from_slice(name.as_bytes());
            bytes
        })
        .collect();
}

/// Audit A033. `blackthorn.md §4` (R441): "The first wrong answer produces a
/// threat naming the second roster entry. ... If the second roster entry is
/// dead, the threatened name and actual victim differ."
#[test]
fn audit_a033_first_threat_names_the_fixed_second_roster_entry() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party.push(audit_g5_companion(3));
    state.party[1].status = b'D';
    state.party[1].hp = 0;
    audit_g5_named(&mut state, &["Avatar", "Deadman", "Iolo", "Shamino"]);
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }

    state.submit_blackthorn_audience_answer("wrong", &dir).unwrap();
    assert!(
        state.message.contains("Deadman die!"),
        "the threat names roster entry two: {:?}",
        state.message
    );
    let _ = fs::remove_dir_all(dir);
}

fn audit_g5_audience_tile(state: &PlayState, x: usize, y: usize) -> Option<u8> {
    state
        .blackthorn_audience_map
        .as_ref()
        .and_then(|map| map.tile(x, y))
}

/// Audit A037. `blackthorn.md §6.1`: the failed-challenge script swap "runs on
/// the **first** wrong answer only. On any *later* wrong answer an open-coded
/// stamp selected by question index writes the hourglass cell directly
/// instead: index 1 writes `0xEB`, index 2 writes `0xE8` ... The visible
/// sequence for an all-wrong run is therefore `0xE8` → `0xE9` → `0xEB` →
/// `0xE8`."
#[test]
fn audit_a037_hourglass_runs_e9_eb_e8_and_the_first_wrong_plays_the_beat() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }

    let sound_before = state.sound_effect_serial;
    state.submit_blackthorn_audience_answer("wrong", &dir).unwrap();
    // §4.1: record 7, "after its scene beat", record 8.
    assert!(
        state
            .sound_effects_after(sound_before)
            .contains(&SoundEffect::BlackthornMovementStinger),
        "the first wrong answer plays the failed-challenge beat"
    );
    assert_eq!(audit_g5_audience_tile(&state, 5, 7), Some(0x82));
    assert_eq!(audit_g5_audience_tile(&state, 5, 9), Some(0xe9));
    // Acknowledge the threat; the second ask follows.
    state.submit_blackthorn_audience_answer("", &dir).unwrap();

    state.submit_blackthorn_audience_answer("wrong", &dir).unwrap();
    assert_eq!(audit_g5_audience_tile(&state, 5, 9), Some(0xeb));
    assert_eq!(audit_g5_audience_tile(&state, 5, 7), Some(0x82));

    state.submit_blackthorn_audience_answer("wrong", &dir).unwrap();
    assert_eq!(audit_g5_audience_tile(&state, 5, 9), Some(0xe8));
    assert_eq!(audit_g5_audience_tile(&state, 5, 7), Some(0x82));

    // Index 3 executes rather than stamping, and does not replay the swap.
    state.submit_blackthorn_audience_answer("wrong", &dir).unwrap();
    assert_eq!(audit_g5_audience_tile(&state, 5, 9), Some(0xe8));
    let _ = fs::remove_dir_all(dir);
}

/// Audit A036. `blackthorn.md §5`: "The roster edit and its repaint both
/// precede the held page ... The correct-answer branch's punishment has no key
/// wait of its own - the key that follows it belongs to the exit beat".
#[test]
fn audit_a036_merciful_death_lifts_the_roster_before_the_exit_key() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }

    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    assert!(state.message.contains("merciful death"), "{:?}", state.message);
    assert!(
        state
            .active_blackthorn
            .as_ref()
            .is_some_and(|c| c.awaiting_closing_acknowledgement()),
        "the exit beat's key is still to come"
    );
    assert_eq!(state.party.len(), 2, "executed at answer time");
    assert_eq!(
        state.stats_panel,
        crate::stats_panel::stats_panel_snapshot_of(&state),
        "and repainted"
    );
    let _ = fs::remove_dir_all(dir);
}

/// Audit A036. `blackthorn.md §5`, pendulum branch: one key wait, on the page
/// that names the victim; after it record `6` and the helper's line feed
/// print and the audience leaves without another wait.
#[test]
fn audit_a036_pendulum_branch_takes_one_key_and_leaves() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    for _ in 0..4 {
        state.submit_blackthorn_audience_answer("wrong", &dir).unwrap();
        if state.active_blackthorn.as_ref().is_some_and(|c| c.awaiting_acknowledgement()) {
            state.submit_blackthorn_audience_answer("", &dir).unwrap();
        }
    }
    assert_eq!(state.party.len(), 2, "the lift precedes the held page");
    assert!(state.message.contains(" is sliced in half! "), "{:?}", state.message);

    let outcome = state.submit_blackthorn_audience_answer("", &dir).unwrap();
    assert!(matches!(
        outcome,
        MoveOutcome::Transition(AreaTransition::EnteredLocation(_))
    ));
    assert!(state.active_blackthorn.is_none());
    let _ = fs::remove_dir_all(dir);
}

fn audit_g5_stingers_after(state: &PlayState, serial: u64) -> usize {
    state
        .sound_effects_after(serial)
        .iter()
        .filter(|effect| **effect == SoundEffect::BlackthornMovementStinger)
        .count()
}

/// Audit A034 / A035. `blackthorn.md §4.2`: the exit beat is "one blocking key
/// wait followed by the viewport-only cutscene vector Section 6.3 lists", and
/// that vector "clears the seated-Blackthorn tableau slot", so "Every branch
/// that runs the exit beat therefore skips that cleanup".
#[test]
fn audit_a034_a035_exit_beat_plays_its_vector_and_skips_the_tableau_cleanup() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    // A solo party: record 9, then the exit beat.
    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    let seated = BlackthornCutsceneActor::SeatedBlackthorn.slot_index() as usize;
    assert!(
        !state.active_objects[seated].is_empty(),
        "the correct answer itself runs no tableau cleanup"
    );

    let before = state.sound_effect_serial;
    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    let exit = BlackthornCutsceneVm::with_audience_setup(Vec::new());
    let mut exit = exit;
    exit.run(blackthorn_cutscene_beat_commands(BlackthornCutsceneBeat::AudienceExit));
    assert_eq!(
        audit_g5_stingers_after(&state, before),
        usize::from(exit.stinger_count),
        "the key plays the exit vector's stingers"
    );
    assert!(state.active_objects[seated].is_empty(), "the vector clears slot 8");
    let before = state.sound_effect_serial;
    state.run_blackthorn_audience_exit_to_handoff(&dir).unwrap();
    assert_eq!(
        audit_g5_stingers_after(&state, before),
        0,
        "no conditional cleanup after the exit beat"
    );
    let _ = fs::remove_dir_all(dir);
}

/// Audit A035. `blackthorn.md §4.2`: "only the punishing branch, which never
/// ran the beat, reaches" the conditional tableau cleanup of §6.3.
#[test]
fn audit_a035_punishing_branch_runs_the_conditional_tableau_cleanup() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    for _ in 0..4 {
        state.submit_blackthorn_audience_answer("wrong", &dir).unwrap();
        if state.active_blackthorn.as_ref().is_some_and(|c| c.awaiting_acknowledgement()) {
            state.submit_blackthorn_audience_answer("", &dir).unwrap();
        }
    }
    let before = state.sound_effect_serial;
    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    let mut cleanup = BlackthornCutsceneVm::with_audience_setup(Vec::new());
    cleanup.run(blackthorn_cutscene_beat_commands(
        BlackthornCutsceneBeat::ConditionalThroneCleanup,
    ));
    assert_eq!(
        audit_g5_stingers_after(&state, before),
        usize::from(cleanup.stinger_count),
        "the punishing tail runs the tableau cleanup"
    );
    let _ = fs::remove_dir_all(dir);
}

/// Audit A072. `conversation.md §7.6`: `F` grants "`+1` (cap `99`)", and
/// "Counter letters never assign a fixed final value; they always perform a
/// saturating add by one."
#[test]
fn audit_a072_f_grant_adds_one_with_the_shared_cap() {
    use crate::tlk_control_codes::TlkActionDispatchVerb;
    let mut state = test_state(open_grid(), 1, 1);
    state.climbing_gear = 3;
    state.apply_tlk_action_grants(&[TlkActionDispatchVerb::SetGrappleGate]);
    assert_eq!(state.climbing_gear, 4);
    state.apply_talk_action_grants(&['F', 'F']);
    assert_eq!(state.climbing_gear, 6);
    state.climbing_gear = 99;
    state.apply_tlk_action_grants(&[TlkActionDispatchVerb::SetGrappleGate]);
    assert_eq!(state.climbing_gear, 99);
}
