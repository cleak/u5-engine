// Regressions for the eighth-pass save-file exploratory-playtest findings
// (`p8save`).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// N15: `input.md §6` - "Three things write the setting and nothing else
/// does" (Ctrl+B, the combat toggle, the free-text reader's save/restore) -
/// and `audio.md §3`: the sound setting "is not part of the saved game; a new
/// process starts with sound enabled again". A moongate transit and a
/// location entry rebuild the scene, and both used to arrive with type-ahead
/// off and sound on.
#[test]
fn playtest_n15_scene_rebuilds_keep_typeahead_and_sound() {
    let dir = debug_game_dir();

    // Location entry (`enter_world_target`).
    let scene = Scene::new(17).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 7;
    let mut state = world_state(grid, 10, 20);
    state.typeahead_buffer_enabled = true;
    state.music_enabled = false;
    assert_eq!(
        state
            .enter_world_target(&dir, WorldPlane::Underworld, PlayTarget::Town(scene), true)
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );
    assert!(state.typeahead_buffer_enabled, "entry keeps Buffer On");
    assert!(!state.music_enabled, "entry keeps Sound Off");

    // Gate travel (`apply_gate_travel`, the moongate and spell transit).
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.typeahead_buffer_enabled = true;
    state.music_enabled = false;
    state
        .apply_gate_travel(&dir, 0, PlayTarget::Town(scene), 0, (6, 7))
        .unwrap();
    assert!(matches!(state.area, Area::Town { .. }), "{:?}", state.area);
    assert!(state.typeahead_buffer_enabled, "transit keeps Buffer On");
    assert!(!state.music_enabled, "transit keeps Sound Off");
    let _ = fs::remove_dir_all(dir);
}

/// N22: `shops.md §8.7` delivery step 2 - the vehicle tile goes into "both
/// the object's type byte and its frame byte ... in both cases the east-facing
/// member of the family (facing index `1`)". `vehicles.md §2` puts east at
/// `1` in the low two bits, so the furled Frigate is `0x25` and the Skiff
/// `0x29`. Both used to be delivered facing north (`0x24`, `0x28`).
#[test]
fn playtest_n22_shipwright_delivery_faces_east() {
    let frigate = PendingVehicleAcquisition::Frigate {
        x: 79,
        y: 109,
        skiffs: 2,
    }
    .active_object(0);
    assert_eq!((frigate.type_byte, frigate.tile), (0x25, 0x25));
    assert_eq!((frigate.aux1, frigate.aux3), (99, 2));
    let skiff = PendingVehicleAcquisition::Skiff {
        x: 79,
        y: 109,
        aux3: 0,
    }
    .active_object(0);
    assert_eq!((skiff.type_byte, skiff.tile), (0x29, 0x29));
    // Boarding copies the object byte into the marker: the party boards
    // facing east.
    assert_eq!(
        transport_from_vehicle_object(frigate.type_byte, frigate.tile, frigate.aux1, frigate.aux3)
            .map(|transport| transport.save_marker()),
        Some(0x25)
    );
}
