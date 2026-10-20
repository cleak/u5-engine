// u5-spec#299 follow-ups 9-12 (spec commit a4aadbf) and the R568 retraction.

/// u5-spec#299 Q10, `blackthorn.md §4.2` "How long it holds" (`RETRACTIONS.md`
/// R568): "The floor is therefore about 72 tick periods, **about 3.95 s**, and
/// the exit beat runs for **about 4 s** from the key to the end of the script
/// on a fast enough host". The engine held a measured 110 ticks (about 6 s)
/// against the withdrawn five-second floor.
#[test]
fn spec299_q10_audience_exit_beat_holds_seventy_two_tick_periods() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    // A solo party: record 9, then the exit beat on the next key.
    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    assert!(state.pending_blackthorn_audience_exit);
    assert!(state.staged_narration_active(), "the exit beat is holding");

    state.advance_staged_narration_to(0.0);
    state.advance_staged_narration_to(crate::narration::bios_ticks_secs(71));
    assert!(state.staged_narration_active(), "still holding one tick short");
    state.advance_staged_narration_to(crate::narration::bios_ticks_secs(72) + 0.001);
    assert!(
        !state.staged_narration_active(),
        "the beat is over after about 72 tick periods (~3.95 s)"
    );
    let _ = fs::remove_dir_all(dir);
}
