#[test]
fn exit_vehicle_skips_clean_plane_transition_landing_cells() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "UNDERWORLD 6 5 BRITANNIA 10 20\n",
    )
    .unwrap();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();

    assert_eq!(
        state.exit_vehicle_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::ExitedVehicle
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 168
            && object.x == 5
            && object.y == 5
            && object.z == WorldPlane::Underworld.save_floor()
    }));
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "ship!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exit_vehicle_skips_clean_waterfall_landing_cells() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_WATERFALL_TABLE_FILE),
        "UNDERWORLD 6 5 EAST 2 5\n",
    )
    .unwrap();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();

    assert_eq!(
        state.exit_vehicle_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::ExitedVehicle
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 168
            && object.x == 5
            && object.y == 5
            && object.z == WorldPlane::Underworld.save_floor()
    }));
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "ship!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exit_vehicle_ignores_retired_town_exit_tile_sidecar() {
    let dir = debug_game_dir();
    fs::write(dir.join("town_exit_tiles.tsv"), "CASTLE:0 0 2 1 16\n").unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.transport = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.sync_player_object();
    assert!(state.player_can_land_on_foot(Some(&dir), 2, 1).unwrap());

    assert_eq!(
        state.exit_vehicle_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::ExitedVehicle
    );

    assert_eq!(
        state.area,
        Area::Town {
            scene: Scene::new(17).unwrap(),
            floor: 0,
        }
    );
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 184 && object.x == 1 && object.y == 1 && object.z == 0
    }));
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "carpet!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exit_vehicle_skips_town_trap_door_landing_cells() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(TOWN_TRAP_DOOR_TABLE_FILE),
        "CASTLE:0 0 2 1 -1 16\n",
    )
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.transport = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.sync_player_object();

    assert_eq!(
        state.exit_vehicle_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::ExitedVehicle
    );

    assert_eq!(
        state.area,
        Area::Town {
            scene: Scene::new(17).unwrap(),
            floor: 0,
        }
    );
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 184 && object.x == 1 && object.y == 1 && object.z == 0
    }));
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "carpet!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exit_vehicle_skips_town_stair_landing_cells() {
    let mut grid = open_grid();
    grid[1 * 32 + 2] = 80;
    let mut state = test_state(grid, 1, 1);
    state.player.transport = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(
        state.area,
        Area::Town {
            scene: Scene::new(17).unwrap(),
            floor: 0,
        }
    );
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 184 && object.x == 1 && object.y == 1 && object.z == 0
    }));
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "carpet!");
}

#[test]
fn exit_vehicle_skips_clean_town_stair_sidecar_landing_cells() {
    let dir = debug_game_dir();
    fs::write(dir.join(TOWN_STAIR_TABLE_FILE), "CASTLE:0 0 2 1 UP 16\n").unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.transport = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.sync_player_object();

    assert_eq!(
        state.exit_vehicle_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::ExitedVehicle
    );

    assert_eq!(
        state.area,
        Area::Town {
            scene: Scene::new(17).unwrap(),
            floor: 0,
        }
    );
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 184 && object.x == 1 && object.y == 1 && object.z == 0
    }));
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "carpet!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exit_vehicle_furled_ship_without_landing_launches_carried_skiff() {
    // doors-and-z-transitions.md §11 / vehicles.md §5: a furled-ship exit
    // without nearby foot landing launches a carried skiff if available.
    // The hull stays parked at the original cell with one fewer skiff.
    let mut state = world_state(vec![1; WORLD_CELLS], 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert!(matches!(
        state.player.transport,
        TransportState::Skiff { .. }
    ));
    assert_eq!((state.player.x, state.player.y), (5, 5));
    let parked = state
        .active_objects
        .iter()
        .skip(1)
        .find(|object| object.type_byte == 168 && object.x == 5 && object.y == 5)
        .copied()
        .expect("ship hull should be parked at original cell");
    assert_eq!(parked.aux1, 77);
    assert_eq!(parked.aux3, 1);
    assert_eq!(state.message, "Launched a skiff from the ship.");
    assert_eq!(state.turn, 1);
}

#[test]
fn exit_vehicle_refuses_furled_ship_with_no_landing_and_no_skiffs() {
    // doors-and-z-transitions.md §11: when foot landing fails AND the
    // ship has no carried skiff, exit refuses with the no-land/no-skiffs
    // line and consumes no turn.
    let mut state = world_state(vec![1; WORLD_CELLS], 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 0,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::Blocked);

    assert_eq!(
        state.player.transport,
        TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 77,
            skiffs: 0,
        }
    );
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.active_objects.len(), 1);
    assert_eq!(state.message, SHIP_NO_SKIFFS_WARNING);
    assert_eq!(state.turn, 0);
}

#[test]
fn exit_vehicle_horse_dismounts_without_nearby_support() {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
    state.player.transport = TransportState::Horse {
        type_byte: FIRST_PLAYABLE_HORSE_TILE,
        tile: FIRST_PLAYABLE_HORSE_TILE,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == FIRST_PLAYABLE_HORSE_TILE && object.x == 5 && object.y == 5
    }));
    assert_eq!(state.message, "horse!");
    assert_eq!(state.turn, 1);
}

#[test]
fn exit_vehicle_carpet_accepts_passable_underfoot_without_nearby_support() {
    let mut grid = vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS];
    grid[world_cell_index(5, 5)] = 5;
    let mut state = world_state(grid, 5, 5);
    state.player.transport = TransportState::Carpet {
        type_byte: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == FIRST_PLAYABLE_MAGIC_CARPET_TILE && object.x == 5 && object.y == 5
    }));
    assert_eq!(state.message, "carpet!");
    assert_eq!(state.turn, 1);
}

#[test]
fn exit_vehicle_skiff_rejects_a_bridge_tile_underfoot_even_with_support() {
    // `vehicles.md`: the skiff's extra X-Xit rejection is the bridge tile
    // pair (0x6A/0x6B) directly underfoot. No water tile is rejected -
    // this test previously pinned deep water, which a skiff normally sits
    // on, and so blocked ordinary shoreline landings.
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = SKIFF_XIT_REJECTED_BRIDGE_FIRST;
    let mut state = world_state(grid, 5, 5);
    state.player.transport = TransportState::Skiff {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::Blocked);

    assert_eq!(
        state.player.transport,
        TransportState::Skiff {
            type_byte: FIRST_PLAYABLE_SKIFF_TILE,
            tile: FIRST_PLAYABLE_SKIFF_TILE,
        }
    );
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.active_objects.len(), 1);
    assert_eq!(state.message, "Not here!");
    assert_eq!(state.turn, 0);
}

#[test]
fn exit_vehicle_ship_can_use_nearby_vehicle_object_as_support_without_relocation() {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: 77,
        skiffs: 0,
    };
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == FIRST_PLAYABLE_FRIGATE_TILE && object.x == 5 && object.y == 5
    }));
    assert_eq!(state.message, format!("ship! {SHIP_NO_SKIFFS_WARNING}"));
    assert_eq!(state.turn, 1);
}

#[test]
fn exit_vehicle_furled_ship_without_support_redeploys_stowed_carpet() {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: 77,
        skiffs: 0,
    };
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(
        state.player.transport,
        TransportState::Carpet {
            type_byte: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
            tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        }
    );
    assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 0);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    let parked = state
        .active_objects
        .iter()
        .skip(1)
        .find(|object| object.type_byte == FIRST_PLAYABLE_FRIGATE_TILE)
        .copied()
        .expect("ship should park before carpet redeploy");
    assert_eq!((parked.x, parked.y), (5, 5));
    assert_eq!(parked.aux1, 77);
    assert_eq!(parked.aux3, 0);
    assert_eq!(
        state.message,
        "Redeployed stowed magic carpet from the ship."
    );
    assert_eq!(state.turn, 1);
}

