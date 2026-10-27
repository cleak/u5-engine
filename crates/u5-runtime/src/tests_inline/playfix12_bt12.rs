// Playtest round 12, Blackthorn batch (bt12). Each test names the published
// sentence it pins.

fn playfix12_palace_password_state() -> PlayState {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(SCENE_LORD_BLACKTHORNS_CASTLE).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: BLACKTHORN_GUARD_DEMAND_DIALOG_ID,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.active_effect_tag = Some(BLACK_BADGE_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = PERMANENT_ACTIVE_EFFECT_DURATION;
    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    assert!(state.blackthorn_guard_password_open());
    state
}

/// P12-01. `blackthorn.md §7a` branch 1: "a blank row and `Your response?`
/// above an input row starting at column zero, **without a colon** ... the
/// typed answer begins immediately at the left edge of that next row."
#[test]
fn playfix12_badge_password_reads_on_a_live_row_and_keeps_the_typed_word() {
    let mut state = playfix12_palace_password_state();
    assert!(state.message.ends_with("Your response?"), "{:?}", state.message);
    // The input row is live - not suppressed as a key wait - and is a
    // continuation: no blank above it, no end cap, no colon.
    assert!(!state.message_window_live_row_suppressed());
    assert!(selector_prompt_row_is_continuation(&state));
    assert!(state.typed_prompt_echo().is_none(), "no colon prefix");

    // An arrow is discarded by the free-text reader and the prompt waits.
    handle_play_key_input(&mut state, '\u{3}', "", Path::new("")).unwrap();
    assert!(state.blackthorn_guard_password_open());

    assert_eq!(
        state.resolve_blackthorn_guard_demand_input('i', "mpera\r", None),
        Some(MoveOutcome::Talked)
    );
    state.flush_message_slot();
    let rows = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    let response = rows
        .iter()
        .rposition(|row| row == "Your response?")
        .expect("question on the transcript");
    assert_eq!(
        &rows[response + 1..],
        &["IMPERA".to_string(), "\"Pass, friend!\"".to_string()],
        "{rows:?}"
    );
}

/// P12-01, the reader's Escape rule (`input.md §8`): Escape "clears the whole
/// current line but does **not** submit or terminate it".
#[test]
fn playfix12_badge_password_bare_escape_does_not_refuse() {
    let mut state = playfix12_palace_password_state();
    assert_eq!(
        state.resolve_blackthorn_guard_demand_input('\u{1b}', "", None),
        Some(MoveOutcome::PromptDeclined)
    );
    assert!(state.blackthorn_guard_password_open());
    assert!(state.pending_town_arrest.is_none());
}

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
