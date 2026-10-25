// Regressions for the pass-9 exploratory-playtest findings (p9misc, Q01-Q16).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

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

/// `town-mode.md §7.2`: away from the harpsichord the town digit handler
/// "falls through to the ordinary solo-member selector, which reports
/// \"acted\" for an ordinary selection and \"no action\" for the
/// deselect-to-whole-party case"; `commands.md §3` runs the per-turn
/// epilogue on "acted" (Q14).
#[test]
fn playtest_q14_town_digit_selection_reports_acted_and_zero_does_not() {
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
    assert_eq!(state.turn, 1, "an ordinary town selection reports acted");
    let mut expected = clock_before;
    expected.advance_minutes(1);
    assert_eq!(state.clock, expected);

    handle_play_key_input(&mut state, '0', "", &dir).unwrap();
    assert_eq!(state.active_player, None);
    assert_eq!(state.turn, 1, "the deselect reports no action");
    assert_eq!(state.clock, expected);
    let _ = fs::remove_dir_all(dir);
}
