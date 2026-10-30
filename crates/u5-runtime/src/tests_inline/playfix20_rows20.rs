// Regressions for the pass-20 exploratory-playtest message-window row
// findings (rows20, P20-xx).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// The transcript rows the message window draws, blank rows included.
fn p20_rows(state: &PlayState) -> Vec<String> {
    let log = message_log_from_entries(state.message_entries(), |text| Some(text.to_string()));
    log.lines().iter().map(|line| line.text.clone()).collect()
}

/// The rows from the last row starting with `echo` to the end.
fn p20_rows_from_last(state: &PlayState, echo: &str) -> Vec<String> {
    let rows = p20_rows(state);
    let start = rows
        .iter()
        .rposition(|row| row.starts_with(echo))
        .unwrap_or_else(|| panic!("no {echo:?} row: {rows:?}"));
    rows[start..].to_vec()
}

/// Playtest P20-03. `commands.md §5.8` gives `Get-North` / blank /
/// `Nothing to get!`; the overworld underfoot notice is stored as
/// `Burning!\n` (one trailing feed) and swamp poisoning prints
/// `Poisoned!\n`, so the notice only moves the next prompt's blank under
/// itself - nothing removes the blank inside the following command. The
/// lava and swamp arms left the row marked open, and the Get result's
/// leading feed was spent closing it.
#[test]
fn p20_03_underfoot_notice_keeps_the_next_commands_inner_blank_row() {
    let dir = debug_game_dir();
    let get_north_rows = |tile: u8, notice: Option<&str>| {
        let mut grid = open_world_grid();
        grid[world_cell_index(5, 5)] = tile;
        let mut state = world_state(grid, 5, 5);
        state.party.truncate(1);
        state.party[0].hp = 200;
        state.party[0].max_hp = 200;
        state.party[0].climb_stat = 0;
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
        if let Some(notice) = notice {
            let rows = p20_rows(&state);
            assert!(rows.iter().any(|row| row == notice), "{rows:?}");
        }
        handle_play_key_input(&mut state, 'G', "", &dir).unwrap();
        handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", &dir).unwrap();
        let rows = p20_rows_from_last(&state, "Get-North");
        rows[..3.min(rows.len())].to_vec()
    };
    let expected = vec![
        "Get-North".to_string(),
        String::new(),
        "Nothing to get!".to_string(),
    ];
    assert_eq!(get_north_rows(0x05, None), expected, "grass control");
    assert_eq!(
        get_north_rows(0x8f, Some(WORLD_BURNING_LINE)),
        expected,
        "after Burning!"
    );
    assert_eq!(
        get_north_rows(BRIT_SWAMP_TILE, Some("Poisoned!")),
        expected,
        "after Poisoned!"
    );
    let _ = fs::remove_dir_all(dir);
}
