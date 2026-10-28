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
