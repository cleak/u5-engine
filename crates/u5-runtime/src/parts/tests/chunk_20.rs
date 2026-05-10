    #[test]
    fn idle_tick_advances_visuals_without_turn_time_doors_or_schedules() {
        let mut grid = open_grid();
        grid[32 + 2] = 16;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(17, 59).unwrap();
        state.door_tracker = Some(DoorTracker {
            previous_tile: 96,
            x: 2,
            y: 1,
            turns_remaining: 1,
        });
        let slots = vec![
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0,
                schedule: [0, 0, 0, 4, 8, 12, 1, 2, 3, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);
        let npcs_before = state.npcs.clone();
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 3,
            y: 1,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(state.clock, GameClock::new(17, 59).unwrap());
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 2,
                y: 1,
                turns_remaining: 1,
            })
        );
        assert_eq!(state.npcs, npcs_before);
        assert_eq!(state.animation.frame, 1);
        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!(object.phase, 0x21);
        assert_eq!(object.tile, 169);
    }

    #[test]
    fn idle_tick_keeps_active_objects_frozen_during_time_stop_without_aging_counter() {
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.time_stop_counter = 3;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(state.turn, 0);
        assert_eq!(state.time_stop_counter, 3);
        assert_eq!(state.animation.frame, 1);
        assert_eq!(state.active_objects[1].phase, 0x22);
        assert_eq!(state.active_objects[1].tile, 168);
    }

    #[test]
    fn open_facing_rewrites_door_and_auto_closes_after_four_turns() {
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        assert_eq!(state.turn, 1);
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.message, "Opened!");
        assert!(state.visibility_dirty);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 2,
                y: 1,
                turns_remaining: 4,
            })
        );

        state.visibility_dirty = false;
        state.advance_turn();
        state.advance_turn();
        state.advance_turn();
        assert_eq!(state.grid[32 + 2], 16);
        assert!(state.door_tracker.is_some());
        assert!(!state.visibility_dirty);

        state.advance_turn();
        assert_eq!(state.grid[32 + 2], 96);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
    }

    #[test]
    fn open_facing_non_door_is_not_a_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.open_facing(), MoveOutcome::Blocked);
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.message, "Nothing to open!");
    }

    #[test]
    fn jimmy_town_door_consumes_turn_without_visit_local_rewrite() {
        let mut grid = open_grid();
        grid[32 + 2] = 97;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.grid[32 + 2], 97);
        assert!(state.message.contains("Jimmy checked door tile 97"));
        assert!(state.message.contains("lock-state table"));
    }

    #[test]
    fn jimmy_wrong_tile_reports_no_lock_without_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.jimmy_facing(), MoveOutcome::Blocked);

        assert_eq!(state.message, "No lock!");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn open_facing_tracked_open_door_consumes_turn_without_resetting_timer() {
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.ambient_light = FULL_DAYLIGHT;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        state.visibility_dirty = false;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);

        assert_eq!(state.turn, 2);
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 2,
                y: 1,
                turns_remaining: 3,
            })
        );
        assert_eq!(state.message, "It's open!");
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn open_facing_runs_auto_close_before_reopening_expiring_door() {
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        state.advance_turn();
        state.advance_turn();
        state.advance_turn();
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 2,
                y: 1,
                turns_remaining: 1,
            })
        );

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.turn, 5);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 2,
                y: 1,
                turns_remaining: 4,
            })
        );
        assert_eq!(state.message, "Opened!");
    }

    #[test]
    fn open_facing_acknowledges_first_open_door_after_second_door_overwrites_timer() {
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        grid[2 * 32 + 1] = 97;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        assert_eq!(state.grid[32 + 2], 16);

        state.player.facing = Direction::South;
        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        assert_eq!(state.grid[2 * 32 + 1], 16);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 97,
                x: 1,
                y: 2,
                turns_remaining: 4,
            })
        );

        state.player.facing = Direction::East;
        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);

        assert_eq!(state.turn, 3);
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.grid[2 * 32 + 1], 16);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 97,
                x: 1,
                y: 2,
                turns_remaining: 3,
            })
        );
        assert_eq!(state.message, "It's open!");
    }

    #[test]
    fn town_open_locked_sidecar_refuses_without_turn() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 97 96\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 97;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Locked!");
        assert_eq!(state.grid[32 + 2], 97);
        assert_eq!(state.turn, 0);
        assert_eq!(state.door_tracker, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_jimmy_locked_sidecar_rewrites_to_unlocked_door() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 97 96\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 97;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.message, "Unlocked!");
        assert_eq!(state.grid[32 + 2], 96);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::DoorOpened
        );
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.message, "Opened!");
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 2,
                y: 1,
                turns_remaining: 4,
            })
        );
        assert_eq!(state.turn, 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_jimmy_magic_lock_sidecar_refuses_without_key_or_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_LOCK_TABLE_FILE),
            "CASTLE:0 0 2 1 96 97 MAGIC\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Magic lock!");
        assert_eq!(state.grid[32 + 2], 96);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_secret_door_entries_accepts_town_and_dungeon_rows() {
        let entries = parse_secret_door_entries(
            "TOWN CASTLE:0 0 2 1 96 24\nDUNGEON DUNGEON:0 0 2 1 0xF0 0x30\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                SecretDoorEntry::Town {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    reveal_tile: 96,
                    expected_tile: Some(24),
                },
                SecretDoorEntry::Dungeon {
                    scene: DungeonScene::new(33).unwrap(),
                    level: 0,
                    x: 2,
                    y: 1,
                    reveal_cell: 0xF0,
                    expected_cell: Some(0x30),
                },
            ]
        );
    }

    #[test]
    fn town_search_uses_clean_sidecar_to_reveal_secret_door() {
        let dir = debug_game_dir();
        fs::write(dir.join(SECRET_DOOR_TABLE_FILE), "TOWN CASTLE:0 0 2 1 96\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.grid[32 + 2], 96);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Revealed secret door at (2, 1).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_search_secret_door_tile_guard_mismatch_is_not_a_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "TOWN CASTLE:0 0 2 1 96 25\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[32 + 2], 24);
        assert_eq!(state.turn, 0);
        assert!(!state.visibility_dirty);
        assert_eq!(state.message, "No secret door found.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_open_revealed_secret_door_stays_open_without_auto_close_tracker() {
        let dir = debug_game_dir();
        fs::write(dir.join(SECRET_DOOR_TABLE_FILE), "TOWN CASTLE:0 0 2 1 96\n").unwrap();
        let scene = Scene::new(17).unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(state.is_revealed_town_secret_door(scene, 0, 2, 1));

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::DoorOpened
        );

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.message, "Opened!");
        assert_eq!(state.turn, 2);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(scene, 0, 2, 1));

        for _ in 0..4 {
            state.advance_turn();
        }

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(scene, 0, 2, 1));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_floor_reload_preserves_opened_secret_door_for_visit() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut pages = vec![16; 16 * 1024];
        let floor_zero = 5 * 1024;
        let floor_one = 6 * 1024;
        pages[floor_zero] = 80;
        pages[floor_zero + 32 + 2] = 24;
        pages[floor_one] = 80;
        fs::write(dir.join("CASTLE.DAT"), pages).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(dir.join(SECRET_DOOR_TABLE_FILE), "TOWN CASTLE:0 0 2 1 96\n").unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::DoorOpened
        );
        assert_eq!(state.grid[32 + 2], 16);

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
        assert_eq!(state.grid[32 + 2], 16);
        assert!(state.is_revealed_town_secret_door(scene, 0, 2, 1));
        assert!(state.is_recorded_open_town_door(scene, 0, 2, 1));
        assert_eq!(state.door_tracker, None);
        assert_eq!(state.turn, 4);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_jimmy_revealed_secret_door_reports_no_lock() {
        let dir = debug_game_dir();
        fs::write(dir.join(SECRET_DOOR_TABLE_FILE), "TOWN CASTLE:0 0 2 1 96\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.grid[32 + 2], 96);
        assert_eq!(state.message, "No lock!");
        assert_eq!(state.turn, 2);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_search_without_matching_sidecar_entry_is_not_a_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[32 + 2], 24);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "No secret door found.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_uppercase_s_routes_to_sidecar_secret_search() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "DUNGEON DUNGEON:0 0 2 1 0xF0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x30;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;

        assert!(state.handle_dungeon_key('S', &dir).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xF0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Revealed dungeon secret door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_secret_door_cell_guard_mismatch_uses_normal_cell_search() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "DUNGEON DUNGEON:0 0 2 1 0xF0 0x30\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x4c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x7c);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Searched dungeon chest at (2, 1)"));
        assert!(!state.message.contains("secret door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_chest_consumes_turn_and_marks_visit_local_passage() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x4c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x7c);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Searched dungeon chest at (2, 1)"));
        assert!(
            state
                .message
                .contains("content/trap generator is out of scope")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_bomb_trap_marks_fired_without_level_change() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x62;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x6a);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert_eq!(
            state.message,
            "Cleared dungeon bomb trap at (2, 1) on DUNGEON:0 level 0."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_fall_trap_reports_feature_without_triggering_drop() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x69;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x69);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("found a pit or trap"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_field_reports_feature_without_applying_effect() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x89;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.party = vec![PartyMember {
            slot: 0,
            status: b'G',
            climb_stat: 30,
            mana: 8,
            hp: 10,
            max_hp: 20,
            level: 8,
        }];

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x89);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("found poison gas field"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_open_chest_consumes_turn_and_marks_visit_local_passage() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.ambient_light = FULL_DARKNESS;
        state.visibility_dirty = false;

        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7b);
        assert_eq!(state.turn, 1);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Opened dungeon chest"));
        assert!(
            state
                .message
                .contains("content/trap generator is out of scope")
        );
    }

    #[test]
    fn dungeon_get_chest_consumes_turn_and_marks_visit_local_passage() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.ambient_light = FULL_DARKNESS;
        state.visibility_dirty = false;

        assert_eq!(
            state.get_dungeon_underfoot(scene, 0),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7c);
        assert_eq!(state.turn, 1);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got dungeon chest"));
        assert!(
            state
                .message
                .contains("content/trap generator is out of scope")
        );
    }

    #[test]
    fn dungeon_get_chest_applies_clean_sidecar_grants() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_CHEST_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x4c GOLD 7 GEMS 2 TORCHES 1\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.gold = 10;
        state.gems = 1;
        state.torches = 0;

        assert_eq!(
            state
                .get_dungeon_underfoot_with_game_dir(Some(&dir), scene, 0)
                .unwrap(),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7c);
        assert_eq!(state.gold, 17);
        assert_eq!(state.gems, 3);
        assert_eq!(state.torches, 1);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got dungeon chest"));
        assert!(
            state
                .message
                .contains("authored chest grants 7 gold, 2 gems, 1 torches")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_open_chest_applies_clean_sidecar_grants() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_CHEST_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x4b KEYS 2 FOOD 5\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.keys = 1;
        state.food = 12;

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7b);
        assert_eq!(state.keys, 3);
        assert_eq!(state.food, 17);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Opened dungeon chest"));
        assert!(
            state
                .message
                .contains("authored chest grants 2 keys, 5 food")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_chest_applies_clean_sidecar_grants_and_guard() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_CHEST_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x4c KEYS 2\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x4c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.keys = 1;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x7c);
        assert_eq!(state.keys, 3);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Searched dungeon chest"));
        assert!(state.message.contains("authored chest grants 2 keys"));

        let mut mismatch_grid = open_dungeon_record();
        mismatch_grid[dungeon_cell_index(0, 2, 1)] = 0x4b;
        let mut mismatch = dungeon_state(mismatch_grid, 0, 1, 1);
        mismatch.player.facing = Direction::East;
        mismatch.keys = 1;

        assert_eq!(
            mismatch.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(mismatch.keys, 1);
        assert_eq!(mismatch.grid[dungeon_cell_index(0, 2, 1)], 0x7b);
        assert!(
            mismatch
                .message
                .contains("content/trap generator is out of scope")
        );
        let _ = fs::remove_dir_all(dir);
    }

