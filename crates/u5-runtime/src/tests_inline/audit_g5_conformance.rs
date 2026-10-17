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

/// Audit A036, correct-answer branch. `blackthorn.md §5` says "The roster
/// edit and its repaint both precede the held page", but the stock DOSBox
/// capture (paired scenario `bt-correct`, beats `right` and `after`) holds
/// the record-5 page with the companion still listed and removes him only
/// with the following key. The capture is authoritative pending
/// cleak/u5-spec#298, so the lift waits for that key.
#[test]
fn audit_a036_merciful_death_lifts_the_roster_on_the_acknowledgement_key() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.repaint_stats_panel();
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
    assert_eq!(state.party.len(), 3, "the held page still lists the companion");
    assert_eq!(
        state.stats_panel,
        crate::stats_panel::stats_panel_snapshot_of(&state),
        "and the panel shows the full party"
    );

    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    assert_eq!(state.party.len(), 2, "the key that follows lifts him");
    assert_eq!(
        state.stats_panel,
        crate::stats_panel::stats_panel_snapshot_of(&state),
        "and repaints the panel"
    );
    let _ = fs::remove_dir_all(dir);
}

/// Audit A036, pendulum branch, reconciled with the stock capture. The
/// `bt-escalate` beat `ask5` holds the record-4 narration (ending on `falls!`)
/// with the victim still listed on the panel. The key after it lifts the
/// victim and repaints, and holds the page that names them; the key after
/// that prints record `6` and leaves without another wait.
#[test]
fn audit_a036_pendulum_narration_is_held_with_the_victim_still_listed() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.repaint_stats_panel();
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
    assert!(state.message.contains("pendulum blade falls!"), "{:?}", state.message);
    assert!(!state.message.contains(" is sliced in half! "), "{:?}", state.message);
    assert_eq!(state.party.len(), 3, "the held narration still lists the victim");
    assert_eq!(
        state.stats_panel,
        crate::stats_panel::stats_panel_snapshot_of(&state),
        "and the panel shows the full party"
    );

    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    assert!(state.message.contains(" is sliced in half! "), "{:?}", state.message);
    assert_eq!(state.party.len(), 2, "the narration's key lifts the victim");
    assert_eq!(
        state.stats_panel,
        crate::stats_panel::stats_panel_snapshot_of(&state),
        "and repaints the panel"
    );

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
    // The held pendulum narration's key (stock `bt-escalate` beat `ask5`)
    // brings up the page that names the victim; the tail follows its key.
    state.submit_blackthorn_audience_answer("", &dir).unwrap();
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

/// Audit A073. `conversation.md §7.6`: "Only the **first** occurrence found
/// is tested ... A typed line whose first hit is mid-word therefore fails
/// even when a clean hit follows it later in the line."
#[test]
fn audit_a073_ask_who_tests_only_the_first_occurrence() {
    let names: [&[u8]; 2] = [b"AVATAR", b"IOLO"];
    assert_eq!(tlk_ask_who_match(b"triolo iolo", &names), 0);
    assert_eq!(tlk_ask_who_match(b"iolo triolo", &names), 2);
}

/// Audit A066. `conversation.md §2` step 5, "The dispatcher's own lines": the
/// silent-figure index prints `"` then `Don't hurt me!` ⏎ `Please go away!`
/// then `"` and a line feed, after the dispatcher's own leading line feed.
#[test]
fn audit_a066_silent_figure_prints_the_published_rows() {
    assert_eq!(
        TOWN_NPC_COWERING_RESPONSE,
        "\"Don't hurt me!\nPlease go away!\""
    );
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 1,
            type_byte: 0x50,
            dialog_id: TOWN_NPC_COWERING_DIALOG_ID,
            schedule: [3, 3, 3, 2, 2, 2, 1, 1, 1, 0, 0, 0, 6, 12, 18, 22],
            name: None,
        },
    ]);
    state.talk_facing_with_dialogue(&HashMap::new());
    assert!(state.message.contains("Don't hurt me!\nPlease go away!"), "{:?}", state.message);
    assert!(!state.message.contains("cower"));
}

