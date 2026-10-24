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

/// u5-spec#299 Q12, `inventory.md §4.4` / `stats-panel.md §8` /
/// `display-driver.md` "Bracket end-caps": under the leaked inverse, "In each
/// end cap the source triangle comes out black and the rest of the cell, its
/// top and bottom scanlines included, comes out in chrome; the cap's two
/// accent strokes are line draws and stay exactly as they are". The caps are
/// reported opaque so the plain cap beneath cannot show through the black
/// triangle; with no effect running the band is untouched.
#[test]
fn spec299_q12_inverted_slot_caps_are_black_triangles_under_unchanged_strokes() {
    let font = chrome_test_font();
    let render = |inverse: bool| {
        let mut state = test_state(open_grid(), 5, 5);
        state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 10;
        state.stats_panel_window_inverse = inverse;
        state.repaint_stats_panel();
        let mut system = TextWindowSystem::new();
        configure_play_text_windows(&mut system);
        paint_stats_panel_text_window(&mut system, &state, state.active_player);
        (state, render_text_window_rgba(&system, &font).unwrap())
    };
    let (plain_state, plain) = render(false);
    let (state, inverted) = render(true);
    let sprite = ribbon_cap_sprite(&font, RibbonCapDirection::Right);
    let (x0, y0) = (30 * 8, 7 * 8);
    for y in 0..8 {
        for x in 0..8 {
            let bit = 0x80u8 >> x;
            let at = |rgba: &[u8]| chrome_index_at(rgba, x0 + x, y0 + y);
            if sprite.white[y] & bit != 0 {
                assert_eq!(at(&inverted), at(&plain), "stroke ({x},{y}) unchanged");
                assert_ne!(at(&inverted), 0, "stroke ({x},{y}) still drawn");
            } else if sprite.ribbon[y] & bit != 0 {
                assert_eq!(at(&inverted), 0, "triangle ({x},{y}) comes out black");
            } else {
                assert_eq!(at(&inverted), CHROME_RIBBON_INDEX, "({x},{y}) is chrome");
            }
        }
    }
    assert_eq!(
        crate::stats_panel::stats_panel_opaque_effect_cap_cells(&state),
        vec![(30, 7), (32, 7)]
    );
    assert!(crate::stats_panel::stats_panel_opaque_effect_cap_cells(&plain_state).is_empty());

    let mut idle = test_state(open_grid(), 5, 5);
    idle.stats_panel_window_inverse = true;
    idle.repaint_stats_panel();
    assert!(
        crate::stats_panel::stats_panel_opaque_effect_cap_cells(&idle).is_empty(),
        "with no effect running the band looks normal"
    );
}

/// `blackthorn.md §4.2` "Where the cursor is left": records `9` and `10`
/// leave the cursor mid-row "through the beat, through the handoff, and into
/// the next scene", so the captive cell's first command row sits directly
/// under the record's last row. Measured on `bt-password` (20261024-001911,
/// beat `onward`): stock `thee!"` with the marker row under it; the engine
/// had a blank row between.
#[test]
fn a_solo_closing_record_keeps_its_row_open_into_the_captive_cell() {
    // Record `9` takes the same helper; its text has no fallback without
    // `MISCMSG.DAT`, so record `10`'s measured fallback carries the test.
    for (answer, last_words) in [("IMPERA", "thee!")] {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
        state.begin_blackthorn_audience_capture(&dir).unwrap();
        while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
            state.submit_blackthorn_audience_answer("", &dir).unwrap();
        }
        state.submit_blackthorn_audience_answer(answer, &dir).unwrap();
        state.flush_staged_narration();
        let record = state
            .message_entries()
            .last()
            .map(|entry| (entry.text.clone(), entry.leaves_row_open))
            .unwrap();
        assert!(record.0.contains(last_words), "{answer}: {record:?}");
        assert!(record.1, "{answer}: the record's row is left open: {record:?}");

        state.submit_blackthorn_audience_answer("", &dir).unwrap();
        state.run_blackthorn_audience_exit_to_handoff(&dir).unwrap();

        assert!(state.active_blackthorn.is_none(), "{answer}: in the captive cell");
        let layout = rc4_message_layout(&state);
        let text = |row: &crate::MessageWindowRow| -> String {
            row.glyphs.iter().map(|glyph| glyph.byte as char).collect()
        };
        let rows: Vec<String> = layout.rows.iter().map(text).collect();
        let command = layout.rows.last().expect("the cell's command row");
        assert!(command.prefixed, "{answer}: a command row: {rows:?}");
        let above = &layout.rows[layout.rows.len() - 2];
        assert_eq!(above.row + 1, command.row, "{answer}: no blank between: {rows:?}");
        assert!(text(above).trim_end().ends_with('"'), "{answer}: {rows:?}");
        let _ = fs::remove_dir_all(dir);
    }
}
