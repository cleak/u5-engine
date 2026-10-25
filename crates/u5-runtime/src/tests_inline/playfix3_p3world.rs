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
    // `rest-and-camp.md §4` (u5-spec#300 Q8): no `(1-9)` at the bed.
    let prompt_row = format!("Hole up- {}3", crate::commands::TOWN_BED_HOURS_PROMPT);
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
fn p3w_w2_town_bed_hours_prompt_zero_cancels_silently_and_spends_the_turn() {
    // `rest-and-camp.md §4` (u5-spec#300 Q8): "Space or `0` cancels, also
    // silently. Neither key is echoed and no line feed prints ... The command
    // still reports acted, so a cancel costs the ordinary one-minute town
    // turn". Escape is discarded and the prompt keeps waiting.
    for cancel in ['0', ' '] {
        let (mut state, dir) = p3w_town_bed_state();
        let mut clock = state.clock;
        handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
        handle_play_key_input(&mut state, '\u{1b}', "", &dir).unwrap();
        assert!(state.active_rest.is_some(), "Escape is discarded");
        handle_play_key_input(&mut state, cancel, "", &dir).unwrap();
        assert!(state.active_rest.is_none(), "{cancel:?} cancels");
        let rows = p3w_rows(&state);
        let prompt_row = format!("Hole up- {}", crate::commands::TOWN_BED_HOURS_PROMPT);
        assert!(rows.iter().any(|row| *row == prompt_row), "{rows:?}");
        assert!(!rows.iter().any(|row| row.contains("None!")), "{rows:?}");
        assert!(
            !rows.iter().any(|row| row.ends_with(&format!("hours? {cancel}"))
                && cancel != ' '),
            "{rows:?}"
        );
        clock.advance_minutes(1);
        assert_eq!(state.clock, clock, "the cancel is acted");
        assert_eq!(state.turn, 1);
        let _ = std::fs::remove_dir_all(dir);
    }
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
    // `commands.md §3` (issue #300 Q14): the cancelled prompt still spends
    // the ordinary one-minute town turn - and no rest (p10 R02).
    assert_eq!(state.turn, 1, "the cancelled prompt spends the town turn");
    assert_eq!(state.clock, GameClock::new(10, 1).unwrap());
    let _ = clock;
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

#[test]
fn p3w_w5_ignite_turn_spends_its_own_light_unit() {
    // `lighting.md §8`: "Outside dungeon scenes, it sets the torch counter to
    // 240 counter units", and "normal dungeon and town turns spend one unit
    // and normal overworld turns spend two". The Ignite turn's cleanup runs
    // after the command (`town-mode.md §7` step 4), so it spends from the
    // new torch.
    let mut town = test_state(open_grid(), 1, 1);
    town.torches = 1;
    assert_eq!(town.ignite_torch(), MoveOutcome::Ignited);
    assert_eq!(town.torch_counter, SURFACE_TORCH_DURATION - 1);

    let mut world = world_state(open_world_grid(), 1, 1);
    world.torches = 1;
    assert_eq!(world.ignite_torch(), MoveOutcome::Ignited);
    assert_eq!(world.torch_counter, SURFACE_TORCH_DURATION - 2);
}

#[test]
fn p3w_w6_month_rollover_ages_every_character_record() {
    // `time.md §8`: "Each of the sixteen character record slots carries a
    // one-byte counter", and implementations "should still preserve and age
    // it on all character records". The rollover adds one only below 25, so
    // the shipped `0xFF` "stays `0xFF` for the whole game" (u5-spec#300 Q8,
    // R587 withdrew "capped at 25").
    let mut state = test_state(open_grid(), 1, 1);
    let active = state.party.len();
    let mut roster = default_party_roster(active.max(1));
    roster.truncate(active);
    let template = roster[0].clone();
    for counter in [0xFF, 3] {
        let mut record = template.clone();
        record.stay_counter = counter;
        roster.push(record);
    }
    state.party_roster = roster;
    state.clock = GameClock {
        day: 28,
        hour: 23,
        minute: 59,
        ..state.clock
    };
    state.advance_turn_with_minutes(MINUTES_PER_INDOOR_TURN);
    assert_eq!(state.clock.day, 1);
    let inactive: Vec<u8> = state.party_roster[active..]
        .iter()
        .map(|record| record.stay_counter)
        .collect();
    assert_eq!(inactive, [0xFF, 4]);
}

/// `rest-and-camp.md §4` (u5-spec#300 Q8): the bed prompt shows as
/// `Hole up- For` / `how many hours?`; after a cancel "the next command
/// prompt starts on the row after `how many hours?`".
#[test]
fn spec300_q8_town_bed_prompt_and_cancel_window_rows() {
    let (mut state, dir) = p3w_town_bed_state();
    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    handle_play_key_input(&mut state, '0', "", &dir).unwrap();
    assert_eq!(
        p4window_rows_from(&p4window_rows(&state), ">Hole up- For"),
        [">Hole up- For", "how many hours?", ">"]
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// `commands.md §5.8` (u5-spec#300 Q22, R605): the off-bed refusal wraps
/// as `Hole up- Only` / `in bed!`, then the ordinary blank row, and it
/// spends the one-minute town turn (Q14, R588).
#[test]
fn spec300_q22_town_off_bed_refusal_wraps_and_is_acted() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 1] = 16;
    let mut state = test_state(grid, 1, 1);
    let mut clock = state.clock;
    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    assert_eq!(
        p4window_rows_from(&p4window_rows(&state), ">Hole up- Only"),
        [">Hole up- Only", "in bed!", "", ">"]
    );
    clock.advance_minutes(1);
    assert_eq!((state.turn, state.clock), (1, clock));
    let _ = std::fs::remove_dir_all(dir);
}

/// `commands.md §5.8` (u5-spec#300 Q22, R605): on the overworld with no
/// Grapple, `Klimb-With` stands on the prompt row, `what?` on the next, then
/// the ordinary blank row and the next prompt; the turn is spent (§3).
#[test]
fn spec300_q22_overworld_klimb_with_what_wraps_and_spends_the_turn() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 10, 20);
    let mut clock = state.clock;
    handle_play_key_input(&mut state, 'K', "", &dir).unwrap();
    assert_eq!(
        p4window_rows_from(&p4window_rows(&state), ">Klimb-With"),
        [">Klimb-With", "what?", "", ">"]
    );
    clock.advance_minutes(MINUTES_PER_OUTDOOR_TURN);
    assert_eq!((state.turn, state.clock), (1, clock));
    let _ = std::fs::remove_dir_all(dir);
}

