// Regressions for the overworld exploratory play-test findings (E21..E31).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// Board the parked object `object_byte` from the party's own cell, then
/// X-it and return the record left behind.
fn playfix_board_then_exit(object_byte: u8, aux1: u8, aux3: u8) -> ActiveObject {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: object_byte,
        tile: object_byte,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1,
        aux3,
    });
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded, "{object_byte:#04x}");
    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle, "{object_byte:#04x}");
    *state
        .active_objects
        .iter()
        .skip(1)
        .find(|object| !object.is_empty())
        .expect("X-it parks a stand-in object")
}

/// E21: `vehicles.md §2` names `0x10/0x11`, `0x1B`, `0x24..0x27` and
/// `0x28..0x2B` as "the objects boarding consumes and exiting leaves
/// behind", and byte 1 of the record is the displayed-frame (actor) byte
/// (`active-objects.md`). The parked record must carry that object byte,
/// not the engine's internal visual index (which drew a frigate as a
/// chest and a horse as a slime).
#[test]
fn playfix_e21_xit_parks_vehicle_with_its_own_display_byte() {
    for (object_byte, parked_byte) in [
        (HORSE_PARKED_FIRST, HORSE_PARKED_FIRST),
        (HORSE_PARKED_FIRST + 1, HORSE_PARKED_FIRST + 1),
        (CARPET_PARKED, CARPET_PARKED),
        (SHIP_PARKED_FIRST + 2, SHIP_PARKED_FIRST + 2),
        (SKIFF_PARKED_FIRST + 1, SKIFF_PARKED_FIRST + 1),
    ] {
        let parked = playfix_board_then_exit(object_byte, 50, 1);
        assert_eq!(parked.type_byte, parked_byte, "{object_byte:#04x}");
        assert_eq!(parked.tile, parked_byte, "{object_byte:#04x} display byte");
        assert_eq!(
            actor_tile_for_byte(parked.tile),
            Some(ACTOR_TILE_BANK_BASE + parked_byte as usize)
        );
    }
}

/// E21: the furled-ship exit into a carried skiff parks the hull as the
/// furled-frigate object too.
#[test]
fn playfix_e21_ship_exit_into_skiff_parks_furled_frigate_art() {
    // All-water grid: no landing support, so the carried skiff launches.
    let mut state = world_state(vec![1; WORLD_CELLS], 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: SHIP_PARKED_FIRST,
        tile: SHIP_PARKED_FIRST,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 77,
        aux3: 2,
    });
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);
    assert!(matches!(state.player.transport, TransportState::Skiff { .. }));
    let parked = state
        .active_objects
        .iter()
        .skip(1)
        .find(|object| !object.is_empty())
        .copied()
        .unwrap();
    assert_eq!(parked.type_byte, SHIP_PARKED_FIRST);
    assert_eq!(parked.tile, SHIP_PARKED_FIRST);
    assert_eq!((parked.aux1, parked.aux3), (77, 1));
}
