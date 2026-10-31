// Regressions for the S25 exploratory play-test row findings (P25-03..P25-08).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// P25-08: `dungeon-mode.md §11`: the camp hours prompt "echoes the key
/// plus a line feed; Space (echoed as a blank) or `0` (echoed) then aborts
/// with no further output". Both cancels close the prompt row, so the next
/// prompt gets the ordinary derived blank row under it.
#[test]
fn playfix25_p25_08_space_cancel_at_camp_hours_closes_the_row_like_zero() {
    let dir = debug_game_dir();
    let mut gaps = Vec::new();
    for cancel in ['0', ' '] {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.clock = GameClock::new(12, 0).unwrap();
        handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
        handle_play_key_input(&mut state, cancel, "", &dir).unwrap();
        assert!(state.active_rest.is_none(), "{cancel:?} aborts");
        // The next command's echo, as the window composes it.
        state.push_message_entry("New Order", true);
        let rows = bare_marker_layout(&state);
        let prompt_row = rows
            .iter()
            .rposition(|(_, text)| text.starts_with("hours? (1-9)"))
            .unwrap_or_else(|| panic!("{cancel:?}: {rows:?}"));
        let echo_row = rows
            .iter()
            .rposition(|(_, text)| text.contains("New Order"))
            .unwrap_or_else(|| panic!("{cancel:?}: {rows:?}"));
        gaps.push(echo_row - prompt_row - 1);
    }
    assert_eq!(gaps, vec![1, 1], "blank rows under the prompt row for 0 and Space");
    let _ = fs::remove_dir_all(dir);
}

/// P25-05: `inventory.md §5.1` publishes the empty-handed refusal as
/// `Thou_art_empty-\nhanded!\n`, and `combat.md §11.1` has a monster's
/// `<monster> teleports!\n` print with "no leading newline of its own". In
/// the arena the refusal left its row open, so the teleport read
/// `handed!Wisp`.
#[test]
fn playfix25_p25_05_combat_empty_handed_refusal_closes_its_row() {
    let mut state = combat2b_two_member_state();
    state.equipment_stock = [0; EQUIPMENT_COUNT];
    state.party_equipment = default_party_equipment(state.party.len());

    assert_eq!(state.start_combat_ready_equipment(0), MoveOutcome::Blocked);
    assert!(
        !state.message_row_open_mid_line,
        "the refusal ends in its own line feed"
    );
    // What `finish_combat_modal_actor_action` hands the arena before the
    // round walker runs the monsters.
    state.flush_message_slot();
    state.combat_transcript_row_open = state.message_row_open_mid_line;
    state.emit_combat_print("Wisp teleports!\n");
    let rows = p4combat_rows_text(&state);
    let handed = rows
        .iter()
        .rposition(|row| row.starts_with("handed!"))
        .unwrap_or_else(|| panic!("rows: {rows:?}"));
    assert_eq!(rows[handed], "handed!", "rows: {rows:?}");
    assert_eq!(rows[handed + 1], "Wisp teleports!", "rows: {rows:?}");
}

/// P25-04: `text-output.md §10.2` ("Held passes and where the next line
/// lands"): after an accepted step or a calm-wind silent consume, "a
/// non-sailing key ... comes back from the helper with no fresh pair; its
/// echo is written on the pending marker row ... and the prompt after that
/// command gets an ordinary pair". An unmapped key's `What?` and the
/// Buffer/Sound toggles are such echoes, laid out like the `D-What?` sibling.
#[test]
fn playfix25_p25_04_unmapped_key_after_a_hoisted_step_takes_the_marker_row() {
    let dir = Path::new("");
    let mut layouts = Vec::new();
    for key in ['D', '\u{1b}', '!'] {
        let mut state = bare_marker_frigate(WindState::Calm, 99, "Yell HOIST!");
        handle_play_key_input(&mut state, 'w', "", dir).unwrap();
        assert!(overworld_prompt_pair_is_suppressed(&state), "{key:?}");
        handle_play_key_input(&mut state, key, "", dir).unwrap();
        assert!(
            !overworld_prompt_pair_is_suppressed(&state),
            "{key:?}: the prompt after the refusal gets an ordinary pair"
        );
        let rows = bare_marker_layout(&state);
        let yell = rows
            .iter()
            .rposition(|row| row.1 == "Yell HOIST!")
            .unwrap_or_else(|| panic!("{rows:?}"));
        layouts.push(rows[yell..].to_vec());
    }
    let expected = vec![
        (true, "Yell HOIST!".to_string()),
        (false, String::new()),
        (true, "D-What?".to_string()),
        (false, String::new()),
        (true, String::new()),
    ];
    assert_eq!(layouts[0], expected, "the D-What? control");
    for layout in &layouts[1..] {
        let mut expected = expected.clone();
        expected[2].1 = "What?".to_string();
        assert_eq!(layout, &expected);
    }

    // The typeahead toggle echoes through the same prompt.
    let mut state = bare_marker_frigate(WindState::Calm, 99, "Yell HOIST!");
    handle_play_key_input(&mut state, 'w', "", dir).unwrap();
    state.toggle_typeahead_buffer();
    assert!(!overworld_prompt_pair_is_suppressed(&state));
}
