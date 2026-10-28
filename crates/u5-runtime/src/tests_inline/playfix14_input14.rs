// Regressions for the fourteenth-pass input exploratory-playtest findings
// (`input14`, P14-##).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.


/// P14-03: `input.md §6` folds a lowercase letter to upper case, and
/// `commands.md §5.4` has Jimmy and Look await a direction that "accepts
/// only the four directions and Space". A bare `j`, `J` followed by a
/// non-direction, and a bare `l` acted on the facing cell instead of opening
/// the prompt.
#[test]
fn playtest_p14_03_jimmy_and_look_without_a_direction_open_the_prompt() {
    let game_dir = Path::new("");
    for (key, suffix, expect_jimmy) in [('j', "", true), ('J', "x", true), ('l', "", false)] {
        let mut state = world_state(open_world_grid(), 5, 5);
        let turn_before = state.turn;

        handle_play_key_input(&mut state, key, suffix, game_dir).unwrap();

        let kind = state.active_direction_prompt.as_ref().map(|session| session.kind);
        if expect_jimmy {
            assert!(
                matches!(kind, Some(DirectionPromptKind::Jimmy)),
                "{key}{suffix}: Jimmy- waits for a direction, got {kind:?}"
            );
        } else {
            assert!(
                matches!(kind, Some(DirectionPromptKind::Look)),
                "{key}{suffix}: Look- waits for a direction, got {kind:?}"
            );
        }
        assert_eq!(state.turn, turn_before, "{key}{suffix}: nothing acted yet");
    }
}

/// P14-04: `dungeon-mode.md §15` - the dungeon's minute "sits at the head of
/// each iteration ... so a command the dispatcher reports as \"no action\"
/// ... still costs a minute underground". A function key answers the same
/// `What?` as `!`, so it costs the same minute.
#[test]
fn playtest_p14_04_dungeon_function_key_costs_the_loop_minute() {
    for key in [INPUT_CODE_F1, INPUT_CODE_F10] {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        let before = state.clock;
        handle_play_key_input(&mut state, char::from(key), "", Path::new("")).unwrap();
        assert_eq!(
            (state.clock.hour, state.clock.minute),
            (before.hour, before.minute + 1),
            "key {key:#04x}"
        );
        assert_eq!(state.turn, 0, "key {key:#04x} takes no turn");
    }
}

/// P14-05: `input.md §4` step 3 - an extended key that does not classify
/// "is returned as 'no key'" and the engine "continues polling". It never
/// reaches a reader, so it answers no `Exit to DOS?` and dismisses no view.
#[test]
fn playtest_p14_05_no_key_leaves_single_key_waits_open() {
    let game_dir = Path::new("");
    let mut state = world_state(open_world_grid(), 5, 5);
    handle_play_key_input(&mut state, PLAY_EXIT_TO_DOS_KEY, "", game_dir).unwrap();
    assert!(state.active_yes_no_prompt.is_some());
    let message_before = state.message.clone();
    handle_play_key_input(&mut state, PLAY_IGNORED_INPUT_KEY, "escape", game_dir).unwrap();
    assert!(
        state.active_yes_no_prompt.is_some(),
        "Exit to DOS? is still waiting"
    );
    assert_eq!(state.message, message_before);

    let mut state = world_state(open_world_grid(), 5, 5);
    state.gems = 3;
    handle_play_key_input(&mut state, 'V', "", game_dir).unwrap();
    assert!(state.active_view_overlay.is_some(), "the gem map is up");
    handle_play_key_input(&mut state, PLAY_IGNORED_INPUT_KEY, "escape", game_dir).unwrap();
    assert!(
        state.active_view_overlay.is_some(),
        "the gem map waits for an accepted key"
    );
}

/// P14-09: `inventory.md §4` - "Outside combat, Z starts with the normal
/// party-member selector, initially indicating the first slot ... the
/// currently active world member does not change the initial indication";
/// `commands.md §6`: New Order's second prompt "starts on the first slot".
#[test]
fn playtest_p14_09_selectors_start_on_the_first_slot_whatever_the_active_player() {
    let mut state = world_state(open_world_grid(), 5, 5);
    three_member_party(&mut state);
    state.active_player = Some(1);
    assert_eq!(state.z_stats_initial_party_index(), 0);

    state.start_party_selector(PartySelectorTarget::NewOrder { first: Some(2) });
    assert_eq!(
        state.active_party_selector.map(|session| session.highlight),
        Some(0)
    );
}
