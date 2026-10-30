// u5-spec#304 Q8 (spec afd2249): shrine kneel pacing.

/// u5-spec#304 Q8, `karma.md §7` "Pacing rule": "model a fixed tick lattice
/// at the stock rate. A one-tick request completes at the first lattice edge
/// after it is issued. Charge host time for every repaint, sting and printed
/// row ... On that model the published step counts give about 49 and about
/// 11.5 ticks at this setting with no fixed fudge constant."
#[test]
fn spec304_q8_shrine_entry_holds_follow_the_lattice_pacing_rule() {
    // "A one-tick request ends at the next edge of a fixed tick lattice, not
    // one tick after it is issued."
    assert_eq!(shrine_lattice_request_end_tick(0), 1);
    assert_eq!(shrine_lattice_request_end_tick(3_700), 4);
    assert_eq!(shrine_lattice_request_end_tick(3_999), 4);
    assert_eq!(shrine_lattice_request_end_tick(4_000), 5);

    let (kneel, question) = shrine_entry_lattice_timeline();
    // "From the `E` keypress the interval is phase-dependent: about
    // 3.7 + r + 44 + 0.7 ticks with the phase residual r in (0, 1], so about
    // 48.4 to 49.4 ticks".
    assert_eq!(kneel, 48_700);
    assert!((48_400..=49_400).contains(&kneel), "{kneel}");
    // "the ten requests end on the twelfth edge after the walk's last".
    let last_walk_edge = (kneel - SHRINE_KNEEL_POSE_REPAINT_MILLITICKS) / 1000;
    assert_eq!(question, (last_walk_edge + 12) * 1000);
    // "Kneel to virtue question: about 11.5 ticks (634 ms), glyph to glyph."
    let gap = question - kneel;
    assert!((11_000..=12_000).contains(&gap), "{gap}");

    // The engine's whole-tick holds: 49 and 11, and between them the
    // question lands on the lattice's sixtieth edge.
    assert_eq!(SHRINE_KNEEL_HOLD_BIOS_TICKS, 49);
    assert_eq!(SHRINE_VIRTUE_QUESTION_HOLD_WORLD_TICKS, 11);
    assert_eq!(
        u32::from(SHRINE_KNEEL_HOLD_BIOS_TICKS + SHRINE_VIRTUE_QUESTION_HOLD_WORLD_TICKS) * 1000,
        question
    );
    // The published counts the holds spend are unchanged.
    assert_eq!(SHRINE_KNEEL_WORLD_STEPS, 46);
    assert_eq!(SHRINE_VIRTUE_QUESTION_WORLD_STEPS, 10);
}

/// The shrine entry's beat times through the frontend's own pump, in ms after
/// the `E` key: when the Approach, Kneel and virtue-question records first
/// reach the message transcript. `karma.md §7` "Pacing rule": the Kneel at
/// the lattice's 48.7-tick mark rounded to 49 whole ticks, the question on the
/// sixtieth edge. Pinned to the values the engine had at `2f81f198`, where
/// `qa/paired/shrine-enter-pacing.tsv` matched stock on every run.
#[test]
fn shrine_entry_beat_times_through_the_narration_pump() {
    let dir = debug_game_dir();
    fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
    let mut miscmsg = Vec::new();
    for index in 0..MISCMSG_DAT_RECORDS {
        let record = match index {
            MISCMSG_SHRINE_ENTRY_NARRATION => "\nApproach placeholder.\n\n".to_string(),
            MISCMSG_SHRINE_KNEEL_NARRATION => "Kneel placeholder.\n\n".to_string(),
            MISCMSG_SHRINE_VIRTUE_QUESTION => "Question placeholder?\n\n:".to_string(),
            _ => format!("rec{index}"),
        };
        miscmsg.extend_from_slice(record.as_bytes());
        miscmsg.push(0);
    }
    fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
    let mut state = britannia_state(grid, 10, 20);

    handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
    let shown = |state: &PlayState, text: &str| {
        state.message_transcript.iter().any(|entry| entry.text == text)
            || state.message.contains(text)
    };
    let mut beats: [Option<u32>; 3] = [None; 3];
    let texts = ["Approach placeholder.", "Kneel placeholder.", "Question placeholder?"];
    for ms in 0..10_000u32 {
        state.advance_staged_narration_to(f64::from(ms) / 1000.0);
        for (slot, text) in beats.iter_mut().zip(texts) {
            if slot.is_none() && shown(&state, text) {
                *slot = Some(ms);
            }
        }
    }
    eprintln!("shrine beat ms after E: {beats:?}");
    // 49 ticks = 2691.0 ms, 60 ticks = 3295.6 ms; the pump releases on the
    // first whole-ms reading at or past each due time.
    assert_eq!(beats, [Some(0), Some(2692), Some(3296)]);
    let _ = fs::remove_dir_all(dir);
}
