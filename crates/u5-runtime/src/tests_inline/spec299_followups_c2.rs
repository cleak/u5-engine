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

/// u5-spec#299 Q9, `blackthorn.md §5` "Rows on the correct-answer arm" with
/// §4.2's "no print, character, window or input event between the record and
/// the end of the handoff": once step 9's feed has closed record `5`'s rows
/// the window ends on a blank row, not an open one, and the exit beat that
/// the key starts still opens no command row under it.
#[test]
fn spec299_q9_merciful_exit_beat_opens_no_command_row() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.repaint_stats_panel();
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    state.flush_staged_narration();
    assert!(!state.message_row_open_mid_line, "step 9 left the cursor at column 0");
    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    assert!(state.pending_blackthorn_audience_exit, "the key starts the exit beat");
    assert!(
        state.message_window_live_row_suppressed(),
        "the exit beat draws no command row"
    );
    let _ = fs::remove_dir_all(dir);
}

/// u5-spec#299 Q11, `blackthorn.md §3` "Party placement (step 6)": with three
/// nondead members the third roster record is placed in slot 2 at `(0,1)`, a
/// manacle cell, so it is drawn as a prisoner like the first two ("Every
/// placed member, the Avatar included, therefore appears as a manacled
/// prisoner").
#[test]
fn spec299_q11_third_member_stands_manacled_at_zero_one() {
    let mut state = test_state(open_grid(), 12, 20);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.blackthorn_audience_map = Some(blackthorn_audience_fixture_map());
    state.install_blackthorn_audience_actors();

    let third = state.active_objects[2];
    assert!(!third.is_empty(), "slot 2 holds the third member");
    assert_eq!((third.x, third.y), (0, 1));
    // "Each placed actor gets the member's class tile" (rest-and-camp §7.2).
    assert_eq!(third.tile, 0x48, "a Fighter");
    assert_eq!(state.active_objects[0].tile, 0x4C, "the Avatar's class tile");
    assert!(state.active_objects[3].is_empty(), "n = 3 places three slots");
    let prisoner = 0x164..=0x167usize;
    for slot in 0..3 {
        let tile = state.blackthorn_audience_actor_atlas_tile(slot).unwrap();
        assert!(prisoner.contains(&tile), "slot {slot} drew {tile:#x}");
    }
}

/// u5-spec#299 Q11: the published cell table, and "The bound is a count, not
/// a filter: a Dead record that falls inside the first *n* is placed like any
/// other, and the last nondead record is then left out."
#[test]
fn spec299_q11_placement_follows_the_nondead_count_table() {
    use crate::blackthorn::blackthorn_audience_party_cell as cell;
    assert_eq!(cell(1, 1), None);
    assert_eq!(cell(4, 2), Some((10, 1)));
    assert_eq!(cell(4, 3), Some((0, 1)));
    assert_eq!(cell(5, 2), Some((9, 1)));
    assert_eq!(cell(5, 3), Some((1, 1)));
    assert_eq!(cell(5, 4), Some((0, 1)));
    assert_eq!(
        (2..6).map(|slot| cell(6, slot)).collect::<Vec<_>>(),
        vec![Some((10, 1)), Some((9, 1)), Some((1, 1)), Some((0, 1))]
    );

    let mut state = test_state(open_grid(), 12, 20);
    let mut dead = audit_g5_companion(1);
    dead.status = b'D';
    dead.hp = 0;
    state.party.push(dead);
    state.party.push(audit_g5_companion(2));
    state.install_blackthorn_audience_actors();
    // n = 2: slots 0 and 1, the Dead record in slot 1; record 2 is left out.
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (7, 5)
    );
    assert!(state.active_objects[2].is_empty());
}

/// u5-spec#299 Q11: "Slots 2 to 5 are never moved or cleared by any audience
/// script ... the (0,1) prisoner, slot 2, survived the exit beat."
#[test]
fn spec299_q11_third_prisoner_survives_the_exit_beat() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.repaint_stats_panel();
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    assert_eq!((state.active_objects[2].x, state.active_objects[2].y), (0, 1));
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    state.flush_staged_narration();
    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    assert!(state.pending_blackthorn_audience_exit);
    let third = state.active_objects[2];
    assert_eq!((third.x, third.y), (0, 1), "the exit vector leaves slot 2 alone");
    assert_eq!(third.aux3, BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER);
    // "They go at the captive-cell handoff, because the town entry it
    // triggers clears actor slots 1 to 31".
    state.run_blackthorn_audience_exit_to_handoff(&dir).unwrap();
    assert!(state
        .active_objects
        .iter()
        .skip(1)
        .all(|object| object.aux3 != BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER || object.is_empty()));
    let _ = fs::remove_dir_all(dir);
}
