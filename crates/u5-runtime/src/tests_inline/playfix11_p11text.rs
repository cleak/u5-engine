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
