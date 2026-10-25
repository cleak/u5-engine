// Regressions for the pass-9 exploratory-playtest findings (p9misc, Q01-Q16).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `magic.md §5` Step 2: the dispatcher prints `Spell name:` and a colon
/// prompt; no spec document carries a `Cast what? Use C1IL/...` help
/// sentence. A `C` suffix with no selector letters prints nothing, the same
/// treatment playtest N07 gave the N/U/M/R suffixes (Q01).
#[test]
fn playtest_q01_cast_suffix_without_letters_prints_no_help_sentence() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    for suffix in ["9", "-"] {
        let outcome = state.cast_spell_from_suffix(suffix, &dir).unwrap();
        assert_eq!(outcome, MoveOutcome::Blocked);
        assert!(state.message.is_empty(), "{suffix:?}: {:?}", state.message);
    }
    assert!(cast_prompt_message().is_empty());
    let _ = fs::remove_dir_all(dir);
}

/// `text-output.md §10.6`: the general typed-string reader, "used for most
/// free-text prompts", has "keys are uppercased"; only character names keep
/// their case. `karma.md §12` reads the shrine's virtue and mantras as
/// ordinary typed strings (Q05).
#[test]
fn playtest_q05_shrine_virtue_and_mantra_answers_are_upper_cased() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    let mut session = ShrineSession::new(ShrineVirtue::Honesty);
    session.phase = ShrinePhase::Virtue;
    state.active_shrine = Some(session);

    state.step_active_shrine('h', "onEsty", &dir).unwrap();
    assert_eq!(
        state.active_shrine.as_ref().unwrap().virtue_buffer,
        "HONESTY"
    );
    state.step_active_shrine('\r', "", &dir).unwrap();
    assert!(
        playtest_transcript_rows(&state)
            .iter()
            .any(|row| row == ":HONESTY"),
        "{:?}",
        playtest_transcript_rows(&state)
    );
    state.flush_staged_narration();
    state.step_active_shrine('a', "hm", &dir).unwrap();
    let session = state.active_shrine.as_ref().unwrap();
    assert_eq!(session.phase, ShrinePhase::Mantra);
    assert_eq!(session.mantra_buffer, "AHM");
    assert_eq!(
        state.shrine_prompt_echo().as_deref(),
        Some(format!("{SHRINE_MANTRA_PROMPT}AHM").as_str())
    );
    let _ = fs::remove_dir_all(dir);
}

/// `magic.md §6` step 4: after the shortfall notice the quantity prompt "is
/// asked again" once, and in its key table "Anything else - space, letter,
/// punctuation - Discarded: no echo, no other effect, prompt still waiting".
/// A stray letter or diagonal code after the notice printed a second
/// `How much?` row (Q07).
#[test]
fn playtest_q07_discarded_key_after_shortfall_does_not_reprint_the_prompt() {
    let dir = debug_game_dir();
    let mut state = p4window_world_state();
    state.reagents = [0; REAGENT_COUNT];
    state.reagents[REAGENT_SULFUR_ASH] = 9;
    p4window_keys(&mut state, &dir, &['M', 'I', 'L', '\r', '1', 'M', '9', '9', '\r']);
    assert!(state.active_mix.is_some(), "{:?}", state.message);
    let prompt_rows = |state: &PlayState| {
        p4window_rows(state)
            .iter()
            .filter(|row| row.trim_end().starts_with(MMIX_QUANTITY_PROMPT_MESSAGE))
            .count()
    };
    // `How much? 99` above the notice and the re-asked prompt below it.
    let before = prompt_rows(&state);
    assert_eq!(before, 2, "{:?}", p4window_rows(&state));

    let diagonal = char::from(crate::INPUT_CODE_NORTHEAST);
    p4window_keys(&mut state, &dir, &['Z', diagonal, 'Q']);
    assert_eq!(prompt_rows(&state), before, "{:?}", p4window_rows(&state));

    // The prompt is still the one waiting: a digit lands on it.
    p4window_keys(&mut state, &dir, &['5']);
    let rows = p4window_rows(&state);
    assert_eq!(prompt_rows(&state), before, "{rows:?}");
    assert!(
        rows.iter().any(|row| row.trim_end() == "How much? 5"),
        "{rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// `town-mode.md §7.2` (u5-spec#300 Q34, R616): away from the harpsichord
/// the town digit reaches the shared active-player selector. `None!` and a
/// successful selection are free; `Invalid!` reports "acted" and costs the
/// one-minute town turn ("`1` and `0` left the clock unchanged, while `9`
/// ... printed `Invalid!` and advanced it by one minute").
#[test]
fn playtest_q14_town_digit_selection_is_free_and_invalid_reports_acted() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: b'F',
        status: b'G',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 0,
        hp: 20,
        max_hp: 20,
        level: 1,
    });
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0"];
    let clock_before = state.clock;

    handle_play_key_input(&mut state, '2', "", &dir).unwrap();
    assert_eq!(state.active_player, Some(1));
    assert_eq!(state.turn, 0, "a successful town selection is free");
    assert_eq!(state.clock, clock_before);

    handle_play_key_input(&mut state, '0', "", &dir).unwrap();
    assert_eq!(state.active_player, None);
    assert_eq!(state.turn, 0, "the deselect reports no action");
    assert_eq!(state.clock, clock_before);

    handle_play_key_input(&mut state, '9', "", &dir).unwrap();
    assert_eq!(state.message, SET_ACTIVE_PLAYER_INVALID_REPLY);
    assert_eq!(state.active_player, None);
    assert_eq!(state.turn, 1, "`Invalid!` reports acted");
    let mut expected = clock_before;
    expected.advance_minutes(1);
    assert_eq!(state.clock, expected);
    let _ = fs::remove_dir_all(dir);
}