fn spec300_arrest_state(dir: &std::path::Path) -> PlayState {
    let record = synthetic_combat_arena_record();
    std::fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut state = test_state(open_grid(), 5, 5);
    state.load_scheduled_npcs(&[
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 2,
            type_byte: 0x70,
            dialog_id: 0,
            schedule: [6, 6, 6, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.pass_turn_with_game_dir(Some(dir)).unwrap();
    assert!(state.pending_town_arrest.is_some());
    state
}

/// `town-mode.md §14` (u5-spec#300 Q7, R591, R592): the refusal echo is
/// `No`; "a blank row, `"Then defend thyself, rogue!"` and the alarm sweep",
/// then cleanup "prints `Attacked!` with its leading line feed and enters the
/// NPC-conflict chain, an ordinary town arena fight". Only `Y`/`y`/`N`/`n`
/// answer.
#[test]
fn spec300_q7_arrest_refusal_echoes_no_and_fights_the_guard() {
    let dir = debug_game_dir();
    let mut state = spec300_arrest_state(&dir);
    handle_play_key_input(&mut state, 'x', "", &dir).unwrap();
    assert!(state.pending_town_arrest.is_some(), "other keys are ignored");
    let turn = state.turn;
    handle_play_key_input(&mut state, 'N', "", &dir).unwrap();
    assert!(state.pending_town_arrest.is_none());
    let rows: Vec<String> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    let at = rows.iter().position(|row| row == ":No").expect("`:No` row");
    assert_eq!(rows[at + 1], "", "{rows:?}");
    assert_eq!(
        rows[at + 2],
        crate::commands::TOWN_ARREST_REFUSAL_CHALLENGE,
        "{rows:?}"
    );
    assert!(rows[at + 3..].iter().any(|row| row == "Attacked!"), "{rows:?}");
    assert!(state.combat_active, "the refusal is followed by the fight");
    assert_eq!(
        state.pending_town_conflict.map(|pending| pending.npc_slot),
        Some(2)
    );
    assert_eq!(state.turn, turn, "no further epilogue");
    let _ = std::fs::remove_dir_all(dir);
}

/// `town-mode.md §14` (u5-spec#300 Q7, Q33, R617): surrender prints the
/// knockout line, a blank row and `Thou dost awaken to...`, and the party
/// wakes at 08:(arrest minute mod 20) - an arrest at 14:39 woke at 08:19 the
/// next day after 53 steps.
#[test]
fn spec300_q33_surrender_awakening_line_and_wake_minute() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.clock = GameClock::new(14, 39).unwrap();
    let day = state.clock.day;
    assert_eq!(state.advance_town_arrest_wait_to_morning(), 53);
    assert_eq!((state.clock.hour, state.clock.minute), (8, 19));
    assert_ne!(state.clock.day, day);

    let _ = std::fs::remove_dir_all(dir);

    // End to end against the configured original assets (skipped when
    // `U5_TEST_ASSET_DIR` is unset; read-only).
    let Some(assets) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let mut state = test_state(open_grid(), 5, 5);
    state.clock = GameClock::new(14, 39).unwrap();
    state.pending_town_arrest = Some(TownArrestPrompt {
        scene_byte: 1,
        floor: 0,
        npc_slot: 2,
    });
    state.emit_message_line(TOWN_ARREST_SURRENDER_PROMPT);
    state.open_message_row();
    handle_play_key_input(&mut state, 'Y', "", &assets).unwrap();
    state.flush_message_slot();
    let rows: Vec<String> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    let at = rows.iter().position(|row| row == ":Yes").expect("`:Yes` row");
    assert_eq!(
        rows[at + 1..],
        [
            String::new(),
            TOWN_ARREST_KNOCKOUT_MESSAGE.to_string(),
            String::new(),
            crate::commands::TOWN_ARREST_AWAKENING_MESSAGE.to_string(),
        ]
    );
    assert_eq!((state.clock.hour, state.clock.minute), (8, 19));
}