/// A one-NPC town conversation whose `WHO` keyword runs a `0x88` ASK-WHO.
fn audit_g5_ask_who_conversation() -> PlayState {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        ["Maris", "a quiet sage", "Greetings", "I read books", "Farewell", "WHO", "Name."]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut who_response = enc("Name.");
    who_response.push(TLK_CODE_ASK_WHO);
    who_response.extend(enc(" Done."));
    who_response.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("WHO"),
            who_response,
        ],
    );
    let mut state = test_state(open_grid(), 1, 1);
    state.party_names = vec![*b"AVATAR\0\0\0"];
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );
    state
}

fn audit_g5_transcript(state: &PlayState) -> String {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Audit A070. `conversation.md §6` step 3: "The ASK-WHO name prompt ...
/// never reaches the farewell path: an empty line there prints the dismissive
/// acknowledgement and returns to the stream".
#[test]
fn audit_a070_empty_ask_who_answer_echoes_no_bye() {
    let mut state = audit_g5_ask_who_conversation();
    handle_play_key_input(&mut state, 'W', "HO", Path::new("")).unwrap();
    assert_eq!(state.message, TLK_ASK_WHO_PROMPT);
    let before = audit_g5_transcript(&state);
    handle_play_key_input(&mut state, '\n', "", Path::new("")).unwrap();
    let after = audit_g5_transcript(&state);
    let added = after.strip_prefix(before.as_str()).unwrap_or(&after);
    assert!(added.contains(TLK_ASK_WHO_DISMISSIVE_LINE), "{added:?}");
    assert!(!added.contains("BYE"), "no farewell echo: {added:?}");
    assert!(state.active_conversation.is_some());
}

/// Audit A069. `conversation.md §7.6`: "a line containing only a space is
/// *not* empty", and §6 step 3's shortcut is for "an empty line" only, so a
/// lone space runs the ordinary scan and ends on the no-match line.
#[test]
fn audit_a069_lone_space_is_not_the_bye_shortcut() {
    let mut state = audit_g5_ask_who_conversation();
    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert!(state.active_conversation.is_some(), "a space does not say goodbye");
    assert!(
        audit_g5_transcript(&state).contains("I cannot help thee with that."),
        "{}",
        audit_g5_transcript(&state)
    );
}

/// Audit A065. `conversation.md §2` step 5, "No-owner Talk": "If no occupied,
/// non-idle NPC record is linked to the faced active object, Talk does not
/// stop with `Nobody's here!`: it runs the dispatcher with an unrelated
/// adjacent record that reads as all zero, so the bare `No response!` is
/// printed for any sprite."
#[test]
fn audit_a065_faced_object_without_an_npc_record_answers_no_response() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
    let z = state.current_floor().unwrap();
    state.active_objects[1] = ActiveObject {
        type_byte: 0x50,
        tile: 0x50,
        x: 2,
        y: 1,
        z,
        ..ActiveObject::empty()
    };
    state.talk_facing_with_dialogue(&HashMap::new());
    assert_eq!(state.message, format!("\n{TALK_NO_RESPONSE_MESSAGE}"));

    state.active_objects[1] = ActiveObject::empty();
    state.talk_facing_with_dialogue(&HashMap::new());
    assert_eq!(state.message, TALK_NOBODY_HERE_LINE);
}