#[test]
fn exit_vehicle_parks_boardable_object_and_returns_to_foot() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sail_cadence = 1;
    state.sail_stall_pending = true;
    state.sync_player_object();
    state.active_objects.push(ActiveObject::empty());
    state.active_objects.push(ActiveObject {
        type_byte: 194,
        tile: 194,
        x: 8,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.active_effect_timing_status(), TimingStatusTag::Normal);
    assert_eq!(state.sail_cadence, 0);
    assert!(!state.sail_stall_pending);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.active_objects[0].tile, PLAYER_TILE);
    assert_eq!(state.active_objects.len(), 3);
    assert_eq!(
        state.active_objects[1],
        ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 77,
            aux3: 2,
        }
    );
    assert_eq!(state.active_objects[2].type_byte, 194);
    assert_eq!(
        (state.active_objects[2].x, state.active_objects[2].y),
        (8, 5)
    );
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "ship!");
}

#[test]
fn exit_vehicle_parked_object_is_written_to_the_live_saved_gam_table() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0xff, 6, 5)).unwrap();
    write_empty_ool_mirrors(&dir);
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();
    state.active_objects.push(ActiveObject::empty());

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    let parked = ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 77,
        aux3: 2,
    };
    assert_eq!(state.active_objects[1], parked);

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
    let underworld = decode_ool_plane_objects(&saved_ool[OOL_PLANE_LEN..SAVED_OOL_LEN]).unwrap();
    assert!(underworld[0].is_empty());

    let saved_gam = fs::read(dir.join("SAVED.GAM")).unwrap();
    // vehicles.md §2: on foot the save always carries 0x1C -
    // the on-foot marker does not encode facing.
    assert_eq!(
        saved_gam[SAVE_TRANSPORT_MARKER_OFFSET],
        FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER
    );
    let saved_active = decode_active_object_table(
        &saved_gam[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
        "SAVED.GAM",
    )
    .unwrap();
    assert_eq!(saved_active[0], parked);

    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(options.target, PlayTarget::World(WorldPlane::Underworld));
    assert_eq!(options.start, Some((5, 5)));
    assert_eq!(options.transport, TransportState::Foot);
    assert_eq!(options.saved_active_objects.as_ref().unwrap()[0], parked);
    let reloaded = PlayState::load_scene(&dir, options).unwrap();
    assert_eq!(reloaded.player.transport, TransportState::Foot);
    assert_eq!(reloaded.active_objects[1], parked);
    assert_eq!(reloaded.world_object_at(5, 5), Some(&parked));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exit_ship_fallbacks_save_load_round_trip_vehicle_state() {
    let skiff_dir = debug_game_dir();
    write_save_template_and_empty_overlays(&skiff_dir, 0, 0xff, 5, 5);
    fs::write(
        skiff_dir.join(UNDER_DAT_FILENAME),
        vec![BRIT_DEEP_WATER_TILE; UNDER_DAT_LEN],
    )
    .unwrap();
    let mut skiff_state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
    skiff_state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    skiff_state.sync_player_object();

    assert_eq!(skiff_state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    let parked_with_one_less_skiff = ActiveObject {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 77,
        aux3: 1,
    };
    assert_eq!(skiff_state.active_objects[1], parked_with_one_less_skiff);
    assert!(matches!(
        skiff_state.player.transport,
        TransportState::Skiff { .. }
    ));

    assert_eq!(
        skiff_state
            .save_game_command(&skiff_dir, Some(true))
            .unwrap(),
        MoveOutcome::Saved
    );

    let skiff_options = load_play_options_from_save(&skiff_dir).unwrap();
    assert_eq!(
        skiff_options.target,
        PlayTarget::World(WorldPlane::Underworld)
    );
    assert_eq!(
        skiff_options.saved_active_objects.as_ref().unwrap()[0],
        parked_with_one_less_skiff
    );
    assert_eq!(
        skiff_options.transport,
        TransportState::Skiff {
            type_byte: TRANSPORT_MARKER_SKIFF_FIRST + 2,
            tile: FIRST_PLAYABLE_SKIFF_TILE + 2,
        }
    );
    let skiff_reloaded = PlayState::load_scene(&skiff_dir, skiff_options).unwrap();
    assert_eq!(skiff_reloaded.active_objects[1], parked_with_one_less_skiff);
    assert_eq!(
        skiff_reloaded.player.transport,
        TransportState::Skiff {
            type_byte: TRANSPORT_MARKER_SKIFF_FIRST + 2,
            tile: FIRST_PLAYABLE_SKIFF_TILE + 2,
        }
    );
    let _ = fs::remove_dir_all(skiff_dir);

    let carpet_dir = debug_game_dir();
    write_save_template_and_empty_overlays(&carpet_dir, 0, 0xff, 5, 5);
    let mut carpet_state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
    carpet_state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: 88,
        skiffs: 0,
    };
    carpet_state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;
    carpet_state.sync_player_object();

    assert_eq!(carpet_state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    let parked_before_carpet = ActiveObject {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 88,
        aux3: 0,
    };
    assert_eq!(carpet_state.active_objects[1], parked_before_carpet);
    assert_eq!(
        carpet_state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX],
        0
    );
    assert_eq!(
        carpet_state.player.transport,
        TransportState::Carpet {
            type_byte: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
            tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        }
    );

    assert_eq!(
        carpet_state
            .save_game_command(&carpet_dir, Some(true))
            .unwrap(),
        MoveOutcome::Saved
    );

    let carpet_options = load_play_options_from_save(&carpet_dir).unwrap();
    assert_eq!(
        carpet_options.saved_active_objects.as_ref().unwrap()[0],
        parked_before_carpet
    );
    assert_eq!(
        carpet_options.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX],
        0
    );
    assert_eq!(
        carpet_options.transport,
        TransportState::Carpet {
            type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
            tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        }
    );
    let carpet_reloaded = PlayState::load_scene(&carpet_dir, carpet_options).unwrap();
    assert_eq!(carpet_reloaded.active_objects[1], parked_before_carpet);
    assert_eq!(
        carpet_reloaded.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX],
        0
    );
    assert_eq!(
        carpet_reloaded.player.transport,
        TransportState::Carpet {
            type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
            tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        }
    );
    let _ = fs::remove_dir_all(carpet_dir);
}

#[test]
fn exit_ship_without_skiffs_reports_public_warning() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 0,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.active_objects.len(), 2);
    assert_eq!(
        state.active_objects[1],
        ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 77,
            aux3: 0,
        }
    );
    assert_eq!(state.message, format!("ship! {SHIP_NO_SKIFFS_WARNING}"));
}

#[test]
fn exit_ship_with_zero_hull_reports_badly_damaged_warning() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 0,
        skiffs: 2,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(
        state.active_objects[1],
        ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 2,
        }
    );
    assert_eq!(state.message, format!("ship! {SHIP_BADLY_DAMAGED_WARNING}"));
}

#[test]
fn ship_sails_toggle_and_block_exit_when_hoisted() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };

    assert_eq!(state.toggle_sails(), MoveOutcome::SailToggled);
    assert_eq!(
        state.player.transport,
        TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: true,
            hull: 77,
            skiffs: 2,
        }
    );
    assert_eq!(state.exit_vehicle(), MoveOutcome::Blocked);
    assert_eq!(state.turn, 1);
}

#[test]
fn ship_sail_toggle_resets_wind_cadence_and_stall_feedback() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sail_cadence = 1;
    state.sail_stall_pending = true;

    assert_eq!(state.toggle_sails(), MoveOutcome::SailToggled);

    assert_eq!(state.sail_cadence, 0);
    assert!(!state.sail_stall_pending);
    assert_eq!(state.message, "HOIST!");

    state.sail_cadence = 1;
    state.sail_stall_pending = true;

    assert_eq!(state.toggle_sails(), MoveOutcome::SailToggled);

    assert_eq!(state.sail_cadence, 0);
    assert!(!state.sail_stall_pending);
    assert_eq!(state.message, "FURL!");
    assert_eq!(state.turn, 2);
}

