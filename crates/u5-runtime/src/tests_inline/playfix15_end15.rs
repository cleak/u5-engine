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

/// Playtest P15-09. `commands.md §3`, the free/acted table's `E` row: on the
/// overworld only `Enter What?` is free and "every non-transition outcome"
/// spends the outdoor turn. The Codex arm restores the scene it left
/// (`karma.md §8`), so it is acted like the shrine arm - the engine charged
/// nothing for it.
#[test]
fn p15_codex_enter_spends_one_outdoor_turn_after_the_reader_closes() {
    let dir = debug_game_dir();
    let (x, y) = crate::PUBLISHED_CODEX_URN_COORDINATE;
    let mut grid = open_world_grid();
    grid[world_cell_index(x, y)] = CODEX_SHRINE_ENTRY_TILE;
    let mut state = world_state(grid, x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.shrine_ordained_mask = 0;
    let turn_before = state.turn;
    let clock_before = state.clock;

    handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
    assert_ne!(state.codex_presentation_steps, 0);
    // §8.2: a no-ordained visit consumes three keys; none of them is a
    // command, and the turn is not paid while the reader is open.
    for _ in 0..3 {
        state.flush_staged_narration();
        assert_eq!(state.turn, turn_before);
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    }
    assert_eq!(state.codex_presentation_steps, 0);
    // The exit walk-back drains, then the next prompt's gate runs the epilogue.
    state.flush_staged_narration();
    let _ = state.apply_exploration_turn_gate(&dir).unwrap();
    assert_eq!(state.turn, turn_before + 1);
    assert_ne!(state.clock, clock_before);
    // Charged once only.
    let _ = state.apply_exploration_turn_gate(&dir).unwrap();
    assert_eq!(state.turn, turn_before + 1);
    let _ = fs::remove_dir_all(dir);
}