/// Audit A067. `conversation.md §2` step 5: at a reached waypoint whose
/// behaviour is `4` the dispatcher "first **rewrites that waypoint's
/// behaviour to the bounded-wander mode** (value `1`, a persisted schedule
/// edit ...)", and a zero live word then prints the bare `No response!`.
#[test]
fn audit_a067_talk_stands_the_reached_waypoint_down_from_four_to_one() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 1,
            type_byte: crate::tlk_control_codes::TALK_GUARD_SPRITE,
            dialog_id: 0,
            schedule: [4, 4, 4, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    let index = state.npcs.iter().position(|npc| npc.slot == 1).unwrap();
    state.npcs[index].cached_wp = 0;
    state.talk_facing_with_dialogue(&HashMap::new());
    assert_eq!(state.npcs[index].schedule[NPC_SCHEDULE_AI_OFFSET], 1);
    assert_eq!(state.message, format!("\n{TALK_NO_RESPONSE_MESSAGE}"));
    let mutation = state
        .town_npc_mutations
        .iter()
        .find(|mutation| mutation.npc_slot == 1)
        .expect("the stand-down is a persisted schedule edit");
    assert_eq!(mutation.ai[0], 1);
}

/// A one-NPC town conversation whose `HOLD` keyword answers with `response`.
fn audit_g5_conversation_with_response(response: Vec<u8>) -> PlayState {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        ["Maris", "a quiet sage", "Greetings", "I read books", "Farewell", "HOLD", "x"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("HOLD"),
            response,
        ],
    );
    let mut state = test_state(open_grid(), 1, 1);
    state.party_names = vec![*b"AVATAR\0\0\0"];
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );
    state
}

fn audit_g5_held_response(first: &str, code: u8, second: &str) -> Vec<u8> {
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut bytes = enc(first);
    bytes.push(code);
    bytes.extend(enc(second));
    bytes.push(TLK_CODE_END_OF_RESPONSE);
    bytes
}

/// Audit A071. `conversation.md §7.3`: `0x83` is "a **timed** pause of up to
/// 28 passes" of one BIOS tick each; "after the 28th pass the pause ends by
/// itself", and "Neither code ever stops a response".
#[test]
fn audit_a071_pause_holds_the_rest_of_the_reply_for_28_ticks() {
    let mut state =
        audit_g5_conversation_with_response(audit_g5_held_response("Wait.", TLK_CODE_PAUSE, " Then."));
    handle_play_key_input(&mut state, 'H', "OLD", Path::new("")).unwrap();
    let held = audit_g5_transcript(&state);
    assert!(held.contains("\"Wait."), "{held:?}");
    assert!(!held.contains("Then."), "the rest waits: {held:?}");
    assert!(!held.ends_with(':'), "no prompt yet: {held:?}");
    assert!(state.text_hold_active());

    let start = 100.0;
    assert!(state.advance_text_hold_to(start));
    let ticks = crate::narration::bios_ticks_secs(28);
    assert!(state.advance_text_hold_to(start + ticks - 0.01));
    assert!(!state.advance_text_hold_to(start + ticks + 0.01));
    let after = audit_g5_transcript(&state);
    assert!(after.contains("\"Wait. Then.\""), "same row, closing quote after: {after:?}");
    assert!(after.ends_with(':'), "the prompt follows the hold: {after:?}");
    assert!(state.active_conversation.is_some());
}

/// Audit A071. §7.3: during a pause "a waiting key is consumed and ends the
/// pause at once" - it is not typed into the next prompt.
#[test]
fn audit_a071_a_key_ends_the_pause_and_is_consumed() {
    let mut state =
        audit_g5_conversation_with_response(audit_g5_held_response("Wait.", TLK_CODE_PAUSE, " Then."));
    handle_play_key_input(&mut state, 'H', "OLD", Path::new("")).unwrap();
    let turns = state.active_conversation.as_ref().unwrap().keyword_turns;
    handle_play_key_input(&mut state, 'J', "", Path::new("")).unwrap();
    assert!(!state.text_hold_active());
    assert_eq!(state.active_conversation.as_ref().unwrap().keyword_turns, turns);
    let after = audit_g5_transcript(&state);
    assert!(after.contains("\"Wait. Then.\""), "{after:?}");
    assert!(!after.contains(":J"), "the key was not typed: {after:?}");
    assert!(!after.contains("I read books"), "the key ran no keyword: {after:?}");
    assert_eq!(state.message, TLK_KEYWORD_PROMPT);
}

