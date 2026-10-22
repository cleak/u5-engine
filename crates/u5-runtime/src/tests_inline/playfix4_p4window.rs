// Regressions for the message-window exploratory play-test findings
// (H06..H45). Each test names the finding it reproduces and the published
// sentence it rests on.

/// The thirteen rows the gameplay window draws for `state`, composed the way
/// the headless `--save-screen` path and `message_window_probe` compose them:
/// the transcript, the unflushed slot, the live row and its kind. The end-cap
/// marker is written as `>` and a blank row as an empty string.
fn p4window_rows(state: &PlayState) -> Vec<String> {
    let keep = |text: &str| (!text.trim().is_empty()).then(|| text.to_string());
    let mut log = message_log_from_entries(state.displayed_message_entries(), keep);
    log.set_top_offset(usize::from(state.message_window_top_offset()));
    if let Some(text) = state.displayed_unflushed_message().and_then(keep) {
        log.push_output(&text);
    }
    let open_prompt = state.open_prompt_line();
    let spell_echo = state.typed_prompt_echo();
    let live_row = if state.message_window_live_row_suppressed() {
        None
    } else {
        Some(spell_echo.as_deref().unwrap_or(""))
    };
    let live_row_kind =
        if spell_echo.is_some() || dungeon_fountain_prompt_row_is_continuation(state) {
            LiveRowKind::Continuation
        } else {
            LiveRowKind::CommandRow
        };
    let layout = layout_message_window_with_continuation(
        &log,
        live_row,
        open_prompt.as_deref(),
        combat_prompt_row_follows_history(state),
        live_row_kind,
    );
    let mut rows = vec![String::new(); MESSAGE_WINDOW_ROWS];
    for row in &layout.rows {
        rows[usize::from(row.row - MESSAGE_WINDOW_TOP)] =
            format!("{}{}", if row.prefixed { ">" } else { "" }, row.text);
    }
    rows
}

/// The rows from the last one equal to `anchor` to the bottom of the window.
fn p4window_rows_from(rows: &[String], anchor: &str) -> Vec<String> {
    let start = rows
        .iter()
        .rposition(|row| row == anchor)
        .unwrap_or_else(|| panic!("`{anchor}` is on screen: {rows:?}"));
    let mut tail = rows[start..].to_vec();
    while tail.last().is_some_and(String::is_empty) {
        tail.pop();
    }
    tail
}

fn p4window_keys(state: &mut PlayState, dir: &Path, keys: &[char]) {
    for key in keys {
        handle_play_key_input(state, *key, "", dir).unwrap();
        // Let any staged presentation (a spell's flash hold) play out, as
        // the headless runner does between keys.
        let mut clock = 0.0;
        while state.advance_staged_narration_to(clock) && clock < 1000.0 {
            clock += 10.0;
        }
    }
}

fn p4window_world_state() -> PlayState {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_player = Some(0);
    state
}

/// H25: `commands.md §5.2` - `Get-North` ⏎ `[blank]` ⏎ `Nothing to get!` -
/// and `text-output.md §10.4`: "nothing tracks an open row", so an earlier
/// cast cannot take that blank away. `magic.md §5.1`: "submitting the name
/// advances to the next row before any result/refusal is printed".
#[test]
fn playfix4_h25_a_submitted_spell_name_leaves_no_open_row_behind() {
    let dir = debug_game_dir();
    let mut plain = p4window_world_state();
    p4window_keys(&mut plain, &dir, &['G', '\u{3}']);
    let expected = p4window_rows_from(&p4window_rows(&plain), ">Get-North");

    let mut state = p4window_world_state();
    // `I` alone matches no token: `No effect!`.
    p4window_keys(&mut state, &dir, &['C', 'I', '\r']);
    assert!(
        state.message_transcript.iter().any(|entry| entry.text == "No effect!"),
        "the cast ran: {:?}",
        p4window_rows(&state)
    );
    assert!(!state.message_row_open_mid_line, "the submitted name closed its row");
    p4window_keys(&mut state, &dir, &['G', '\u{3}']);
    let after_cast = p4window_rows_from(&p4window_rows(&state), ">Get-North");
    assert_eq!(after_cast, expected);
    assert_eq!(after_cast.get(1).map(String::as_str), Some(""), "{after_cast:?}");
    let _ = fs::remove_dir_all(dir);
}

/// H31 (a): `magic.md §5.1` - In Lor prints no result, and "submitting the
/// name advances to the next row", so `:IN LOR`, a blank row, then the
/// marker. `text-output.md §10.2`/`§10.4`: that pair is emitted once, before
/// the key read, so typing the next key must not take the blank away.
#[test]
fn playfix4_h31_the_blank_under_a_resultless_cast_survives_the_next_key() {
    let dir = debug_game_dir();
    let mut state = p4window_world_state();
    state.spell_charges[spell_index_from_code("IL").unwrap()] = 1;
    state.party[0].mana = 50;
    state.party[0].level = 8;
    p4window_keys(&mut state, &dir, &['C', 'I', 'L', '\r']);
    let idle = p4window_rows_from(&p4window_rows(&state), ":IN LOR");
    assert_eq!(idle, [":IN LOR", "", ">"], "waiting frame");

    p4window_keys(&mut state, &dir, &['R']);
    let rows = p4window_rows(&state);
    let after = p4window_rows_from(&rows, ":IN LOR");
    assert_eq!(after.get(1).map(String::as_str), Some(""), "{rows:?}");
    assert_eq!(after.get(2).map(String::as_str), Some(">Ready..."), "{rows:?}");
    let _ = fs::remove_dir_all(dir);
}

/// H31 (b): Escape at R-Ready's `Player: ` prints nothing (`inventory.md
/// §4.3`, §5.1), so the row is still open and `text-output.md §10.4`'s next
/// poll closes it: "the prompt marker lands on the row directly beneath".
/// The waiting frame and the echo typed into it must agree.
#[test]
fn playfix4_h31_an_escaped_ready_player_prompt_keeps_its_marker_beneath() {
    let dir = debug_game_dir();
    let mut state = p4window_world_state();
    state.party.push(audit_g5_companion(1));
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0"];
    state.equipment_stock[0x10] = 1;
    p4window_keys(&mut state, &dir, &['R', '\u{1b}']);
    let idle = p4window_rows_from(&p4window_rows(&state), "Player:");
    assert_eq!(idle, ["Player:", ">"], "waiting frame");

    p4window_keys(&mut state, &dir, &['S']);
    let after = p4window_rows_from(&p4window_rows(&state), "Player:");
    assert_eq!(after.get(1).map(String::as_str), Some(">Search-"), "{after:?}");
    let _ = fs::remove_dir_all(dir);
}
