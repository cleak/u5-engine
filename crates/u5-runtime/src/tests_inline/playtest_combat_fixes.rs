// Regressions for the combat exploratory-playtest findings (E32-E42).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `combat.md §10` / `magic.md §7`: with Negate Magic's `N` tag active, `C`
/// is absorbed "before the shared spell dispatcher or spell-selection prompt
/// runs. The exact command output is `Cast...\nAbsorbed!\n`". "There is no
/// spell-selection prompt", no charge or MP is spent, and the attempt
/// "completes the current combat action". Playtest E35: the engine opened
/// `Spell name:` and absorbed only after the spell-name exchange.
#[test]
fn playtest_e35_negate_magic_absorbs_combat_cast_before_the_spell_prompt() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.active_player = Some(0);
    state.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = NEGATE_MAGIC_ACTIVE_EFFECT_DURATION;
    let spell_index = spell_index_from_code("GP").unwrap();
    state.spell_charges[spell_index] = 1;
    state.party[0].mana = 9;
    state.party[0].level = 8;
    state.ensure_pending_combat_player_turn();
    let serial = state.sound_effect_serial;

    handle_play_key_input(&mut state, 'C', "", game_dir).unwrap();

    assert!(
        state.active_cast.is_none(),
        "no spell-selection prompt may open"
    );
    let rows = combat_message_window_rows(&state);
    assert!(
        !rows.iter().any(|row| row.contains("Spell name")),
        "rows: {rows:?}"
    );
    let cast_row = rows
        .iter()
        .position(|row| row == ">Cast...")
        .unwrap_or_else(|| panic!("the Cast... echo is drawn: {rows:?}"));
    assert_eq!(rows[cast_row + 1], "Absorbed!", "rows: {rows:?}");
    assert!(
        state
            .sound_effects_after(serial)
            .contains(&SoundEffect::MagicAbsorbed)
    );
    assert_eq!(state.spell_charges[spell_index], 1);
    assert_eq!(state.party[0].mana, 9);
}

/// `combat.md §8.4`: "the shared cast dispatcher emits `Not here!\n`, `None
/// mixed!\n` ... Combat supplies the following banner's leading feed", and
/// `magic.md §7`: the result literals carry trailing feeds and "Combat's
/// subsequent actor banner has its own leading feed, which leaves a blank
/// row after a completed result". Playtest E40: the cast result ran straight
/// into the next banner.
#[test]
fn playtest_e40_a_combat_cast_result_is_followed_by_a_blank_row() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.active_player = Some(0);
    state.party[0].mana = 9;
    state.party[0].level = 8;
    state.ensure_pending_combat_player_turn();

    for key in ['C', 'I', 'L', '\r'] {
        handle_play_key_input(&mut state, key, "", game_dir).unwrap();
    }

    let rows = combat_message_window_rows(&state);
    let result = rows
        .iter()
        .rposition(|row| row == "Not here!")
        .unwrap_or_else(|| panic!("the Not here! result is drawn: {rows:?}"));
    assert_eq!(rows[result + 1], "", "rows: {rows:?}");
    assert_eq!(rows[result + 2], "Avatar, armed", "rows: {rows:?}");
}
