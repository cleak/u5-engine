// Playtest round 12, vehicles (veh12) findings.

/// P12-07: `commands.md §3` - B forwards "acted" on every path, "all four
/// mounts and both refusals", except the unknown-target `What?`. The
/// `On foot` refusal therefore spends the ordinary outdoor turn.
#[test]
fn playfix12_p12_07_board_on_foot_refusal_spends_the_turn() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Horse {
        type_byte: 160,
        tile: 160,
    };
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: SKIFF_PARKED_FIRST,
        tile: SKIFF_PARKED_FIRST,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    let turn = state.turn;
    let clock = state.clock;
    assert!(state.handle_top_down_key_with_inline('B', &dir, None, None, None, None).unwrap());
    assert_eq!(state.message, BOARD_ON_FOOT_REFUSAL);
    assert_eq!(state.turn, turn + 1);
    let mut expected = clock;
    expected.advance_minutes(2);
    assert_eq!(state.clock, expected);
}

/// P12-07: the unknown-target `Board What?` stays free.
#[test]
fn playfix12_p12_07_board_what_stays_free() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    let turn = state.turn;
    assert!(state.handle_top_down_key_with_inline('B', &dir, None, None, None, None).unwrap());
    assert_eq!(state.message, "What?");
    assert_eq!(state.turn, turn);
}
