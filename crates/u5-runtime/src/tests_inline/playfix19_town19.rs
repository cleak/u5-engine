// Regressions for the pass-19 exploratory playtest findings (town19).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// Playtest P19-02 (A). `time.md §5`, provision counter already zero: "The
/// party receives the starvation warning, and then each party slot ...
/// takes" its damage - one upkeep pass. `town-mode.md §7` step 4 runs that
/// pass (the underfoot handler's last act) before the post-action cleanup
/// that opens a contact, so `Starving!` follows the command's result and
/// precedes the greeting. It used to wait under the next command.
#[test]
fn playtest_p19_02_town_contact_turn_prints_starving_before_the_greeting() {
    let dir = debug_game_dir();
    let keeper = NpcSlot {
        slot: 1,
        type_byte: 0x54,
        dialog_id: 0x81,
        schedule: [4, 4, 4, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
        name: None,
    };
    let idle = NpcSlot {
        slot: 0,
        type_byte: 0,
        dialog_id: 0,
        schedule: [0; 16],
        name: None,
    };
    let mut state = test_state(open_grid(), 5, 5);
    state.load_scheduled_npcs(&[idle, keeper]);
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.food = 0;
    state.clock = GameClock::new(11, 0).unwrap();
    state.cleanup_previous_hour = 10;
    state.pending_town_status_provision_pass = true;
    state.begin_command_echo(top_down_command_echo('O').unwrap());
    state.message = "Nothing to open!".to_string();
    state.advance_npc_schedules();
    assert!(state.pending_town_npc_contact.is_some());
    state.turn = 1;

    let outcome = state
        .apply_top_down_post_turn_effects_after_turn(0, &dir)
        .unwrap();
    assert!(outcome.is_some(), "the contact takes the turn");
    assert!(state.party[0].hp < 60, "the crossing starves the party");
    assert!(
        state.pending_hourly_status_message.is_none(),
        "the warning is not held for the next command"
    );
    state.flush_message_slot();
    let rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    // The slot joins the report to the command's result the way every other
    // post-turn arm does; what matters here is that it lands with the turn
    // that crossed the hour, under the result and above the greeting.
    let text = rows.join("\n");
    let result = text.find("Nothing to open!").expect("result row");
    let warning = text
        .find(STARVATION_WARNING)
        .unwrap_or_else(|| panic!("rows: {rows:?}"));
    let greeting = text
        .find("welcome")
        .unwrap_or_else(|| panic!("rows: {rows:?}"));
    assert!(result < warning && warning < greeting, "rows: {rows:?}");
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P19-02 (B). `dungeon-mode.md §12`: L-Look "retains the resident
/// dispatcher's acted result", so the drink's turn is the loop's acted charge
/// and the `§4` step 6 post-action pass follows it; `time.md §5` prints the
/// starvation warning in that turn's upkeep. After `Yes.  Gulp!` the warning
/// used to wait under the next command, while `No.` printed it at once.
#[test]
fn playtest_p19_02_dungeon_fountain_yes_prints_starving_on_the_drink_turn() {
    for answer in ['Y', 'N'] {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 0)] = 0x50;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::North;
        state.torch_counter = 50;
        state.party[0].hp = 60;
        state.party[0].max_hp = 150;
        state.food = 0;
        state.clock = GameClock::new(10, 59).unwrap();
        state.cleanup_previous_hour = 10;

        handle_play_key_input(&mut state, 'L', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", Path::new("")).unwrap();
        assert!(
            state.active_yes_no_prompt.is_some(),
            "{answer}: drink prompt"
        );
        handle_play_key_input(&mut state, answer, "", Path::new("")).unwrap();

        assert_eq!(state.clock.hour, 11);
        assert_eq!(state.turn, 1, "{answer}: one acted turn");
        assert!(
            state.pending_hourly_status_message.is_none(),
            "{answer}: the warning is not held for the next command"
        );
        let log = message_log_from_entries(state.message_entries(), |text| Some(text.to_string()));
        let rows: Vec<&str> = log.lines().iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            rows.iter().rev().find(|row| !row.is_empty()).copied(),
            Some(STARVATION_WARNING),
            "{answer}: rows {rows:?}"
        );
    }
}