/// `overworld.md §5` step 5 (u5-spec#300 Q34, R618): overworld digits reach
/// the same selector, not a speed selector. `1` and `0` are free; `8`
/// printed `Invalid!` and ran the per-turn block ("advanced the clock 08:39
/// to 08:41").
#[test]
fn spec300_q34_overworld_digit_reaches_the_active_player_selector() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.clock = GameClock::with_date(139, 4, 5, 8, 39).unwrap();

    handle_play_key_input(&mut state, '1', "", &dir).unwrap();
    assert_eq!(state.active_player, Some(0));
    handle_play_key_input(&mut state, '0', "", &dir).unwrap();
    assert_eq!(state.active_player, None);
    assert_eq!((state.turn, state.clock.hour, state.clock.minute), (0, 8, 39));

    handle_play_key_input(&mut state, '8', "", &dir).unwrap();
    assert_eq!(state.message, SET_ACTIVE_PLAYER_INVALID_REPLY);
    assert_eq!(state.turn, 1);
    assert_eq!((state.clock.hour, state.clock.minute), (8, 41));
    let _ = fs::remove_dir_all(dir);
}

/// Review of Q14: the epilogue an acted town digit now runs can print
/// (`time.md §5`'s `Starving!\n`). The member's name is already on the
/// transcript by then (an `Invalid!` digit is the acted one, u5-spec#300 Q34), so the warning takes a row of its own; it used to be
/// appended to the spent slot, and the next flush logged `Shamino` twice.
#[test]
fn playtest_q14_town_digit_epilogue_line_does_not_reprint_the_answer() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: b'F',
        status: b'G',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 0,
        hp: 60,
        max_hp: 60,
        level: 1,
    });
    state.party[0].hp = 60;
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0"];
    state.clock = GameClock::with_date(139, 4, 5, 17, 59).unwrap();
    state.food = 0;

    handle_play_key_input(&mut state, '9', "", &dir).unwrap();
    state.flush_message_slot();
    let rows = playtest_transcript_rows(&state);
    assert_eq!(state.clock.hour, 18, "{rows:?}");
    assert_eq!(
        rows.iter().filter(|row| row.as_str() == SET_ACTIVE_PLAYER_INVALID_REPLY).count(),
        1,
        "{rows:?}"
    );
    assert!(rows.iter().any(|row| row == "Starving!"), "{rows:?}");
    let _ = fs::remove_dir_all(dir);
}

/// `shops.md §8.C` names only "Record 191" for the solo Leave refusal and
/// `§8.B` only record `119` for the shipwright's menu. The engine carried
/// transcriptions of both as fallbacks; without `SHOPPE.DAT` nothing of the
/// records is shown now (Q16).
#[test]
fn playtest_q16_inn_solo_leave_without_shoppe_dat_shows_no_transcribed_record() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(2).unwrap(),
        floor: 0,
    };
    assert_eq!(state.party.len(), 1);
    state.gold = 999;
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ServiceMenu {
        inn: Inn::TheWayfarerInn,
    }));
    handle_play_key_input(&mut state, 'L', "", Path::new("")).unwrap();
    state.flush_message_slot();
    let rows = playtest_transcript_rows(&state);
    assert!(
        !rows.iter().any(|row| row.contains("inn!") || row.contains("SLEEP")),
        "{rows:?}"
    );
}
