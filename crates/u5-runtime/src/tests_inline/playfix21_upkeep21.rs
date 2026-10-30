// Regressions for the pass-21 exploratory playtest findings (upkeep21).
//
// All of them concern where the per-turn epilogue - the clock, the party
// status/provision pass with its `Starving!`, Rough seas, the walkers - runs
// relative to a modal wait, a scene or floor change, New Order and a party
// wipe. Each test names the published sentence it pins and the playtest id
// whose reproduction it captures.

fn p21_rows(state: &PlayState) -> Vec<String> {
    let mut rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    if !state.message.is_empty() && state.message != state.message_flushed {
        rows.extend(state.message.lines().map(str::to_string));
    }
    rows
}

fn p21_row_index(rows: &[String], text: &str) -> usize {
    rows.iter()
        .position(|row| row == text)
        .unwrap_or_else(|| panic!("no {text:?} row in {rows:?}"))
}

fn p21_starving(state: &mut PlayState, hour: u8, minute: u8) {
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.food = 0;
    state.clock = GameClock::new(hour, minute).unwrap();
    state.cleanup_previous_hour = hour;
}

/// Playtest P21-12 / P21-03, gem View outdoors. `view.md §4`: the local view
/// overlay "is modal ... It waits for a keypress", inside the `V` command,
/// and `overworld.md §5` step 6 runs the per-turn block only after the
/// command returns. The clock, the hourly starvation and its `Starving!`
/// used to run while the map still waited for its key.
#[test]
fn playtest_p21_12_gem_view_turn_waits_for_the_dismissing_key() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 5, 5);
    p21_starving(&mut state, 12, 59);
    state.gems = 1;

    handle_play_key_input(&mut state, 'V', "", &dir).unwrap();
    assert!(state.active_view_overlay.is_some(), "the map waits");
    assert_eq!(state.turn, 0, "no epilogue under the open overlay");
    assert_eq!(state.clock, GameClock::new(12, 59).unwrap());
    assert_eq!(state.party[0].hp, 60);
    assert!(!p21_rows(&state).iter().any(|row| row == STARVATION_WARNING));

    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert!(state.active_view_overlay.is_none());
    assert_eq!(state.turn, 1, "the View's turn is spent on dismissal");
    assert_eq!(state.clock, GameClock::new(13, 1).unwrap());
    assert!(state.party[0].hp < 60);
    assert!(state.pending_hourly_status_message.is_none());
    let rows = p21_rows(&state);
    assert!(p21_row_index(&rows, "View a gem!") < p21_row_index(&rows, STARVATION_WARNING));
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P21-12, Peer outdoors. `view.md §4`: the Peer overlay is the
/// same modal local view, so the cast's turn waits for its key too.
#[test]
fn playtest_p21_12_peer_turn_waits_for_the_dismissing_key() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 5, 5);
    p21_starving(&mut state, 12, 59);
    state.spell_charges[PEER_SPELL_INDEX] = 1;
    state.party[0].mana = PEER_COST + 1;
    state.party[0].level = PEER_COST;

    handle_play_key_input(&mut state, 'C', "1QWI", &dir).unwrap();
    assert!(state.active_view_overlay.is_some());
    assert_eq!(state.turn, 0);
    assert_eq!(state.party[0].hp, 60);

    // The cast's whole-view flash is a hold that owns the keyboard.
    state.flush_staged_narration();
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert!(state.active_view_overlay.is_none());
    assert_eq!(state.turn, 1);
    assert!(state.party[0].hp < 60);
    assert!(p21_rows(&state).iter().any(|row| row == STARVATION_WARNING));
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P21-03, the telescope's night sky in town. `view.md §4.2.2`: the
/// sky renderer prints `the night sky! ` and "busy-wait[s] until any key is
/// available" inside the Look handler; the town epilogue runs "When the
/// dispatcher returns" (`town-mode.md §7` step 4). Once dismissed, the
/// feed-terminated `Starving!` closes the sky's open row.
#[test]
fn playtest_p21_03_night_sky_turn_waits_for_the_dismissing_key() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = TELESCOPE_LOOK_TRIGGER_TILE;
    let mut state = test_state(grid, 1, 1);
    p21_starving(&mut state, 21, 59);

    handle_play_key_input(&mut state, 'L', "", &dir).unwrap();
    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();
    assert!(state.active_view_overlay.is_some(), "the sky waits");
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::new(21, 59).unwrap());
    assert_eq!(state.party[0].hp, 60);

    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert!(state.active_view_overlay.is_none());
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(22, 0).unwrap());
    assert!(state.party[0].hp < 60);
    let rows = p21_rows(&state);
    assert!(p21_row_index(&rows, "the night sky! ") < p21_row_index(&rows, STARVATION_WARNING));
    assert!(
        !state.message_row_open_mid_line,
        "the warning's own feed closed the sky's row"
    );
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P21-07. `karma.md §7.1`: the ruined-shrine restoration "always
/// asks four questions" inline inside the Yell command, which reports
/// acted once they are answered. The outdoor epilogue used to run as the
/// virtue question opened, so `Starving!` split it from its `:` row.
#[test]
fn playtest_p21_07_restoration_turn_waits_for_the_last_answer() {
    let dir = debug_game_dir();
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    let mut grid = open_world_grid();
    grid[world_cell_index(shrine_x, shrine_y)] = WORLD_RUINED_SHRINE_TILE;
    let mut state = britannia_state(grid, shrine_x, shrine_y - 1);
    state.shrine_ruin_flags[0] = 0x80;
    p21_starving(&mut state, 12, 59);

    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    handle_play_key_input(&mut state, 'F', "ALLAX", &dir).unwrap();
    assert!(state.active_shrine_restoration.is_some());
    assert_eq!(state.turn, 0, "no epilogue under the open question");
    assert_eq!(state.party[0].hp, 60);
    assert!(!p21_rows(&state).iter().any(|row| row == STARVATION_WARNING));

    for (key, rest) in [('H', "ONESTY"), ('A', "HM"), ('A', "HM")] {
        handle_play_key_input(&mut state, key, rest, &dir).unwrap();
        assert_eq!(state.turn, 0);
    }
    handle_play_key_input(&mut state, 'A', "HM", &dir).unwrap();
    assert!(state.active_shrine_restoration.is_none());
    assert_eq!(state.turn, 1, "the Yell's turn is spent once answered");
    assert!(state.party[0].hp < 60);
    let rows = p21_rows(&state);
    assert!(p21_row_index(&rows, "restored!") < p21_row_index(&rows, STARVATION_WARNING));
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P21-02. `town-mode.md §7` step 4, the capability recheck in
/// front of the epilogue: "If nobody can act and nobody is asleep, the
/// **whole** epilogue is skipped: no minute". The daylight telescope's sun
/// applies its point "with the ordinary ... death handling" (`view.md
/// §4.2.1`), so a Look that kills the last member spends no minute.
#[test]
fn playtest_p21_02_town_look_that_wipes_the_party_spends_no_minute() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = TELESCOPE_LOOK_TRIGGER_TILE;
    let mut state = test_state(grid, 1, 1);
    state.clock = GameClock::new(12, 0).unwrap();
    state.party[0].hp = 1;

    handle_play_key_input(&mut state, 'L', "", &dir).unwrap();
    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();
    assert_eq!(state.party[0].status, b'D');
    assert_eq!(state.turn, 0, "the whole epilogue is skipped");
    assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P21-08. `commands.md §6`: every New Order outcome "reports acted
/// and spends the ordinary turn ... In town that turn is one minute plus the
/// upkeep pass; on the overworld it is the ordinary outdoor turn". The
/// leader refusal charged the clock but left the pass and its `Starving!`
/// for the next command.
#[test]
fn playtest_p21_08_new_order_runs_its_turns_upkeep_pass() {
    for outdoors in [false, true] {
        let dir = debug_game_dir();
        let mut state = if outdoors {
            britannia_state(open_world_grid(), 5, 5)
        } else {
            test_state(open_grid(), 5, 5)
        };
        p21_starving(&mut state, 12, 59);

        handle_play_key_input(&mut state, 'N', "", &dir).unwrap();
        assert!(state.active_party_selector.is_some());
        handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
        assert!(state.active_party_selector.is_none());
        assert_eq!(state.turn, 1, "outdoors={outdoors}");
        assert!(state.party[0].hp < 60, "outdoors={outdoors}");
        assert!(!state.pending_town_status_provision_pass);
        assert!(state.pending_hourly_status_message.is_none());
        assert!(
            p21_rows(&state).iter().any(|row| row == STARVATION_WARNING),
            "outdoors={outdoors}: {:?}",
            p21_rows(&state)
        );
        let _ = fs::remove_dir_all(dir);
    }

    // `time.md §5`: the poison point is taken on every invocation, so the
    // New Order turn ticks once and the next turn does not tick twice.
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party[0].hp = 60;
    state.party[0].status = b'P';
    state.clock = GameClock::new(10, 20).unwrap();
    state.cleanup_previous_hour = 10;
    handle_play_key_input(&mut state, 'N', "", &dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
    assert_eq!(state.party[0].hp, 59);
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert_eq!(state.party[0].hp, 58);
    let _ = fs::remove_dir_all(dir);
}
