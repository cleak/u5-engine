// Regressions for the town-shop exploratory play-test findings (H01..H47).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// H01: `shops.md §8.10` places "a boardable horse object for the party" on
/// an adjacent town cell, and `vehicles.md §4` B-Board "probes the current
/// position" (with a town-only `Nay!` availability check). The party must be
/// able to step onto that horse's cell in a town and board it there.
#[test]
fn playfix4_h01_party_steps_onto_a_town_horse_and_boards_it() {
    let mut state = test_state(open_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
        tile: HORSE_PARKED_FIRST,
        x: 5,
        y: 6,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(state.step(Direction::South), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (5, 6));
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert!(state.player.transport.is_horse());
}

/// H01: a mounted party still cannot walk onto a second town horse, and an
/// ordinary town object still blocks.
#[test]
fn playfix4_h01_unboardable_town_objects_still_block() {
    let mut state = test_state(open_grid(), 5, 5);
    state.player.transport =
        transport_from_vehicle_object(HORSE_PARKED_FIRST, HORSE_PARKED_FIRST, 0, 0).unwrap();
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
        tile: HORSE_PARKED_FIRST,
        x: 5,
        y: 6,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(state.step(Direction::South), MoveOutcome::Blocked);
    assert_eq!((state.player.x, state.player.y), (5, 5));
}
