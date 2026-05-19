    #[test]
    fn town_climb_scrubs_entry_markers_on_reloaded_floor() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut pages = vec![16; 16 * 1024];
        let floor1 = 1024;
        pages[floor1] = 0x2a;
        pages[floor1 + 1] = 0x48;
        pages[floor1 + 2] = 0x49;
        pages[floor1 + 3] = 0xc8;
        fs::write(dir.join("CASTLE.DAT"), pages).unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state.climb(&dir, ClimbIntent::Up).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.grid[0], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[1], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[2], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[3], 0xc8);
        assert!(
            harvest_location_markers(&state.grid)
                .spawn_markers
                .is_empty()
        );
        assert!(harvest_location_markers(&state.grid).npc_markers.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_k_climbs_when_only_one_connected_floor_exists() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state.klimb_command(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 1 });
        assert_eq!(state.grid[0], 1);
        assert_eq!(state.active_objects[0].z, 1);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_step_onto_stair_auto_climbs_when_only_one_connected_floor_exists() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[1] = 80;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 1 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 1);
        assert_eq!(state.active_objects[0].z, 1);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_step_onto_facing_stair_family_climbs_up_when_direction_matches() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[1] = 0xc5;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 1 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 1);
        assert_eq!(state.active_objects[0].z, 1);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_step_onto_facing_stair_family_climbs_down_from_opposite_direction() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[1] = 0xc7;
        let mut state = test_state(grid, 0, 0);
        state.area = Area::Town { scene, floor: 1 };
        state.active_objects[0].z = 1;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 0 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 0);
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_step_onto_facing_stair_family_side_crossing_moves_without_floor_change() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[1] = 0xc4;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 0xc4);
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Moved to (1, 0).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_step_onto_clean_trap_door_changes_to_target_floor() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(
            dir.join(TOWN_TRAP_DOOR_TABLE_FILE),
            "CASTLE:0 0 1 0 -1 55\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[1] = 55;
        let mut state = test_state(grid, 0, 0);
        state.visibility_dirty = false;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: -1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: -1 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 4);
        assert_eq!(state.active_objects[0].z, -1);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("trap door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn consumed_top_down_action_on_clean_town_trap_door_applies_underfoot_transition() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        write_castle_trap_door_fixture(&dir);
        let mut state = town_trap_door_origin_state();

        assert!(
            state
                .handle_top_down_key_with_inline('I', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.area, Area::Town { scene, floor: -1 });
        assert_eq!(state.turn, 1);
        assert_eq!(state.active_objects[0].z, -1);
        assert!(state.message.contains("Ignited a torch"));
        assert!(state.message.contains("trap door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn no_turn_top_down_action_on_clean_town_trap_door_skips_underfoot_transition() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_TRAP_DOOR_TABLE_FILE),
            "CASTLE:0 0 1 1 -1 55\n",
        )
        .unwrap();
        let mut state = town_trap_door_origin_state();

        assert!(
            state
                .handle_top_down_key_with_inline('Z', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(
            state.area,
            Area::Town {
                scene: Scene::new(17).unwrap(),
                floor: 0
            }
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[32 + 1], 55);
        assert!(state.message.contains("Z-stats:"));
        assert!(!state.message.contains("trap door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_clean_town_trap_door_applies_underfoot_transition() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        write_castle_trap_door_fixture(&dir);
        let mut state = town_trap_door_origin_state();

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: -1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: -1 });
        assert_eq!(state.turn, 1);
        assert_eq!(state.grid[32 + 1], 4);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("trap door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn inline_cast_on_clean_town_trap_door_applies_underfoot_transition() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        write_castle_trap_door_fixture(&dir);
        let mut state = town_trap_door_origin_state();
        state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
        state.party[0].mana = IN_LOR_COST;
        state.party[0].level = IN_LOR_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IL", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Town { scene, floor: -1 });
        assert_eq!(state.turn, 1);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert!(state.message.contains("Light!"));
        assert!(state.message.contains("trap door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_trap_door_scrubs_entry_markers_on_reloaded_floor() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut pages = vec![16; 16 * 1024];
        let basement = 4 * 1024;
        pages[basement] = 0x2a;
        pages[basement + 1] = 0x48;
        pages[basement + 2] = 0x49;
        pages[basement + 3] = 0xc9;
        fs::write(dir.join("CASTLE.DAT"), pages).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(
            dir.join(TOWN_TRAP_DOOR_TABLE_FILE),
            "CASTLE:0 0 1 0 -1 55\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[1] = 55;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: -1 })
        );

        assert_eq!(state.grid[0], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[1], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[2], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[3], 0xc9);
        assert!(
            harvest_location_markers(&state.grid)
                .spawn_markers
                .is_empty()
        );
        assert!(harvest_location_markers(&state.grid).npc_markers.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_step_onto_clean_exit_tile_restores_return_world_transport() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join(TOWN_EXIT_TILE_TABLE_FILE), "CASTLE:0 0 1 0 55\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 55;
        let mut state = test_state(grid, 0, 0);
        let mut world_grid = open_world_grid();
        world_grid[world_cell_index(10, 20)] = 7;
        let world_object = ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 11,
            y: 20,
            z: -1,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        };
        let transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        state.return_world = Some(WorldReturn {
            plane: WorldPlane::Underworld,
            x: 10,
            y: 20,
            transport,
            timing_status: TimingStatusTag::HalfTime,
            sail_cadence: 1,
            sail_stall_pending: true,
            grid: world_grid,
            active_objects: vec![
                ActiveObject {
                    type_byte: PLAYER_TILE,
                    tile: PLAYER_TILE,
                    x: 10,
                    y: 20,
                    z: -1,
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                },
                world_object,
            ],
            pending_vehicle: None,
        });
        state.visibility_dirty = false;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Leave CASTLE:0?");

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.player.transport, transport);
        assert_eq!(state.timing_status, TimingStatusTag::HalfTime);
        assert_eq!(state.sail_cadence, 1);
        assert!(state.sail_stall_pending);
        assert_eq!(state.active_objects[0].tile, 184);
        assert_eq!(state.grid[world_cell_index(10, 20)], 7);
        assert_eq!(state.world_object_at(11, 20), Some(&world_object));
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("town exit tile"));
        assert!(state.message.contains("debug return point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_exit_tile_uses_clean_location_table_when_no_return_snapshot() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        fs::write(dir.join(TOWN_EXIT_TILE_TABLE_FILE), "CASTLE:0 0 1 0 55\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 55;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Leave CASTLE:0?");

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.grid[world_cell_index(10, 20)], 5);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("town exit tile"));
        assert!(state.message.contains("world-location table point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejecting_town_exit_tile_prompt_stays_in_location_without_turn() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        fs::write(dir.join(TOWN_EXIT_TILE_TABLE_FILE), "CASTLE:0 0 1 0 55\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 55;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(
            handle_play_key_input(&mut state, 'N', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.active_yes_no_prompt, None);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "No.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_native_exit_threshold_tile_uses_location_table_without_sidecar() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[1] = TOWN_EXIT_THRESHOLD_TILE;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );
        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 0);
        assert!(matches!(
            state.active_yes_no_prompt,
            Some(YesNoPromptSession {
                kind: YesNoPromptKind::TownExit {
                    entry,
                    advance_turn: true
                }
            }) if entry.scene == scene && entry.floor == 0 && entry.x == 1 && entry.y == 0
        ));
        assert_eq!(state.message, "Leave CASTLE:0?");

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("town exit tile"));
        assert!(state.message.contains("world-location table point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn consumed_top_down_action_on_clean_town_exit_tile_prompts_then_exits_on_accept() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        write_castle_exit_tile_fixture(&dir);
        let mut state = town_exit_tile_origin_state();

        assert!(
            state
                .handle_top_down_key_with_inline('I', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(
            state.area,
            Area::Town { scene, floor: 0 }
        );
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Ignited a torch"));
        assert!(state.message.contains("Leave CASTLE:0?"));
        assert!(matches!(
            state.active_yes_no_prompt,
            Some(YesNoPromptSession {
                kind: YesNoPromptKind::TownExit {
                    entry,
                    advance_turn: false
                }
            }) if entry.scene == scene && entry.floor == 0 && entry.x == 1 && entry.y == 1
        ));

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("town exit tile"));
        assert!(state.message.contains("world-location table point"));
        assert_eq!(
            state.pending_moongate, None,
            "exit transitions should not queue the previous town cell as a moongate"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn no_turn_top_down_action_on_clean_town_exit_tile_skips_underfoot_exit() {
        let dir = debug_game_dir();
        write_castle_exit_tile_fixture(&dir);
        let mut state = town_exit_tile_origin_state();

        assert!(
            state
                .handle_top_down_key_with_inline('Z', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(
            state.area,
            Area::Town {
                scene: Scene::new(17).unwrap(),
                floor: 0
            }
        );
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[32 + 1], 55);
        assert!(state.message.contains("Z-stats:"));
        assert!(!state.message.contains("town exit tile"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_clean_town_exit_tile_prompt_can_be_refused() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        write_castle_exit_tile_fixture(&dir);
        let mut state = town_exit_tile_origin_state();

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("Leave CASTLE:0?"));
        assert!(matches!(
            state.active_yes_no_prompt,
            Some(YesNoPromptSession {
                kind: YesNoPromptKind::TownExit {
                    entry,
                    advance_turn: false
                }
            }) if entry.scene == scene && entry.floor == 0 && entry.x == 1 && entry.y == 1
        ));

        assert_eq!(
            handle_play_key_input(&mut state, 'N', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.active_yes_no_prompt, None);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "No.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_native_town_exit_threshold_tile_prompts_then_exits_on_accept() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = TOWN_EXIT_THRESHOLD_TILE;
        let mut state = test_state(grid, 1, 1);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("Leave CASTLE:0?"));
        assert!(matches!(
            state.active_yes_no_prompt,
            Some(YesNoPromptSession {
                kind: YesNoPromptKind::TownExit {
                    entry,
                    advance_turn: false
                }
            }) if entry.scene == scene && entry.floor == 0 && entry.x == 1 && entry.y == 1
        ));

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("town exit tile"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_exit_tile_clears_visit_local_door_state() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        fs::write(dir.join(TOWN_EXIT_TILE_TABLE_FILE), "CASTLE:0 0 1 0 55\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 55;
        let mut state = test_state(grid, 0, 0);
        state.door_tracker = Some(DoorTracker {
            previous_tile: 96,
            x: 3,
            y: 1,
            turns_remaining: 4,
        });
        state.record_open_town_door(scene, 0, 3, 1);
        state.record_revealed_town_secret_door(scene, 0, 4, 1);
        state.record_open_town_door(scene, 0, 4, 1);
        state.set_town_npc_alarm_state(scene, 0, 2, TownNpcAlarmState::Fleeing);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!(state.door_tracker, None);
        assert!(state.opened_town_doors.is_empty());
        assert!(state.revealed_town_secret_doors.is_empty());
        assert!(state.town_npc_alarm_states.is_empty());
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_exit_tile_missing_return_metadata_stays_in_location() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join(TOWN_EXIT_TILE_TABLE_FILE), "CASTLE:0 0 1 0 55\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 55;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );
        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("town exit tile"));
        assert!(
            state
                .message
                .contains("missing clean return-coordinate metadata")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_exit_tile_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join(TOWN_EXIT_TILE_TABLE_FILE), "CASTLE:0 0 1 0 56\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 16;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 16);
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("town exit tile"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_trap_door_tile_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(
            dir.join(TOWN_TRAP_DOOR_TABLE_FILE),
            "CASTLE:0 0 1 0 -1 56\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[1] = 16;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 16);
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("trap door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_k_prompts_when_both_floor_directions_are_connected() {
        let dir = debug_game_dir();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(state.klimb_command(&dir).unwrap(), MoveOutcome::Observed);

        assert_eq!(state.message, "Klimb-");
        assert_eq!(
            state.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::Klimb)
        );
        assert_eq!(
            state.area,
            Area::Town {
                scene: Scene::new(17).unwrap(),
                floor: 0
            }
        );
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '>', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::Town {
                scene: Scene::new(17).unwrap(),
                floor: -1
            }
        );
        assert_eq!(state.turn, 1);
        assert!(state.active_direction_prompt.is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_k_uses_clean_stair_sidecar_to_choose_one_way_direction() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(dir.join(TOWN_STAIR_TABLE_FILE), "CASTLE:0 0 0 0 UP 80\n").unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state.klimb_command(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 1 });
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Changed to CASTLE:0 floor 1"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_clean_stair_sidecar_refuses_wrong_manual_direction() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(dir.join(TOWN_STAIR_TABLE_FILE), "CASTLE:0 0 0 0 UP 80\n").unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state.climb(&dir, ClimbIntent::Down).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!(state.message, "Not climbable!");
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_k_non_ladder_reports_public_refusal_without_turn() {
        let mut state = test_state(open_grid(), 1, 1);

        assert_eq!(
            state.klimb_command(Path::new("")).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Not climbable!");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn town_step_onto_clean_stair_sidecar_can_trigger_non_family_tile() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(dir.join(TOWN_STAIR_TABLE_FILE), "CASTLE:0 0 1 0 UP 55\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 55;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 1 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 6);
        assert_eq!(state.active_objects[0].z, 1);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_clean_stair_tile_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(dir.join(TOWN_STAIR_TABLE_FILE), "CASTLE:0 0 1 0 UP 56\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 16;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("Changed to"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_step_onto_stair_prompts_when_both_floor_directions_are_connected() {
        let dir = debug_game_dir();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 80;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(state.message, "Klimb-");
        assert_eq!(
            state.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::Klimb)
        );
        assert_eq!(
            state.area,
            Area::Town {
                scene: Scene::new(17).unwrap(),
                floor: 0
            }
        );
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.active_objects[0].x, 1);
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '<', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::Town {
                scene: Scene::new(17).unwrap(),
                floor: 1
            }
        );
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Changed to CASTLE:0 floor 1"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_k_requires_climbing_gear_without_turn() {
        let mut state = world_state(open_world_grid(), 10, 20);

        assert_eq!(
            state.klimb_command(Path::new("")).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "With what?");
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn world_k_requires_foot_when_gear_present() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.climbing_gear = 1;
        mount_horse(&mut state);

        assert_eq!(
            state.klimb_command(Path::new("")).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "On foot!");
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn world_k_refuses_non_climbable_target_without_turn() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.climbing_gear = 1;
        state.player.facing = Direction::East;

        assert_eq!(
            state.klimb_command(Path::new("")).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Not climbable!");
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn world_k_refuses_impassable_target_without_turn() {
        let mut grid = open_world_grid();
        grid[world_cell_index(11, 20)] = 24;
        let mut state = world_state(grid, 10, 20);
        state.climbing_gear = 1;
        state.player.facing = Direction::East;

        assert_eq!(
            state.klimb_command(Path::new("")).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Impassable!");
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn world_k_refuses_clean_lava_sidecar_target_without_turn() {
        let dir = debug_game_dir();
        // The damage-tile sidecar takes an optional expected_tile after
        // the effect; 0x0c is "mountains" per LOOK2.DAT (was 10 when
        // the old code treated 10..=15 as a single mountain band).
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 11 20 LAVA 12\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(11, 20)] = 0x0c;
        let mut state = britannia_state(grid, 10, 20);
        state.climbing_gear = 1;
        state.player.facing = Direction::East;

        assert_eq!(state.klimb_command(&dir).unwrap(), MoveOutcome::Blocked);

        assert_eq!(state.message, "Impassable!");
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 0);
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_k_climbs_class_derived_mountain_family_with_gear() {
        let mut grid = open_world_grid();
        grid[world_cell_index(11, 20)] = 0x0c;
        let mut state = world_state(grid, 10, 20);
        state.climbing_gear = 1;
        state.player.facing = Direction::East;

        assert_eq!(
            state.klimb_command(Path::new("")).unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (11, 20));
        assert_eq!(
            (state.active_objects[0].x, state.active_objects[0].y),
            (11, 20)
        );
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Climbed East"));
        assert!(state.message.contains("fall checks passed for 1 living"));
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP);
    }

    #[test]
    fn world_k_applies_clean_plane_transition_after_successful_climb() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 11 20 UNDERWORLD 30 40\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(11, 20)] = 0x0c;
        let mut state = britannia_state(grid, 10, 20);
        state.climbing_gear = 1;
        state.player.facing = Direction::East;

        assert_eq!(
            state.klimb_command(&dir).unwrap(),
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
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Climbed East"));
        assert!(state.message.contains("F-A-L-L-S"));
        let _ = fs::remove_dir_all(dir);
    }

