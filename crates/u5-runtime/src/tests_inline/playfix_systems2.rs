// Regressions for the systems exploratory play-test findings (F52..F58).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// F55: `inventory.md §4.7` publishes the Equipment screen as "(food, gold,
/// keys, gems, torches, grapple)" with the `\n_Grapple` label after
/// `\n_Torches....`; a party holding the grapple must see that row.
#[test]
fn playfix_f55_z_stats_equipment_page_lists_the_owned_grapple() {
    let mut state = world_state(open_world_grid(), 5, 5);
    let session = ZStatsSession {
        selected_party_index: 0,
        page: ZStatsPage::Counters,
        inventory_cursor: 0,
    };

    state.climbing_gear = 0;
    let rows = state.z_stats_panel_rows(&session);
    assert!(rows[6].starts_with(" Torches...."));
    assert!(!rows.iter().any(|row| row.contains("Grapple")));

    state.climbing_gear = 1;
    let rows = state.z_stats_panel_rows(&session);
    assert!(rows[6].starts_with(" Torches...."));
    assert_eq!(rows[7], " Grapple");
}

/// F57: `blackthorn.md §7` step 11 spins the clock to 06:00, and from noon
/// that crosses midnight. `time.md §7` / `stats-panel.md §2.4`: "The day
/// rollover also repaints the stats panel ... exactly once per in-game day
/// boundary", so the panel's date row must show the new day after the
/// handoff, not the day of the wipe.
#[test]
fn playfix_f57_blackthorn_rescue_spin_across_midnight_repaints_the_panel_date() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::new(12, 0).unwrap();
    state.clock.day = 5;
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.repaint_stats_panel();
    let wipe_day_row = state.displayed_stats_panel().date_row.clone();

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

    assert_eq!(state.clock.day, 6);
    assert_eq!(state.clock.hour, crate::blackthorn::BLACKTHORN_RESCUE_HANDOFF_HOUR);
    let live = crate::stats_panel::stats_panel_snapshot_of(&state);
    assert_ne!(state.displayed_stats_panel().date_row, wipe_day_row);
    assert_eq!(state.displayed_stats_panel().date_row, live.date_row);
    let _ = fs::remove_dir_all(dir);
}
