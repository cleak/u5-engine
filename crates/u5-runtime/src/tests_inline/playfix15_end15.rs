// Regressions for the end15 exploratory play-test findings (P15-06..P15-09).

/// Playtest P15-06. `endgame.md §8.4`: after the sixth narrative window and
/// one more key the presentation draws the parchment and `§9` prints the
/// certificate onto it; `§9.5` "Nothing follows". The plain-text frames fell
/// back to the gameplay stats panel and an empty message window instead.
#[test]
fn p15_endgame_certificate_text_frame_is_the_full_screen_certificate() {
    let clock = GameClock::with_date(141, 5, 6, 12, 0).unwrap();
    let mut endgame = EndgameState::terminal(
        true,
        true,
        true,
        endgame_certificate_fields("MARIA", clock),
        endgame_final_report(clock),
        None,
        None,
    );
    for _ in 0..64 {
        if endgame.cinematic.step == crate::endgame_cinematic::EndgameCinematicStep::Certificate {
            break;
        }
        if !endgame.cinematic.advance_fade_to_black() {
            endgame.advance_cinematic();
        }
    }
    assert_eq!(
        endgame.cinematic.step,
        crate::endgame_cinematic::EndgameCinematicStep::Certificate
    );
    let mut state = test_state(open_grid(), 1, 1);
    state.party_names = vec![*b"MARIA\0\0\0\0"];
    state.message.clear();
    state.endgame = Some(endgame);
    assert!(crate::endgame_certificate_frame::endgame_certificate_on_screen(&state));

    let frame = state.render_text_window_frame(None);
    let rows: Vec<&str> = frame.lines().collect();
    assert_eq!(rows.len(), usize::from(TEXT_SCREEN_ROWS), "{frame}");
    // §9.2's fixed rows, centred by `(40 - len) / 2`.
    assert_eq!(rows[1].trim_end(), "          Be it known that on", "{frame}");
    assert_eq!(rows[8].trim(), "MARIA the Avatar", "{frame}");
    assert_eq!(rows[16].trim(), "THE QUEST OF THE AVATAR", "{frame}");
    assert_eq!(rows[21].trim(), "Report now, thy Quest compleat in");
    assert_eq!(rows[22].trim(), "2 years, 1 month, 1 day");
    assert_eq!(rows[23].trim(), "to Lord British at Origin Systems!");
    // No gameplay layout: the food/gold row of the stats panel is gone.
    assert!(!frame.contains("F:"), "{frame}");

    // §9.5: the terminal hold keeps the same page.
    state.endgame.as_mut().unwrap().advance_cinematic();
    state.endgame.as_mut().unwrap().advance_cinematic();
    assert_eq!(state.render_text_window_frame(None), frame);
}
