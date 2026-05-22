    #[test]
    fn sync_player_object_repairs_slot_zero_and_clears_duplicate_player_records() {
        let mut state = world_state(open_world_grid(), 4, 5);
        state.player.transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        state.active_objects[0] = ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 9,
            y: 9,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x21,
            aux1: 7,
            aux3: 8,
        };
        state.active_objects.push(ActiveObject {
            type_byte: PLAYER_TILE,
            tile: 201,
            x: 1,
            y: 1,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x66,
            aux1: 0x55,
            aux3: 0x77,
        });

        state.sync_player_object();

        assert_eq!(
            state.active_objects[0],
            ActiveObject {
                type_byte: PLAYER_TILE,
                tile: 184,
                x: 4,
                y: 5,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 0,
                tile: 201,
                x: 1,
                y: 1,
                z: WorldPlane::Underworld.save_floor(),
                phase: 0x66,
                aux1: 0x55,
                aux3: 0x77,
            }
        );
        assert!(state.active_objects[1].is_empty());
    }

    #[test]
    fn sync_player_object_recreates_empty_active_object_table() {
        let mut state = dungeon_state(open_dungeon_record(), 3, 2, 4);
        state.active_objects.clear();

        state.sync_player_object();

        assert_eq!(
            state.active_objects,
            vec![ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: 2,
                y: 4,
                z: 3,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }]
        );
    }

    #[test]
    fn movement_blocks_impassable_tiles_without_spending_turn() {
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;

        assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
        assert_eq!(state.animation.frame, 0);
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn movement_uses_optional_passability_bitmap() {
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.passability = Some(passability_with_tiles(&[24]));

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn movement_blocks_same_floor_active_object_without_spending_turn() {
        let mut state = test_state(open_grid(), 1, 1);
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

        assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn movement_ignores_other_floor_active_object() {
        let mut state = test_state(open_grid(), 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn movement_out_of_bounds_missing_return_metadata_stays_in_location() {
        let scene = Scene::new(0x11).unwrap();
        let mut state = test_state(open_grid(), 0, 3);

        assert_eq!(state.step(Direction::West), MoveOutcome::Blocked);
        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (0, 3));
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, 1);
        assert!(
            state
                .message
                .contains("missing clean return-coordinate metadata")
        );
    }

    #[test]
    fn world_movement_wraps_and_advances_outdoor_time() {
        let mut state = world_state(open_world_grid(), 255, 0);
        state.clock = GameClock::new(12, 58).unwrap();

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.active_objects[0].x, 0);
        assert_eq!(state.active_objects[0].z, -1);
        assert_eq!(state.clock, GameClock::new(13, 0).unwrap());
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn skiff_world_movement_uses_half_time_with_one_minute_floor() {
        let mut state = world_state(vec![1; WORLD_CELLS], 0, 0);
        state.player.transport = TransportState::Skiff {
            type_byte: 176,
            tile: 176,
        };
        state.timing_status = TimingStatusTag::for_transport(state.player.transport);
        state.sync_player_object();
        state.clock = GameClock::new(12, 58).unwrap();

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.clock, GameClock::new(12, 59).unwrap());
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn timing_status_q_halves_world_time_without_skiff() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.timing_status = TimingStatusTag::HalfTime;
        state.clock = GameClock::new(12, 58).unwrap();
        state.torch_counter = 3;
        state.light_spell_counter = 2;

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.clock, GameClock::new(12, 59).unwrap());
        assert_eq!(state.torch_counter, 2);
        assert_eq!(state.light_spell_counter, 1);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn timing_status_t_skips_minutes_and_light_but_runs_cleanup() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.timing_status = TimingStatusTag::NoMinuteLight;
        state.clock = GameClock::new(12, 58).unwrap();
        state.torch_counter = 5;
        state.light_spell_counter = 4;
        state.visibility_dirty = false;

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.clock, GameClock::new(12, 58).unwrap());
        assert_eq!(state.torch_counter, 5);
        assert_eq!(state.light_spell_counter, 4);
        assert_eq!(state.ambient_light, TORCH_LIGHT_FLOOR);
        assert!(state.visibility_dirty);
        assert_eq!(state.animation.frame, 1);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn world_movement_blocks_impassable_tiles_without_turn() {
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 0)] = 24;
        let mut state = world_state(grid, 0, 0);

        assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);

        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.clock, GameClock::default());
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn world_movement_blocks_active_object_without_turn() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 170,
            tile: 170,
            x: 1,
            y: 0,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);

        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.clock, GameClock::default());
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("world object tile 170"));
        assert!(state.message.contains("slot 1"));
        assert!(state.message.contains("no terrain-combat arena selected"));
        assert!(!state.message.contains("out of scope"));
    }

    #[test]
    fn world_movement_into_combat_class_object_selects_brit_cbt_arena() {
        let dir = debug_game_dir();
        let record = synthetic_combat_arena_record();
        fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 0x50,
            tile: 0xc0,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Used
        );

        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.turn, 1);
        assert!(state.combat_active);
        assert_eq!(state.pending_combat_terrain_trigger_slot, Some(1));
        assert!(state.message.contains("slot 1"));
        assert!(state.message.contains("entered terrain combat"));
        assert!(state.message.contains("BRIT.CBT arena 4"));
        assert!(state.message.contains("Orc"));
        assert_eq!(state.active_objects[6].tile, 0xc0);
        assert_eq!(
            (state.active_objects[6].x, state.active_objects[6].y),
            (0, 15)
        );
        assert!(!state.message.contains("out of scope"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn board_vehicle_uses_facing_active_object_and_clears_parked_slot() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        let parked = ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 77,
            aux3: 2,
        };
        state.active_objects.push(parked);

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

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
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(state.active_objects[0].tile, 168);
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 0,
                ..parked
            }
        );
        assert!(state.active_objects[1].is_empty());
        assert!(state.world_object_at(1, 0).is_none());
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Boarded ship.");
    }

    #[test]
    fn board_vehicle_removed_parked_object_is_included_in_saved_overworld_overlay() {
        let dir = debug_game_dir();
        fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0xff, 0, 0)).unwrap();
        fs::write(
            dir.join(UNDER_DAT_FILENAME),
            vec![BRIT_DEEP_WATER_TILE; UNDER_DAT_LEN],
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 77,
            aux3: 2,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
        assert!(state.active_objects[1].is_empty());

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
        let underworld =
            decode_ool_plane_objects(&saved_ool[OOL_PLANE_LEN..SAVED_OOL_LEN]).unwrap();
        assert!(underworld[0].is_empty());

        let saved_gam = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(
            saved_gam[SAVE_TRANSPORT_MARKER_OFFSET],
            TRANSPORT_MARKER_SHIP_FURLED_FIRST
        );
        let saved_active = decode_active_object_table(
            &saved_gam[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
            "SAVED.GAM",
        )
        .unwrap();
        assert!(saved_active[0].is_empty());

        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.target, PlayTarget::World(WorldPlane::Underworld));
        assert_eq!(options.start, Some((0, 0)));
        assert_eq!(
            options.transport,
            TransportState::Ship {
                type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
                tile: FIRST_PLAYABLE_FRIGATE_TILE,
                sails_hoisted: false,
                hull: 77,
                skiffs: 2,
            }
        );
        assert!(options.saved_active_objects.as_ref().unwrap()[0].is_empty());
        let reloaded = PlayState::load_scene(&dir, options).unwrap();
        assert_eq!(
            reloaded.player.transport,
            TransportState::Ship {
                type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
                tile: FIRST_PLAYABLE_FRIGATE_TILE,
                sails_hoisted: false,
                hull: 77,
                skiffs: 2,
            }
        );
        assert!(reloaded.active_objects[1].is_empty());
        assert!(reloaded.world_object_at(1, 0).is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn board_skiff_sets_half_time_timing_status_and_exit_clears_it() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 176,
            tile: 176,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert_eq!(
            state.player.transport,
            TransportState::Skiff {
                type_byte: 176,
                tile: 176,
            }
        );
        assert_eq!(state.timing_status, TimingStatusTag::HalfTime);

        assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.timing_status, TimingStatusTag::Normal);
    }

    #[test]
    fn board_ship_accepts_waterborne_transport_state() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.transport = TransportState::Skiff {
            type_byte: 176,
            tile: 176,
        };
        state.timing_status = TimingStatusTag::HalfTime;
        state.player.facing = Direction::East;
        state.sync_player_object();
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert_eq!(
            state.player.transport,
            TransportState::Ship {
                type_byte: 168,
                tile: 168,
                sails_hoisted: false,
                hull: 0,
                skiffs: 0,
            }
        );
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(state.active_objects[0].tile, 168);
        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.timing_status, TimingStatusTag::Normal);
        assert_eq!(state.turn, 1);
        assert_eq!(
            state.message,
            format!("Boarded ship. {SHIP_BADLY_DAMAGED_WARNING} {SHIP_NO_SKIFFS_WARNING}")
        );
    }

    #[test]
    fn board_ship_accepts_public_object_byte_and_preserves_save_marker() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST + 1,
            tile: 0,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 88,
            aux3: 1,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert_eq!(
            state.player.transport,
            TransportState::Ship {
                type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST + 1,
                tile: FIRST_PLAYABLE_FRIGATE_TILE + 1,
                sails_hoisted: false,
                hull: 88,
                skiffs: 1,
            }
        );
        assert_eq!(
            state.player.transport.save_marker(),
            TRANSPORT_MARKER_SHIP_FURLED_FIRST + 1
        );
        assert_eq!(state.active_objects[0].tile, FIRST_PLAYABLE_FRIGATE_TILE + 1);
        assert!(state.active_objects[1].is_empty());
    }

    #[test]
    fn vehicle_directional_step_refreshes_transport_marker_and_player_tile() {
        let mut state = world_state(open_world_grid(), 4, 4);
        state.player.transport = TransportState::Carpet {
            type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
            tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        };
        state.sync_player_object();

        assert_eq!(
            state
                .step_with_game_dir(Direction::West, None)
                .expect("world carpet step is in-memory"),
            MoveOutcome::Moved
        );

        assert_eq!(
            state.player.transport.save_marker(),
            TRANSPORT_MARKER_MAGIC_CARPET_LAST
        );
        assert_eq!(state.active_objects[0].tile, FIRST_PLAYABLE_MAGIC_CARPET_TILE + 3);
    }

    #[test]
    fn board_ship_accepts_carpet_north_east_and_stows_carpet() {
        for marker in [TRANSPORT_MARKER_MAGIC_CARPET_FIRST, TRANSPORT_MARKER_MAGIC_CARPET_FIRST + 1] {
            let mut state = world_state(open_world_grid(), 0, 0);
            state.player.transport = TransportState::Carpet {
                type_byte: marker,
                tile: transport_visual_tile_for_marker(marker).unwrap(),
            };
            state.player.facing = Direction::East;
            state.sync_player_object();
            state.active_objects.push(ActiveObject {
                type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
                tile: 0,
                x: 1,
                y: 0,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 77,
                aux3: 2,
            });

            assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

            assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 1);
            assert!(matches!(state.player.transport, TransportState::Ship { .. }));
            assert_eq!(state.message, "Boarded ship.");
        }
    }

    #[test]
    fn board_ship_refuses_carpet_south_west() {
        for marker in [TRANSPORT_MARKER_MAGIC_CARPET_FIRST + 2, TRANSPORT_MARKER_MAGIC_CARPET_FIRST + 3] {
            let mut state = world_state(open_world_grid(), 0, 0);
            state.player.transport = TransportState::Carpet {
                type_byte: marker,
                tile: transport_visual_tile_for_marker(marker).unwrap(),
            };
            state.player.facing = Direction::East;
            state.sync_player_object();
            state.active_objects.push(ActiveObject {
                type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
                tile: 0,
                x: 1,
                y: 0,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 77,
                aux3: 2,
            });

            assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);

            assert_eq!(
                state.player.transport,
                TransportState::Carpet {
                    type_byte: marker,
                    tile: transport_visual_tile_for_marker(marker).unwrap(),
                }
            );
            assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 0);
            assert_eq!(state.message, "On foot.");
            assert!(!state.active_objects[1].is_empty());
        }
    }

    #[test]
    fn board_ship_with_zero_hull_reports_badly_damaged_warning() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 2,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert_eq!(
            state.player.transport,
            TransportState::Ship {
                type_byte: 168,
                tile: 168,
                sails_hoisted: false,
                hull: 0,
                skiffs: 2,
            }
        );
        assert_eq!(
            state.message,
            format!("Boarded ship. {SHIP_BADLY_DAMAGED_WARNING}")
        );
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn board_ship_with_hull_below_ten_reports_badly_damaged_warning() {
        // vehicles.md §4: ship boarding warns when hull condition is below
        // ten, not just zero.
        for hull in [1u8, 5, 9] {
            let mut state = world_state(open_world_grid(), 0, 0);
            state.player.facing = Direction::East;
            state.active_objects.push(ActiveObject {
                type_byte: 168,
                tile: 168,
                x: 1,
                y: 0,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: hull,
                aux3: 2,
            });

            assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

            assert!(
                state.message.contains(SHIP_BADLY_DAMAGED_WARNING),
                "hull={hull} should report badly-damaged"
            );
        }
    }

    #[test]
    fn board_ship_with_hull_at_ten_or_above_omits_badly_damaged_warning() {
        // vehicles.md §4: hull condition of ten or higher does not warn.
        for hull in [10u8, 50, 100] {
            let mut state = world_state(open_world_grid(), 0, 0);
            state.player.facing = Direction::East;
            state.active_objects.push(ActiveObject {
                type_byte: 168,
                tile: 168,
                x: 1,
                y: 0,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: hull,
                aux3: 2,
            });

            assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

            assert!(
                !state.message.contains(SHIP_BADLY_DAMAGED_WARNING),
                "hull={hull} should not report badly-damaged"
            );
        }
    }

    #[test]
    fn board_non_ship_vehicle_still_requires_foot() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.transport = TransportState::Skiff {
            type_byte: 176,
            tile: 176,
        };
        state.player.facing = Direction::East;
        state.sync_player_object();
        state.active_objects.push(ActiveObject {
            type_byte: 160,
            tile: 160,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);

        assert_eq!(
            state.player.transport,
            TransportState::Skiff {
                type_byte: 176,
                tile: 176,
            }
        );
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("On foot"));
    }

    #[test]
    fn board_vehicle_accepts_magic_carpet_from_foot() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 184,
            tile: 184,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert_eq!(
            state.player.transport,
            TransportState::Carpet {
                type_byte: 184,
                tile: 184,
            }
        );
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(state.active_objects[0].tile, 184);
        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("carpet"));
    }

    #[test]
    fn board_vehicle_refuses_unpromoted_balloon_family() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 188,
            tile: 188,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);

        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn board_town_horse_refuses_occupied_object_with_nay_without_turn() {
        let mut state = test_state(vec![5; 32 * 32], 0, 0);
        state.player.facing = Direction::South;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 0,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 160,
            tile: 160,
            x: 0,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);

        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.message, "Nay!");
        assert_eq!(state.turn, 0);
        assert!(!state.active_objects[1].is_empty());
        assert!(!state.active_objects[2].is_empty());
    }

    #[test]
    fn board_world_horse_ignores_town_occupancy_refusal() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 160,
            tile: 160,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert_eq!(
            state.player.transport,
            TransportState::Horse {
                type_byte: 160,
                tile: 160,
            }
        );
        assert!(!state.active_objects[1].is_empty());
        assert!(state.active_objects[2].is_empty());
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn carpet_world_movement_uses_standard_outdoor_time() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        state.sync_player_object();
        state.clock = GameClock::new(12, 58).unwrap();

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.clock, GameClock::new(13, 0).unwrap());
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn balloon_world_movement_drifts_with_wind_over_blocked_terrain() {
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 0)] = 24;
        let mut state = world_state(grid, 0, 0);
        mount_balloon(&mut state);
        state.wind = WindState::East;

        assert_eq!(state.step(Direction::South), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.active_objects[0].tile, FIRST_PLAYABLE_BALLOON_TILE);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Moved East"));
        assert!(state.message.contains("underfoot wall"));
    }

    #[test]
    fn balloon_calm_wind_consumes_turn_without_moving() {
        let mut state = world_state(open_world_grid(), 10, 10);
        mount_balloon(&mut state);

        assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);

        assert_eq!((state.player.x, state.player.y), (10, 10));
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert!(state.message.contains("calm wind"));
    }

    #[test]
    fn exit_vehicle_parks_magic_carpet_object_and_returns_to_foot() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.player.transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        state.sync_player_object();

        assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert!(state.active_objects.iter().skip(1).any(|object| {
            object.type_byte == 184
                && object.tile == 184
                && object.x == 5
                && object.y == 5
                && object.z == WorldPlane::Underworld.save_floor()
        }));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("carpet"));
    }

    #[test]
    fn exit_vehicle_parks_debug_balloon_when_current_cell_can_land() {
        let mut state = world_state(open_world_grid(), 5, 5);
        mount_balloon(&mut state);

        assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert!(state.active_objects.iter().skip(1).any(|object| {
            object.type_byte == FIRST_PLAYABLE_BALLOON_TILE
                && object.tile == FIRST_PLAYABLE_BALLOON_TILE
                && object.x == 5
                && object.y == 5
                && object.z == WorldPlane::Underworld.save_floor()
        }));
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "balloon!");
    }

    #[test]
    fn exit_vehicle_refuses_debug_balloon_on_mountain_or_wall_without_turn() {
        // 0x0c = "mountains" per LOOK2.DAT; 0x18 = "a dungeon" landmark.
        // Earlier this test used 10 ("tropical forest") as a mountain
        // stand-in, but tropical forest is dense forest, not a mountain
        // -- a balloon CAN land on dense forest per the spec but cannot
        // land on a mountain or wall.
        for tile in [0x0c, 24] {
            let mut grid = open_world_grid();
            grid[world_cell_index(5, 5)] = tile;
            let mut state = world_state(grid, 5, 5);
            mount_balloon(&mut state);

            assert_eq!(state.exit_vehicle(), MoveOutcome::Blocked);

            assert_eq!(
                state.player.transport,
                TransportState::Balloon {
                    type_byte: FIRST_PLAYABLE_BALLOON_TILE,
                    tile: FIRST_PLAYABLE_BALLOON_TILE,
                }
            );
            assert_eq!((state.player.x, state.player.y), (5, 5));
            assert_eq!(state.active_objects.len(), 1);
            assert_eq!(state.turn, 0);
            assert_eq!(state.message, "Not here!");
        }
    }

    #[test]
    fn exit_vehicle_reports_on_foot_without_turn_when_walking() {
        let mut state = world_state(open_world_grid(), 5, 5);

        assert_eq!(state.exit_vehicle(), MoveOutcome::Blocked);

        assert_eq!(state.message, "On foot!");
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn exit_vehicle_refuses_dungeon_before_vehicle_landing_logic() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
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
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.message, "Not here!");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn exit_vehicle_skips_occupied_landing_cells() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 77,
            skiffs: 2,
        };
        state.sync_player_object();
        state.active_objects.push(ActiveObject {
            type_byte: 194,
            tile: 194,
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
            object.type_byte == 168
                && object.x == 5
                && object.y == 5
                && object.z == WorldPlane::Underworld.save_floor()
        }));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn exit_vehicle_skips_clean_lava_sidecar_for_foot_landing() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 6 5 LAVA 5\n",
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
    fn exit_vehicle_skips_foot_damaging_sidecar_landing_cells() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 6 5 DROWNING 5\n",
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

        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "ship!");
        let _ = fs::remove_dir_all(dir);
    }