#[test]
fn under_sail_ship_auto_furls_only_on_exact_pier_tile() {
    let mut grid = open_world_grid();
    grid[world_cell_index(6, 5)] = OVERWORLD_PIER_TILE;
    let mut state = world_state(grid, 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 77,
        skiffs: 2,
    };
    state.wind = WindState::East;
    state.sail_cadence = u8::MAX;
    state.sail_stall_pending = true;

    assert_eq!(
        state
            .step_world(Direction::East, 6, 5, WorldPlane::Underworld, None)
            .unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(OVERWORLD_PIER_TILE, 0x47);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(!state.player.transport.is_ship_under_sail());
    assert_eq!(state.sail_cadence, 0);
    assert!(!state.sail_stall_pending);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, "Docked!");

    let mut ordinary_water_grid = open_world_grid();
    ordinary_water_grid[world_cell_index(6, 5)] = BRIT_DEEP_WATER_TILE;
    let mut neighbor = world_state(ordinary_water_grid, 5, 5);
    neighbor.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 77,
        skiffs: 2,
    };
    neighbor.wind = WindState::East;
    neighbor.sail_cadence = u8::MAX;

    assert_eq!(
        neighbor
            .step_world(Direction::East, 6, 5, WorldPlane::Underworld, None)
            .unwrap(),
        MoveOutcome::Moved
    );
    assert!(neighbor.player.transport.is_ship_under_sail());
}

#[test]
fn refused_hoisted_frigate_step_runs_exact_collision_payload_without_a_turn() {
    for (destination_tile, expected_message) in [(0x03, "BREAKING UP!"), (0x05, "COLLISION!")] {
        let mut grid = open_world_grid();
        grid[world_cell_index(6, 5)] = destination_tile;
        let mut state = world_state(grid, 5, 5);
        state.player.transport = TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
            tile: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
            sails_hoisted: true,
            hull: 100,
            skiffs: 0,
        };
        state.wind = WindState::East;
        state.sail_cadence = u8::MAX;
        state.sail_stall_pending = true;
        state.prng_state = 0x2468;
        let mut expected_prng = state.prng_state;
        let expected_roll = u5_prng_range_u16(
            &mut expected_prng,
            u16::from(OUTDOOR_IMPACT_HULL_ROLL_LOW),
            u16::from(OUTDOOR_IMPACT_HULL_ROLL_HIGH),
        ) as u8;
        let sound_serial = state.sound_effect_serial;

        assert_eq!(
            state
                .step_world(Direction::East, 6, 5, WorldPlane::Underworld, None)
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, expected_message);
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(
            state.player.transport,
            TransportState::Ship {
                type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
                tile: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
                sails_hoisted: true,
                hull: 100 - expected_roll,
                skiffs: 0,
            }
        );
        assert_eq!(
            state.sound_effects_after(sound_serial),
            vec![SoundEffect::ShipCollisionRumble]
        );
    }
}

#[test]
fn rough_seas_hits_each_non_dead_member_after_one_repaint_tick() {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = BRIT_DEEP_WATER_TILE;
    let mut state = world_state(grid, 5, 5);
    state.player.transport = TransportState::Skiff {
        type_byte: TRANSPORT_MARKER_SKIFF_FIRST,
        tile: TRANSPORT_MARKER_SKIFF_FIRST,
    };
    state.party = six_member_party(40);
    state.party[2].status = PARTY_STATUS_DEAD;
    state.party[2].hp = 0;
    state.prng_state = 0x2468;
    // `overworld.md §6.2.5` puts "one world repaint tick" between the
    // impact rumble and absorption and scores the whole ordered sequence at
    // "exactly `N` gameplay draws" - repeated by its conformance vector,
    // "consume exactly `N` gameplay draws in `1..8`". The repaint therefore
    // costs the gameplay stream nothing, so the expected stream position is
    // the absorption draws alone.
    let mut expected_prng = state.prng_state;
    let expected_rolls = [0usize, 1, 3, 4, 5]
        .map(|slot| {
            (
                slot,
                u5_prng_range_u16(
                    &mut expected_prng,
                    u16::from(OUTDOOR_IMPACT_MEMBER_DAMAGE_LOW),
                    u16::from(OUTDOOR_IMPACT_MEMBER_DAMAGE_HIGH),
                ) as u8,
            )
        })
        .to_vec();
    let animation_before = state.animation.frame;
    let sound_serial = state.sound_effect_serial;

    let Some(OutdoorImpactAbsorption::PartyDamaged(damage)) =
        state.apply_rough_seas_if_eligible()
    else {
        panic!("deep water under a skiff must run rough seas");
    };

    assert_eq!(
        damage
            .iter()
            .map(|entry| (entry.slot, entry.roll))
            .collect::<Vec<_>>(),
        expected_rolls
    );
    assert_eq!(state.party[2].hp, 0);
    assert_eq!(state.prng_state, expected_prng);
    assert_eq!(
        state.animation.frame,
        (animation_before + 1) % STATIC_TILE_ANIMATION_PERIOD_TICKS
    );
    assert_eq!(state.turn, 0, "rough seas never creates a second turn");
    assert_eq!(
        state.sound_effects_after(sound_serial),
        vec![
            SoundEffect::RoughSeasImpactRumble,
            SoundEffect::DamageRumble,
            SoundEffect::DamageRumble,
            SoundEffect::DamageRumble,
            SoundEffect::DamageRumble,
            SoundEffect::DamageRumble,
        ]
    );
    assert_eq!(
        state.message_entries().last().map(|entry| entry.text.as_str()),
        Some("Rough seas!")
    );
}

#[test]
fn rough_seas_requires_exact_deep_water_and_skiff_or_carpet_marker() {
    for (tile, transport, expected) in [
        (
            BRIT_DEEP_WATER_TILE,
            TransportState::Carpet {
                type_byte: CARPET_MARKER_FRAMES[0],
                tile: CARPET_MARKER_FRAMES[0],
            },
            true,
        ),
        (BRIT_DEEP_WATER_TILE, TransportState::Foot, false),
        (
            0x02,
            TransportState::Skiff {
                type_byte: TRANSPORT_MARKER_SKIFF_FIRST,
                tile: TRANSPORT_MARKER_SKIFF_FIRST,
            },
            false,
        ),
    ] {
        let mut grid = open_world_grid();
        grid[world_cell_index(5, 5)] = tile;
        let mut state = world_state(grid, 5, 5);
        state.player.transport = transport;
        assert_eq!(state.rough_seas_trigger_is_eligible(), expected);
    }
}

#[test]
fn y_yell_shipboard_toggles_sails_and_non_ship_prompts_without_turn() {
    let mut ship = world_state(open_world_grid(), 5, 5);
    ship.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };

    assert_eq!(
        handle_play_key_input(&mut ship, 'Y', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(ship.player.transport.is_ship_under_sail());
    assert_eq!(ship.message, "HOIST!");
    assert_eq!(ship.turn, 1);

    let mut foot = world_state(open_world_grid(), 5, 5);
    assert_eq!(
        handle_play_key_input(&mut foot, 'Y', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(foot.active_yell.is_some());
    assert!(foot.message.contains("Yell what?"));
    assert_eq!(foot.turn, 0);
}

#[test]
fn y_yell_shipboard_scene_gate_accepts_town_and_dungeon_bands() {
    let mut town = test_state(open_grid(), 5, 5);
    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    for state in [&mut town, &mut dungeon] {
        state.player.transport = TransportState::Ship {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
            skiffs: 2,
        };

        assert_eq!(state.start_yell_prompt(), MoveOutcome::SailToggled);
        assert!(state.player.transport.is_ship_under_sail());
        assert_eq!(state.message, YELL_SAILS_HOISTED_MESSAGE);
        assert_eq!(state.turn, 1);
        assert!(state.active_yell.is_none());
    }
}

#[test]
fn y_yell_shipboard_high_scene_byte_uses_the_word_prompt() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 2,
    };
    state.combat_active = true;

    assert_eq!(state.start_yell_prompt(), MoveOutcome::Observed);
    assert!(state.active_yell.is_some());
    assert!(state.message.contains("Yell what?"));
    assert_eq!(state.turn, 0);
    assert!(matches!(
        state.player.transport,
        TransportState::Ship {
            sails_hoisted: false,
            ..
        }
    ));

    assert_eq!(state.yell_command(Some("")), MoveOutcome::Used);
    assert_eq!(state.message, YELL_NOTHING_SAID_MESSAGE);
    assert_eq!(state.turn, 1);
    assert!(!state.player.transport.is_ship_under_sail());
}

#[test]
fn y_yell_empty_prompt_submission_is_acted_in_every_exploration_mode() {
    let states = [
        ("world", world_state(open_world_grid(), 5, 5)),
        ("town", test_state(open_grid(), 5, 5)),
        ("dungeon", dungeon_state(open_dungeon_record(), 0, 1, 1)),
    ];

    for (mode, mut state) in states {
        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue,
            "{mode} prompt dispatch"
        );
        assert!(state.active_yell.is_some(), "{mode} prompt missing");
        assert_eq!(state.turn, 0, "{mode} prompt must remain free");

        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue,
            "{mode} empty submission"
        );
        assert!(state.active_yell.is_none(), "{mode} prompt remained active");
        assert_eq!(state.turn, 1, "{mode} empty Yell was not acted");
        assert_eq!(state.message, YELL_NOTHING_SAID_MESSAGE, "{mode} result");
    }
}

