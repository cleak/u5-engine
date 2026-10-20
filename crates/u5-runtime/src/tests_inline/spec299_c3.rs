// Conformance for u5-spec a031913 (issue #299 follow-ups 13-19, R569-R572).
// Each test names the answer it pins and the published sentence it rests on.

/// u5-spec#299 Q17 / `RETRACTIONS.md` R571, `blackthorn.md §7` step 11: the
/// rescue spins "the ordinary clock advance in nine-minute steps until the
/// hour is 6. The hour is checked before each step and the minute is never
/// reset." The worked vectors: "4-28-139 23:55 becomes 5-1-139 06:04, and
/// 13-28-139 13:10 becomes 1-1-140 06:07"; "an entry hour of 6 makes no step
/// and keeps its date and minute unchanged"; an entry hour of 0 to 5 "ends the
/// same day at 06:00 to 06:08".
#[test]
fn spec299_q17_rescue_clock_spins_in_nine_minute_steps_to_hour_six() {
    let cases = [
        ((139, 4, 28, 23, 55), (139, 5, 1, 6, 4)),
        ((139, 13, 28, 13, 10), (140, 1, 1, 6, 7)),
        ((139, 3, 9, 6, 30), (139, 3, 9, 6, 30)),
        ((139, 3, 9, 5, 55), (139, 3, 9, 6, 4)),
        ((139, 3, 9, 0, 0), (139, 3, 9, 6, 0)),
    ];
    for ((year, month, day, hour, minute), expected) in cases {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::with_date(year, month, day, hour, minute).unwrap();
        state.party[0].status = b'D';
        state.party[0].hp = 0;
        state.repaint_stats_panel();

        state.apply_blackthorn_rescue_refuge(&dir).unwrap();
        state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

        let clock = state.clock;
        assert_eq!(
            (clock.year, clock.month, clock.day, clock.hour, clock.minute),
            expected,
            "rescue from {year}-{month}-{day} {hour:02}:{minute:02}"
        );
        // "A midnight crossing runs the ordinary day rollover once, which
        // repaints the whole stats panel straight away, so the date row shows
        // the new date."
        let live = crate::stats_panel::stats_panel_snapshot_of(&state);
        assert_eq!(state.displayed_stats_panel().date_row, live.date_row);
        let _ = fs::remove_dir_all(dir);
    }
}
