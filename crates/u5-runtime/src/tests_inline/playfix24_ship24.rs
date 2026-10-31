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

/// P24-05: `vehicles.md §6` - docking "prints the exact line `Docked!`
/// followed by a line break", and `text-output.md §10.2` writes it on the
/// marker row of a released hoisted step. A dock reached by the input
/// helper's wind-released wait pass left the line in the slot, and the next
/// command dropped it.
#[test]
fn playtest_s24_p24_05_a_wait_pass_dock_keeps_its_line() {
    let dir = debug_game_dir();
    // East wind: sailing north is a crosswind and releases on the first pass.
    let mut state = ship24_hoisted_frigate(WindState::East, 99);
    state.grid[world_cell_index(100, 99)] = OVERWORLD_PIER_TILE;
    state.push_message_entry("Yell HOIST!", true);
    state.sail_cached_direction = Some(Direction::North);
    state.sail_cadence = 0;
    for _ in 0..2 {
        state.idle_wait_pass(None).unwrap();
    }
    assert!(!state.player.transport.is_ship_under_sail(), "the dock furled");
    assert!(!state.message_row_open_mid_line);

    let tail = p13text_get_north_rows(&mut state, &dir);
    assert_eq!(&tail[..3], &[">Get-North", "", "Nothing to get!"], "{tail:?}");
    let rows = p4window_rows(&state);
    let get = rows.iter().rposition(|row| row == ">Get-North").unwrap();
    assert_eq!(&rows[get - 2..get], &[">Docked!", ""], "{rows:?}");
    let _ = fs::remove_dir_all(dir);
}

fn ship24_clock_minutes(state: &PlayState) -> u32 {
    u32::from(state.clock.hour) * 60 + u32::from(state.clock.minute)
}

/// P24-07 / P24-16: `weather.md §5` - the HMS Cape rigging flag "affects the
/// wait-pass timing", and `timing.md §8.2` gives the one-minute increment to
/// the input helper's auto-advance pass only. A course change (`vehicles.md
/// §6`: "consumes a full turn, in calm wind too") and the calm-wind silent
/// consume are commands, so they keep the two-minute outdoor turn.
#[test]
fn playtest_s24_p24_07_rigging_leaves_hoisted_commands_at_two_minutes() {
    let dir = debug_game_dir();
    let mut state = ship24_hoisted_frigate(WindState::Calm, 99);
    state.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = 2;
    assert!(state.ship_rigging_active());
    assert_eq!(state.sail_cached_direction, None);

    // Calm, facing unchanged: consumed silently.
    let before = ship24_clock_minutes(&state);
    handle_play_key_input(&mut state, 'w', "", &dir).unwrap();
    assert_eq!(ship24_clock_minutes(&state) - before, 2, "calm silent consume");

    // A course change.
    let before = ship24_clock_minutes(&state);
    handle_play_key_input(&mut state, 'd', "", &dir).unwrap();
    assert_eq!(state.overworld_frigate_facing(), Some(Direction::East));
    assert_eq!(ship24_clock_minutes(&state) - before, 2, "Head East");

    // The control: a real wait pass still takes the rigged minute.
    state.wind = WindState::West;
    state.sail_cached_direction = Some(Direction::East);
    state.sail_cadence = 0;
    let before = ship24_clock_minutes(&state);
    state.run_under_sail_advance_body(None).unwrap();
    assert_eq!(ship24_clock_minutes(&state) - before, 1, "rigged wait pass");
    let _ = fs::remove_dir_all(dir);
}