/// Audit A071. §7.3: `0x8F` blocks "for exactly one command keystroke" and
/// emits nothing - it is not a line feed and it never times out.
#[test]
fn audit_a071_wait_key_blocks_for_one_key_and_prints_nothing() {
    let mut state =
        audit_g5_conversation_with_response(audit_g5_held_response("One.", TLK_CODE_WAIT_KEY, "Two."));
    handle_play_key_input(&mut state, 'H', "OLD", Path::new("")).unwrap();
    assert!(state.advance_text_hold_to(0.0));
    assert!(state.advance_text_hold_to(1_000.0), "a key wait never times out");
    assert!(!audit_g5_transcript(&state).contains("Two."));
    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert!(!state.text_hold_active());
    let after = audit_g5_transcript(&state);
    assert!(after.contains("\"One.Two.\""), "{after:?}");
}

/// Audit A068. `conversation.md §6`: the rebuke prints its line with the
/// reserved-response quote/newline framing, then "runs a bounded pause loop"
/// of at most twenty-eight passes that a key cuts short, then reprompts.
#[test]
fn audit_a068_rebuke_is_framed_and_pauses_before_the_prompt() {
    let mut state = audit_g5_ask_who_conversation();
    handle_play_key_input(&mut state, 'A', "SS HAT", Path::new("")).unwrap();
    let held = audit_g5_transcript(&state);
    assert!(
        held.contains("\"With language like that, how did you become an Avatar?\""),
        "{held:?}"
    );
    assert!(state.text_hold_active(), "the pause loop runs before the prompt");
    assert!(!held.ends_with(':'), "{held:?}");
    assert!(state.advance_text_hold_to(5.0));
    assert!(!state.advance_text_hold_to(5.0 + crate::narration::bios_ticks_secs(28) + 0.01));
    assert!(audit_g5_transcript(&state).ends_with(':'));
    assert!(state.active_conversation.is_some());
}

/// Audit A012. `formats/signs-dat.md §4`: "Byte `0x0D` pauses for a keypress
/// and then resumes printing", and §6 "Pause on the embedded pause code".
#[test]
fn audit_a012_sign_pause_byte_waits_for_a_key_before_the_rest() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    fs::write(
        dir.join(SIGNS_DAT_FILE),
        signs_dat_bytes_for_test(&[(17, 0, 1, 2, b"Read\x0dMore")]),
    )
    .unwrap();
    // `view.md §3` row 5 (audit A180): the sign path is keyed on the live
    // terrain tile, so the sign is placed in the map, not as an object.
    let mut grid = open_grid();
    grid[32 + 2] = 0xa0;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.look_facing_with_game_dir(&dir).unwrap();
    state.flush_message_slot();
    assert_eq!(state.message, "Sign:\nRead");
    assert!(state.text_hold_active());
    assert!(state.advance_text_hold_to(1_000.0), "a key wait never times out");
    assert!(!audit_g5_transcript(&state).contains("More"));

    handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap();
    assert!(!state.text_hold_active());
    assert!(state.active_z_stats.is_none(), "the key only resumed the sign");
    assert!(
        audit_g5_transcript(&state).ends_with("ReadMore"),
        "{:?}",
        audit_g5_transcript(&state)
    );
    let _ = fs::remove_dir_all(dir);
}