#[test]
fn y_yell_wrong_scene_words_and_names_consume_turn_with_generic_no_effect() {
    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);

    assert_eq!(
        handle_play_key_input(&mut dungeon, 'Y', "fallax", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(dungeon.turn, 1);
    assert!(dungeon.message.contains("Yelled FALLAX"));
    assert!(dungeon.message.contains("Nothing happens."));
    assert!(!dungeon.message.contains("Word of Power"));

    let mut world = world_state(open_world_grid(), 5, 5);

    assert_eq!(
        handle_play_key_input(&mut world, 'Y', "faulinei", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(world.turn, 1);
    assert_eq!(world.message, "Yelled FAULINEI. Nothing happens.");
}

#[test]
fn y_yell_word_of_power_opens_matching_surface_seal_only_at_target() {
    let seal = word_of_power_seal_for_word("FALLAX").unwrap();
    let mut world = world_state(open_world_grid(), 241, 73);
    world.area = Area::World {
        // The horizontal coordinate predicate deliberately ignores plane.
        plane: WorldPlane::Underworld,
    };
    let idx = world_cell_index(240, 73);
    world.grid[idx] = WORD_OF_POWER_SEALED_TILE;
    world.refresh_world_live_chunks_for_current_area().unwrap();

    assert_eq!(world.yell_command(Some("fallax")), MoveOutcome::Used);

    assert_eq!(world.turn, 1);
    assert_eq!(world.grid[idx], seal.unsealed_tile);
    assert_eq!(
        world.word_of_power_seal_flags[0] & SAVE_QUEST_TILE_FLAG_HIGH_BIT,
        SAVE_QUEST_TILE_FLAG_HIGH_BIT
    );
    assert!(world.visibility_dirty);
    assert!(world.message.contains("A word of power is uttered"));
    assert!(world.message.contains("The seal opens."));

    assert_eq!(world.player.x, 241);
    assert_eq!(world.current_scene_byte(), SCENE_OVERWORLD);
    assert_eq!(world.world_live_tile_at(240, 73), seal.unsealed_tile);
    assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
    assert_eq!(world.grid[idx], WORD_OF_POWER_SEALED_TILE);
    assert_eq!(
        world.word_of_power_seal_flags[0] & SAVE_QUEST_TILE_FLAG_HIGH_BIT,
        0
    );
    assert!(world.message.contains("collapses shut"));

    let mut wrong_place = world_state(open_world_grid(), 5, 5);
    wrong_place.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    let wrong_idx = world_cell_index(4, 5);
    wrong_place.grid[wrong_idx] = WORD_OF_POWER_SEALED_TILE;
    assert_eq!(
        handle_play_key_input(&mut wrong_place, 'Y', "fallax", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(wrong_place.grid[wrong_idx], WORD_OF_POWER_SEALED_TILE);
    assert_eq!(wrong_place.word_of_power_seal_flags[0], 0);
    assert!(wrong_place.message.contains("A word of power is uttered"));
    assert!(wrong_place.message.contains("Nothing happens."));
}

#[test]
fn y_yell_veramocor_opens_underworld_doom_seal() {
    let seal = word_of_power_seal_for_word("VERAMOCOR").unwrap();
    let mut world = world_state(open_world_grid(), 129, 128);
    world.area = Area::World {
        plane: WorldPlane::Underworld,
    };
    let idx = world_cell_index(128, 128);
    world.grid[idx] = WORD_OF_POWER_SEALED_TILE;
    world.refresh_world_live_chunks_for_current_area().unwrap();

    assert_eq!(
        handle_play_key_input(&mut world, 'Y', "veramocor", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(world.grid[idx], seal.unsealed_tile);
    assert_ne!(world.word_of_power_seal_flags[7] & 0x80, 0);
    assert!(world.message.contains("Word of Power for Doom"));
    assert!(world.message.contains("The seal opens."));
}

#[test]
fn y_yell_word_target_scan_prefers_west_and_leaves_ruined_shrine_for_issue_135() {
    let seal = word_of_power_seal_for_word("FALLAX").unwrap();
    let mut world = world_state(open_world_grid(), 239, 73);
    let west = world_cell_index(238, 73);
    let east = world_cell_index(240, 73);
    world.grid[west] = WORLD_RUINED_SHRINE_TILE;
    world.grid[east] = WORD_OF_POWER_SEALED_TILE;
    world.refresh_world_live_chunks_for_current_area().unwrap();

    assert_eq!(
        world.open_word_of_power_seal(0, seal),
        WordOfPowerTargetOutcome::RuinedShrine { x: 238, y: 73 }
    );
    assert_eq!(world.grid[west], WORLD_RUINED_SHRINE_TILE);
    assert_eq!(world.grid[east], WORD_OF_POWER_SEALED_TILE);
    assert_eq!(world.word_of_power_seal_flags[0], 0);
    assert!(!world.visibility_dirty);
}

#[test]
fn y_yell_ruined_shrine_four_response_success_restores_only_shrine_state() {
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    let mut world = world_state(open_world_grid(), shrine_x + 1, shrine_y);
    world.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    let shrine_index = world_cell_index(shrine_x, shrine_y);
    world.grid[shrine_index] = WORLD_RUINED_SHRINE_TILE;
    world.shrine_ruin_flags[0] = 0x85;
    world.word_of_power_seal_flags[0] = 0xa7;
    world.visibility_dirty = false;

    assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
    assert_eq!(world.turn, 1);
    assert!(world.active_shrine_restoration.is_some());
    assert!(world.message.contains(SHRINE_RESTORATION_VIRTUE_PROMPT));
    assert_eq!(world.grid[shrine_index], WORLD_RUINED_SHRINE_TILE);
    assert_eq!(world.shrine_ruin_flags[0], 0x85);
    assert_eq!(world.word_of_power_seal_flags[0], 0xa7);

    assert_eq!(world.step_active_shrine_restoration('H', "onesty"), None);
    assert_eq!(world.step_active_shrine_restoration('A', "hm"), None);
    assert_eq!(world.step_active_shrine_restoration('x', "AHM x"), None);
    assert_eq!(
        world.step_active_shrine_restoration('a', "hm forever"),
        Some(MoveOutcome::Used)
    );

    assert!(world.active_shrine_restoration.is_none());
    assert_eq!(world.grid[shrine_index], WORLD_SHRINE_TILE);
    assert_eq!(world.shrine_ruin_flags[0], 0x05);
    assert_eq!(world.word_of_power_seal_flags[0], 0xa7);
    assert!(world.visibility_dirty);
    assert!(world.message.contains(SHRINE_RESTORATION_SUCCESS_BANNER));
    assert!(world.message.contains(&format!(
        "ahm forever{SHRINE_RESTORATION_SUCCESS_BANNER}{}",
        PlayState::word_of_power_presentation_message()
    )));
    assert_eq!(
        world
            .message
            .matches(PlayState::word_of_power_presentation_message())
            .count(),
        2
    );
}

#[test]
fn y_yell_ruined_shrine_failure_still_asks_all_fields_and_escape_only_clears() {
    let mut world = world_state(open_world_grid(), 11, 10);
    let ruined_index = world_cell_index(10, 10);
    world.grid[ruined_index] = WORLD_RUINED_SHRINE_TILE;
    world.shrine_ruin_flags[0] = 0x83;
    world.word_of_power_seal_flags[0] = 0x55;
    world.refresh_world_live_chunks_for_current_area().unwrap();

    assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
    world
        .active_shrine_restoration
        .as_mut()
        .unwrap()
        .buffer
        .push_str("Honesty");
    assert_eq!(world.step_active_shrine_restoration('\u{1b}', ""), None);
    let session = world.active_shrine_restoration.as_ref().unwrap();
    assert!(session.buffer.is_empty());
    assert_eq!(session.response_index, 0);

    // Enter now submits the cleared, failing virtue response, but the
    // three mantra questions still follow.
    assert_eq!(world.step_active_shrine_restoration('\r', ""), None);
    assert_eq!(world.step_active_shrine_restoration('A', "hm"), None);
    assert_eq!(world.step_active_shrine_restoration('A', "hm"), None);
    assert_eq!(
        world.step_active_shrine_restoration('A', "hm"),
        Some(MoveOutcome::Used)
    );

    assert!(world.active_shrine_restoration.is_none());
    assert_eq!(world.grid[ruined_index], WORLD_RUINED_SHRINE_TILE);
    assert_eq!(world.shrine_ruin_flags[0], 0x83);
    assert_eq!(world.word_of_power_seal_flags[0], 0x55);
    assert!(!world.message.contains("No effect!"));
    assert!(!world.message.contains(SHRINE_RESTORATION_SUCCESS_BANNER));
    assert!(world.message.ends_with("Ahm\n"));
    assert_eq!(
        world
            .message
            .matches(SHRINE_RESTORATION_MANTRA_PROMPT)
            .count(),
        3
    );
}

#[test]
fn y_yell_shadowlord_name_observes_vanquished_state() {
    let mut town = test_state(open_grid(), 5, 5);
    town.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 0,
    };
    town.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = SHADOWLORD_VANQUISHED;

    assert_eq!(
        handle_play_key_input(&mut town, 'Y', "faulinei", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(town.turn, 1);
    assert_eq!(town.message, "Yelled FAULINEI. Nothing happens.");
}

#[test]
fn y_yell_shadowlord_name_spawns_in_any_eternal_flame_keep() {
    let mut town = test_state(open_grid(), 5, 5);
    town.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 0,
    };
    // `time.md §7`: a new game seeds every Shadowlord slot to `0`
    // ("not yet placed"), so a test that needs a living Faulinei must
    // name a hideout. Id 4 is the Yew town scene byte.
    town.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 4;
    town.visibility_dirty = false;

    assert_eq!(
        handle_play_key_input(&mut town, 'Y', "faulinei", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(town.turn, 1);
    assert!(town.message.contains("Falsehood appears"));
    assert_eq!(town.active_objects.len(), OOL_SLOTS);
    assert_eq!(
        town.active_objects[OOL_SLOTS - 1],
        ActiveObject {
            type_byte: SHADOWLORD_OBJECT_TILE_BASE,
            tile: SHADOWLORD_OBJECT_TILE_BASE,
            x: 5,
            y: 3,
            z: 0,
            phase: active_object_phase_toward_player(0, -2),
            aux1: 0,
            aux3: 0,
        }
    );
    assert_eq!(town.summoned_shadowlord, Some(SHADOWLORD_FALSEHOOD_INDEX));
    assert!(town.visibility_dirty);

    let mut wrong_town = test_state(open_grid(), 5, 5);
    wrong_town.area = Area::Town {
        scene: Scene::new(2).unwrap(),
        floor: 0,
    };

    assert_eq!(
        handle_play_key_input(&mut wrong_town, 'Y', "faulinei", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(wrong_town.active_objects.len(), 1);
    assert_eq!(wrong_town.message, "Yelled FAULINEI. Nothing happens.");
}

#[test]
fn y_yell_shadowlord_name_refuses_when_no_active_object_slot_is_free() {
    let mut town = test_state(open_grid(), 5, 5);
    town.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 0,
    };
    town.active_objects.resize(
        OOL_SLOTS,
        ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 0,
            y: 0,
            z: 1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        },
    );
    town.visibility_dirty = false;

    assert_eq!(
        handle_play_key_input(&mut town, 'Y', "faulinei", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(town.turn, 1);
    assert!(
        town.message.contains("Nothing happens."),
        "unexpected message: {}",
        town.message
    );
    assert!(!town.visibility_dirty);
    assert_eq!(town.summoned_shadowlord, None);
    assert!(
        town.active_objects
            .iter()
            .skip(1)
            .all(|object| object.type_byte == 0x10)
    );
}

#[test]
fn shadowlord_name_allocation_detaches_an_empty_descriptor_from_stale_npc_ownership() {
    let mut town = test_state(open_grid(), 5, 5);
    town.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 0,
    };
    // `time.md §7`: a new game seeds every Shadowlord slot to `0`
    // ("not yet placed"), so a test that needs a living Faulinei must
    // name a hideout. Id 4 is the Yew town scene byte.
    town.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 4;
    town.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
    let acquired_slot = OOL_SLOTS - 1;
    let mut stale_owner = RuntimeNpc::from_slot(
        &NpcSlot {
            slot: 1,
            type_byte: 0x70,
            dialog_id: 0,
            schedule: [0; NPC_SCHEDULE_RECORD_LEN],
            name: None,
        },
        town.clock.hour,
    );
    stale_owner.active_object = Some(acquired_slot);
    town.npcs.push(stale_owner);

    assert_eq!(
        town.place_shadowlord_name_encounter(SHADOWLORD_FALSEHOOD_INDEX),
        Some(acquired_slot)
    );
    assert_eq!(town.npcs[0].active_object, None);
    assert!(PlayState::is_shadowlord_actor(
        town.active_objects[acquired_slot]
    ));
    assert_eq!(town.summoned_shadowlord, Some(SHADOWLORD_FALSEHOOD_INDEX));
    town.sync_player_object();
    assert!(PlayState::is_shadowlord_actor(
        town.active_objects[acquired_slot]
    ));
}

#[test]
fn shadowlord_helpers_track_living_vanquished_and_all_done() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.shadowlord_hideouts = [1, SHADOWLORD_VANQUISHED, 0x80];

    assert_eq!(
        PlayState::shadowlord_name_index("ASTAROTH"),
        Some(SHADOWLORD_HATRED_INDEX)
    );
    assert!(state.shadowlord_alive(SHADOWLORD_FALSEHOOD_INDEX));
    assert!(state.shadowlord_vanquished(SHADOWLORD_HATRED_INDEX));
    assert!(!state.shadowlord_alive(SHADOWLORD_COWARDICE_INDEX));
    assert!(!state.all_shadowlords_vanquished());

    assert!(state.vanquish_shadowlord(SHADOWLORD_FALSEHOOD_INDEX));
    assert_eq!(
        state.removed_town_npc_flags.get(&STONEGATE_SCENE_BYTE),
        Some(&(1 << SHADOWLORD_FALSEHOOD_STONEGATE_NPC_SLOT))
    );
    assert!(!state.vanquish_shadowlord(SHADOWLORD_FALSEHOOD_INDEX));
    assert_eq!(
        state.removed_town_npc_flags.get(&STONEGATE_SCENE_BYTE),
        Some(&(1 << SHADOWLORD_FALSEHOOD_STONEGATE_NPC_SLOT))
    );
    state.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX] = SHADOWLORD_VANQUISHED;

    assert!(state.all_shadowlords_vanquished());
}

#[test]
fn stonegate_entry_presentation_uses_sceptre_and_living_shadowlord_slots() {
    let mut state = test_state(open_grid(), 5, 5);
    state.area = Area::Town {
        scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
        floor: 0,
    };
    state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.shadowlord_hideouts = [1, SHADOWLORD_VANQUISHED, 0];
    state.message = "Entered KEEP:4.".to_string();

    state.append_stonegate_entry_presentation_message();

    assert!(state.message.contains("Sceptre prelude"));
    assert!(state.message.contains("air of Falsehood"));
    assert!(!state.message.contains("air of Hatred"));
    assert!(!state.message.contains("air of Cowardice"));

    state.area = Area::Town {
        scene: Scene::new(17).unwrap(),
        floor: 0,
    };
    assert_eq!(state.stonegate_entry_presentation_message(), None);
}

#[test]
fn removed_npc_mask_is_scene_wide_and_not_keyed_by_floor() {
    let scene = Scene::new(17).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    let slots = [
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 0x0E,
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
            name: None,
        },
    ];

    assert!(state.mark_removed_town_npc_once(scene, 1));
    assert!(!state.mark_removed_town_npc_once(scene, 1));
    state.area = Area::Town { scene, floor: 5 };
    state.load_scheduled_npcs(&slots);

    assert!(state.npcs.is_empty());
    assert_eq!(state.removed_town_npc_flags.get(&scene.byte), Some(&0b10));
}

#[test]
fn a_attack_prompts_for_direction_without_turn_or_movement() {
    let mut state = test_state(open_grid(), 5, 5);

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.turn, 0);
    assert!(state.active_direction_prompt.is_some());
    // `commands.md` section 5.4: "The shared direction prompt prints
    // **nothing** before waiting. The hyphen at the end of the verb echo
    // *is* the prompt." Section 5.2 publishes the surface literal as
    // `Attack-`, so the bespoke question this used to pin is withdrawn.
    assert_eq!(state.message, "Attack-");
}

#[test]
fn a_attack_inline_direction_consumes_turn_without_moving() {
    let mut state = test_state(open_grid(), 5, 5);

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "6", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "Attacked East at (6, 5); no target.");
}

#[test]
fn a_attack_adjacent_non_npc_object_reports_no_town_target() {
    let mut state = test_state(open_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: 42,
        tile: 42,
        x: 6,
        y: 5,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "6", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Attacked object tile 42"));
    assert!(state.message.contains("to the East"));
    assert!(state.message.contains("no attackable town NPC"));
    assert!(!state.message.contains("pending"));
    assert!(!state.message.contains("out of scope"));
}

#[test]
fn a_attack_adjacent_activation_mask_npc_removes_linked_runtime_actor() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    let slots = [
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 0x0E,
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
            name: None,
        },
    ];
    state.load_scheduled_npcs(&slots);
    let object_slot = state.npcs[0].active_object.unwrap();

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "6", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.turn, 1);
    assert!(state.npcs.is_empty());
    assert!(state.active_objects[object_slot].is_empty());
    assert_eq!(state.removed_town_npc_flags.get(&17), Some(&0b10));
    assert!(state.visibility_dirty);
    assert!(state.message.contains("Attacked NPC slot 1"));
    assert!(state.message.contains("type 0x0E"));
    assert!(state.message.contains("target removed"));
    // town-mode.md §14's NPC-conflict chain covers the location's
    // actors - the ordinary townsperson band and the guard group. Tag
    // 0x0E is the §4 story-object slot, not a combatant, so A-Attack on
    // it stays inside town mode and never frames an arena.
    assert!(!state.message.contains("combat"));
    assert!(!state.message.contains("pending"));

    state.load_scheduled_npcs(&slots);
    assert!(
        state.npcs.is_empty(),
        "removed NPC slot must not relink during the current scene visit"
    );
}

#[test]
fn a_attack_ordinary_town_npc_enters_the_arena_then_records_removal_on_exit() {
    // town-mode.md §14: "An earlier revision of this section also said
    // the town overlay 'does not call the combat framer or swap to a
    // .CBT arena'; that is withdrawn. The town overlay has a live
    // NPC-conflict chain, entered both from A-Attack and from
    // post-action cleanup, that hands the target NPC's linked
    // active-object slot to the same terrain-combat entry the overworld
    // uses, so a town fight is an ordinary arena fight: ordinary town
    // ground resolves to the cobble arena, and the scene-keyed
    // town-style override forces the monster count to one unless the
    // target's class is Guard ... On exit the town chain clears the NPC
    // slot, reloads the town map, and re-runs the Shadowlord install
    // pass of Section 13".
    let dir = debug_game_dir();
    let record = synthetic_combat_arena_record();
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    let slots = [
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 0x50,
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
            name: None,
        },
    ];
    state.load_scheduled_npcs(&slots);
    let object_slot = state.npcs[0].active_object.unwrap();

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "6", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.turn, 1);
    assert!(state.combat_active);
    assert_eq!(state.message, combat_banner_line());
    // "ordinary town ground resolves to the cobble arena" - selector
    // arena 8 (encounters.md §4: "anything else | 2 when the scene byte
    // is zero (overworld), otherwise 8").
    // "the scene-keyed town-style override forces the monster count to
    // one unless the target's class is Guard".
    assert_eq!(
        state.pending_town_conflict.map(|pending| pending.npc_slot),
        Some(1)
    );

    // "On exit the town chain clears the NPC slot" - and §4's
    // removal-mask policy still records an ordinary townsperson:
    // "killing a townsperson or a named character is permanent: that
    // slot is never placed again in that location."
    state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);
    assert!(!state.combat_active);
    assert!(state.npcs.is_empty());
    assert!(state.active_objects[object_slot].is_empty());
    assert_eq!(state.removed_town_npc_flags.get(&17), Some(&0b10));

    // "reloads the town map, and re-runs the Shadowlord install pass".
    assert!(state.drain_pending_town_conflict(&dir).unwrap());
    assert!(state.pending_town_conflict.is_none());

    state.load_scheduled_npcs(&slots);
    assert!(state.npcs.is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_attack_guard_like_town_npc_raises_alarm_and_opens_an_eight_monster_arena() {
    // town-mode.md §14: "the scene-keyed town-style override forces the
    // monster count to one unless the target's class is Guard (whose
    // stat row carries the sentinel count eight)." §4 still records
    // nothing for a guard: "Killing a guard or a monster is not recorded
    // at all, and those slots are placed again on the very next entry."
    let dir = debug_game_dir();
    let mut record = synthetic_combat_arena_record();
    // Arena 8 in the shipped bank is an all-cobble (`0x44`) town-conflict
    // arena. Keep this synthetic regression faithful to that collision case
    // and give every party slot an in-bounds authored seat.
    for row in 0..COMBAT_ARENA_SIDE {
        let row_start = row * COMBAT_ARENA_ROW_STRIDE;
        record[row_start..row_start + COMBAT_ARENA_SIDE].fill(0x44);
    }
    let party_positions = [(5u8, 7u8), (6, 8), (4, 8), (5, 9), (7, 9), (3, 9)];
    for (slot, (x, y)) in party_positions.into_iter().enumerate() {
        record[3 * COMBAT_ARENA_ROW_STRIDE + 11 + slot] = x;
        record[3 * COMBAT_ARENA_ROW_STRIDE + 17 + slot] = y;
    }
    for slot in 0..16 {
        record[6 * COMBAT_ARENA_ROW_STRIDE + 11 + slot] = 1 + (slot % 4) as u8 * 2;
        record[7 * COMBAT_ARENA_ROW_STRIDE + 11 + slot] = 1 + (slot / 4) as u8 * 2;
    }
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    let slots = [
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 0x70,
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
            name: None,
        },
    ];
    state.load_scheduled_npcs(&slots);
    let object_slot = state.npcs[0].active_object.unwrap();

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "6", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.turn, 1);
    assert!(state.combat_active);
    assert_eq!(state.message, combat_banner_line());
    assert_eq!(&state.npcs[0].schedule[..3], &[7, 7, 7]);
    assert_eq!(&state.npcs[0].schedule[12..16], &[0, 0, 0, 0]);

    // `combat.md` Section 12 (`RETRACTIONS.md` R336): a Guard brings its
    // class attack value of 30 flat, less the party's inclusive `1..7`
    // defence draw, so eight of them take a shipped party member apart in
    // one walk. This regression is about the alarm, the arena and the
    // transcript, not about surviving eight of them, so it holds all but
    // the first hostile off their phase (Section 7: an actor comes round
    // only when its phase counter reaches zero) and leaves the seated
    // member on the shipped roster's 60 HP, which one guard's flat 30 less
    // the party's `1..7` defence draw cannot end. (The freeze has to follow
    // combat entry, which seats the actors.)
    for slot in COMBAT_PARTY_ACTOR_SLOTS + 1..COMBAT_ACTOR_SLOTS {
        state.combat_actors[slot].phase_counter = u8::MAX;
    }
    assert_eq!(
        state.party[0].hp, DEFAULT_PARTY_HP,
        "the fixture must exercise a shipped roster HP, not an inflated one"
    );

    let walk = state
        .ensure_pending_combat_player_turn()
        .expect("guard combat must advance to a player-controlled actor");
    assert_eq!(walk.stop_reason, CombatRoundWalkStopReason::AwaitingPlayer);
    let actor_slot = state
        .pending_combat_actor_slot
        .expect("guard combat exposes a player actor");

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    // `combat.md §8.2` (`RETRACTIONS.md` R309): `A` prints `Attack-` and
    // then `Aim! ` immediately before the targeting cursor opens.
    assert_eq!(state.message, "Attack-Aim! ");
    assert_eq!(state.pending_combat_actor_slot, Some(actor_slot));

    // `combat.md §8.2`: the cursor owns the keystroke. A direction code
    // steers the cursor and leaves the attacker on its seat; Enter confirms.
    let seat = (
        state.combat_actors[actor_slot].x,
        state.combat_actors[actor_slot].y,
    );
    assert_eq!(
        handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        (
            state.combat_actors[actor_slot].x,
            state.combat_actors[actor_slot].y
        ),
        seat
    );
    assert_eq!(
        handle_play_key_input(&mut state, '\r', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_combat_targeting.is_none());
    // `combat.md §8.1`: the transcript ends with the turn banner for the
    // next keyboard-driven actor, printed "before any key is read". Trim it
    // (and its leading blank line) before asserting on the attack result.
    let result_lines = state
        .message
        .lines()
        .take_while(|line| !line.contains(COMBAT_TURN_BANNER_ARMED_WITH))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(result_lines.first().copied(), Some("Attack-Aim! Nothing!"));
    assert!(result_lines.len() > 1);
    assert!(
        result_lines
            .iter()
            .skip(1)
            .all(|line| *line == "Guard missed!" || *line == "Avatar hit!")
    );
    assert!(!state
        .message_entries()
        .iter()
        .any(|entry| entry.text.contains("BRIT.CBT") || entry.text.contains("combatant")));

    state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);
    assert!(!state.combat_active);
    assert!(state.npcs.is_empty());
    assert!(state.active_objects[object_slot].is_empty());
    // A guard's removal is never recorded in the per-scene mask.
    assert!(state.removed_town_npc_flags.is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_attack_adjacent_combat_class_object_selects_brit_cbt_arena() {
    let dir = debug_game_dir();
    let record = synthetic_combat_arena_record();
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: 0xc0,
        tile: 0xc0,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "6", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.turn, 1);
    assert!(state.combat_active);
    assert_eq!(state.pending_combat_terrain_trigger_slot, Some(1));
    assert_eq!(state.message, combat_banner_line());
    // `combat.md §5`: monster descriptors start at index six, but their
    // active-object records "continue from the first record left free by
    // the seated party", so "the descriptor's active-object link byte is
    // the authoritative pairing ... an engine should follow the link
    // rather than assume the two indexes are equal." The stock roster is
    // one live member, so the first monster's record is one.
    let first_monster = state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS];
    let first_monster_record = usize::from(first_monster.active_object_slot);
    assert_eq!(first_monster_record, 1);
    assert_eq!(state.active_objects[first_monster_record].tile, 0xc0);
    assert_eq!(
        (
            state.active_objects[first_monster_record].x,
            state.active_objects[first_monster_record].y
        ),
        (0, 15)
    );
    assert!(!state.message.contains("pending"));
    assert!(!state.message.contains("out of scope"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_attack_reports_published_base_combat_class_from_sprite_run() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: 0xc0,
        tile: 0xc0,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.attack_command(Some(Direction::East)),
        MoveOutcome::Used
    );

    assert!(state.message.contains("selected BRIT.CBT arena 2"));
    assert!(state.message.contains("base class Orc (32)"));
}

#[test]
fn ship_fire_requires_ship_and_inline_broadside_direction_without_turn() {
    let mut foot = world_state(open_world_grid(), 5, 5);

    assert_eq!(
        foot.fire_ship_broadside(Some(Direction::North)),
        MoveOutcome::Blocked
    );

    assert_eq!(foot.message, "What?");
    assert_eq!(foot.turn, 0);

    let mut ship = world_state(open_world_grid(), 5, 5);
    ship.player.facing = Direction::East;
    ship.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 0,
        skiffs: 0,
    };
    ship.sync_player_object();

    assert_eq!(ship.fire_ship_broadside(None), MoveOutcome::Blocked);
    assert!(ship.message.contains("which direction"));
    assert_eq!(ship.turn, 0);

    assert_eq!(
        ship.fire_ship_broadside(Some(Direction::NorthEast)),
        MoveOutcome::Blocked
    );
    assert_eq!(ship.message, "Fire broadsides only!");
    assert_eq!(ship.turn, 0);

    assert_eq!(
        ship.fire_ship_broadside(Some(Direction::East)),
        MoveOutcome::Blocked
    );
    assert_eq!(ship.message, "Fire broadsides only!");
    assert_eq!(ship.turn, 0);

    assert_eq!(
        ship.fire_ship_broadside(Some(Direction::West)),
        MoveOutcome::Blocked
    );
    assert_eq!(ship.message, "Fire broadsides only!");
    assert_eq!(ship.turn, 0);
}

