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

/// P12-09: `vehicles.md §5` - a furled frigate with no landing support
/// "launches into a skiff facing the same direction" by advancing the
/// marker one sprite run, so a furled east frigate `0x25` becomes skiff
/// `0x29`, whatever the party's separate facing field still holds.
#[test]
fn playfix12_p12_09_ship_to_skiff_launch_keeps_the_ship_facing() {
    let mut state = world_state(vec![1; WORLD_CELLS], 5, 5);
    state.player.facing = Direction::North;
    state.player.transport = TransportState::Ship {
        type_byte: 0x25,
        tile: transport_visual_tile_for_marker(0x25).unwrap(),
        sails_hoisted: false,
        hull: 8,
        skiffs: 1,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert!(matches!(state.player.transport, TransportState::Skiff { .. }));
    assert_eq!(state.player.transport.save_marker(), 0x29);
    assert_eq!(state.player.facing, Direction::East);
}

/// P12-12: `inventory.md §7.1` - U-Use "selects one of the two carpet
/// frames", markers `0x14`/`0x15`; `vehicles.md §5` - X-it then parks a
/// carpet object that "must remain boardable", which B-Board takes as
/// `Board carpet` (§4).
#[test]
fn playfix12_p12_12_used_carpet_parks_a_boardable_carpet_on_xit() {
    for seed in 0..4u16 {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.prng_state = state.prng_state.wrapping_add(seed);
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;

        assert_eq!(state.use_magic_carpet(), MoveOutcome::Boarded);
        let marker = state.player.transport.save_marker();
        assert!(CARPET_MARKER_FRAMES.contains(&marker), "{marker:#04x}");

        assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);
        assert!(state.active_objects.iter().skip(1).any(|object| {
            object.type_byte == CARPET_PARKED && object.x == 5 && object.y == 5
        }));

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
        assert_eq!(state.message, "carpet");
    }
}

/// P12-12: the furled-ship carpet redeploy writes a carpet marker too.
#[test]
fn playfix12_p12_12_ship_carpet_redeploy_parks_a_carpet_on_xit() {
    let mut state = world_state(vec![1; WORLD_CELLS], 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 0x25,
        tile: transport_visual_tile_for_marker(0x25).unwrap(),
        sails_hoisted: false,
        hull: 8,
        skiffs: 0,
    };
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);
    let marker = state.player.transport.save_marker();
    assert!(CARPET_MARKER_FRAMES.contains(&marker), "{marker:#04x}");
    let parked = state.player.transport.parked_object(5, 5, 0).unwrap();
    assert_eq!(parked.type_byte, CARPET_PARKED);
}
