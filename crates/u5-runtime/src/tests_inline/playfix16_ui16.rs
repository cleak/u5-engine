// Regressions for the message-window exploratory-playtest findings (ui16,
// P16-xx).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `view.md §3`: "A tile that matched an appender row returns as soon as its
/// suffix is printed and never reaches the cleanup ... So a clock tile emits
/// its description and time and nothing further." The cleanup is the feed
/// that closes the look row (`text-output.md §10.4`), so an appender result
/// leaves its row open and the next prompt marker lands directly beneath it,
/// while a plain description still closes its row (P16-03).
#[test]
fn playtest_p16_03_look_appender_rows_leave_the_look_row_open() {
    let table = parse_look2_dat(&look2_bytes(&[
        (0x40, "a target"),
        (0xfa, "a grandfather clock, showing: "),
        (ETERNAL_FLAME_LOOK_TILE as usize, "the Flame of "),
    ]))
    .unwrap();

    let last_entry = |state: &PlayState| {
        let last = state
            .message_transcript
            .last()
            .expect("the look result is in the transcript");
        (last.text.clone(), last.leaves_row_open)
    };

    for (tile, scene, tail) in [
        (0xfa, None, "2:05 PM"),
        (ETERNAL_FLAME_LOOK_TILE, Some(SCENE_THE_LYCAEUM), "Truth"),
    ] {
        let mut grid = open_grid();
        grid[4 * 32 + 5] = tile;
        let mut state = test_state(grid, 5, 5);
        if let Some(scene) = scene {
            state.area = Area::Town {
                scene: Scene::new(scene).unwrap(),
                floor: 0,
            };
        }
        state.clock = GameClock::new(14, 5).unwrap();
        state.look_direction_with_table(Direction::North, Some(&table));
        state.flush_message_slot();
        let (text, open) = last_entry(&state);
        assert!(text.ends_with(tail), "{tile:#04x}: {text:?}");
        assert!(open, "{tile:#04x}: an appender row skips the cleanup feed");
        assert!(state.message_row_open_mid_line);
    }

    let mut grid = open_grid();
    grid[4 * 32 + 5] = 0x40;
    let mut state = test_state(grid, 5, 5);
    state.look_direction_with_table(Direction::North, Some(&table));
    state.flush_message_slot();
    let (text, open) = last_entry(&state);
    assert_eq!(text, "a target");
    assert!(!open, "a plain description still runs the cleanup");
}

/// `commands.md §5.8`: "An ineligible pick prints `Disabled!\n\n` and repeats
/// the prompt"; `inventory.md §4.3` has R-Ready use the same member-selection
/// surface with its own `Player:_` question. After the refusal the window has
/// to show `Player: ` again, not a command marker while the selector is still
/// waiting (P16-10).
#[test]
fn playtest_p16_10_ready_repeats_player_prompt_after_disabled() {
    let dir = debug_game_dir();
    let mut state = p3magic_three_member_world_state();
    state.party[1].status = b'D';

    p4window_keys(&mut state, &dir, &['R', '2', '\r']);

    let ready = state
        .active_ready
        .as_ref()
        .expect("the selector still waits");
    assert!(ready.selected_party_index.is_none());
    let rows = p3magic_transcript_rows(&state);
    let disabled = rows
        .iter()
        .rposition(|row| row == DUNGEON_ACTING_MEMBER_DISABLED.trim_end())
        .unwrap_or_else(|| panic!("`Disabled!` prints: {rows:?}"));
    assert_eq!(rows[disabled + 1], "", "{rows:?}");
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert_eq!(
        state.open_prompt_line().as_deref(),
        Some(PARTY_SELECTION_PROMPT),
        "the repeated `Player: ` is the prompt the window shows"
    );
    let window = p4window_rows(&state);
    assert_eq!(
        window
            .iter()
            .rev()
            .find(|row| !row.is_empty())
            .map(String::as_str),
        Some(PARTY_SELECTION_PROMPT.trim_end()),
        "{window:?}"
    );

    // An eligible pick then completes the repeated row with the name.
    p4window_keys(&mut state, &dir, &['1', '\r']);
    let rows = p3magic_transcript_rows(&state);
    assert!(
        rows[disabled + 1..]
            .iter()
            .any(|row| row.starts_with(PARTY_SELECTION_PROMPT)
                && row.len() > PARTY_SELECTION_PROMPT.len()),
        "{rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}
