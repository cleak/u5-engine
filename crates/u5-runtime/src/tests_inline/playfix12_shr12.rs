// Regressions for the pass-12 shrine exploratory-playtest findings
// (shr12, P12-16 through P12-19).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

fn shr12_rows(state: &PlayState) -> Vec<String> {
    state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// A Britannia party one cell east of the Honesty shrine, which is ruined.
fn shr12_ruined_honesty_state() -> PlayState {
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    let mut world = world_state(open_world_grid(), shrine_x + 1, shrine_y);
    world.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    world.grid[world_cell_index(shrine_x, shrine_y)] = WORLD_RUINED_SHRINE_TILE;
    world.shrine_ruin_flags[0] = 0x81;
    world
}

/// `karma.md §7.1`: the restoration prints the uttered-word line, then
/// `\nUpon what virtue\ndost thou\nmeditate?\n\n:`, then `\nMantra:` three
/// times, and nothing rewrites an earlier row - the window only scrolls
/// (`text-output.md §10`). The session used to write its whole cumulative
/// transcript back into the slot on every answer, which re-logged the
/// question and left stale prompt rows above the uttered-word line (P12-16).
#[test]
fn playtest_p12_16_yell_restoration_logs_each_row_once() {
    let mut world = shr12_ruined_honesty_state();
    world.emit_message_line("Yell what?\n:");
    let before = shr12_rows(&world).len();

    assert_eq!(world.yell_command(Some("fallax")), MoveOutcome::Used);
    assert!(world.active_shrine_restoration.is_some());
    world.flush_message_slot();
    let opened = shr12_rows(&world);
    assert_eq!(opened[before - 1], ":FALLAX");
    // The virtue's answer row is live, not logged.
    assert!(
        opened.ends_with(&["meditate?".to_string(), String::new()]),
        "{opened:?}"
    );
    assert_eq!(world.typed_prompt_echo().as_deref(), Some(":"));

    assert_eq!(world.step_active_shrine_restoration('\r', "honesty"), None);
    assert_eq!(world.typed_prompt_echo().as_deref(), Some("Mantra:"));
    assert_eq!(world.step_active_shrine_restoration('\r', "ahm"), None);
    assert_eq!(world.step_active_shrine_restoration('\r', "ahm"), None);
    assert_eq!(
        world.step_active_shrine_restoration('\r', "ahm"),
        Some(MoveOutcome::Used)
    );
    world.flush_message_slot();
    let rows = shr12_rows(&world);
    // Everything up to the open answer row is untouched...
    assert_eq!(rows[..opened.len()], opened[..], "{rows:?}");
    // ...and each answer, then the banner, is appended beneath it once.
    assert_eq!(
        rows[opened.len()..],
        [
            ":HONESTY",
            "Mantra:AHM",
            "Mantra:AHM",
            "Mantra:AHM",
            "",
            "The Shrine is",
            "restored!",
        ]
        .map(String::from),
        "{rows:?}"
    );
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    assert_eq!(world.grid[world_cell_index(shrine_x, shrine_y)], WORLD_SHRINE_TILE);
    assert!(!world.message_row_open_mid_line);
    assert!(world.typed_prompt_echo().is_none());
}

/// `text-output.md §10.6`: the "General typed string" reader, "used for most
/// free-text prompts", has "keys are uppercased"; `karma.md §7.1`'s four
/// reads are ordinary ones. Matching stays case-insensitive (P12-19).
#[test]
fn playtest_p12_19_yell_restoration_echoes_answers_upper_cased() {
    let mut world = shr12_ruined_honesty_state();
    assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
    assert!(world.typed_prompt_echo_uppercases());
    assert_eq!(world.step_active_shrine_restoration('h', "onesty"), None);
    assert_eq!(world.step_active_shrine_restoration('a', "hm"), None);
    let rows = shr12_rows(&world);
    assert!(
        rows.ends_with(&[":HONESTY".to_string(), "Mantra:AHM".to_string()]),
        "{rows:?}"
    );
    assert_eq!(world.step_active_shrine_restoration('a', "hm"), None);
    assert_eq!(
        world.step_active_shrine_restoration('a', "hm"),
        Some(MoveOutcome::Used)
    );
    assert_eq!(world.shrine_ruin_flags[0], 0x01);
}

/// `karma.md §7`: "Return submits without echoing a newline. A blank
/// submission therefore ends meditation on the current prompt row, without a
/// result message". The `:` row stays, open, and the next command's feed
/// closes it with its marker directly beneath - the shape a blank mantra's
/// `Mantra:` row already had. The session was dropped with its live row
/// unlogged, so the `:` vanished (P12-18).
#[test]
fn playtest_p12_18_blank_virtue_answer_keeps_its_colon_row() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    let mut session = ShrineSession::new(ShrineVirtue::Honor);
    session.phase = ShrinePhase::Virtue;
    state.active_shrine = Some(session);
    state.emit_message_line(format!("{SHRINE_VIRTUE_PROMPT}\n\n"));

    assert_eq!(
        state.step_active_shrine('\r', "", &dir).unwrap(),
        Some(MoveOutcome::PromptDeclined)
    );
    let rows = shr12_rows(&state);
    assert!(
        rows.ends_with(&[
            SHRINE_VIRTUE_PROMPT.to_string(),
            String::new(),
            ":".to_string()
        ]),
        "{rows:?}"
    );
    assert!(state.message_row_open_mid_line);
    assert!(state.surface_command_row_follows_history);
    let _ = fs::remove_dir_all(dir);
}

/// `text-output.md §10.2`: the end cap is the mode loop's; "a prompt a
/// command parks at in the middle of its own run" shows its row with the
/// cursor on it "and nothing underneath". `karma.md §7`: record `34`'s
/// trailing space belongs to the prompt; §8.2: Codex keys "produce no
/// ordinary command echo"; `input.md §3`: the shared wait draws the cursor
/// at the current text cursor cell (P12-17).
#[test]
fn playtest_p12_17_shrine_handler_waits_open_no_command_row() {
    // The offering chooser keeps the cursor on the question's own row.
    let mut offering = world_state(open_world_grid(), 5, 5);
    let mut session = ShrineSession::new(ShrineVirtue::Honor);
    session.phase = ShrinePhase::Offering;
    offering.active_shrine = Some(session);
    offering.emit_message_line("\n\nOffer how many\ngold? ");
    assert!(open_row_owns_the_cursor(&offering));

    // The Codex reader: inline after a record that ends mid-row...
    let mut codex = world_state(open_world_grid(), 5, 5);
    codex.codex_presentation_steps = 1;
    codex.emit_message_line("\nThe Codex lies before thee...");
    assert!(codex.message_window_live_row_suppressed());
    // ...and a bare cursor, no end cap, on the row a record's feed opened.
    codex.emit_message_line("\nThe book is open!\n\n");
    assert!(!codex.message_window_live_row_suppressed());
    assert!(shop_pause_row_is_continuation(&codex));
    // An ordinary command row is unaffected once the reader is done.
    codex.codex_presentation_steps = 0;
    assert!(!shop_pause_row_is_continuation(&codex));

    // The restoration's typed reads carry their own live row, a
    // continuation of the block above - no end-capped command row.
    let mut world = shr12_ruined_honesty_state();
    assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
    assert_eq!(world.typed_prompt_echo().as_deref(), Some(":"));
    world.step_active_shrine_restoration('\r', "honesty");
    assert_eq!(world.typed_prompt_echo().as_deref(), Some("Mantra:"));
}
