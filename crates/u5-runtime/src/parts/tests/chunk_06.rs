    #[test]
    fn town_fire_source_runs_auto_close_before_target_scan() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 3] = 16;
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: 96,
            x: 3,
            y: 1,
            turns_remaining: 1,
        });
        state.record_open_town_door(Scene::new(17).unwrap(), 0, 3, 1);

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], 16);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(Scene::new(17).unwrap(), 0, 3, 1));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("destroyed door tile 96"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_fire_source_ticks_door_tracker_once_on_consumed_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 5] = 16;
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: 96,
            x: 5,
            y: 1,
            turns_remaining: 4,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 5,
                y: 1,
                turns_remaining: 3,
            })
        );
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("object tile 192"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_fire_source_destroying_door_clears_unrelated_auto_close_tracker() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 3] = 96;
        grid[32 + 5] = 16;
        let scene = Scene::new(17).unwrap();
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: 97,
            x: 5,
            y: 1,
            turns_remaining: 4,
        });
        state.record_open_town_door(scene, 0, 5, 1);

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], 16);
        assert_eq!(state.grid[32 + 5], 16);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(scene, 0, 3, 1));
        assert!(state.is_recorded_open_town_door(scene, 0, 5, 1));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("destroyed door tile 96"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_fire_source_removes_object_before_farther_door() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 3] = 96;
        let mut state = test_state(grid, 0, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], 96);
        assert!(state.active_objects[1].is_empty());
        assert!(state.object_at_current_floor(2, 1).is_none());
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("object tile 192"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_town_pushable_entries_accepts_optional_tile_guard() {
        let entries = parse_town_pushable_entries("CASTLE:0 0 2 1 44\nCASTLE:0 0 3 1\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownPushableEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    expected_tile: Some(44),
                },
                TownPushableEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 3,
                    y: 1,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_pushable_entries("CASTLE:0 0 32 1 44\n").is_err());
        assert!(parse_town_pushable_entries("DUNGEON:0 0 2 1 44\n").is_err());
    }

    #[test]
    fn parse_object_pickup_entries_accepts_world_and_town_rows() {
        let entries = parse_object_pickup_entries(
            "BRITANNIA 0 5 5 GEMS 2 210\nCASTLE:0 0 2 1 KEYS 1\nUNDERWORLD -1 1 2 TORCHES 3\nBRITANNIA 0 6 5 FOOD 4\nBRITANNIA 0 7 5 GOLD 9\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                ObjectPickupEntry {
                    target: PlayTarget::World(WorldPlane::Britannia),
                    floor: 0,
                    x: 5,
                    y: 5,
                    kind: ObjectPickupKind::Gems,
                    amount: 2,
                    expected_tile: Some(210),
                },
                ObjectPickupEntry {
                    target: PlayTarget::Town(Scene::new(17).unwrap()),
                    floor: 0,
                    x: 2,
                    y: 1,
                    kind: ObjectPickupKind::Keys,
                    amount: 1,
                    expected_tile: None,
                },
                ObjectPickupEntry {
                    target: PlayTarget::World(WorldPlane::Underworld),
                    floor: -1,
                    x: 1,
                    y: 2,
                    kind: ObjectPickupKind::Torches,
                    amount: 3,
                    expected_tile: None,
                },
                ObjectPickupEntry {
                    target: PlayTarget::World(WorldPlane::Britannia),
                    floor: 0,
                    x: 6,
                    y: 5,
                    kind: ObjectPickupKind::Food,
                    amount: 4,
                    expected_tile: None,
                },
                ObjectPickupEntry {
                    target: PlayTarget::World(WorldPlane::Britannia),
                    floor: 0,
                    x: 7,
                    y: 5,
                    kind: ObjectPickupKind::Gold,
                    amount: 9,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_object_pickup_entries("DUNGEON:0 0 2 1 GEMS 1\n").is_err());
        assert!(parse_object_pickup_entries("BRITANNIA -1 5 5 GEMS 1\n").is_err());
        assert!(parse_object_pickup_entries("CASTLE:0 0 32 1 KEYS 1\n").is_err());
        assert!(parse_object_pickup_entries("BRITANNIA 0 5 5 GEMS 0\n").is_err());
        assert!(
            parse_object_pickup_entries("BRITANNIA 0 5 5 GEMS 1\nBRITANNIA 0 5 5 KEYS 1\n")
                .is_err()
        );
    }

    #[test]
    fn object_pickup_gold_updates_save_backed_counter_with_saturation() {
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = u16::MAX - 1;

        state.apply_object_pickup(ObjectPickupKind::Gold, 5);

        assert_eq!(state.gold, u16::MAX);
    }

    #[test]
    fn world_get_consumes_clean_object_pickup_before_blocking_refusal() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "BRITANNIA 0 5 5 GEMS 2 210\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 5,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.gems, DEFAULT_GEM_STOCK + 2);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got 2 gems"));
        assert!(state.message.contains("active-object tile 210"));
        let overlay = state.world_overlays.get(WorldPlane::Britannia).unwrap();
        assert!(overlay[0].is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_get_consumes_clean_object_pickup_before_blocking_refusal() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "CASTLE:0 0 2 1 KEYS 1 210\n",
        )
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.keys, DEFAULT_KEY_STOCK + 1);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got 1 keys"));
        assert!(state.message.contains("CASTLE:0 floor 0"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn object_pickup_tile_guard_mismatch_falls_back_to_blocking_refusal_without_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "BRITANNIA 0 5 5 GEMS 2 211\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 5,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert!(!state.active_objects[1].is_empty());
        assert_eq!(state.gems, DEFAULT_GEM_STOCK);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to get there.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn top_down_g_key_routes_to_object_pickup_sidecar() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "BRITANNIA 0 5 5 TORCHES 3 210\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 5,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert!(
            state
                .handle_top_down_key_with_inline('G', &dir, None, None, None, None)
                .unwrap()
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.torches, DEFAULT_TORCH_STOCK + 3);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got 3 torches"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_world_get_tile_entries_accepts_replacement_and_optional_tile_guard() {
        let entries = parse_world_get_tile_entries(
            "UNDERWORLD 2 1 5 55\nBRITANNIA 3 1 5\nBRITANNIA 4 1 5 GOLD 7\nUNDERWORLD 5 1 5 55 FOOD 4\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                WorldGetTileEntry {
                    plane: WorldPlane::Underworld,
                    x: 2,
                    y: 1,
                    replacement_tile: 5,
                    expected_tile: Some(55),
                    grant: None,
                },
                WorldGetTileEntry {
                    plane: WorldPlane::Britannia,
                    x: 3,
                    y: 1,
                    replacement_tile: 5,
                    expected_tile: None,
                    grant: None,
                },
                WorldGetTileEntry {
                    plane: WorldPlane::Britannia,
                    x: 4,
                    y: 1,
                    replacement_tile: 5,
                    expected_tile: None,
                    grant: Some(ObjectPickupGrant {
                        kind: ObjectPickupKind::Gold,
                        amount: 7,
                    }),
                },
                WorldGetTileEntry {
                    plane: WorldPlane::Underworld,
                    x: 5,
                    y: 1,
                    replacement_tile: 5,
                    expected_tile: Some(55),
                    grant: Some(ObjectPickupGrant {
                        kind: ObjectPickupKind::Food,
                        amount: 4,
                    }),
                },
            ]
        );
        assert!(parse_world_get_tile_entries("DUNGEON:0 2 1 5 55\n").is_err());
        assert!(parse_world_get_tile_entries("UNDERWORLD 2 1 55 55\n").is_err());
        assert!(parse_world_get_tile_entries("UNDERWORLD 2 1 5 GOLD 0\n").is_err());
        assert!(parse_world_get_tile_entries("UNDERWORLD 2 1 5 JUNK 1\n").is_err());
        assert!(parse_world_get_tile_entries("UNDERWORLD 2 1 5 55\nUNDER 2 1 5 44\n").is_err());
    }

    #[test]
    fn world_get_wraps_and_rewrites_clean_sidecar_tile() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_GET_TILE_TABLE_FILE),
            "UNDERWORLD 0 0 5 55 GOLD 7\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut state = world_state(grid, 255, 0);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[world_cell_index(0, 0)], 5);
        assert_eq!(state.gold, DEFAULT_GOLD_STOCK + 7);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got world tile 55"));
        assert!(state.message.contains("UNDERWORLD"));
        assert!(state.message.contains("added 7 gold"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_get_refuses_missing_or_mismatched_sidecar_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut state = world_state(grid, 255, 0);
        state.player.facing = Direction::East;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.turn, 0);

        fs::write(dir.join(WORLD_GET_TILE_TABLE_FILE), "UNDERWORLD 0 0 5 44\n").unwrap();
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.grid[world_cell_index(0, 0)], 55);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to get here.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn top_down_g_key_routes_to_world_get_sidecar() {
        let dir = debug_game_dir();
        fs::write(dir.join(WORLD_GET_TILE_TABLE_FILE), "UNDERWORLD 0 0 5 55\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut state = world_state(grid, 255, 0);
        state.player.facing = Direction::East;

        assert!(
            state
                .handle_top_down_key_with_inline('G', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.grid[world_cell_index(0, 0)], 5);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got world tile 55"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_town_get_tile_entries_accepts_replacement_and_optional_tile_guard() {
        let entries = parse_town_get_tile_entries(
            "CASTLE:0 0 2 1 16 55\nCASTLE:0 0 3 1 16\nCASTLE:0 0 4 1 16 KEYS 2\nCASTLE:0 0 5 1 16 55 GEMS 3\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                TownGetTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    replacement_tile: 16,
                    expected_tile: Some(55),
                    grant: None,
                },
                TownGetTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 3,
                    y: 1,
                    replacement_tile: 16,
                    expected_tile: None,
                    grant: None,
                },
                TownGetTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 4,
                    y: 1,
                    replacement_tile: 16,
                    expected_tile: None,
                    grant: Some(ObjectPickupGrant {
                        kind: ObjectPickupKind::Keys,
                        amount: 2,
                    }),
                },
                TownGetTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 5,
                    y: 1,
                    replacement_tile: 16,
                    expected_tile: Some(55),
                    grant: Some(ObjectPickupGrant {
                        kind: ObjectPickupKind::Gems,
                        amount: 3,
                    }),
                },
            ]
        );
        assert!(parse_town_get_tile_entries("CASTLE:0 0 32 1 16 55\n").is_err());
        assert!(parse_town_get_tile_entries("DUNGEON:0 0 2 1 16 55\n").is_err());
        assert!(parse_town_get_tile_entries("CASTLE:0 0 2 1 55 55\n").is_err());
        assert!(parse_town_get_tile_entries("CASTLE:0 0 2 1 16 KEYS 0\n").is_err());
        assert!(parse_town_get_tile_entries("CASTLE:0 0 2 1 16 JUNK 1\n").is_err());
    }

    #[test]
    fn town_get_uses_clean_sidecar_to_rewrite_facing_tile() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_GET_TILE_TABLE_FILE),
            "CASTLE:0 0 2 1 16 55 KEYS 2\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK + 2);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got tile 55"));
        assert!(state.message.contains("added 2 keys"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_get_refuses_missing_or_mismatched_sidecar_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.turn, 0);

        fs::write(dir.join(TOWN_GET_TILE_TABLE_FILE), "CASTLE:0 0 2 1 16 44\n").unwrap();
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.grid[32 + 2], 55);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to get here.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn top_down_g_key_routes_to_town_get_sidecar() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_GET_TILE_TABLE_FILE), "CASTLE:0 0 2 1 16 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert!(
            state
                .handle_top_down_key_with_inline('G', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got tile 55"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_world_moonstone_surfaces_highest_matching_phase_and_get_invalidates_slot() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 5,
            y: 5,
            z: 0,
        };
        state.moonstone_slots[3] = MoonstoneGateSlot {
            scene: 0,
            x: 5,
            y: 5,
            z: 0,
        };

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Found a strange rock for Moonstone phase 4.");
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(
            state.active_objects[1],
            ActiveObject::moonstone_pickup(3, 5, 5, WorldPlane::Britannia.save_floor())
        );

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(state.turn, 1);
        assert_eq!(
            state.message,
            "Moonstone phase 4 is already surfaced as a strange rock."
        );

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.moonstone_slots[1].scene, 0);
        assert_eq!(state.moonstone_slots[3], MoonstoneGateSlot::invalid());
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
        assert_eq!(
            state.message,
            "Recovered Moonstone phase 4; Gate Travel slot cleared."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_town_moonstone_uses_scene_floor_and_surface_get_clears_slot() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.moonstone_slots[0] = MoonstoneGateSlot {
            scene: 0x11,
            x: 2,
            y: 1,
            z: 0,
        };

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(
            state.active_objects[1],
            ActiveObject::moonstone_pickup(0, 2, 1, 0)
        );
        assert_eq!(state.message, "Found a strange rock for Moonstone phase 1.");

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.moonstone_slots[0], MoonstoneGateSlot::invalid());
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(
            state.message,
            "Recovered Moonstone phase 1; Gate Travel slot cleared."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn use_moonstone_world_records_gate_slot_and_can_drive_gate_travel() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = 9;
        state.party[0].level = 8;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "3", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.moonstone_slots[2],
            MoonstoneGateSlot {
                scene: 0,
                x: 4,
                y: 5,
                z: 0,
            }
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(
            state.message,
            "Buried Moonstone phase 3 at BRITANNIA (4, 5)."
        );

        state.player.x = 8;
        state.player.y = 9;
        state.sync_player_object();

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PRV3", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (4, 5));
        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
        assert_eq!(state.message, "Gate Travel phase 3 -> BRITANNIA at (4, 5).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn use_moonstone_town_records_scene_floor_and_clears_stale_pickup() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = 5;
        let mut state = test_state(grid, 1, 1);
        state.moonstone_slots[7] = MoonstoneGateSlot {
            scene: 0x11,
            x: 2,
            y: 1,
            z: 0,
        };
        state
            .active_objects
            .push(ActiveObject::moonstone_pickup(7, 2, 1, 0));
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "8", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.moonstone_slots[7],
            MoonstoneGateSlot {
                scene: 0x11,
                x: 1,
                y: 1,
                z: 0,
            }
        );
        assert!(state.active_objects[1].is_empty());
        assert!(state.visibility_dirty);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(
            state.message,
            "Buried Moonstone phase 8 at CASTLE:0 (1, 1)."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn use_moonstone_rejects_missing_phase_bad_tile_and_dungeon_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(4, 5)] = 16;
        let mut state = britannia_state(grid, 4, 5);

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, use_prompt_message());

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "1", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.moonstone_slots[0], MoonstoneGateSlot::invalid());
        assert_eq!(state.message, "Cannot bury Moonstone on tile 16.");

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        assert!(dungeon.handle_dungeon_key('U', &dir).unwrap());
        assert_eq!(dungeon.turn, 0);
        assert_eq!(dungeon.message, use_prompt_message());
        assert_eq!(
            handle_play_key_input(&mut dungeon, 'U', "1", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(dungeon.turn, 0);
        assert_eq!(dungeon.message, "Not here!");
        let _ = fs::remove_dir_all(dir);
    }

