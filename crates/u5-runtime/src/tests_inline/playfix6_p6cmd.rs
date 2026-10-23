// Regressions for the sixth-pass command exploratory-playtest findings
// (`p6cmd`, K04-K21).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `combat.md §8`: the arena's control codes are Escape, Ctrl-S, Ctrl-B,
/// Space, digits and directions, and "Anything else - the stock `What?`
/// refusal and a free re-prompt"; `commands.md §9`: "Combat replaces the
/// scheme entirely with its own parser". Playtest K15: Ctrl-V printed the
/// version banner and Ctrl-E opened `Exit to DOS?` mid-fight.
#[test]
fn playtest_k15_combat_refuses_the_exploration_control_bindings() {
    let game_dir = std::path::Path::new(".");
    for key in [
        PLAY_MORAL_STANDING_KEY,
        PLAY_VERSION_BANNER_KEY,
        PLAY_EXIT_TO_DOS_KEY,
    ] {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.active_player = Some(0);
        state.ensure_pending_combat_player_turn();
        let before = state.message_entries().len();

        let disposition = handle_play_key_input(&mut state, key, "", game_dir).unwrap();

        assert_eq!(disposition, PlayInputDisposition::Continue, "{key:?}");
        assert!(state.active_yes_no_prompt.is_none(), "{key:?}");
        assert!(state.combat_active, "{key:?}");
        assert_eq!(state.pending_combat_actor_slot, Some(0), "{key:?}");
        let rows: Vec<String> = state.message_entries()[before..]
            .iter()
            .map(|entry| entry.text.clone())
            .filter(|text| !text.is_empty())
            .collect();
        assert_eq!(rows, vec!["What?".to_string()], "{key:?}");
    }
}
