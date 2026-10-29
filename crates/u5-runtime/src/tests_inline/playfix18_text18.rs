// Regressions for the pass-18 exploratory-playtest findings (text18, P18-xx).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// Two `Get-North` commands, each checked for `commands.md §5.8`'s
/// `Get-North` ⏎ `[blank]` ⏎ `Nothing to get!` rows.
fn p18_assert_get_north_keeps_its_blank_row(state: &mut PlayState, dir: &Path) {
    for _ in 0..2 {
        handle_play_key_input(state, 'G', "", dir).unwrap();
        handle_play_key_input(state, char::from(INPUT_CODE_NORTH), "", dir).unwrap();
        state.flush_staged_narration();
        let tail = p4window_rows_from(&p4window_rows(state), ">Get-North");
        assert_eq!(
            tail.get(..3).map(<[String]>::to_vec),
            Some(vec![
                ">Get-North".to_string(),
                String::new(),
                "Nothing to get!".to_string()
            ]),
            "{:?}",
            p4window_rows(state)
        );
    }
}

/// P18-03: `save-load.md §5.2` prints `Yes`, `Saving...` and `Done.` and
/// returns to the mode loop; nothing there changes later commands' spacing,
/// so every later `Get-North` keeps the blank row `commands.md §5.8` shows.
#[test]
fn playtest_p18_03_save_does_not_leave_the_row_open_for_later_commands() {
    let dir = debug_game_dir();
    let mut template = saved_game_seed_bytes(0, 0, 5, 5);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    fs::write(dir.join("SAVED.GAM"), template).unwrap();
    write_empty_ool_mirrors(&dir);
    let mut state = britannia_state(open_world_grid(), 5, 5);
    handle_play_key_input(&mut state, 'Q', "", &dir).unwrap();
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    assert_eq!(state.message, SAVE_DONE_MESSAGE);
    assert!(dir.join("SAVED.OOL").exists());
    p18_assert_get_north_keeps_its_blank_row(&mut state, &dir);
    let _ = fs::remove_dir_all(dir);
}

/// P18-03, troll variant: `overworld.md §7.1` step 3 echoes the accepted
/// letter "and a line feed", and step 5's paid toll prints nothing more, so
/// the answered question's row is closed and later results keep their blank.
#[test]
fn playtest_p18_03_paid_troll_toll_does_not_leave_the_row_open() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.gold = 100;
    state.emit_message_line(format!(
        "{TROLL_BRIDGE_CAUGHT_HEADING}54{TROLL_BRIDGE_TOLL_SUFFIX}{TROLL_BRIDGE_TOLL_QUESTION}"
    ));
    assert!(state.message_row_open_mid_line);
    state.resolve_troll_bridge_toll_payment(54, &dir).unwrap();
    assert_eq!(state.gold, 46);
    p18_assert_get_north_keeps_its_blank_row(&mut state, &dir);
    let _ = fs::remove_dir_all(dir);
}