fn p19_03_rows(state: &PlayState) -> Vec<String> {
    let log = message_log_from_entries(state.message_entries(), |text| Some(text.to_string()));
    log.lines().iter().map(|line| line.text.clone()).collect()
}

fn p19_03_starving_town_state(tile: u8) -> PlayState {
    let mut grid = open_grid();
    grid[32 + 2] = tile;
    let mut state = test_state(grid, 1, 1);
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.food = 0;
    state.clock = GameClock::new(10, 59).unwrap();
    state.cleanup_previous_hour = 10;
    state
}

/// Playtest P19-03, fountain. `view.md §3` (Fountains): the handler prints
/// `Who will drink?`, opens the selector, prints `Refreshing...` and "runs
/// **once** ... it returns to the command loop"; `town-mode.md §7` runs the
/// epilogue (minute, status pass with its starvation warning, walkers)
/// "When the dispatcher returns". The epilogue used to run as the selector
/// opened, so `Starving!` split the question from its answer.
#[test]
fn playtest_p19_03_fountain_look_turn_waits_for_the_drinker_pick() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    let mut state = p19_03_starving_town_state(0xd9);

    handle_play_key_input(&mut state, 'L', "", &dir).unwrap();
    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();
    assert!(
        state.active_party_selector.is_some(),
        "drinker selector open"
    );
    assert_eq!(state.turn, 0, "no epilogue under the open question");
    assert_eq!(state.clock, GameClock::new(10, 59).unwrap());
    assert_eq!(state.party[0].hp, 60);

    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
    assert!(state.active_party_selector.is_none());
    assert_eq!(state.turn, 1, "the Look's acted turn is spent on return");
    assert_eq!(state.clock, GameClock::new(11, 0).unwrap());
    assert!(state.party[0].hp < 60);
    let rows = p19_03_rows(&state);
    let text = rows.join("\n");
    let question = text
        .find(FOUNTAIN_DRINK_PROMPT)
        .unwrap_or_else(|| panic!("{rows:?}"));
    let answer = text
        .find(FOUNTAIN_DRINK_REFRESHED)
        .unwrap_or_else(|| panic!("{rows:?}"));
    let warning = text
        .find(STARVATION_WARNING)
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert!(question < answer && answer < warning, "rows: {rows:?}");
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P19-03, wishing well. `view.md §3` "Wishing-well text": the Y/N
/// answer "is appended immediately as `Yes\n`", and the epilogue waits for
/// the handler's return after the wish. The `Yes` echo used to be lost to a
/// `Starving!` row printed under the open coin question.
#[test]
fn playtest_p19_03_wishing_well_look_turn_waits_for_the_wish() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    let mut state = p19_03_starving_town_state(0xa1);
    state.gold = 5;

    handle_play_key_input(&mut state, 'L', "", &dir).unwrap();
    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();
    assert!(state.active_wishing_well.is_some(), "coin prompt open");
    assert_eq!(state.turn, 0, "no epilogue under the open question");

    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    assert_eq!(state.turn, 0, "the wish is still open");
    let rows = p19_03_rows(&state);
    assert!(
        rows.iter()
            .any(|row| *row == format!("{WISHING_WELL_COIN_PROMPT}{WISHING_WELL_COIN_YES_REPLY}")),
        "the Yes echo stays on the question row: {rows:?}"
    );
    assert!(
        !rows.iter().any(|row| row == STARVATION_WARNING),
        "{rows:?}"
    );

    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
    assert!(state.active_wishing_well.is_none());
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(11, 0).unwrap());
    let rows = p19_03_rows(&state);
    let text = rows.join("\n");
    let wish = text
        .find(WISHING_WELL_EMPTY_WISH_LINE.trim_end())
        .unwrap_or_else(|| panic!("{rows:?}"));
    let warning = text
        .find(STARVATION_WARNING)
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert!(wish < warning, "rows: {rows:?}");
    let _ = fs::remove_dir_all(dir);
}
