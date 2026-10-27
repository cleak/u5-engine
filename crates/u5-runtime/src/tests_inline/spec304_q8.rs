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
