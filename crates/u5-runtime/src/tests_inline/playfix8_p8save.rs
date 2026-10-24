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

/// N23: `save-load.md §4.2` - "The load path recomputes and normalises no
/// restored byte" - and `formats/saved-gam.md §10`: the door auto-close X and
/// Y are "Restored verbatim", and Journey Onward "restores" the countdown.
/// Only town location setup clears the previous-tile gate
/// (`doors-and-z-transitions.md §5`). A load into the overworld or a dungeon
/// used to drop the whole block, so the next save wrote `00 00 00 00`.
#[test]
fn playtest_n23_an_overworld_load_keeps_the_door_tracker_bytes() {
    let tracker = DoorTracker {
        previous_tile: 0,
        x: 5,
        y: 6,
        turns_remaining: 0xfd,
    };
    let dir = debug_game_dir();
    let world = PlayState::load_world_scene(
        &dir,
        WorldPlane::Britannia,
        PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            start: Some((10, 20)),
            door_tracker: Some(tracker),
            ..PlayOptions::default()
        },
    )
    .unwrap();
    assert_eq!(world.door_tracker, Some(tracker), "overworld load");
    let _ = fs::remove_dir_all(dir);

    // An armed gate rides along too, and never fires outside a town.
    let armed = DoorTracker {
        previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
        turns_remaining: 1,
        ..tracker
    };
    let mut state = world_state(open_world_grid(), 5, 5);
    state.door_tracker = Some(armed);
    let grid_before = state.grid.clone();
    state.tick_door_tracker();
    assert_eq!(state.grid, grid_before, "the overworld map is untouched");
    assert_eq!(state.door_tracker, Some(armed));
}

/// N23, the stock half: leaving a town cancels a pending auto-close but
/// leaves the X, Y and countdown resident, so a save made on the overworld
/// afterwards still carries them.
#[test]
fn playtest_n23_town_exit_keeps_the_tracker_bytes_inert() {
    let mut state = test_state(open_grid(), 3, 3);
    state.door_tracker = Some(DoorTracker {
        previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
        x: 5,
        y: 6,
        turns_remaining: 2,
    });
    state.clear_town_visit_state();
    assert_eq!(
        state.door_tracker,
        Some(DoorTracker {
            previous_tile: 0,
            x: 5,
            y: 6,
            turns_remaining: 2,
        })
    );
}

/// N24: `formats/saved-gam.md §6` - `0x02EE` "Combat entry uses it to save
/// the pre-combat scene for later restore". A save made after a fight kept
/// whatever the template carried. Before any fight the template byte still
/// round-trips (§10: it "must round-trip exactly").
#[test]
fn playtest_n24_combat_entry_records_the_pre_combat_scene() {
    let dir = debug_game_dir();
    let mut template = saved_game_seed_bytes(0x11, 0, 3, 4);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    template[SAVE_SAVED_SCENE_SCRATCH_OFFSET] = 0x33;
    fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);

    let mut state = test_state(open_grid(), 3, 4);
    state.write_save_files(&dir).unwrap();
    let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(saved[SAVE_SAVED_SCENE_SCRATCH_OFFSET], 0x33, "no fight yet");

    state
        .enter_combat_frame(
            vec![ActiveObject::empty(); OOL_SLOTS],
            [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        )
        .unwrap();
    let snapshot = state.combat_frame_snapshot.take().unwrap();
    state.restore_combat_frame(snapshot);
    state.combat_active = false;
    state.write_save_files(&dir).unwrap();
    let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(
        saved[SAVE_SAVED_SCENE_SCRATCH_OFFSET], 0x11,
        "the town the fight was entered from"
    );
    let _ = fs::remove_dir_all(dir);
}
