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

/// Playtest P23-01. `overworld.md §5` step 1: overworld entry makes
/// zero-minute clock calls and chunk reloads, and "none of it is a world
/// tick"; §9 places gates in "the once-per-turn refresh" of the world tick.
/// The draw-order list runs the moongate hook (item 4) before the input
/// helper's entry tick (item 5), so a Gate Travel at night onto a slot cell in
/// a freshly loaded window finds grass, and the player gets one command there
/// before the §9.2 hook transits. The engine stamped the gate during the
/// arrival and transited at once, before any key was read.
#[test]
fn playtest_p23_01_night_gate_travel_arrival_gets_one_command_before_the_hook() {
    let dir = debug_game_dir();
    let mut state = gate23_britannia_night_state();
    // Phase 2 lands on a slot cell far outside the starting window.
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 100,
        y: 100,
        z: 0,
    };
    // The natural gate would lead to an empty slot: the transit plays and
    // warps nowhere, so its effect is visible on the cell alone.
    state.set_cached_moon_glyph_slots(2, 2);
    let idx = world_cell_index(100, 100);

    handle_play_key_input(&mut state, 'C', "1PRV2", &dir).unwrap();

    assert_eq!((state.player.x, state.player.y), (100, 100));
    assert!(
        state.last_natural_moongate_transit.is_none(),
        "no transit before the first command at the destination"
    );
    assert_eq!(state.turn, 0, "the successful cast reports no action");

    // The first command is read and dispatched; the hook then sees the gate.
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert_eq!(state.turn, 1, "the Pass was dispatched as a command");
    assert!(
        state.last_natural_moongate_transit.is_some(),
        "the hook fires on the loop pass after that command"
    );
    assert_eq!(state.grid[idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
    assert!(!state.natural_moongate_hook_held_for_warp_arrival);
    let _ = fs::remove_dir_all(dir);
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
