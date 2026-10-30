// Regressions for the pass-23 exploratory playtest findings (gate23).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

fn gate23_britannia_night_state() -> PlayState {
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.clock = GameClock::new(22, 0).unwrap();
    state.natural_moongate_counter = NATURAL_MOONGATE_COUNTER_MAX;
    state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    state.party[0].mana = 9;
    state.party[0].level = 8;
    for slot in state.moonstone_slots.iter_mut() {
        *slot = MoonstoneGateSlot {
            scene: MOONSTONE_INVALID_SCENE,
            x: 0,
            y: 0,
            z: 0,
        };
    }
    state
}

/// Playtest P23-02. `shops.md §8.7`: "The next overworld entry consumes that
/// state and places the watercraft", and "The delivery pass runs once before
/// the first overworld turn after the purchase". The Gate Travel warp to an
/// overworld slot is an overworld entry (`magic.md`, Gate travel row), but it
/// rebuilt the scene with no pending vehicle, so the frigate stayed queued
/// until a later load or town-edge exit. A dungeon exit through
/// `restore_world_at` skipped the delivery the same way.
#[test]
fn playtest_p23_02_gate_travel_and_dungeon_exit_deliver_a_pending_vessel() {
    let dir = debug_game_dir();
    let frigate = PendingVehicleAcquisition::Frigate {
        x: 79,
        y: 109,
        skiffs: 2,
    };
    let delivered = |state: &PlayState| {
        state
            .active_objects
            .iter()
            .skip(1)
            .any(|object| (object.type_byte, object.x, object.y) == (0x25, 79, 109))
    };

    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.pending_vehicle_save = PendingVehicleSaveState::from_acquisition(frigate);
    state
        .apply_gate_travel(&dir, 2, PlayTarget::World(WorldPlane::Britannia), 0, (100, 100))
        .unwrap();
    assert!(delivered(&state), "{:?}", state.active_objects);
    assert_eq!(state.pending_vehicle_save.acquisition(), None);

    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.pending_vehicle_save = PendingVehicleSaveState::from_acquisition(frigate);
    state
        .restore_world_at(&dir, WorldPlane::Britannia, 10, 10)
        .unwrap();
    assert!(delivered(&state), "{:?}", state.active_objects);
    assert_eq!(state.pending_vehicle_save.acquisition(), None);
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P23-03. `magic.md`, Gate travel row: the cast "teleports only if
/// that slot has a valid saved scene/X/Y/Z destination", otherwise the
/// ordinary `Failed!`. A slot naming CASTLE:0 floor 5 - outside its published
/// floor run - reached the scene loader and ended the program with an error.
#[test]
fn playtest_p23_03_gate_travel_to_an_out_of_range_floor_fails_cleanly() {
    let dir = debug_game_dir();
    // Use the published floor-page table, not the fixture's override.
    let _ = fs::remove_file(dir.join(LOCATION_FLOOR_TABLE_FILE));
    let mut state = gate23_britannia_night_state();
    state.clock = GameClock::new(10, 0).unwrap();
    state.natural_moongate_counter = 0;
    state.moonstone_slots[0] = MoonstoneGateSlot {
        scene: 17,
        x: 15,
        y: 15,
        z: 5,
    };

    handle_play_key_input(&mut state, 'C', "1PRV1", &dir).unwrap();

    assert!(matches!(state.area, Area::World { .. }), "{:?}", state.area);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.message.contains("Failed!"), "{:?}", state.message);
    let _ = fs::remove_dir_all(dir);
}
