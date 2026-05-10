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
        assert_eq!((state.player.x, state.player.y), (5, 6));
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
        assert_eq!((state.player.x, state.player.y), (5, 6));
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
    fn exit_vehicle_skips_active_moongate_origin_landing_cells() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.ambient_light = FULL_DAYLIGHT;
        state.moongates.push(MoongateEntry {
            x: 6,
            y: 5,
            destination_plane: WorldPlane::Britannia,
            destination_x: 10,
            destination_y: 20,
            active_hours: None,
            expected_tile: None,
        });
        state.player.transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        state.sync_player_object();

        assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!((state.player.x, state.player.y), (5, 6));
        assert!(state.pending_moongate.is_none());
        assert!(state.active_objects.iter().skip(1).any(|object| {
            object.type_byte == 184
                && object.x == 5
                && object.y == 5
                && object.z == WorldPlane::Britannia.save_floor()
        }));
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "carpet!");
    }

    #[test]
    fn exit_vehicle_skips_town_exit_tile_landing_cells() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_EXIT_TILE_TABLE_FILE), "CASTLE:0 0 2 1 16\n").unwrap();
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
        assert_eq!((state.player.x, state.player.y), (1, 2));
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
        assert_eq!((state.player.x, state.player.y), (1, 2));
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
        assert_eq!((state.player.x, state.player.y), (1, 2));
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
        assert_eq!((state.player.x, state.player.y), (1, 2));
        assert!(state.active_objects.iter().skip(1).any(|object| {
            object.type_byte == 184 && object.x == 1 && object.y == 1 && object.z == 0
        }));
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "carpet!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn exit_vehicle_refuses_without_foot_landing() {
        let mut state = world_state(vec![1; WORLD_CELLS], 5, 5);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 77,
            skiffs: 2,
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
                skiffs: 2,
            }
        );
        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.active_objects.len(), 1);
        assert_eq!(state.message, "Not here!");
        assert_eq!(state.turn, 0);
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
        assert_eq!(state.timing_status, TimingStatusTag::Normal);
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!((state.player.x, state.player.y), (6, 5));
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
    fn exit_vehicle_parked_object_is_included_in_saved_overworld_overlay() {
        let dir = debug_game_dir();
        fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0xff, 6, 5)).unwrap();
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
        let underworld =
            decode_ool_plane_objects(&saved_ool[OOL_PLANE_LEN..SAVED_OOL_LEN]).unwrap();
        assert_eq!(underworld[0], parked);

        let saved_gam = fs::read(dir.join("SAVED.GAM")).unwrap();
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
        let _ = fs::remove_dir_all(dir);
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
        assert_eq!(state.message, "Sails hoisted.");

        state.sail_cadence = 1;
        state.sail_stall_pending = true;

        assert_eq!(state.toggle_sails(), MoveOutcome::SailToggled);

        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.message, "Sails furled.");
        assert_eq!(state.turn, 2);
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
    fn ship_fire_removed_target_is_included_in_saved_overworld_overlay() {
        let dir = debug_game_dir();
        fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0, 10, 10)).unwrap();
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
            parse_town_fire_source_entries("CASTLE:0 0 1 1 EAST 0x50\nCASTLE:0 1 2 1 WEST\n")
                .unwrap();

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
    fn town_fire_source_tile_guard_mismatch_refuses_without_door_tick() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST 0x50\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 0x51;
        grid[32 + 3] = 96;
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: 96,
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
        assert_eq!(state.grid[32 + 3], 96);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 3,
                y: 1,
                turns_remaining: 1,
            })
        );
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
        grid[32 + 3] = 96;
        let mut state = test_state(grid, 0, 1);
        state.visibility_dirty = false;
        state.door_tracker = Some(DoorTracker {
            previous_tile: 96,
            x: 3,
            y: 1,
            turns_remaining: 1,
        });

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], 16);
        assert_eq!(state.door_tracker, None);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("BOOOM!"));
        assert!(state.message.contains("destroyed door tile 96"));
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
            "CASTLE:0 0 3 1 96 97 MAGIC\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 3] = 96;
        let scene = Scene::new(17).unwrap();
        let mut state = test_state(grid, 0, 1);

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], 16);
        assert!(state.is_recorded_open_town_door(scene, 0, 3, 1));
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("destroyed door tile 96"));
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
        pages[floor_zero] = 80;
        pages[floor_zero + 32 + 3] = 96;
        pages[floor_one] = 80;
        fs::write(dir.join("CASTLE.DAT"), pages).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        grid[32 + 3] = 96;
        let mut state = test_state(grid, 0, 1);

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);
        assert_eq!(state.grid[32 + 3], 16);
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
        assert_eq!(state.grid[32 + 3], 96);
        assert!(!state.is_recorded_open_town_door(scene, 0, 3, 1));
        assert_eq!(state.door_tracker, None);
        assert_eq!(state.turn, 3);
        let _ = fs::remove_dir_all(dir);
    }

