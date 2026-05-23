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

