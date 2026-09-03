// The arena's message-window rows, row for row.
//
// `combat.md` Section 4.1 publishes the entry transcript, Section 8.1 the turn
// banner and the line feed the turn handler emits before the command byte is
// read, Section 8.2 the `Attack-` / `Aim! ` prompt, and Section 11.1 the order
// every narrated step prints in - each one opening with a newline.
// `text-output.md` Section 10.2 owns the end-cap marker, Section 10.3 the echo
// literals, Section 10.4 what a leading line feed costs, and Section 10.6 the
// live input line and its inline cursor.
//
// Measured against a side-by-side capture of the original and the engine
// driven through the same twenty keystrokes (`A`/direction/`Enter`/`Space`
// into three bats east of Britain).

/// The thirteen rows the gameplay message window draws, top to bottom, with
/// the end-cap marker written as `>` and a blank row as an empty string.
///
/// This composes the log and the layout exactly as the Bevy shell does,
/// including its "drop empty output lines" filter, so a blank row that
/// survives here is one a producer asked for.
fn combat_message_window_rows(state: &PlayState) -> Vec<String> {
    let log = message_log_from_entries(state.message_entries(), |text| {
        (!text.trim().is_empty()).then(|| text.to_string())
    });
    let layout = layout_message_window_with_prompt(
        &log,
        Some(""),
        state.open_prompt_line().as_deref(),
        combat_prompt_row_follows_history(state),
    );
    let mut rows = vec![String::new(); MESSAGE_WINDOW_ROWS];
    for row in &layout.rows {
        rows[usize::from(row.row - MESSAGE_WINDOW_TOP)] =
            format!("{}{}", if row.prefixed { ">" } else { "" }, row.text);
    }
    rows
}

/// A combat state parked exactly where terrain setup leaves it: the conflict
/// banner printed and the cursor at column 0 of the row below it.
///
/// `combat.md §4.1`: the banner "fills the message window edge to edge ...
/// The row is full when the line feed is reached inside the same source
/// string, so the printer's full-row suppression consumes it. The cursor is
/// left at column 0 of the following row and **no blank row** appears under
/// the banner."
fn combat_state_after_the_conflict_banner(monster_x: u8, monster_y: u8) -> PlayState {
    let mut state = combat_player_command_state(monster_x, monster_y);
    state.message_transcript.clear();
    state.message.clear();
    state.message_flushed.clear();
    state.pending_combat_actor_slot = None;
    state.emit_centered_message_line(combat_banner_line());
    state.combat_transcript_row_open = false;
    state
}

#[test]
fn opening_a_turn_after_the_conflict_banner_prints_the_banner_once() {
    // The regression this pins: the turn banner used to be appended to the
    // message slot the conflict banner had already flushed, so the safety-net
    // flush recorded the whole slot again and `*** CONFLICT ***` was drawn on
    // two consecutive rows. `combat.md §4.1`'s entry transcript has exactly
    // one such row.
    let mut state = combat_state_after_the_conflict_banner(6, 5);

    state.ensure_pending_combat_player_turn();

    let rows = combat_message_window_rows(&state);
    assert_eq!(
        rows.iter()
            .filter(|row| row.as_str() == combat_banner_line())
            .count(),
        1,
        "rows: {rows:?}"
    );
    // §8.1's leading newline lands on the row the full-width banner left the
    // cursor on, so exactly one blank row separates the two.
    assert_eq!(
        rows[8..],
        [
            combat_banner_line(),
            String::new(),
            "Avatar, armed".to_string(),
            "with bare hands:".to_string(),
            ">".to_string(),
        ]
    );
}

#[test]
fn the_marker_row_follows_the_turn_banner_with_no_blank_between_them() {
    // `combat.md §8.1`: the turn handler "emits the line feed itself,
    // unconditionally, between printing the banner and reading the command
    // byte". That line feed is the one `text-output.md §10.2` puts before the
    // end-cap, so the marker row sits directly under the banner's last row
    // and §10.4's blank row is not spent a second time.
    let mut state = combat_state_after_the_conflict_banner(6, 5);
    state.ensure_pending_combat_player_turn();

    let rows = combat_message_window_rows(&state);
    assert_eq!(rows[usize::from(MESSAGE_WINDOW_BOTTOM - MESSAGE_WINDOW_TOP)], ">");
    assert_eq!(
        rows[usize::from(MESSAGE_WINDOW_BOTTOM - MESSAGE_WINDOW_TOP) - 1],
        "with bare hands:"
    );
}

