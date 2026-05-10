    #[test]
    fn use_command_routes_inline_torch_and_gem_requests() {
        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon.torches = 1;

        assert_eq!(
            handle_play_key_input(&mut dungeon, 'U', "T", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(dungeon.torches, 0);
        assert!((112..=127).contains(&dungeon.torch_counter));
        assert_eq!(dungeon.turn, 1);
        assert!(dungeon.message.contains("Ignited a torch"));

        let mut world = britannia_state(open_world_grid(), 1, 1);
        world.gems = 1;

        assert_eq!(
            handle_play_key_input(&mut world, 'U', "G", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(world.gems, 0);
        assert_eq!(world.turn, 0);
        assert!(world.message.contains("Gem view of BRITANNIA"));
    }

    #[test]
    fn use_command_routes_inline_key_requests_to_lock_handlers() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 97 96\n").unwrap();
        fs::write(
            dir.join(DUNGEON_DOOR_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70 0xF2\n",
        )
        .unwrap();

        let mut town_grid = open_grid();
        town_grid[32 + 2] = 97;
        let mut town = test_state(town_grid, 1, 1);
        town.player.facing = Direction::East;
        town.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut town, 'U', "K", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(town.grid[32 + 2], 96);
        assert_eq!(town.turn, 1);
        assert!(town.visibility_dirty);
        assert_eq!(town.message, "Unlocked!");

        let mut dungeon_grid = open_dungeon_record();
        dungeon_grid[dungeon_cell_index(0, 1, 1)] = 0xF2;
        let mut dungeon = dungeon_state(dungeon_grid, 0, 1, 1);
        dungeon.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut dungeon, 'U', "J", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(dungeon.grid[dungeon_cell_index(0, 1, 1)], 0x70);
        assert_eq!(dungeon.turn, 1);
        assert!(dungeon.visibility_dirty);
        assert_eq!(dungeon.message, "Unlocked!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_push_uses_clean_sidecar_to_swap_target_into_destination() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_PUSHABLE_TABLE_FILE), "CASTLE:0 0 2 1 44\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 44;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.push_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Pushed
        );

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.grid[32 + 3], 44);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Pushed tile 44 East"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_push_refuses_missing_or_mismatched_sidecar_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = 44;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.push_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.turn, 0);

        fs::write(dir.join(TOWN_PUSHABLE_TABLE_FILE), "CASTLE:0 0 2 1 45\n").unwrap();
        assert_eq!(
            state.push_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.grid[32 + 2], 44);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to push there.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_push_consumes_turn_when_pushable_destination_is_blocked() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_PUSHABLE_TABLE_FILE), "CASTLE:0 0 2 1 44\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 44;
        grid[32 + 3] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.push_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[32 + 2], 44);
        assert_eq!(state.grid[32 + 3], 24);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Push blocked"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ship_transport_can_move_over_water_that_blocks_foot() {
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 0)] = 1;
        let mut foot = world_state(grid.clone(), 0, 0);

        assert_eq!(foot.step(Direction::East), MoveOutcome::Blocked);

        let mut ship = world_state(grid, 0, 0);
        ship.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 0,
            skiffs: 0,
        };
        ship.sync_player_object();

        assert_eq!(ship.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((ship.player.x, ship.player.y), (1, 0));
        assert_eq!(ship.active_objects[0].tile, 168);
    }

    #[test]
    fn hoisted_ship_stalls_in_calm_wind_and_consumes_turn() {
        let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: true,
            hull: 0,
            skiffs: 0,
        };
        state.sync_player_object();

        assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);

        assert_eq!((state.player.x, state.player.y), (10, 10));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert!(state.message.contains("calm wind"));
        assert!(state.sail_stall_pending);
    }

    #[test]
    fn pass_reports_and_clears_sail_stall_feedback() {
        let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: true,
            hull: 0,
            skiffs: 0,
        };
        state.sync_player_object();

        assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);
        assert!(state.sail_stall_pending);

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
        assert!(state.message.contains("stalled by the wind"));
        assert!(!state.sail_stall_pending);

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);
        assert_eq!(state.message, "Passed.");
    }

    #[test]
    fn hoisted_ship_advances_with_matching_wind() {
        let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
        state.wind = WindState::East;
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: true,
            hull: 0,
            skiffs: 0,
        };
        state.sync_player_object();

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (11, 10));
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
    }

    #[test]
    fn hoisted_ship_against_wind_uses_slow_cadence() {
        let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
        state.wind = WindState::East;
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: true,
            hull: 0,
            skiffs: 0,
        };
        state.sync_player_object();

        assert_eq!(state.step(Direction::West), MoveOutcome::SailStalled);
        assert_eq!((state.player.x, state.player.y), (10, 10));
        assert_eq!(state.turn, 1);

        assert_eq!(state.step(Direction::West), MoveOutcome::Moved);
        assert_eq!((state.player.x, state.player.y), (9, 10));
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
    }

    #[test]
    fn horse_world_movement_strides_two_cells_on_grass_and_path() {
        let mut grass = world_state(open_world_grid(), 0, 0);
        mount_horse(&mut grass);

        assert_eq!(grass.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((grass.player.x, grass.player.y), (2, 0));
        assert_eq!(grass.turn, 1);
        assert!(grass.message.contains("Rode East"));

        let mut path_grid = open_world_grid();
        path_grid[world_cell_index(1, 0)] = 16;
        path_grid[world_cell_index(2, 0)] = 20;
        let mut path = world_state(path_grid, 0, 0);
        mount_horse(&mut path);

        assert_eq!(path.step(Direction::East), MoveOutcome::Moved);
        assert_eq!((path.player.x, path.player.y), (2, 0));
    }

    #[test]
    fn horse_world_movement_uses_one_cell_on_rough_terrain() {
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 0)] = 7;
        let mut state = world_state(grid, 0, 0);
        mount_horse(&mut state);

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Moved East"));
    }

    #[test]
    fn horse_world_stride_stops_before_blocked_second_cell() {
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 0)] = 24;
        let mut state = world_state(grid, 0, 0);
        mount_horse(&mut state);

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn horse_world_stride_does_not_skip_first_cell_plane_transition() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 1 0 UNDERWORLD 10 20\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 0, 0);
        mount_horse(&mut state);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld,
            })
        );

        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn horse_world_stride_accepts_second_cell_plane_transition() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 2 0 UNDERWORLD 30 40\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 0)] = 24;
        let mut state = britannia_state(grid, 0, 0);
        mount_horse(&mut state);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld,
            })
        );

        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("F-A-L-L-S"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn horse_world_stride_accepts_second_cell_waterfall_sidecar() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_WATERFALL_TABLE_FILE),
            "UNDERWORLD 2 0 EAST 1 24\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 0)] = 24;
        let mut state = world_state(grid, 0, 0);
        mount_horse(&mut state);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (3, 0));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Rode East to (2, 0)"));
        assert!(
            state
                .message
                .contains("waterfall swept party 1 step(s) East")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_world_plane_transition_entries_accepts_optional_tile_guard() {
        let entries = parse_world_plane_transition_entries(
            "BRITANNIA 10 20 UNDERWORLD 30 40 0x18\nUNDERWORLD 30 40 BRITANNIA 10 20\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                WorldPlaneTransitionEntry {
                    from_plane: WorldPlane::Britannia,
                    x: 10,
                    y: 20,
                    to_plane: WorldPlane::Underworld,
                    to_x: 30,
                    to_y: 40,
                    expected_tile: Some(0x18),
                },
                WorldPlaneTransitionEntry {
                    from_plane: WorldPlane::Underworld,
                    x: 30,
                    y: 40,
                    to_plane: WorldPlane::Britannia,
                    to_x: 10,
                    to_y: 20,
                    expected_tile: None,
                }
            ]
        );
    }

    #[test]
    fn world_plane_transition_table_rejects_duplicate_source_coordinate_rows() {
        let text = "\
BRITANNIA 10 20 UNDERWORLD 30 40
BRITANNIA 10 20 UNDERWORLD 31 41
";

        assert!(parse_world_plane_transition_entries(text).is_err());
    }

    #[test]
    fn world_plane_transition_table_rejects_duplicate_destination_coordinate_rows() {
        let text = "\
BRITANNIA 10 20 UNDERWORLD 30 40
BRITANNIA 11 21 UNDERWORLD 30 40
";

        assert!(parse_world_plane_transition_entries(text).is_err());
    }

    #[test]
    fn world_plane_transition_table_requires_plane_change() {
        assert!(parse_world_plane_transition_entries("BRITANNIA 10 20 BRITANNIA 30 40\n").is_err());
    }

    #[test]
    fn parse_world_waterfall_entries_accepts_direction_steps_and_optional_tile_guard() {
        let entries =
            parse_world_waterfall_entries("BRITANNIA 10 20 EAST 3 1\nUNDERWORLD 1 2 north 1\n")
                .unwrap();

        assert_eq!(
            entries,
            vec![
                WorldWaterfallEntry {
                    plane: WorldPlane::Britannia,
                    x: 10,
                    y: 20,
                    direction: Direction::East,
                    steps: 3,
                    expected_tile: Some(1),
                },
                WorldWaterfallEntry {
                    plane: WorldPlane::Underworld,
                    x: 1,
                    y: 2,
                    direction: Direction::North,
                    steps: 1,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_world_waterfall_entries("BRITANNIA 10 20 EAST 0\n").is_err());
        assert!(parse_world_waterfall_entries("BRITANNIA 10 20 NORTHEAST 1\n").is_err());
        assert!(
            parse_world_waterfall_entries("BRITANNIA 10 20 EAST 1\nBRITANNIA 10 20 WEST 1\n")
                .is_err()
        );
    }

    #[test]
    fn parse_world_damage_tile_entries_accepts_lava_water_and_optional_tile_guard() {
        let entries = parse_world_damage_tile_entries(
            "BRITANNIA 10 20 LAVA 0x0e\nUNDERWORLD 1 2 water\nBRITANNIA 3 4 DROWNING 1\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                WorldDamageTileEntry {
                    plane: WorldPlane::Britannia,
                    x: 10,
                    y: 20,
                    effect: WorldDamageEffect::Lava,
                    expected_tile: Some(14),
                },
                WorldDamageTileEntry {
                    plane: WorldPlane::Underworld,
                    x: 1,
                    y: 2,
                    effect: WorldDamageEffect::Drowning,
                    expected_tile: None,
                },
                WorldDamageTileEntry {
                    plane: WorldPlane::Britannia,
                    x: 3,
                    y: 4,
                    effect: WorldDamageEffect::Drowning,
                    expected_tile: Some(1),
                },
            ]
        );
        assert!(parse_world_damage_tile_entries("BRITANNIA 10 20 ACID\n").is_err());
        assert!(
            parse_world_damage_tile_entries("BRITANNIA 10 20 LAVA\nBRITANNIA 10 20 LAVA\n")
                .is_err()
        );
    }

    #[test]
    fn parse_world_encounter_entries_accepts_clean_rows_and_rejects_bad_values() {
        let entries = parse_world_encounter_entries(
            "BRITANNIA 5 30 192 8 0\nUNDERWORLD 0x0e 12 255 -8 4 0x12\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                WorldEncounterEntry {
                    plane: WorldPlane::Britannia,
                    tile: 5,
                    threshold: 30,
                    type_byte: 192,
                    dx: 8,
                    dy: 0,
                    phase: active_object_phase_from_direction(Direction::West, 0),
                },
                WorldEncounterEntry {
                    plane: WorldPlane::Underworld,
                    tile: 14,
                    threshold: 12,
                    type_byte: 255,
                    dx: -8,
                    dy: 4,
                    phase: 0x12,
                },
            ]
        );
        assert!(parse_world_encounter_entries("BRITANNIA 5 31 192 8 0\n").is_err());
        assert!(parse_world_encounter_entries("BRITANNIA 5 30 160 8 0\n").is_err());
        assert!(parse_world_encounter_entries("BRITANNIA 5 30 192 0 0\n").is_err());
        assert!(
            parse_world_encounter_entries("BRITANNIA 5 30 192 8 0\nBRITANNIA 5 20 194 -8 0\n")
                .is_err()
        );
    }

    #[test]
    fn world_encounter_sidecar_spawns_one_actor_after_consumed_overworld_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_ENCOUNTER_TABLE_FILE),
            "BRITANNIA 5 30 192 2 0\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![5; WORLD_CELLS], 10, 10);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 192,
                tile: 192,
                x: 12,
                y: 10,
                z: WorldPlane::Britannia.save_floor(),
                phase: active_object_phase_from_direction(Direction::West, 0),
                aux1: 0,
                aux3: 0,
            }
        );
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Wandering encounter spawned"));
    }

    #[test]
    fn world_encounter_sidecar_respects_zero_threshold_and_blocked_spawn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_ENCOUNTER_TABLE_FILE),
            "BRITANNIA 5 0 192 2 0\n",
        )
        .unwrap();
        let mut zero_threshold = britannia_state(vec![5; WORLD_CELLS], 10, 10);

        assert_eq!(
            zero_threshold.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );
        assert_eq!(zero_threshold.active_objects.len(), 1);

        fs::write(
            dir.join(WORLD_ENCOUNTER_TABLE_FILE),
            "BRITANNIA 5 30 192 2 0\n",
        )
        .unwrap();
        let mut blocked = britannia_state(vec![5; WORLD_CELLS], 10, 10);
        blocked.active_objects.push(ActiveObject {
            type_byte: 194,
            tile: 194,
            x: 12,
            y: 10,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            blocked.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );
        assert_eq!(blocked.active_objects.len(), 2);
        assert_eq!(blocked.active_objects[1].type_byte, 194);
        assert!(!blocked.message.contains("Wandering encounter spawned"));
    }

    #[test]
    fn world_encounter_spawn_is_included_in_saved_overworld_overlay() {
        let dir = debug_game_dir();
        fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0, 10, 10)).unwrap();
        fs::write(
            dir.join(WORLD_ENCOUNTER_TABLE_FILE),
            "BRITANNIA 5 30 192 2 0\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![5; WORLD_CELLS], 10, 10);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );
        assert_eq!(state.active_objects[1].type_byte, 192);

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
        let britannia = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
        assert_eq!(britannia[0], state.active_objects[1]);

        let saved_gam = fs::read(dir.join("SAVED.GAM")).unwrap();
        let saved_active = decode_active_object_table(
            &saved_gam[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
            "SAVED.GAM",
        )
        .unwrap();
        assert_eq!(saved_active[0], state.active_objects[1]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_waterfall_sidecar_sweeps_after_successful_water_movement() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_WATERFALL_TABLE_FILE),
            "BRITANNIA 1 0 EAST 3 1\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 20,
            skiffs: 1,
        };
        state.sync_player_object();

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (4, 0));
        assert_eq!(
            (state.active_objects[0].x, state.active_objects[0].y),
            (4, 0)
        );
        assert_eq!(state.turn, 1);
        assert!(
            state
                .message
                .contains("waterfall swept party 3 step(s) East")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_waterfall_sweep_stops_before_clean_lava_sidecar_for_non_carpet() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_WATERFALL_TABLE_FILE),
            "BRITANNIA 1 0 EAST 3 1\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 3 0 LAVA 1\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 20,
            skiffs: 1,
        };
        state.sync_player_object();

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (2, 0));
        assert_eq!(state.turn, 1);
        assert!(
            state
                .message
                .contains("waterfall swept party 1 step(s) East")
        );
        assert!(!state.message.contains("lava damage"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_waterfall_sweep_applies_clean_plane_transition() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_WATERFALL_TABLE_FILE),
            "BRITANNIA 1 0 EAST 3 1\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 3 0 UNDERWORLD 30 40\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 20,
            skiffs: 1,
        };
        state.sync_player_object();

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld,
            })
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(
            state.active_objects[0].z,
            WorldPlane::Underworld.save_floor()
        );
        assert_eq!(state.turn, 1);
        assert!(
            state
                .message
                .contains("waterfall swept party 2 step(s) East")
        );
        assert!(state.message.contains("F-A-L-L-S!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_waterfall_sweep_queues_moongate_landing_prompt() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_WATERFALL_TABLE_FILE),
            "BRITANNIA 1 0 EAST 3 1\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
        state.ambient_light = FULL_DAYLIGHT;
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 20,
            skiffs: 1,
        };
        state.moongates.push(MoongateEntry {
            x: 3,
            y: 0,
            destination_plane: WorldPlane::Britannia,
            destination_x: 30,
            destination_y: 40,
            active_hours: None,
            expected_tile: None,
        });
        state.sync_player_object();

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (3, 0));
        assert_eq!(state.turn, 1);
        assert_eq!(state.pending_moongate, state.moongates.first().copied());
        assert!(
            state
                .message
                .contains("waterfall swept party 2 step(s) East")
        );
        assert!(state.message.contains("moongate! Enter?"));

        assert_eq!(
            state.resolve_moongate_prompt('y', &dir).unwrap(),
            Some(MoveOutcome::Transition(
                AreaTransition::MoongateTeleported {
                    from: WorldPlane::Britannia,
                    to: WorldPlane::Britannia,
                }
            ))
        );
        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(state.turn, 1);
        assert_eq!(state.pending_moongate, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_waterfall_tile_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_WATERFALL_TABLE_FILE),
            "BRITANNIA 1 0 EAST 3 2\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 20,
            skiffs: 1,
        };
        state.sync_player_object();

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("waterfall swept"));
        let _ = fs::remove_dir_all(dir);
    }