/// u5-spec#298 Q14 (`RETRACTIONS.md` R531). `conversation.md §7.5`: `0x8B` is
/// ALARM - "Run the town alarm sweep of `systems/town-mode.md` Section 14:
/// forced pursuit for every occupied guard ... slot". A guard standing in the
/// roster is put into pursuit by a reply that reaches the code, and the reply
/// text is untouched.
#[test]
fn q14_alarm_code_in_a_reply_runs_the_town_alarm_sweep() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        ["Maris", "a quiet sage", "Greetings", "I read books", "Farewell", "HOLD", "x"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("HOLD"),
            audit_g5_held_response("Guards!", TLK_CODE_ALARM, " Seize them!"),
        ],
    );
    let mut state = test_state(open_grid(), 1, 1);
    state.party_names = vec![*b"AVATAR\0\0\0"];
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
        NpcSlot {
            slot: 2,
            type_byte: TOWN_NPC_ALARM_GUARD_TYPE,
            dialog_id: 0,
            schedule: [0, 0, 0, 9, 9, 9, 9, 9, 9, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );
    let guard = state.npcs.iter().position(|npc| npc.slot == 2).unwrap();
    let before = state.npcs[guard].schedule;
    assert!(state.town_npc_mutations.iter().all(|mutation| mutation.npc_slot != 2));
    handle_play_key_input(&mut state, 'H', "OLD", Path::new("")).unwrap();
    assert!(
        audit_g5_transcript(&state).contains("Guards! Seize them!"),
        "{}",
        audit_g5_transcript(&state)
    );
    assert_ne!(state.npcs[guard].schedule, before, "the guard is forced into pursuit");
    assert!(
        state.town_npc_mutations.iter().any(|mutation| mutation.npc_slot == 2),
        "the sweep's rewrite is persisted"
    );
}

fn q8_audience_with_avatar_gender(gender: u8) -> (PlayState, std::path::PathBuf) {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    audit_g5_named(&mut state, &["Shalla"]);
    let mut roster = state.synced_party_roster();
    roster[0].gender = gender;
    state.party_roster = roster;
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    (state, dir)
}

/// u5-spec#298 Q8. `blackthorn.md §3` step 8: the greeting to roster record
/// 0, held for a key, then the release order by record 0's gender byte - the
/// three published forms, `goodman` really joined.
#[test]
fn q8_greeting_and_release_lines_follow_record_zero() {
    for (gender, expected) in [
        (SAVE_GENDER_FEMALE_BYTE, "GUARD! Release this good lady at once!\""),
        (SAVE_GENDER_MALE_BYTE, "GUARD! Release this goodman at once!\""),
        (0x00, "GUARD! Release this goodat once!\""),
    ] {
        let (mut state, dir) = q8_audience_with_avatar_gender(gender);
        let held = audit_g5_transcript(&state);
        assert!(
            held.ends_with("Blackthorn says:\n\n\"Ah, Shalla!\n'Tis indeed an honour to meet thee at last! "),
            "{held:?}"
        );
        assert!(!held.contains("GUARD!"), "the release waits for the greeting's key");
        assert!(state.message_row_open_mid_line, "the greeting ends mid-row");

        state.submit_blackthorn_audience_answer("", &dir).unwrap();
        state.flush_message_slot();
        let after = audit_g5_transcript(&state);
        assert!(after.contains(expected), "{gender:#04x}: {after:?}");
        let _ = fs::remove_dir_all(dir);
    }
}

/// u5-spec#298 Q8. `blackthorn.md §5`: "Wrong answers past the first advance
/// the clock by two minutes ...; the first wrong answer makes no cleanup
/// call, so the interrogation itself adds six minutes on an all-wrong run".
#[test]
fn q8_wrong_answers_past_the_first_each_cost_two_minutes() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.clock = GameClock::new(10, 0).unwrap();
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    let minutes = |state: &PlayState| u32::from(state.clock.hour) * 60 + u32::from(state.clock.minute);
    let start = minutes(&state);
    let mut after = Vec::new();
    for _ in 0..4 {
        state.submit_blackthorn_audience_answer("wrong", &dir).unwrap();
        after.push(minutes(&state) - start);
        if state.active_blackthorn.as_ref().is_some_and(|c| c.awaiting_acknowledgement()) {
            state.submit_blackthorn_audience_answer("", &dir).unwrap();
        }
    }
    assert_eq!(after, vec![0, 2, 4, 6]);
    let _ = fs::remove_dir_all(dir);
}
