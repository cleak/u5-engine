// Playtest round 12, Blackthorn batch (bt12). Each test names the published
// sentence it pins.

/// P12-14. `input.md §8`: "Any other byte (function key, direction code,
/// etc.) - discarded." The wish (`view.md` "the shared twelve-character text
/// input") and the audience answer (`blackthorn.md §4` "The bounded text
/// input then accepts the answer") are that reader.
#[test]
fn playfix12_free_text_readers_discard_direction_codes_and_del() {
    for key in ['\u{3}', '\u{1}', '\u{7f}'] {
        let mut state = world_state(open_world_grid(), 4, 5);
        state.active_wishing_well = Some(crate::z_stats::WishingWellSession {
            direction: Direction::North,
            coin_accepted: true,
        });
        state.message = "Thy wish?".to_string();
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert!(
            state.active_wishing_well.is_some(),
            "{key:?} must not submit the wish"
        );
        assert!(!state.message.contains("No effect"), "{:?}", state.message);
    }

    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state
        .active_blackthorn
        .as_ref()
        .is_some_and(|c| c.current_prompt().is_none())
    {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    let asked = state.active_blackthorn.as_ref().unwrap().current_prompt();
    state.flush_message_slot();
    let rows = state.message_entries().len();
    handle_play_key_input(&mut state, '\u{3}', "", &dir).unwrap();
    let challenge = state.active_blackthorn.as_ref().expect("still asking");
    assert_eq!(challenge.current_prompt(), asked);
    state.flush_message_slot();
    assert_eq!(state.message_entries().len(), rows, "nothing was submitted");
}
