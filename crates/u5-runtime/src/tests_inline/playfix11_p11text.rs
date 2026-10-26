// Regressions for the pass-11 exploratory-playtest findings (p11text, S##).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// A Britannia state on a Honesty shrine marker, mid-meditation at the
/// first `Mantra:` ask.
fn p11text_shrine_at_first_mantra(dir: &Path) -> PlayState {
    fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
    let mut state = britannia_state(grid, 10, 20);
    handle_play_key_input(&mut state, 'E', "", dir).unwrap();
    state.flush_staged_narration();
    handle_play_key_input(&mut state, 'H', "ONESTY\r", dir).unwrap();
    state.flush_staged_narration();
    state.adopt_shrine_prompt_row();
    state
}

/// S18: `karma.md §7` ends the meditation on a blank mantra "on the current
/// prompt row", and `text-output.md §10.4` has the next poll's feed close
/// that row. Nothing lets the closed meditation suppress later feeds:
/// `commands.md §5.8` is `Get-North` ⏎ `[blank]` ⏎ `Nothing to get!` for
/// every later command, not just the first one.
#[test]
fn playtest_s18_blank_mantra_does_not_leave_the_row_open_for_later_commands() {
    let dir = debug_game_dir();
    let mut state = p11text_shrine_at_first_mantra(&dir);
    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
    assert!(state.active_shrine.is_none());
    state.flush_staged_narration();
    for _ in 0..2 {
        handle_play_key_input(&mut state, 'G', "", &dir).unwrap();
        handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", &dir).unwrap();
        state.flush_staged_narration();
        let tail = p4window_rows_from(&p4window_rows(&state), ">Get-North");
        assert_eq!(
            tail.get(..3).map(<[String]>::to_vec),
            Some(vec![
                ">Get-North".to_string(),
                String::new(),
                "Nothing to get!".to_string()
            ]),
            "{:?}",
            p4window_rows(&state)
        );
    }
    let _ = fs::remove_dir_all(dir);
}

/// S19: `karma.md §7`, "Both key waits precede the closing record": "the
/// closing line wrapped across the message strip, one blank row, and then
/// the prompt row: the ordinary command-boundary blank of
/// systems/text-output.md Section 10.4 is spent here like anywhere else."
/// The quest record's open-row one-shot belongs to the second key wait and
/// must not survive it onto the next command.
#[test]
fn playtest_s19_closing_record_key_spends_the_quest_rows_one_shot() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    let mut session = ShrineSession::new(ShrineVirtue::Honesty);
    session.phase = ShrinePhase::AltarQuest;
    state.active_shrine = Some(session);
    // What the `AltarAnnouncement` arm leaves for the second key wait.
    state.surface_command_row_follows_history = true;
    let _ = state.step_active_shrine(' ', "", &dir);
    assert!(state.active_shrine.is_none());
    assert!(!state.surface_command_row_follows_history);
    let _ = fs::remove_dir_all(dir);
}

/// S23: `magic.md §6` step 4 - "The `Type M to mix:` instruction line
/// carries no trailing feed of its own" - and step 3's Escape "cancels
/// before any inventory change" with no text. `text-output.md §10.4`: the
/// next poll's leading feed closes the open row "and no blank row appears:
/// the prompt marker lands on the row directly beneath the partly written
/// line" - the same shape the zero-quantity exit already has.
#[test]
fn playtest_s23_escape_at_the_reagent_list_leaves_no_blank_under_the_help() {
    let dir = debug_game_dir();
    let mut state = p4window_world_state();
    state.reagents = [0; REAGENT_COUNT];
    state.reagents[REAGENT_SULFUR_ASH] = 9;
    p4window_keys(&mut state, &dir, &['M', 'I', 'L', '\r']);
    assert!(state.active_mix.is_some(), "{:?}", state.message);
    p4window_keys(&mut state, &dir, &['\u{1b}']);
    assert!(state.active_mix.is_none());
    let rows = p4window_rows(&state);
    let help = rows
        .iter()
        .rposition(|row| row == "Type M to mix:")
        .unwrap_or_else(|| panic!("help on screen: {rows:?}"));
    assert_eq!(rows.get(help + 1).map(String::as_str), Some(">"), "{rows:?}");
    let _ = fs::remove_dir_all(dir);
}

/// S17/S21: `commands.md §5.5`, overworld Enter class table: "anything else
/// | `What?\n`, producing `Enter_What?\n`; no action", and §3: "Overworld:
/// only `Enter What?` on an unrecognised tile ... Town and dungeon: `Enter
/// what?`". Plain grass is the commonest unrecognised tile.
#[test]
fn playtest_s17_overworld_enter_on_grass_prints_capital_what() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 10, 20);
    state.begin_command_echo_for(Command::Enter);
    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    state.commit_command_echo();
    assert_eq!(state.turn, 0);
    assert_eq!(
        playtest_transcript_rows(&state).last().map(String::as_str),
        Some("Enter What?")
    );
    let _ = fs::remove_dir_all(dir);
}
