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
