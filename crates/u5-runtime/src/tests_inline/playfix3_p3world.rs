// Regressions for the third exploratory playtest's world/time findings
// (W1..W6). Each test names the finding and the published sentence it
// rests on.

#[test]
fn p3w_w1_arrest_wait_to_morning_runs_no_party_status_pass() {
    // `time.md §12`: the status/provision pass "is invoked from exactly four
    // places: the overworld per-turn epilogue, the town per-turn underfoot
    // handler, the dungeon post-action handler, and the town-bed rest loop's
    // ten-minute step". The arrest loop (`time.md §10`) is a twenty-minute
    // cleanup caller and not one of them, so a poisoned party crossing the
    // 06:00 meal takes no poison points and eats nothing.
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::new(0, 30).unwrap();
    state.party[0].status = b'P';
    state.party[0].hp = 12;
    state.food = 63;
    state.toll_progress = 0;
    let ticks = state.advance_town_arrest_wait_to_morning();
    assert_eq!(ticks, 23);
    assert_eq!(state.clock.hour, 8);
    assert_eq!(state.party[0].hp, 12, "no poison point per arrest step");
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.food, 63, "the 06:00 meal is not eaten in the loop");
    assert_eq!(
        state.toll_progress, 0,
        "the turn-step counter's only increment is the upkeep pass"
    );
}

fn p3w_rows(state: &PlayState) -> Vec<String> {
    state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// A town state standing on a sidecar-authorised bed cell.
fn p3w_town_bed_state() -> (PlayState, std::path::PathBuf) {
    let dir = debug_game_dir();
    std::fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
    let mut grid = open_grid();
    grid[32 + 1] = 55;
    let mut state = test_state(grid, 1, 1);
    state.clock = GameClock::new(10, 0).unwrap();
    (state, dir)
}

#[test]
fn p3w_w2_town_bed_hours_prompt_echoes_the_accepted_digit() {
    // `town-mode.md §12`: "After the duration digit is accepted and echoed
    // with a line feed, the town-hours path runs one burst". The prompt row
    // is the `Hole up- ` echo plus `For how many hours? (1-9) `, so the
    // digit lands on that row and the sleep line takes the next one.
    let (mut state, dir) = p3w_town_bed_state();
    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    assert!(state.active_rest.is_some());
    handle_play_key_input(&mut state, '3', "", &dir).unwrap();
    let rows = p3w_rows(&state);
    let prompt_row = format!("Hole up- {REST_HOURS_PROMPT}3");
    let at = rows
        .iter()
        .position(|row| *row == prompt_row)
        .unwrap_or_else(|| panic!("the digit is echoed on the prompt row: {rows:?}"));
    assert_eq!(
        rows[at + 1],
        crate::commands::TOWN_BED_REST_SLEEP_LINE,
        "{rows:?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn p3w_w2_town_bed_hours_prompt_zero_echoes_and_aborts_without_none() {
    // `rest-and-camp.md §4`: the resident handler "accepts the same visible
    // duration shape as the town-hours path: Space and `0` cancel", and the
    // resident shape echoes the key "followed by one line feed; Space ... or
    // `0` ... then aborts with no further output" (`dungeon-mode.md §11`).
    let (mut state, dir) = p3w_town_bed_state();
    let clock = state.clock;
    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    handle_play_key_input(&mut state, '0', "", &dir).unwrap();
    assert!(state.active_rest.is_none(), "`0` cancels");
    let rows = p3w_rows(&state);
    assert_eq!(
        rows.last().cloned(),
        Some(format!("Hole up- {REST_HOURS_PROMPT}0")),
        "{rows:?}"
    );
    assert!(!rows.iter().any(|row| row.contains("None!")), "{rows:?}");
    assert!(!state.message_slot_needs_flush());
    assert_eq!(state.clock, clock, "no rest elapsed");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn p3w_w3_town_bed_rest_warns_of_starvation_once_per_crossed_hour() {
    // `rest-and-camp.md §5`: the town-bed loop "invokes the shared party
    // status/provision pass ... once per ten-minute step", and "any hour the
    // loop crosses applies the provision or starvation branch"; that branch
    // (`time.md §5`) gives "the starvation warning" every time it runs.
    let (mut state, dir) = p3w_town_bed_state();
    state.food = 0;
    assert_eq!(
        state.hole_up_command(&dir, Some(3)).unwrap(),
        MoveOutcome::Rested
    );
    assert_eq!(state.clock.hour, 13);
    let pending = state.pending_hourly_status_message.clone().unwrap_or_default();
    assert_eq!(
        pending.split('\n').filter(|row| *row == STARVATION_WARNING).count(),
        3,
        "one warning per crossed hour: {pending:?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}
