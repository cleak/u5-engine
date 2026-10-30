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
