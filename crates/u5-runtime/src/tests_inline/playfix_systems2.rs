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

/// F57: `blackthorn.md §7` step 11 spins the clock in nine-minute steps until
/// the hour is 6 (u5-spec#299 Q17, R571), and from noon that crosses midnight. `time.md §7` / `stats-panel.md §2.4`: "The day
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

/// `blackthorn.md §7` step 11 (issue #299 Q28): the rescue spin "runs the
/// clock advance's midnight work once" - the hideout reroll, the day advance
/// and, past day 28, the month bundle - lowers the camp cooldown once per
/// hour crossed, rewrites the twelve-hour value, and leaves the previous-hour
/// snapshot at 5 against hour 6. "An entry hour of 6 runs none of this
/// work".
#[test]
fn blackthorn_rescue_spin_runs_the_published_midnight_and_hour_work() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::new(23, 0).unwrap();
    state.clock.day = 28;
    let month_before = state.clock.month;
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.shadowlord_hideouts = [1, 2, 3];
    state.fortunes_of_war = 5;
    state.camp_cooldown = 10;
    state.party_stay_counters = vec![3; state.party.len()];

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

    // Nine-minute steps (§7 step 11, R571): 23:00 plus 47 x 9 minutes.
    assert_eq!(
        (state.clock.hour, state.clock.minute, state.clock.day),
        (6, 3, 1)
    );
    assert_ne!(state.clock.month, month_before);
    // A rerolled slot never keeps a value any slot held.
    assert!(state.shadowlord_hideouts.iter().all(|id| ![1, 2, 3].contains(id)));
    assert_eq!(state.fortunes_of_war, 0, "month-boundary bundle");
    assert!(state.party_stay_counters.iter().all(|counter| *counter == 4));
    // 23:00 -> 06:00 crosses seven hours.
    assert_eq!(state.camp_cooldown, 3);
    assert_eq!(state.twelve_hour_audio_repeats, 6);
    assert_eq!(state.cleanup_previous_hour, 5);
    let _ = fs::remove_dir_all(dir);

    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::new(6, 30).unwrap();
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.shadowlord_hideouts = [1, 2, 3];
    state.camp_cooldown = 10;
    state.cleanup_previous_hour = 4;
    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    state.run_blackthorn_rescue_to_handoff(&dir).unwrap();
    assert_eq!((state.clock.hour, state.clock.minute), (6, 30));
    assert_eq!(state.shadowlord_hideouts, [1, 2, 3]);
    assert_eq!(state.camp_cooldown, 10);
    assert_eq!(state.cleanup_previous_hour, 4);
    let _ = fs::remove_dir_all(dir);
}

/// F58: `formats/end-dat.md §3` - "`_` Soft hyphen: draws nothing" - and
/// `text-output.md §8.2` make the soft hyphen and the `{` indent invisible.
/// The message slot that the headless frames and the composed screen print
/// must not carry either marker; only the proportional page renderer sees
/// them.
#[test]
fn playfix_f58_endgame_narrative_message_slot_drops_end_dat_markers() {
    let mut raw = Vec::new();
    let mut table = String::new();
    for index in 0..6 {
        let start = raw.len();
        raw.extend_from_slice(b"{\"In_stead, famil_iar furn_i_ture");
        raw.push(0);
        table.push_str(&format!("{} {start} {}\n", index + 1, raw.len()));
    }
    let ranges = parse_end_narrative_window_ranges(&table).unwrap();
    let narrative = EndNarrative::new(raw).with_window_ranges(ranges);
    let mut endgame = EndgameState::terminal(
        true,
        true,
        true,
        endgame_certificate_fields("MARIA", GameClock::with_date(141, 5, 6, 12, 0).unwrap()),
        endgame_final_report(GameClock::with_date(141, 5, 6, 12, 0).unwrap()),
        None,
        Some(narrative),
    );
    assert_eq!(endgame.current_narrative_layout_text(), None);
    endgame.advance_cinematic();
    assert!(endgame.advance_cinematic_frame_operation());

    assert_eq!(endgame.current_cinematic_text(), "\"Instead, familiar furniture");
    assert_eq!(
        endgame.current_narrative_layout_text().as_deref(),
        Some("{\"In_stead, famil_iar furn_i_ture")
    );
}

/// `inventory.md §4.7` (issue #299 Q27): "Food and Gold are printed
/// right-justified in a four-cell field padded with spaces. Keys, Gems and
/// Torches are printed right-justified in a two-cell field padded with
/// spaces, starting at that thirteenth cell, so a single digit sits in the
/// fourteenth cell; a zero count prints `0` and those three rows are always
/// shown." Every nonzero grapple byte prints the same bare label.
#[test]
fn z_stats_counters_page_uses_the_published_field_widths() {
    let mut state = world_state(open_world_grid(), 5, 5);
    let session = ZStatsSession {
        selected_party_index: 0,
        page: ZStatsPage::Counters,
        inventory_cursor: 0,
    };
    state.food = 63;
    state.gold = 1234;
    state.keys = 0;
    state.gems = 7;
    state.torches = 12;
    state.climbing_gear = 0x80;
    let rows = state.z_stats_panel_rows(&session);
    assert_eq!(rows[1], " Food:   63");
    assert_eq!(rows[2], " Gold: 1234");
    assert_eq!(rows[4], " Keys....... 0");
    assert_eq!(rows[5], " Gems....... 7");
    assert_eq!(rows[6], " Torches....12");
    assert_eq!(rows[7], " Grapple");
}
