    /// The reloaded floor scrubs its NPC start markers and keeps the
    /// beacon's `0x2A` light source, which the floor-transition path also
    /// harvests (`formats/location-dat.md §6`).
    #[test]
    fn town_climb_scrubs_npc_markers_and_keeps_the_beacon_source() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut pages = vec![16; 16 * 1024];
        let floor1 = 1024;
        pages[floor1] = BEACON_BRIGHT_LIGHT_TILE;
        pages[floor1 + 1] = 0x48;
        pages[floor1 + 2] = 0x49;
        pages[floor1 + 3] = 0xc8;
        fs::write(dir.join("CASTLE.DAT"), pages).unwrap();
        let mut grid = open_grid();
        grid[0] = TOWN_KLIMB_ASCEND_TILE;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state.climb(&dir, ClimbIntent::Up).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.grid[0], BEACON_BRIGHT_LIGHT_TILE);
        assert_eq!(state.grid[1], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[2], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[3], 0xc8);
        assert_eq!(
            state.light_beacon.sources,
            [Some((0, 0)), None],
            "the floor reached by stairs harvests its beacon source"
        );
        assert!(
            harvest_location_npc_start_markers(&state.grid)
                .npc_markers
                .is_empty()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_k_ascend_link_climbs_up() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[0] = TOWN_KLIMB_ASCEND_TILE;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state.klimb_command(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 1 });
        assert_eq!(state.grid[0], 1);
        assert_eq!(state.active_objects[0].z, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Up!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_step_onto_stair_auto_climbs_when_only_one_connected_floor_exists() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[1] = TOWN_STAIR_TILE_FIRST + 1;
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

        let turn_before = state.turn;
        let step = state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap();
        assert_eq!(step, MoveOutcome::Moved);
        assert_eq!(
            state
                .apply_post_turn_effects_after_outcome(turn_before, &dir, step)
                .unwrap(),
            Some(MoveOutcome::Transition(AreaTransition::ChangedFloor {
                scene,
                floor: -1
            }))
        );

        assert_eq!(state.area, Area::Town { scene, floor: -1 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.grid[1], 4);
        assert_eq!(state.active_objects[0].z, -1);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("A TRAPDOOR!"));
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
        assert!(state.message.contains("A TRAPDOOR!"));
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
        assert!(state.message.contains("Player:"));
        assert!(!state.message.contains("A TRAPDOOR!"));
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
        assert!(state.message.contains("A TRAPDOOR!"));
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
        assert!(state.message.contains("A TRAPDOOR!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_trap_door_scrubs_npc_markers_and_keeps_the_beacon_source() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut pages = vec![16; 16 * 1024];
        let basement = 4 * 1024;
        pages[basement] = BEACON_BRIGHT_LIGHT_TILE;
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

        let turn_before = state.turn;
        let step = state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap();
        assert_eq!(step, MoveOutcome::Moved);
        assert_eq!(
            state
                .apply_post_turn_effects_after_outcome(turn_before, &dir, step)
                .unwrap(),
            Some(MoveOutcome::Transition(AreaTransition::ChangedFloor {
                scene,
                floor: -1
            }))
        );

        assert_eq!(state.grid[0], BEACON_BRIGHT_LIGHT_TILE);
        assert_eq!(state.grid[1], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[2], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[3], 0xc9);
        assert_eq!(
            state.light_beacon.sources,
            [Some((0, 0)), None],
            "the floor reached by trapdoor harvests its beacon source"
        );
        assert!(
            harvest_location_npc_start_markers(&state.grid)
                .npc_markers
                .is_empty()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_boundary_exit_reloads_canonical_table_and_preserves_live_transport() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        write_save_template_and_empty_overlays(&dir, 0, 0xff, 10, 20);
        let mut state = test_state(open_grid(), 0, 0);
        let world_object = ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 11,
            y: 20,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        };
        let transport = TransportState::Carpet {
            type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
            tile: 184,
        };
        state.player.transport = transport;
        let canonical_player = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 12,
            y: 21,
            z: 0,
            phase: 0x42,
            aux1: 0xa5,
            aux3: 0x5a,
        };
        let canonical_table = vec![canonical_player, world_object];
        fs::write(
            dir.join(BRIT_OOL_FILENAME),
            encode_active_object_table(&canonical_table).unwrap(),
        )
        .unwrap();
        state.return_world = Some(WorldReturn {
            plane: WorldPlane::Britannia,
            x: 99,
            y: 98,
            transport: TransportState::Foot,
            sail_cadence: 1,
            sail_stall_pending: true,
            grid: vec![0xff; WORLD_SIDE * WORLD_SIDE],
            active_objects: Vec::new(),
            pending_vehicle: None,
        });
        state.visibility_dirty = false;

        assert_eq!(
            state
                .step_with_game_dir(Direction::North, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );
        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (0, 0));
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
        assert_eq!(state.player.transport, transport);
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.world_object_at(11, 20), Some(&world_object));
        assert_eq!(state.active_objects[0].phase, 0x42);
        assert_eq!(state.active_objects[0].aux1, 0xa5);
        assert_eq!(state.active_objects[0].aux3, 0x5a);
        assert!(state.return_world.is_none());
        assert_eq!(state.turn, 0);
        assert!(state.visibility_dirty);
        assert_eq!(
            state.message,
            "Yes. Left CASTLE:0 for BRITANNIA via the canonical outdoor table."
        );

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );
        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.target, PlayTarget::World(WorldPlane::Britannia));
        assert_eq!(options.start, Some((10, 20)));
        assert_eq!(
            options.transport,
            TransportState::Carpet {
                type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
                tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
            }
        );
        assert_eq!(
            options.saved_active_objects.as_ref().unwrap()[0],
            world_object
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_entry_accepts_every_published_transport_family_and_writes_all_32_slots() {
        let scene = Scene::new(17).unwrap();
        let transports = [
            TransportState::Foot,
            TransportState::Horse {
                type_byte: 0x10,
                tile: FIRST_PLAYABLE_HORSE_TILE,
            },
            TransportState::Carpet {
                type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST + 1,
                tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE + 1,
            },
            TransportState::Ship {
                type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST + 2,
                tile: FIRST_PLAYABLE_FRIGATE_TILE + 2,
                sails_hoisted: true,
                hull: 61,
                skiffs: 3,
            },
            TransportState::Skiff {
                type_byte: TRANSPORT_MARKER_SKIFF_FIRST + 3,
                tile: FIRST_PLAYABLE_SKIFF_TILE + 3,
            },
        ];

        for transport in transports {
            let dir = debug_game_dir();
            let mut state = world_state(open_world_grid(), 10, 20);
            state.area = Area::World {
                plane: WorldPlane::Britannia,
            };
            state.player.transport = transport;
            state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
            state.sync_player_object();
            state.active_objects[0].phase = 0x42;
            state.active_objects[31] = ActiveObject {
                type_byte: 0x83,
                tile: 0x84,
                x: 77,
                y: 88,
                z: 0,
                phase: 0x65,
                aux1: 0x54,
                aux3: 0x76,
            };
            let expected_table = state.active_objects.clone();
            let turn_before = state.turn;

            assert_eq!(
                state
                    .enter_world_target(
                        &dir,
                        WorldPlane::Britannia,
                        PlayTarget::Town(scene),
                        false,
                    )
                    .unwrap(),
                MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
            );

            assert_eq!(state.player.transport, transport);
            assert_eq!(state.turn, turn_before);
            assert!(state.return_world.is_none());
            assert_eq!(state.active_objects[0].phase, 0x42);
            assert_eq!(
                decode_full_ool_plane_table(
                    &fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap()
                )
                .unwrap(),
                expected_table
            );
            assert!(state.active_objects.iter().skip(1).all(|object| {
                object.type_byte != 0x83 || (object.x, object.y) != (77, 88)
            }));
            let _ = fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn direct_town_exit_reloads_before_materializing_queued_shipwright_delivery() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        let canonical_object = ActiveObject {
            type_byte: 0x81,
            tile: 0x82,
            x: 44,
            y: 45,
            z: 0,
            phase: 0x33,
            aux1: 0x22,
            aux3: 0x11,
        };
        let mut canonical_table = vec![ActiveObject::empty(); OOL_SLOTS];
        canonical_table[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 99,
            y: 98,
            z: 0,
            phase: 0x67,
            aux1: 0x56,
            aux3: 0x78,
        };
        canonical_table[2] = canonical_object;
        fs::write(
            dir.join(BRIT_OOL_FILENAME),
            encode_active_object_table(&canonical_table).unwrap(),
        )
        .unwrap();

        let pending = PendingVehicleAcquisition::Frigate {
            x: 136,
            y: 158,
            skiffs: 3,
        };
        let mut state = test_state(open_grid(), 0, 0);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.active_objects[1] = ActiveObject {
            type_byte: 0xee,
            tile: 0xef,
            x: 1,
            y: 1,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        state.pending_vehicle_save = PendingVehicleSaveState::from_acquisition(pending);
        state.return_world = None;

        assert_eq!(
            state
                .resolve_town_boundary_exit_transition(&dir, scene, 0)
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedLocation(scene))
        );

        assert_eq!(state.active_objects.len(), OOL_SLOTS);
        assert_eq!(state.active_objects[1], pending.active_object(0));
        assert_eq!(state.active_objects[2], canonical_object);
        assert_eq!(state.active_objects[0].phase, 0x67);
        assert_eq!(state.active_objects[0].aux1, 0x56);
        assert_eq!(state.active_objects[0].aux3, 0x78);
        assert_eq!(state.pending_vehicle_save.class_byte, 0);
        assert!(state.return_world.is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_boundary_exit_uses_clean_location_table_without_return_snapshot() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        let mut state = test_state(open_grid(), 31, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Observed
        );
        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (31, 0));
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
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.message,
            "Yes. Left CASTLE:0 for BRITANNIA via the canonical outdoor table."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ordinary_town_exit_ignores_conflicting_underworld_return_snapshot() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        write_save_template_and_empty_overlays(&dir, 0, 0, 10, 20);
        let mut state = test_state(open_grid(), 31, 0);
        state.return_world = Some(WorldReturn {
            plane: WorldPlane::Underworld,
            x: 99,
            y: 98,
            transport: TransportState::Carpet {
                type_byte: 184,
                tile: 184,
            },
            sail_cadence: 1,
            sail_stall_pending: true,
            grid: open_world_grid(),
            active_objects: Vec::new(),
            pending_vehicle: None,
        });

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
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert!(state.return_world.is_none());
        assert_eq!(state.active_objects[0].z, WorldPlane::Britannia.save_floor());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_boundary_exit_prompts_from_each_edge_without_committing_the_step() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        for ((x, y), direction) in [
            ((5, 0), Direction::North),
            ((31, 5), Direction::East),
            ((5, 31), Direction::South),
            ((0, 5), Direction::West),
        ] {
            let mut state = test_state(open_grid(), x, y);
            assert_eq!(
                state
                    .step_with_game_dir(direction, Some(&dir))
                    .unwrap(),
                MoveOutcome::Observed
            );
            assert_eq!(state.area, Area::Town { scene, floor: 0 });
            assert_eq!((state.player.x, state.player.y), (x, y));
            assert!(matches!(
                state.active_yes_no_prompt,
                Some(YesNoPromptSession {
                    kind: YesNoPromptKind::TownExit {
                        scene: prompt_scene,
                        floor: 0,
                    }
                }) if prompt_scene == scene
            ));
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn refusing_or_cancelling_town_boundary_exit_discards_the_step() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        for answer in ['N', '\u{1b}'] {
            let mut state = test_state(open_grid(), 0, 9);
            assert_eq!(
                state
                    .step_with_game_dir(Direction::West, Some(&dir))
                    .unwrap(),
                MoveOutcome::Observed
            );
            assert_eq!(
                handle_play_key_input(&mut state, answer, "", &dir).unwrap(),
                PlayInputDisposition::Continue
            );
            assert_eq!(state.area, Area::Town { scene, floor: 0 });
            assert_eq!((state.player.x, state.player.y), (0, 9));
            assert_eq!(state.active_yes_no_prompt, None);
            assert_eq!(state.turn, 1);
            assert_eq!(state.message, "No.");
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_boundary_exit_uses_southeast_corner_terrain_for_every_edge() {
        let dir = debug_game_dir();
        for ((x, y), key) in [((5, 0), '8'), ((31, 5), '6'), ((5, 31), '2'), ((0, 5), '4')]
        {
            let mut grid = open_grid();
            grid[31 * 32 + 31] = BRIT_DEEP_WATER_TILE;
            let mut state = test_state(grid, x, y);

            assert_eq!(
                handle_play_key_input(&mut state, key, "", &dir).unwrap(),
                PlayInputDisposition::Continue
            );
            assert_eq!((state.player.x, state.player.y), (x, y));
            assert_eq!(state.active_yes_no_prompt, None);
            assert_eq!(state.turn, 1);
            assert!(state.message.contains("Blocked by"));
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_boundary_exit_occupancy_uses_the_true_out_of_grid_candidate() {
        let mut state = test_state(open_grid(), 0, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 31,
            y: 31,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert!(state.blocking_town_object_at_candidate(-1, 5).is_none());
        assert!(state.blocking_town_object_at_candidate(32, 5).is_none());
        assert_eq!(state.step(Direction::West), MoveOutcome::Observed);
        assert!(state.active_yes_no_prompt.is_some());
    }

    #[test]
    fn unrelated_town_exit_prompt_key_waits_without_spending_a_turn() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 0, 5);
        assert_eq!(state.step(Direction::West), MoveOutcome::Observed);

        assert_eq!(
            handle_play_key_input(&mut state, 'Q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 0);
        assert!(state.active_yes_no_prompt.is_some());
        assert_eq!(state.message, "Leave CASTLE:0?");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn telescope_tile_and_retired_exit_sidecar_never_raise_leave_prompt() {
        let dir = debug_game_dir();
        fs::write(
            dir.join("town_exit_tiles.tsv"),
            "CASTLE:0 0 1 0 16\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[1] = 16;
        grid[32 + 1] = TELESCOPE_LOOK_TRIGGER_TILE;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.active_yes_no_prompt, None);

        state.player.x = 1;
        state.player.y = 1;
        state.sync_player_object();
        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );
        assert_eq!(state.active_yes_no_prompt, None);
        assert!(state.message.starts_with("Passed."));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_boundary_exit_clears_visit_local_door_state() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        let mut state = test_state(open_grid(), 0, 0);
        state.door_tracker = Some(DoorTracker {
            previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
            x: 3,
            y: 1,
            turns_remaining: 4,
        });
        state.record_open_town_door(scene, 0, 3, 1);
        state.record_revealed_town_secret_door(scene, 0, 4, 1);
        state.record_open_town_door(scene, 0, 4, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::North, Some(&dir))
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

    /// `cleak/u5-spec#51`: seed the PRNG so the first
    /// `u5_prng_range_u16(_, 0, 29)` roll returns a nonzero value,
    /// triggering a deterministic poison hit for a zero-Dexterity test
    /// member.
    fn poison_gas_first_poison_seed() -> u16 {
        for candidate in 0..=u16::MAX {
            let mut state = candidate;
            if u5_prng_range_u16(&mut state, 0, TOWN_GAS_DOORWAY_RANGE_MAX) > 0 {
                return candidate;
            }
        }
        unreachable!("PRNG range cycle must hit a nonzero value")
    }

    /// `town-mode.md §17` "Underfoot-effect cadence is fixed": "The underfoot
    /// handler is a per-turn post-action pass, not a step-commit hook. Any
    /// earlier statement that the poison-gas effect 'fires from the step path'
    /// is retracted". A test that wants an underfoot effect therefore has to
    /// run the post-action pass the dispatcher runs, after the command's own
    /// clock advance - the step alone no longer produces one.
    fn step_then_post_turn(
        state: &mut PlayState,
        direction: Direction,
        dir: &std::path::Path,
    ) -> MoveOutcome {
        let turn_before = state.turn;
        let outcome = state.step_with_game_dir(direction, Some(dir)).unwrap();
        state
            .apply_post_turn_effects_after_outcome(turn_before, dir, outcome)
            .unwrap()
            .unwrap_or(outcome)
    }

    #[test]
    fn town_movement_onto_native_poison_gas_tile_poisons_eligible_member() {
        // `cleak/u5-spec#51`: every eligible member rolls
        // `prng_range(0, 29)` independently per step; the member is
        // poisoned when the roll is greater than Dexterity.
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = TOWN_POISON_GAS_LIVE_TILE;
        let mut state = test_state(grid, 0, 1);
        state.prng_state = poison_gas_first_poison_seed();
        state.player.facing = Direction::East;
        state.party[0].status = b'G';
        state.party[0].climb_stat = 0;
        state.party[0].hp = 10;

        assert_eq!(
            step_then_post_turn(&mut state, Direction::East, &dir),
            MoveOutcome::Moved
        );

        assert_eq!(state.party[0].status, b'P');
        assert!(state.message.contains("Avatar is poisoned!"));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn town_movement_onto_poison_gas_tile_requires_foot_transport() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = TOWN_POISON_GAS_LIVE_TILE;
        let mut state = test_state(grid, 0, 1);
        state.prng_state = poison_gas_first_poison_seed();
        state.player.facing = Direction::East;
        state.player.transport = TransportState::Horse {
            type_byte: 0x10,
            tile: 0x10,
        };
        state.party[0].status = b'G';
        state.party[0].climb_stat = 0;
        state.party[0].hp = 10;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.party[0].status, b'G');
        assert!(!state.message.contains("poison gas doorway"));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn town_movement_poison_gas_coordinate_sidecar_no_longer_triggers() {
        let dir = debug_game_dir();
        fs::write(
            dir.join("town_poison_gas.tsv"),
            "CASTLE:0 0 1 1 0x37\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 0x37;
        let mut state = test_state(grid, 0, 1);
        state.prng_state = poison_gas_first_poison_seed();
        state.player.facing = Direction::East;
        state.party[0].status = b'G';
        state.party[0].climb_stat = 0;
        state.party[0].hp = 10;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.party[0].status, b'G');
        assert!(!state.message.contains("poison gas doorway"));
    }

    #[test]
    fn town_movement_poison_gas_tile_attribute_sidecar_no_longer_triggers() {
        let dir = debug_game_dir();
        fs::write(dir.join("town_tile_attributes.tsv"), "0x37 4 0x1C\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 0x37;
        let mut state = test_state(grid, 0, 1);
        state.prng_state = poison_gas_first_poison_seed();
        state.player.facing = Direction::East;
        state.party[0].status = b'G';
        state.party[0].climb_stat = 0;
        state.party[0].hp = 10;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.party[0].status, b'G');
        assert!(!state.message.contains("poison gas doorway"));
    }

    #[test]
    fn town_poison_gas_rolls_only_non_poisoned_status_slots() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = TOWN_POISON_GAS_LIVE_TILE;
        let mut state = test_state(grid, 0, 1);
        state.prng_state = poison_gas_first_poison_seed();
        state.player.facing = Direction::East;
        state.party[0].status = b'P';
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'A',
            status: b'S',
            climb_stat: 0,
            mana: 0,
            hp: 10,
            max_hp: 10,
            level: 1,
        });

        assert_eq!(
            step_then_post_turn(&mut state, Direction::East, &dir),
            MoveOutcome::Moved
        );

        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[1].status, b'P');
        assert!(state.message.contains("Party member 1 is poisoned!"));
    }

    #[test]
    fn town_poison_gas_skips_poisoned_slots_without_advancing_prng() {
        let mut state = test_state(open_grid(), 0, 1);
        state.prng_state = poison_gas_first_poison_seed();
        state.party[0].status = b'P';
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'A',
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 10,
            max_hp: 10,
            level: 1,
        });
        let mut expected_prng = state.prng_state;
        let expected_roll =
            u5_prng_range_u16(&mut expected_prng, 0, TOWN_GAS_DOORWAY_RANGE_MAX);
        assert!(expected_roll > 0);

        let report = state.apply_town_poison_gas(TownPoisonGasEntry {
            scene: Scene::new(17).unwrap(),
            floor: 0,
            x: 0,
            y: 1,
            expected_tile: None,
        });

        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(report, "Party member 1 is poisoned!");
    }

    /// `town-mode.md §17` "Underfoot-effect cadence is fixed": "The underfoot
    /// handler is a per-turn post-action pass, not a step-commit hook. Any
    /// earlier statement that the poison-gas effect 'fires from the step path'
    /// is retracted: it fires once per turn-consuming action while the party
    /// occupies the tile, including turns spent passing in place, and it fires
    /// **after that turn's clock advance**."
    ///
    /// This test previously pinned the retracted step-commit cadence under the
    /// name `town_poison_gas_step_rolls_before_turn_clock_tick`.
    #[test]
    fn town_poison_gas_step_rolls_after_turn_clock_tick() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = TOWN_POISON_GAS_LIVE_TILE;
        let mut state = test_state(grid, 0, 1);
        state.prng_state = poison_gas_first_poison_seed();
        state.clock = GameClock::new(8, 59).unwrap();
        state.player.facing = Direction::East;
        state.party[0].status = b'G';
        state.party[0].climb_stat = 0;
        state.party[0].hp = 10;

        // The bare step commits the move and this turn's clock advance but
        // produces no underfoot effect: the gas arm is not a step-commit hook
        // any more.
        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );
        assert_eq!(state.clock.hour, 9);
        assert_eq!(state.party[0].status, b'G');

        // The post-action pass is what rolls, and it runs after that advance.
        state
            .apply_post_turn_effects_after_outcome(0, &dir, MoveOutcome::Moved)
            .unwrap();

        assert_eq!(state.clock.hour, 9);
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(
            state.party[0].hp, 9,
            "the trailing status pass damages a member poisoned this turn",
        );
    }

    /// `town-mode.md §17`: the poison-gas effect "fires once per turn-consuming
    /// action while the party occupies the tile, **including turns spent
    /// passing in place**". §10 says the same thing from the other side:
    /// "standing on a gas tile is not safe: every turn spent on it is a fresh
    /// save for every eligible member, so a party that lingers will eventually
    /// be poisoned."
    ///
    /// This test previously pinned the retracted step-only reading under the
    /// name `pass_turn_on_native_poison_gas_tile_does_not_reroll_underfoot`.
    #[test]
    fn pass_turn_on_native_poison_gas_tile_rerolls_underfoot() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = TOWN_POISON_GAS_LIVE_TILE;
        let mut state = test_state(grid, 1, 1);
        state.prng_state = poison_gas_first_poison_seed();
        state.party[0].status = b'G';
        state.party[0].climb_stat = 0;
        state.party[0].hp = 10;

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );

        assert_eq!(
            state.party[0].status,
            b'P',
            "a passed turn on the gas tile is a fresh save",
        );
        assert_ne!(
            state.prng_state,
            poison_gas_first_poison_seed(),
            "the passed turn consumed a save roll",
        );
        assert!(
            state.message.starts_with("Passed."),
            "the pass line is kept and the gas report appended: {}",
            state.message,
        );
        assert!(
            state.message.contains("is poisoned!"),
            "the gas report is appended to the pass line: {}",
            state.message,
        );
    }

    #[test]
    fn town_k_on_non_link_prompts_for_adjacent_climb_over() {
        let dir = debug_game_dir();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        let mut grid = open_grid();
        grid[1] = TOWN_KLIMB_FENCE_FIRST;
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
            handle_play_key_input(&mut state, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Town { scene: Scene::new(17).unwrap(), floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(state.active_direction_prompt.is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_k_directional_descend_links_go_down_without_prompt() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        for tile in [TOWN_KLIMB_DESCEND_TILE, TOWN_KLIMB_DESCEND_GRATE_TILE] {
            let mut grid = open_grid();
            grid[0] = tile;
            let mut state = test_state(grid, 0, 0);
            state.area = Area::Town { scene, floor: 1 };
            state.active_objects[0].z = 1;

            assert_eq!(
                state.klimb_command(&dir).unwrap(),
                MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 0 })
            );
            assert_eq!(state.area, Area::Town { scene, floor: 0 });
            assert_eq!(state.turn, 1);
            assert_eq!(state.message, "Down!");
            assert!(state.active_direction_prompt.is_none());
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_k_on_horse_refuses_for_free_even_on_floor_link() {
        let mut grid = open_grid();
        grid[0] = TOWN_KLIMB_ASCEND_TILE;
        let mut state = test_state(grid, 0, 0);
        mount_horse(&mut state);

        assert_eq!(
            state.klimb_command(Path::new("")).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "-On foot!");
        assert_eq!(state.turn, 0);
        assert_eq!((state.player.x, state.player.y), (0, 0));
    }

    /// `commands.md §5.3` lists Klimb among the commands whose trailing
    /// hyphen awaits a direction, so `Klimb-` is the shared adjacent-tile
    /// prompt of `commands.md §5.4`: "The prompt loop accepts only the
    /// four directions and Space ... Escape does not reach a cancellation
    /// arm: it emits nothing and the prompt reads again. An earlier
    /// revision of this table listed `Space` **or** `Esc` as producing
    /// `Pass` and a cancelled result ... both are retracted." This test
    /// used to press Escape and expect `Pass` plus a spent turn.
    #[test]
    fn town_k_adjacent_invalid_target_is_free_and_pass_costs_action() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);

        assert_eq!(state.klimb_command(&dir).unwrap(), MoveOutcome::Observed);
        assert_eq!(
            handle_play_key_input(&mut state, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "What?");
        assert_eq!(state.turn, 0);

        assert_eq!(state.klimb_command(&dir).unwrap(), MoveOutcome::Observed);
        // Escape emits nothing and the prompt reads again: no `Pass`, no
        // turn, and the session is still waiting for a direction.
        assert_eq!(
            handle_play_key_input(&mut state, '\u{1b}', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_some());
        assert_eq!(state.message, "Klimb-");
        assert_eq!(state.turn, 0);

        // Space is the one pass key, and it is what spends the action.
        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_none());
        assert_eq!(state.message, DIRECTION_PROMPT_LABEL_PASS);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn native_town_trapdoor_is_post_turn_damaging_floor_transition() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        let mut grid = open_grid();
        grid[1] = TOWN_TRAPDOOR_LIVE_TILE;
        let mut state = test_state(grid, 0, 0);
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: CharacterStatus::Dead.save_byte(),
            climb_stat: 0,
            mana: 0,
            hp: 7,
            max_hp: 7,
            level: 1,
        });

        assert_eq!(
            handle_play_key_input(&mut state, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Town { scene, floor: -1 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!((12..=19).contains(&state.party[0].hp));
        assert_eq!(state.party[1].hp, 7, "Dead slots are not rolled or damaged");
        assert_eq!(state.party[1].status, CharacterStatus::Dead.save_byte());
        assert!(state.message.contains("A TRAPDOOR!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stonegate_trapdoor_applies_exact_scripted_defeat_without_transition() {
        let dir = debug_game_dir();
        let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = TOWN_TRAPDOOR_LIVE_TILE;
        let mut state = test_state(grid, 1, 1);
        state.area = Area::Town { scene, floor: 0 };
        state.player.transport = TransportState::Horse {
            type_byte: HORSE_MOUNTED_FIRST,
            tile: HORSE_MOUNTED_FIRST,
        };
        state.sync_player_object();
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.active_objects[1] = ActiveObject {
            type_byte: 0x40,
            tile: 0x40,
            x: 2,
            y: 3,
            z: 0,
            aux1: 4,
            phase: 5,
            aux3: 6,
        };
        state.party[0].hp = 20;
        state.party[0].max_hp = 31;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: CharacterStatus::Dead.save_byte(),
            climb_stat: 0,
            mana: 0,
            hp: 7,
            max_hp: 42,
            level: 1,
        });
        state.clock = GameClock::new(5, 59).unwrap();
        state.food = 50;
        state.active_player = Some(0);
        let transport_before = state.player.transport;
        let max_hp_before = state.party.iter().map(|member| member.max_hp).collect::<Vec<_>>();

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Used
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.transport, transport_before);
        assert_eq!(state.turn, 1);
        assert_eq!((state.clock.hour, state.clock.minute), (6, 0));
        assert_eq!(state.food, 50, "the post-death status tail sees zero eaters");
        assert_eq!(state.active_player, None);
        assert!(state.grid.iter().all(|tile| *tile == STONEGATE_TRAPDOOR_GRID_TILE));
        assert!(state.visibility_dirty);

        assert_eq!(state.active_objects.len(), OOL_SLOTS);
        assert_eq!(
            state.active_objects[0],
            ActiveObject {
                x: 1,
                y: 1,
                z: 0,
                ..ActiveObject::empty()
            }
        );
        assert!(state.active_objects[1..].iter().all(|object| object.is_empty()));

        assert!(state.party.iter().all(|member| member.hp == 0));
        assert!(
            state
                .party
                .iter()
                .all(|member| member.status == CharacterStatus::Dead.save_byte())
        );
        assert_eq!(
            state.party.iter().map(|member| member.max_hp).collect::<Vec<_>>(),
            max_hp_before
        );
        assert_eq!(state.message, "Passed. A TRAPDOOR!");
        assert_eq!(
            state.take_pending_stonegate_trapdoor_playback(),
            Some(StonegateTrapdoorPlayback::complete(2))
        );
        assert_eq!(state.take_pending_stonegate_trapdoor_playback(), None);
    }

    #[test]
    fn magic_carpet_suppresses_native_stonegate_trapdoor() {
        let dir = debug_game_dir();
        let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = TOWN_TRAPDOOR_LIVE_TILE;
        let mut state = test_state(grid, 1, 1);
        state.area = Area::Town { scene, floor: 0 };
        state.player.transport = TransportState::Carpet {
            type_byte: CARPET_MOUNTED,
            tile: CARPET_MOUNTED,
        };
        state.sync_player_object();
        let hp = state.party[0].hp;

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );
        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!(state.party[0].hp, hp);
        assert_eq!(state.grid[32 + 1], TOWN_TRAPDOOR_LIVE_TILE);
        assert_eq!(state.message, "Passed.");
        assert_eq!(state.pending_stonegate_trapdoor_playback, None);
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
        assert_eq!(state.message, "Up!");
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
    fn town_k_non_ladder_starts_direction_prompt_without_turn() {
        let mut state = test_state(open_grid(), 1, 1);

        assert_eq!(
            state.klimb_command(Path::new("")).unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(state.message, "Klimb-");
        assert_eq!(
            state.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::Klimb)
        );
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
    fn town_k_on_walk_on_stair_does_not_offer_vertical_climb() {
        let dir = debug_game_dir();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        let mut grid = open_grid();
        grid[1] = TOWN_STAIR_TILE_FIRST;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
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
        assert_eq!(state.turn, 1);

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
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.active_objects[0].x, 1);
        assert_eq!(state.turn, 1);

        assert_eq!(handle_play_key_input(&mut state, '6', "", &dir).unwrap(), PlayInputDisposition::Continue);

        assert_eq!(state.area, Area::Town { scene: Scene::new(17).unwrap(), floor: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "What?");
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
        grid[world_cell_index(11, 20)] = 0x2e;
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
