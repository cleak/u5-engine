// Regressions for the second overworld exploratory play-test batch
// (`over2`: F18..F23, F56). Each test names the finding it reproduces and
// the published sentence it rests on.

/// F23: `inventory.md §5` timing contract - outside combat R-Ready's "only
/// charge is Once in the post-action mode epilogue", and `commands.md §4`
/// has R and Z "always report the default 'acted'", consumed by the mode
/// loop after the handler returns. No clock minute and no world turn may
/// run while `Player: ` still waits; closing the exchange pays exactly one.
#[test]
fn playfix_f23_ready_and_z_stats_charge_after_their_prompts_close() {
    for key in ['R', 'Z'] {
        let mut state = world_state(open_world_grid(), 5, 5);
        let turn = state.turn;
        let clock = state.clock;
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert!(
            state.active_ready.is_some() || state.active_party_selector.is_some(),
            "{key}: the member prompt is open"
        );
        assert_eq!(state.turn, turn, "{key}: no world turn while `Player: ` waits");
        assert_eq!(state.clock, clock, "{key}: no clock minute while `Player: ` waits");

        handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
        assert!(state.active_ready.is_none() && state.active_party_selector.is_none());
        assert_eq!(state.turn, turn + 1, "{key}: the exchange costs one turn");
        assert_ne!(state.clock, clock, "{key}: the epilogue advances the clock");
        assert!(!state.exploration_acted_charge_owed);
    }
}

/// F23: a browsed Z-stats sheet is charged once, when the sheet closes, not
/// when the member is chosen.
#[test]
fn playfix_f23_z_stats_sheet_charges_on_exit() {
    let mut state = world_state(open_world_grid(), 5, 5);
    let turn = state.turn;
    handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert!(state.active_z_stats.is_some());
    assert_eq!(state.turn, turn);
    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    assert!(state.active_z_stats.is_none());
    assert_eq!(state.turn, turn + 1);
}
