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

/// E22: `vehicles.md §4` B-Board probes only the party's own cell and §5
/// says "a vehicle left behind must remain boardable through active-object
/// persistence". The party must be able to step onto a parked vehicle's
/// cell - including the water under a frigate - and then board it.
#[test]
fn playfix_e22_party_steps_onto_parked_vehicles_and_boards_them() {
    for (object_byte, under) in [
        (SHIP_PARKED_FIRST, BRIT_DEEP_WATER_TILE),
        (SKIFF_PARKED_FIRST + 3, BRIT_DEEP_WATER_TILE),
        (HORSE_PARKED_FIRST, 5),
        (CARPET_PARKED, 5),
    ] {
        let mut grid = open_world_grid();
        grid[world_cell_index(6, 5)] = under;
        let mut state = world_state(grid, 5, 5);
        state.active_objects.push(ActiveObject {
            type_byte: object_byte,
            tile: object_byte,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 60,
            aux3: 1,
        });
        assert_eq!(state.step(Direction::East), MoveOutcome::Moved, "{object_byte:#04x}");
        assert_eq!((state.player.x, state.player.y), (6, 5));
        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded, "{object_byte:#04x}");
    }
}

/// E22: the round trip of the finding - park a frigate with X-it, step off
/// in its skiff, step back onto it and board it again.
#[test]
fn playfix_e22_frigate_left_by_skiff_can_be_reboarded() {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
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
    assert_eq!(state.step(Direction::South), MoveOutcome::Moved);
    assert_eq!(state.step(Direction::North), MoveOutcome::Moved);
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert!(matches!(
        state.player.transport,
        TransportState::Ship { hull: 77, skiffs: 1, .. }
    ));
}

/// E22: a parked vehicle the current transport cannot board still blocks
/// (a mounted party cannot board a second horse, `vehicles.md §4`).
#[test]
fn playfix_e22_unboardable_parked_vehicle_still_blocks() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport =
        transport_from_vehicle_object(HORSE_PARKED_FIRST, HORSE_PARKED_FIRST, 0, 0).unwrap();
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
        tile: HORSE_PARKED_FIRST,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
    assert_eq!((state.player.x, state.player.y), (5, 5));
}

/// E23: `blackthorn.md §7` "Party restoration": per in-party slot the
/// cinematic revives, copies maximum into current hit points "and repaints
/// the whole stats panel, before moving to the next slot". The panel must
/// not keep the dead roster once the rescue has restored the party.
#[test]
fn playfix_e23_rescue_restoration_repaints_the_stats_panel() {
    let mut state = test_state(open_grid(), 1, 1);
    for member in &mut state.party {
        member.hp = 0;
        member.status = CharacterStatus::Dead.save_byte();
    }
    state.repaint_stats_panel();
    assert_eq!(
        state.displayed_stats_panel().statuses[0],
        Some(CharacterStatus::Dead.save_byte())
    );
    state.pending_blackthorn_rescue =
        Some(crate::blackthorn::BlackthornRescuePhase::AwaitingAcknowledgement);
    let game_dir = std::path::Path::new("/nonexistent");
    assert_eq!(state.step_blackthorn_rescue(game_dir, true).unwrap(), None);
    state.flush_staged_narration();

    let good = CharacterStatus::Good.save_byte();
    assert_eq!(state.party[0].status, good);
    assert_eq!(state.displayed_stats_panel().statuses[0], Some(good));
    assert_eq!(
        state.displayed_stats_panel(),
        &crate::stats_panel::stats_panel_snapshot_of(&state)
    );
}

/// E24: `overworld.md §9`: when the refresh stamps `0xDC`, "Any actual tile
/// change marks the viewport dirty and refreshes local light", and §9.1 draws
/// the cell by its counter, not by whether the party has moved. The stamp
/// must reach the live chunk window the viewport and light pass read.
#[test]
fn playfix_e24_gate_rising_while_party_stands_still_reaches_live_window() {
    let mut state = britannia_state(open_world_grid(), 4, 5);
    state.clock = GameClock::new(21, 0).unwrap();
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 5,
        y: 5,
        z: WorldPlane::Britannia.save_floor() as u8,
    };

    assert!(state.refresh_natural_moongates());

    assert_eq!(state.grid[world_cell_index(5, 5)], NATURAL_MOONGATE_TERRAIN_TILE);
    assert_eq!(state.world_live_tile_at(5, 5), NATURAL_MOONGATE_TERRAIN_TILE);
    assert_eq!(
        state.world_live_chunks.as_ref().unwrap().tile_at(5, 5),
        NATURAL_MOONGATE_TERRAIN_TILE
    );
}
