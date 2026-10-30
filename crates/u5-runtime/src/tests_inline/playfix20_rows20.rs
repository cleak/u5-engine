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

/// Playtest P20-05. `view.md §3` row 5c: an appender row returns without the
/// line-spacing cleanup, so a short final row stays open. But
/// `text-output.md §6`: "When a chunk exactly fills the row, the per-cell
/// emitter has already wrapped by the time the chunk ends", and "a
/// row-filling string still ends at column zero of the following row, so the
/// next command prompt still spends one blank row". `dungeon Covetous` is
/// sixteen cells; `dungeon Destard` is fifteen and stays open.
#[test]
fn p20_05_a_row_filling_appender_look_result_is_not_left_open() {
    let table = parse_look2_dat(&look2_bytes(&[(
        WORD_OF_POWER_SEALED_TILE as usize,
        "the collapsed entrance to the dungeon ",
    )]))
    .unwrap();
    for (x, tail, full) in [(156, "dungeon Covetous", true), (72, "dungeon Destard", false)] {
        let mut grid = open_world_grid();
        grid[world_cell_index(x, 27)] = WORD_OF_POWER_SEALED_TILE;
        let mut state = world_state(grid, x, 28);
        state.look_direction_with_table(Direction::North, Some(&table));
        state.flush_message_slot();
        let last = state.message_transcript.last().expect("a look result");
        assert!(last.text.ends_with(tail), "{x}: {:?}", p20_rows(&state));
        assert_eq!(
            last.leaves_row_open, !full,
            "{x}: only a row the emitter did not wrap stays open"
        );
        assert_eq!(state.message_row_open_mid_line, !full, "{x}");
    }
}
