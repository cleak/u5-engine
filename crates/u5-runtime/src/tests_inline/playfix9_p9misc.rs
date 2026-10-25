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
