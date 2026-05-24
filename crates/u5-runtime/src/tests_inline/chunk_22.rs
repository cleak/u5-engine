    #[test]
    fn ambient_world_actor_directs_step_toward_player_on_phase_zero() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (4, 5));
        assert_eq!(object.phase, 0x62);
        assert_eq!(object.tile, 192);
    }

    #[test]
    fn active_object_walker_uses_high_to_low_outdoor_order() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 194,
            tile: 194,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (4, 5)
        );
        assert_eq!(
            (state.active_objects[2].x, state.active_objects[2].y),
            (6, 4)
        );
        assert_eq!(state.active_objects[1].phase, 0x62);
        assert_eq!(state.active_objects[2].phase, 0x02);
    }

    #[test]
    fn ambient_world_actor_countdown_animates_without_wandering() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));
        assert_eq!(object.phase, 0x21);
        assert_eq!(object.tile, 193);
    }

    #[test]
    fn half_time_world_epilogue_alternates_active_object_animation() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.timing_status = TimingStatusTag::HalfTime;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();
        assert_eq!(state.active_objects[1].phase, 0x22);
        assert_eq!(state.active_objects[1].tile, 192);

        state.advance_turn();
        assert_eq!(state.active_objects[1].phase, 0x21);
        assert_eq!(state.active_objects[1].tile, 193);
    }

    #[test]
    fn no_minute_light_world_epilogue_skips_active_object_animation() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.timing_status = TimingStatusTag::NoMinuteLight;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();
        assert_eq!(state.active_objects[1].phase, 0x22);
        assert_eq!(state.active_objects[1].tile, 192);
    }

    #[test]
    fn ambient_world_actor_wander_respects_terrain_and_player_collision() {
        let mut blocked_grid = open_world_grid();
        blocked_grid[world_cell_index(4, 5)] = 1;
        let mut terrain_blocked = world_state(blocked_grid, 0, 0);
        terrain_blocked.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        terrain_blocked.advance_turn();

        let object = terrain_blocked
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));
        assert_eq!(object.phase, 0x60);

        let mut player_blocked = world_state(open_world_grid(), 4, 5);
        player_blocked.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        player_blocked.advance_turn();

        let object = player_blocked
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));
        assert_eq!(object.phase, 0x60);
    }

    fn town_free_roaming_grid() -> Vec<u8> {
        vec![0x05; TOWN_GRID_SIDE * TOWN_GRID_SIDE]
    }

    #[test]
    fn town_free_roaming_actor_uses_prng_and_rewrites_record_on_success() {
        let mut state = test_state(town_free_roaming_grid(), 1, 1);
        state.prng_state = 0x0070;
        state.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: 0x66,
            aux1: 7,
            aux3: 9,
        });

        state.advance_active_objects();

        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (5, 6));
        assert_eq!(state.active_objects[1].type_byte, 0x10);
        assert_eq!(state.active_objects[1].tile, 0x10);
        assert_eq!(state.active_objects[1].phase, 0x66);
        assert_eq!(state.active_objects[1].z, 0);
        assert_eq!(state.active_objects[1].aux1, 7);
        assert_eq!(state.active_objects[1].aux3, 9);
        let mut expected_prng = 0x0070;
        for _ in 0..3 {
            expected_prng = u5_prng_advance_state(expected_prng);
        }
        assert_eq!(state.prng_state, expected_prng);
        assert!(state.visibility_dirty);
    }

    #[test]
    fn town_free_roaming_actor_skips_ineligible_and_off_floor_without_prng() {
        let mut state = test_state(town_free_roaming_grid(), 1, 1);
        state.prng_state = 0x1234;
        state.active_objects.push(ActiveObject {
            type_byte: 0x12,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 6,
            y: 5,
            z: 1,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        state.advance_active_objects();

        assert_eq!(state.prng_state, 0x1234);
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (5, 5));
        assert_eq!((state.active_objects[2].x, state.active_objects[2].y), (6, 5));
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn town_free_roaming_actor_chance_skip_consumes_one_draw() {
        let mut state = test_state(town_free_roaming_grid(), 1, 1);
        state.prng_state = 0x0008;
        state.active_objects.push(ActiveObject {
            type_byte: 0x11,
            tile: 0x11,
            x: 5,
            y: 5,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_active_objects();

        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (5, 5));
        assert_eq!(state.active_objects[1].phase, 0x22);
        assert_eq!(state.prng_state, u5_prng_advance_state(0x0008));
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn town_free_roaming_actor_pen_and_destination_failures_do_not_dirty() {
        let mut pen_blocked_grid = town_free_roaming_grid();
        pen_blocked_grid[5 * TOWN_GRID_SIDE + 6] = 0x43;
        let mut pen_blocked = test_state(pen_blocked_grid, 1, 1);
        pen_blocked.prng_state = 0x0070;
        pen_blocked.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        pen_blocked.advance_active_objects();
        assert_eq!((pen_blocked.active_objects[1].x, pen_blocked.active_objects[1].y), (5, 5));
        assert_eq!(pen_blocked.prng_state, u5_prng_advance_state(0x0070));
        assert!(!pen_blocked.visibility_dirty);

        let mut destination_blocked_grid = town_free_roaming_grid();
        destination_blocked_grid[6 * TOWN_GRID_SIDE + 5] = 0x04;
        let mut object_blocked = test_state(destination_blocked_grid, 1, 1);
        object_blocked.prng_state = 0x0070;
        object_blocked.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        object_blocked.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        object_blocked.advance_active_objects();
        assert_eq!((object_blocked.active_objects[1].x, object_blocked.active_objects[1].y), (5, 5));
        let mut expected_prng = 0x0070;
        for _ in 0..3 {
            expected_prng = u5_prng_advance_state(expected_prng);
        }
        assert_eq!(object_blocked.prng_state, expected_prng);
        assert!(!object_blocked.visibility_dirty);
    }

    #[test]
    fn outdoor_water_creature_uses_water_predicate_and_rewrites_facing() {
        let mut grid = open_world_grid();
        grid[world_cell_index(4, 5)] = 1;
        let mut state = world_state(grid, 0, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 0x2c,
            tile: 0x2c,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: SEA_CREATURE_SPAWN_AUX_SEED,
            aux3: 0,
        });

        state.advance_turn();

        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (4, 5)
        );
        assert_eq!(state.active_objects[1].type_byte, 0x2f);
        assert_eq!(state.active_objects[1].tile, 0x2f);
        assert_eq!(state.active_objects[1].phase, 0x62);
    }

    #[test]
    fn world_post_turn_adjacent_hostile_enters_terrain_combat() {
        let dir = debug_game_dir();
        let record = synthetic_combat_arena_record();
        fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 0x44,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let outcome = state.pass_turn_with_game_dir(Some(&dir)).unwrap();

        assert_eq!(outcome, MoveOutcome::Used);
        assert!(state.combat_active);
        assert_eq!(state.pending_combat_terrain_trigger_slot, Some(1));
        assert!(state.message.contains("World object tile 192 engaged"));
        assert!(state.message.contains("entered terrain combat"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terrain_combat_main_path_uses_class_default_spawn_count() {
        let dir = debug_game_dir();
        let record = synthetic_combat_arena_record();
        fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
        let replacement_tile = terrain_combat_raw_replacement_tile_for_arena(12).unwrap();
        let replacement_seed = (0..=u16::MAX)
            .find(|seed| {
                let mut prng = *seed;
                u5_prng_range_u16(
                    &mut prng,
                    0,
                    u16::from(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR - 1),
                ) == 0
            })
            .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.prng_state = replacement_seed;
        let object = ActiveObject {
            type_byte: 0x70,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        let message = state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, object)
            .unwrap();

        assert!(message.contains("BRIT.CBT arena 12"));
        assert!(message.contains("requested Orc"));
        assert_eq!(state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].tile, 0xc0);
        assert_eq!(replacement_tile, 12);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terrain_combat_main_path_uses_selected_brit_cbt_record_metadata() {
        let dir = debug_game_dir();
        let mut bank = Vec::with_capacity(BRIT_CBT_FILE_LEN);
        for arena in 0..BRIT_CBT_RECORDS {
            let mut record = synthetic_combat_arena_record();
            let terrain_tag = 0x40 + arena as u8;
            record[0] = terrain_tag;
            record[6 * COMBAT_ARENA_ROW_STRIDE + 11] = arena as u8;
            record[7 * COMBAT_ARENA_ROW_STRIDE + 11] = 15 - arena as u8;
            bank.extend_from_slice(&record);
        }
        fs::write(dir.join(BRIT_CBT_FILE), bank).unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        let object = ActiveObject {
            type_byte: 0x70,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        let message = state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, object)
            .unwrap();

        assert!(message.contains("BRIT.CBT arena 12"));
        assert_eq!(state.combat_terrain[0][0], 0x4c);
        assert_eq!(
            (
                state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].x,
                state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].y,
            ),
            (12, 3)
        );
        assert_eq!(
            (
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].x,
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].y,
            ),
            (12, 3)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn overworld_prunes_far_non_vehicle_objects_but_keeps_vehicles() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 40,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 80,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        assert_eq!(state.active_objects.len(), 3);
        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.active_objects[2].type_byte, 168);
        assert_eq!(state.active_objects[2].x, 80);
    }

    #[test]
    fn overworld_pruning_uses_public_scroll_base_window() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 16,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 193,
            tile: 193,
            x: 224,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            world_scroll_base(state.player.x, state.player.y),
            (240, 240)
        );

        state.advance_turn();

        assert_eq!(state.active_objects[1].type_byte, 192);
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (16, 0)
        );
        assert!(state.active_objects[2].is_empty());
    }

    #[test]
    fn overworld_prunes_after_post_tick_wander_position() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 17,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (16, 0));
        assert_eq!(object.phase, 0x62);
    }