#[test]
fn ship_fire_broadside_removes_first_target_in_three_cell_trace() {
    let mut state = world_state(open_world_grid(), 10, 10);
    state.player.facing = Direction::East;
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 0,
        skiffs: 0,
    };
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 10,
        y: 8,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    state.active_objects.push(ActiveObject {
        type_byte: 194,
        tile: 194,
        x: 10,
        y: 7,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.fire_ship_broadside(Some(Direction::North)),
        MoveOutcome::Fired
    );

    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
    assert!(state.message.contains("BOOOM!"));
    assert!(state.message.contains("object tile 192"));
    assert_eq!(state.active_objects.len(), 3);
    assert!(state.active_objects[1].is_empty());
    assert!(state.world_object_at(10, 8).is_none());
    assert_eq!(state.active_objects[2].type_byte, 194);
    assert_eq!(
        (state.active_objects[2].x, state.active_objects[2].y),
        (10, 7)
    );
}

#[test]
fn ship_fire_removed_target_is_written_to_the_live_saved_gam_table() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0, 10, 10)).unwrap();
    write_empty_ool_mirrors(&dir);
    let mut state = britannia_state(open_world_grid(), 10, 10);
    state.player.facing = Direction::East;
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 1,
    };
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 10,
        y: 8,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.fire_ship_broadside(Some(Direction::North)),
        MoveOutcome::Fired
    );
    assert!(state.active_objects[1].is_empty());

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
    let britannia = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
    assert!(britannia[0].is_empty());

    let saved_gam = fs::read(dir.join("SAVED.GAM")).unwrap();
    let saved_active = decode_active_object_table(
        &saved_gam[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
        "SAVED.GAM",
    )
    .unwrap();
    assert!(saved_active[0].is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn ship_fire_damage_and_removal_save_load_round_trip_overlay_state() {
    let dir = debug_game_dir();
    write_save_template_and_empty_overlays(&dir, 0, 0, 10, 10);
    write_britannia_world_files(&dir, BRIT_DEEP_WATER_TILE);
    let mut state = britannia_state(open_world_grid(), 10, 10);
    state.player.facing = Direction::East;
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 1,
    };
    state.sync_player_object();
    let durable_target = ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 10,
        y: 8,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 100,
        aux3: 0,
    };
    let fragile_target = ActiveObject {
        type_byte: 194,
        tile: 194,
        x: 10,
        y: 12,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    state.active_objects.push(durable_target);
    state.active_objects.push(fragile_target);

    assert_eq!(
        state.fire_ship_broadside(Some(Direction::North)),
        MoveOutcome::Fired
    );
    let damaged_target = state.active_objects[1];
    assert_eq!(damaged_target.type_byte, durable_target.type_byte);
    assert!(damaged_target.aux1 < durable_target.aux1);
    assert_eq!(
        state.fire_ship_broadside(Some(Direction::South)),
        MoveOutcome::Fired
    );
    assert!(state.active_objects[2].is_empty());

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved_ool = fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap();
    let britannia = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
    assert!(britannia[0].is_empty());
    assert!(britannia[1].is_empty());
    let saved_gam = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    let saved_active = decode_saved_active_objects(&saved_gam).unwrap();
    assert_eq!(saved_active[0], damaged_target);
    assert!(saved_active[1].is_empty());

    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(options.target, PlayTarget::World(WorldPlane::Britannia));
    assert_eq!(options.transport, state.player.transport);
    assert_eq!(
        options.saved_active_objects.as_ref().unwrap()[0],
        damaged_target
    );
    assert!(options.saved_active_objects.as_ref().unwrap()[1].is_empty());
    let reloaded = PlayState::load_scene(&dir, options).unwrap();
    assert_eq!(reloaded.active_objects[1], damaged_target);
    assert!(reloaded.active_objects[2].is_empty());
    assert_eq!(reloaded.player.transport, state.player.transport);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn ship_fire_broadside_miss_still_consumes_turn() {
    let mut state = world_state(open_world_grid(), 10, 10);
    state.player.facing = Direction::East;
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 0,
        skiffs: 0,
    };
    state.sync_player_object();

    assert_eq!(
        state.fire_ship_broadside(Some(Direction::South)),
        MoveOutcome::Fired
    );

    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
    assert!(state.message.contains("no target in range"));
}

#[test]
fn parse_town_fire_source_entries_accepts_clean_rows() {
    let entries =
        parse_town_fire_source_entries("CASTLE:0 0 1 1 EAST 0x50\nCASTLE:0 1 2 1 WEST\n").unwrap();

    assert_eq!(
        entries,
        vec![
            TownFireSourceEntry {
                scene: Scene::new(17).unwrap(),
                floor: 0,
                x: 1,
                y: 1,
                direction: Direction::East,
                expected_tile: Some(0x50),
            },
            TownFireSourceEntry {
                scene: Scene::new(17).unwrap(),
                floor: 1,
                x: 2,
                y: 1,
                direction: Direction::West,
                expected_tile: None,
            },
        ]
    );
    assert!(parse_town_fire_source_entries("CASTLE:0 0 32 1 EAST\n").is_err());
    assert!(parse_town_fire_source_entries("DUNGEON:0 0 1 1 EAST\n").is_err());
}

#[test]
fn town_fire_source_requires_clean_sidecar_and_adjacent_source() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 0, 1);

    assert_eq!(
        state.fire_command(None, &dir).unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.message, "What?");
    assert_eq!(state.turn, 0);

    fs::write(
        dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
        "CASTLE:0 0 10 10 EAST\n",
    )
    .unwrap();
    assert_eq!(
        state.fire_command(None, &dir).unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, "What?");
    assert_eq!(state.turn, 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_fire_source_tile_guard_mismatch_refuses_after_pre_search_door_tick() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
        "CASTLE:0 0 1 1 EAST 0x50\n",
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32 + 1] = 0x51;
    grid[32 + 3] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    let mut state = test_state(grid, 0, 1);
    state.door_tracker = Some(DoorTracker {
        previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
        x: 3,
        y: 1,
        turns_remaining: 1,
    });

    assert_eq!(
        state.fire_command(None, &dir).unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.message, "What?");
    assert_eq!(state.turn, 0);
    assert_eq!(state.grid[32 + 3], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
    // The auto-close fired; the original keeps the four-byte block
    // resident afterwards rather than zeroing it.
    assert_eq!(
        state.door_tracker,
        Some(DoorTracker {
            previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
            x: 3,
            y: 1,
            turns_remaining: 0,
        })
    );
    assert!(state.door_tracker_closed);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_fire_uses_adjacent_static_cannon_without_sidecar() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 1] = TOWN_CANNON_TILE_FIRST + 1;
    grid[32 + 3] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    let mut state = test_state(grid, 0, 1);

    assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

    assert_eq!(state.grid[32 + 3], TOWN_DOOR_CLEARED_TILE);
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("BOOOM! Door destroyed!"));
    assert!(state.message.contains("fired East"));
    assert!(state.message.contains("destroyed door tile 184"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_fire_sidecar_source_takes_priority_over_adjacent_static_cannon() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
        "CASTLE:0 0 1 0 EAST\n",
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32] = TOWN_CANNON_TILE_FIRST + 1;
    grid[3] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    grid[32 + 3] = TOWN_DOOR_WINDOWED_UNLOCKED_TILE;
    let mut state = test_state(grid, 1, 1);

    assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

    assert_eq!(state.grid[3], TOWN_DOOR_CLEARED_TILE);
    assert_eq!(state.grid[32 + 3], TOWN_DOOR_WINDOWED_UNLOCKED_TILE);
    assert!(state.message.contains("fired East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_fire_source_destroys_first_door_in_clean_trace() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
        "CASTLE:0 0 1 1 EAST\n",
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32 + 3] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    let mut state = test_state(grid, 0, 1);
    state.visibility_dirty = false;
    state.door_tracker = Some(DoorTracker {
        previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
        x: 3,
        y: 1,
        turns_remaining: 1,
    });

    assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

    assert_eq!(state.grid[32 + 3], TOWN_DOOR_CLEARED_TILE);
    assert_eq!(state.door_tracker, None);
    assert_eq!(state.turn, 1);
    assert!(state.visibility_dirty);
    assert!(state.message.contains("BOOOM!"));
    assert!(state.message.contains("destroyed door tile 184"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_fire_source_bypasses_magic_lock_sidecar_for_door_target() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
        "CASTLE:0 0 1 1 EAST\n",
    )
    .unwrap();
    fs::write(
        dir.join(TOWN_LOCK_TABLE_FILE),
        "CASTLE:0 0 3 1 151 184 MAGIC\n",
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32 + 3] = TOWN_DOOR_MAGIC_PLAIN_TILE;
    let scene = Scene::new(17).unwrap();
    let mut state = test_state(grid, 0, 1);

    assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

    assert_eq!(state.grid[32 + 3], TOWN_DOOR_CLEARED_TILE);
    assert!(state.is_recorded_open_town_door(scene, 0, 3, 1));
    assert_eq!(state.keys, DEFAULT_KEY_STOCK);
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("destroyed door tile 151"));
    assert!(!state.message.contains("Magic lock"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_fire_destroyed_door_reverts_after_floor_reload() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    let mut pages = vec![16; 16 * 1024];
    let floor_zero = 5 * 1024;
    let floor_one = 6 * 1024;
    pages[floor_zero] = TOWN_KLIMB_ASCEND_TILE;
    pages[floor_zero + 32 + 3] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    pages[floor_one] = TOWN_KLIMB_DESCEND_TILE;
    fs::write(dir.join("CASTLE.DAT"), pages).unwrap();
    fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
    fs::write(
        dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
        "CASTLE:0 0 1 1 EAST\n",
    )
    .unwrap();
    let mut grid = open_grid();
    grid[0] = TOWN_KLIMB_ASCEND_TILE;
    grid[32 + 3] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    let mut state = test_state(grid, 0, 1);

    assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);
    assert_eq!(state.grid[32 + 3], TOWN_DOOR_CLEARED_TILE);
    assert!(state.is_recorded_open_town_door(scene, 0, 3, 1));

    state.player.x = 0;
    state.player.y = 0;
    state.sync_player_object();

    assert_eq!(
        state.climb(&dir, ClimbIntent::Up).unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
    );
    assert_eq!(
        state.climb(&dir, ClimbIntent::Down).unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 0 })
    );

    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert_eq!(state.grid[32 + 3], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
    assert!(!state.is_recorded_open_town_door(scene, 0, 3, 1));
    assert_eq!(state.door_tracker, None);
    assert_eq!(state.turn, 3);
    let _ = fs::remove_dir_all(dir);
}
