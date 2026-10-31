// Exploratory playtest S24 (ships): u5-spec-head fixes for P24-04..P24-16.

/// A hoisted north-facing frigate on open deep water at (100, 100).
fn ship24_hoisted_frigate(wind: WindState, hull: u8) -> PlayState {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 100, 100);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull,
        skiffs: 1,
    };
    state.sync_player_object();
    state.wind = wind;
    assert_eq!(state.overworld_frigate_facing(), Some(Direction::North));
    state
}

/// P24-04: `text-output.md §10.2` makes `BREAKING UP!` / `COLLISION!` a
/// closed line on the marker row, and `commands.md §5.8` frames every later
/// Get as `Get-North` / blank / `Nothing to get!`. The collision left the
/// row flagged open for the rest of the session.
#[test]
fn playtest_s24_p24_04_a_sailing_collision_does_not_reframe_later_results() {
    let dir = debug_game_dir();
    let mut state = ship24_hoisted_frigate(WindState::East, 99);
    state.grid[world_cell_index(100, 99)] = 0x03;
    handle_play_key_input(&mut state, 'w', "", &dir).unwrap();
    assert!(
        state
            .message_entries()
            .iter()
            .any(|entry| entry.text == "BREAKING UP!"),
        "{:?}",
        state.message_entries()
    );
    assert!(!state.message_row_open_mid_line);

    let tail = p13text_get_north_rows(&mut state, &dir);
    assert_eq!(&tail[..3], &[">Get-North", "", "Nothing to get!"], "{tail:?}");
    let _ = fs::remove_dir_all(dir);
}