#[test]
fn a_pass_turn_draws_the_rows_the_original_draws() {
    // The original's twenty-key capture settles into one shape: the echo on
    // the marker row, then one blank row above every line the round prints,
    // then the next actor's banner, then the fresh marker row.
    //
    // `text-output.md §10.3`: the echo literal is "`Pass` + newline |
    // complete" - no full stop - and it carries the end cap because
    // §10.2 draws that before the key is read. §10.4: the newline the next
    // print opens with "advances the row *and* returns the column to the
    // window's left edge", so it costs a blank row whenever the cursor was
    // already at column 0 - which the completed echo left it at.
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(6, 5);
    state.ensure_pending_combat_player_turn();

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    // `Avatar is poisoned!` is nineteen characters, so the sixteen-column
    // window wraps it onto two rows (`text-output.md` 4 and 6).
    assert_eq!(combat_message_window_rows(&state), [
        String::new(),
        combat_banner_line(),
        String::new(),
        "Avatar, armed".to_string(),
        "with bare hands:".to_string(),
        ">Pass".to_string(),
        String::new(),
        "Avatar is".to_string(),
        "poisoned!".to_string(),
        String::new(),
        "Avatar, armed".to_string(),
        "with bare hands:".to_string(),
        ">".to_string(),
    ]);
}

#[test]
fn the_pass_echo_is_a_marker_row_not_an_output_line() {
    // `text-output.md §10.2`: "echoed command lines carry it and pure output
    // lines do not". Combat's `Space` is an echo like the world loops', so it
    // is drawn with the end cap and the bare literal of §10.3.
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(6, 5);
    state.ensure_pending_combat_player_turn();

    handle_play_key_input(&mut state, ' ', "", game_dir).unwrap();

    let echo = state
        .message_entries()
        .iter()
        .find(|entry| entry.is_command_echo)
        .expect("the Pass keystroke echoed on the marker row");
    assert_eq!(echo.text, "Pass");
    assert!(echo.continues_open_row);
    assert!(!state.message_entries().iter().any(|entry| entry.text == "Pass."));
}

#[test]
fn the_aim_prompt_keeps_the_marker_row_and_carries_the_cursor_inline() {
    // `combat.md §8.2`: each attempt "prints `Attack-`" and, "immediately
    // before the cursor opens", `Aim! ` - a literal with a trailing space and
    // no newline. `text-output.md §10.6`: a prompt that is waiting for a key
    // keeps its own line, "so the visible layout is a log whose final line is
    // being edited".
    //
    // Measured on the original: the prompt row inks cells 0..11 and 13 of the
    // sixteen-cell window, i.e. the end cap in absolute column 24, the twelve
    // characters of `Attack-Aim! ` from column 25, and the input cursor in
    // column 37.
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(6, 5);
    state.ensure_pending_combat_player_turn();

    handle_play_key_input(&mut state, 'A', "", game_dir).unwrap();

    assert!(state.active_combat_targeting.is_some());
    assert_eq!(
        state.open_prompt_line().as_deref(),
        Some(concat!("Attack-", "Aim! "))
    );

    let log = message_log_from_entries(state.message_entries(), |text| {
        (!text.trim().is_empty()).then(|| text.to_string())
    });
    let layout = layout_message_window_with_prompt(
        &log,
        Some(""),
        state.open_prompt_line().as_deref(),
        combat_prompt_row_follows_history(&state),
    );
    let prompt = layout.rows.last().expect("the prompt row is drawn");
    assert_eq!(prompt.text, "Attack-Aim!");
    assert!(prompt.prefixed);
    assert_eq!(prompt.column, MESSAGE_WINDOW_LEFT + 1);
    assert_eq!(prompt.row, MESSAGE_WINDOW_BOTTOM);
    assert_eq!(layout.inline_cursor, Some((MESSAGE_WINDOW_LEFT + 13, MESSAGE_WINDOW_BOTTOM)));
}

#[test]
fn a_free_re_prompt_after_a_refusal_keeps_the_blank_row_the_banner_paid_for() {
    // `combat.md §8.1` buys the marker row with the banner's own line feed -
    // the turn handler "emits the line feed itself, unconditionally, between
    // printing the banner and reading the command byte" - and that is the
    // only prompt row that comes free. The same section says a free
    // re-prompt after a refusal "uses the short form and does **not**
    // reprint the banner", so nothing spends a line feed for it and
    // `text-output.md §10.4`'s derived blank row stands above it, exactly as
    // it does for a world-loop prompt.
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.active_player = Some(0);
    state.ensure_pending_combat_player_turn();

    assert!(
        combat_prompt_row_follows_history(&state),
        "the banner's line feed opened this marker row"
    );
    let banner_rows = combat_message_window_rows(&state);
    let banner_prompt = banner_rows
        .iter()
        .rposition(|row| row.starts_with('>'))
        .expect("the marker row is drawn");
    assert!(
        !banner_rows[banner_prompt - 1].is_empty(),
        "no blank row under the banner: {banner_rows:?}"
    );

    // A blocked step is one of `combat.md §8.1`'s refusals: it prints its
    // line, costs no turn and hands the same actor its prompt back.
    state.combat_terrain[4][5] = 0x0c;
    assert_eq!(
        handle_play_key_input(&mut state, 'w', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.pending_combat_actor_slot, Some(0));

    assert!(
        !combat_prompt_row_follows_history(&state),
        "the re-prompt reprinted no banner, so it spent no line feed"
    );
    let rows = combat_message_window_rows(&state);
    let prompt = rows
        .iter()
        .rposition(|row| row.starts_with('>'))
        .expect("the marker row is drawn");
    assert!(
        rows[prompt - 1].is_empty(),
        "the re-prompt keeps §10.4's blank row: {rows:?}"
    );
}
