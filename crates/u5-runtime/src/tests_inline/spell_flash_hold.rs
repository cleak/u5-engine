// `magic.md §8` (issue #231 captures, 2026-10-16): while a spell's
// whole-view flash plays, the message window holds only the cast, spell-name
// and spell rows; the result and the next actor's prompt print after it.

fn spell_flash_transcript_texts(entries: &[MessageEntry]) -> Vec<String> {
    entries.iter().map(|entry| entry.text.clone()).collect()
}

/// A combat Sanct Lor cast typed at the spell-name prompt by party slot 1,
/// with party slot 2 next to act, as in
/// `combat_input_dispatch_cast_uses_pending_actor_as_caster`.
fn sanct_lor_flash_state() -> PlayState {
    let mut state = combat_player_command_state(8, 5);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: 1,
        status: b'G',
        climb_stat: 0,
        mana: 0,
        hp: 20,
        max_hp: 20,
        level: INVISIBILITY_COST,
    });
    state.party[0].mana = INVISIBILITY_COST;
    state.party[0].level = INVISIBILITY_COST;
    state.party_experience.push(0);
    state.party_strengths.push(20);
    state.party_intelligence.push(20);
    state.party_equipment = default_party_equipment(2);
    state.spell_charges[INVISIBILITY_SPELL_INDEX] = 1;
    state.active_objects[1] = ActiveObject {
        type_byte: 0x81,
        tile: 0x81,
        x: 4,
        y: 5,
        ..ActiveObject::empty()
    };
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 0, 4, 5]);
    state.next_combat_actor_slot = 1;
    assert_eq!(
        state.start_combat_cast_spell_prompt(0, true),
        MoveOutcome::Observed
    );
    state.pending_combat_actor_slot = Some(0);
    state
}

#[test]
fn a_spell_flash_holds_the_window_on_the_cast_rows_until_it_ends() {
    let game_dir = std::path::Path::new(".");
    let mut state = sanct_lor_flash_state();
    handle_play_key_input(&mut state, 'S', "L", game_dir).unwrap();
    let before_submit = state.message_entries().len();
    handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();

    // The cast resolved: the composed result is all there, unchanged.
    assert!(state.combat_actors[0].is_phase_suppressed());
    assert!(state.message.starts_with(crate::commands::SPELL_SUCCESS_LINE));

    // But the window still shows the rows from before the flash: the
    // committed spell row is the last one, and no `Success!` yet.
    assert!(state.message_window_flash_hold_active());
    assert!(state.staged_narration_active());
    let shown = spell_flash_transcript_texts(state.displayed_message_entries());
    assert_eq!(shown.len(), before_submit + 1, "only the spell row: {shown:?}");
    assert_eq!(shown.last().map(|row| row.trim_end()), Some(":SANCT LOR"));
    assert!(!shown.iter().any(|row| row.contains("Success!")));
    assert_eq!(state.displayed_unflushed_message(), None);
    // "A frame taken mid-flash shows the three rows alone": no command row,
    // and no cursor while the flash reads no key.
    assert!(state.message_window_live_row_suppressed());
    assert!(state.message_window_cursor_suppressed());

    // A key typed during the flash waits for it rather than acting.
    handle_play_key_input(&mut state, 'A', "", game_dir).unwrap();
    assert_eq!(state.keys_buffered_during_hold.len(), 1);

    // The hold is the flash's own length, then `Success!` follows it.
    let flash_secs = SoundEffect::SharedVariant { variant: 7 }
        .program(&mut crate::audio::RumbleJitter::default())
        .duration(true)
        .as_secs_f64();
    assert!(flash_secs > 0.0);
    assert!(state.advance_staged_narration_to(100.0));
    assert!(state.message_window_flash_hold_active());
    assert!(!state.advance_staged_narration_to(100.0 + flash_secs + 0.01));
    assert!(!state.message_window_flash_hold_active());
    let shown = spell_flash_transcript_texts(state.displayed_message_entries());
    assert_eq!(
        shown,
        spell_flash_transcript_texts(state.message_entries()),
        "the window is live again"
    );
    state.flush_message_slot();
    let live = spell_flash_transcript_texts(state.message_entries());
    let spell_row = live
        .iter()
        .rposition(|row| row.trim_end() == ":SANCT LOR")
        .expect("spell row");
    assert_eq!(
        live.get(spell_row + 1).map(String::as_str),
        Some(crate::commands::SPELL_SUCCESS_LINE)
    );
}

#[test]
fn flushing_the_staged_narration_releases_the_spell_flash_hold() {
    let game_dir = std::path::Path::new(".");
    let mut state = sanct_lor_flash_state();
    handle_play_key_input(&mut state, 'S', "L", game_dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();
    assert!(state.message_window_flash_hold_active());
    state.flush_staged_narration();
    assert!(!state.message_window_flash_hold_active());
    assert!(!state.staged_narration_active());
}

#[test]
fn every_spell_flash_holds_the_window_but_a_template_spell_has_none() {
    // Out of combat too: An Nox's shared variant is its flash, and the
    // `Success!` it prints follows it (`magic.md §5.1`, `audio.md §6.1`).
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].mana = 1;
    state.party[0].level = 1;
    state.party[0].status = b'P';
    state.spell_charges[spell_index_from_code("AN").unwrap()] = 1;
    state.cast_spell_from_suffix("1AN1", std::path::Path::new("")).unwrap();
    assert_eq!(state.message, crate::commands::SPELL_SUCCESS_LINE);
    assert!(state.message_window_flash_hold_active());
    state.flush_staged_narration();

    // The same shared variant played outside a cast - a scroll, a potion -
    // is not a spell flash and holds nothing here.
    let mut state = test_state(open_grid(), 1, 1);
    state.emit_scroll_shared_variant(3);
    assert!(!state.message_window_flash_hold_active());
    assert!(!state.staged_narration_active());

    // Magic Missile plays the combat effect template, not a shared variant
    // (`audio.md §6.1`), so its result is not held.
    let mut state = combat_player_command_state(6, 5);
    let spell_index = spell_index_from_code("GP").unwrap();
    state.party[0].mana = 1;
    state.party[0].level = 1;
    state.spell_charges[spell_index] = 1;
    let _ = state.cast_active_target_combat_spell(
        0,
        spell_index,
        CombatSpellDamageKind::MagicMissile,
        8,
    );
    assert!(!state.message_window_flash_hold_active());
}