/// P24-13: `input.md §2` - a Y/N prompt waits through the shared wait, whose
/// no-key pass runs the world tick only, and `timing.md §8.2` puts the
/// under-sail auto-advance at the command-input step alone. A pass taken
/// while `Save game?` was open sailed the ship on and charged the turn.
#[test]
fn playtest_s24_p24_13_an_open_prompt_does_not_sail_the_ship() {
    let dir = debug_game_dir();
    let mut state = ship24_hoisted_frigate(WindState::East, 99);
    state.sail_cached_direction = Some(Direction::North);
    state.sail_cadence = 0;
    handle_play_key_input(&mut state, 'Q', "", &dir).unwrap();
    assert!(state.active_yes_no_prompt.is_some(), "Save game? is open");
    assert!(state.prompt_waits_in_shared_wait());
    let (turn, clock) = (state.turn, state.clock);
    for _ in 0..4 {
        state.idle_wait_pass(Some(&dir)).unwrap();
    }
    assert_eq!((state.player.x, state.player.y), (100, 100));
    assert_eq!(state.turn, turn);
    assert_eq!(state.clock, clock);
    assert!(state.active_yes_no_prompt.is_some(), "the prompt is still open");
    let _ = fs::remove_dir_all(dir);
}

/// P24-14: `stats-panel.md §2.2.1` - the ship-damage survive arm repaints
/// the panel immediately. A collision reached on the idle auto-advance pass
/// has no command epilogue, and the hull figure stayed stale.
#[test]
fn playtest_s24_p24_14_a_wait_pass_collision_repaints_the_hull() {
    let mut state = ship24_hoisted_frigate(WindState::East, 77);
    state.grid[world_cell_index(100, 99)] = 0x03;
    state.sail_cached_direction = Some(Direction::North);
    state.sail_cadence = 0;
    state.repaint_stats_panel();
    assert!(state.displayed_stats_panel().counter_row.contains("Ship:77"));
    for _ in 0..2 {
        state.idle_wait_pass(None).unwrap();
    }
    let TransportState::Ship { hull, .. } = state.player.transport else {
        panic!("still aboard");
    };
    assert!(hull < 77, "the shoal took {hull}");
    let counter = &state.displayed_stats_panel().counter_row;
    assert!(counter.contains(&format!("Ship:{hull}")), "{counter:?}");
}

/// P24-06: `time.md §5` - the poison point goes through the shared
/// party-damage path, which "repaints the stats panel immediately, as its
/// last act". An under-sail wait pass is "a fully consumed game turn"
/// (`weather.md §5`) with an upkeep pass but no command epilogue, and the
/// HP cell stayed stale across it.
#[test]
fn playtest_s24_p24_06_a_wait_pass_poison_point_repaints_the_panel() {
    // North wind: sailing north is upwind, two wait ticks before a move.
    let mut state = ship24_hoisted_frigate(WindState::North, 99);
    state.party[0].status = b'P';
    state.party[0].hp = 40;
    state.sail_cached_direction = Some(Direction::North);
    state.sail_cadence = 0;
    state.repaint_stats_panel();
    state.run_under_sail_advance_body(None).unwrap();
    assert_eq!(state.party[0].hp, 39, "the wait pass charged the poison point");
    assert_eq!(
        state.displayed_stats_panel(),
        &crate::stats_panel::stats_panel_snapshot_of(&state),
        "the panel shows the poisoned member's new HP"
    );
}

/// P24-15: after a wipe aboard a frigate the rescue hands control to the
/// castle on foot, and town entry setup redraws the panel (`town-mode.md
/// §5`, `stats-panel.md §2.2.1`). The counters row showed only the gold form
/// off the ship family (`stats-panel.md §6`), but it kept `Ship:NN`.
#[test]
fn playtest_s24_p24_15_the_rescue_handoff_repaints_the_on_foot_counters() {
    let dir = debug_game_dir();
    let mut state = ship24_hoisted_frigate(WindState::Calm, 77);
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.repaint_stats_panel();
    assert!(state.displayed_stats_panel().counter_row.contains("Ship:77"));

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    let outcome = state.run_blackthorn_rescue_to_handoff(&dir).unwrap();
    assert!(matches!(
        outcome,
        Some(MoveOutcome::Transition(AreaTransition::EnteredLocation(_)))
    ));
    assert!(state.player.transport.is_foot());
    let counter = &state.displayed_stats_panel().counter_row;
    assert!(!counter.contains("Ship:"), "{counter:?}");
    assert_eq!(
        state.displayed_stats_panel(),
        &crate::stats_panel::stats_panel_snapshot_of(&state)
    );
    let _ = fs::remove_dir_all(dir);
}
