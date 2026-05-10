    #[test]
    fn cast_dungeon_field_rejections_keep_public_resource_ordering() {
        let mut missing_direction = dungeon_state(open_dungeon_record(), 0, 1, 1);
        missing_direction.spell_charges[FIRE_FIELD_SPELL_INDEX] = 1;
        missing_direction.party[0].mana = FIELD_SPELL_COST;
        missing_direction.party[0].level = FIELD_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut missing_direction, 'C', "1FGI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_direction.spell_charges[FIRE_FIELD_SPELL_INDEX], 1);
        assert_eq!(missing_direction.party[0].mana, FIELD_SPELL_COST);
        assert_eq!(missing_direction.turn, 0);
        assert_eq!(
            missing_direction.message,
            "Direction? Use C1FGI6/C1GIN6/C1GIZ6/C1GIS6."
        );

        let mut wrong_scene = test_state(open_grid(), 1, 1);
        wrong_scene.spell_charges[FIRE_FIELD_SPELL_INDEX] = 1;
        wrong_scene.party[0].mana = FIELD_SPELL_COST;
        wrong_scene.party[0].level = FIELD_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut wrong_scene, 'C', "1FGI6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(wrong_scene.spell_charges[FIRE_FIELD_SPELL_INDEX], 1);
        assert_eq!(wrong_scene.party[0].mana, FIELD_SPELL_COST);
        assert_eq!(wrong_scene.turn, 0);
        assert_eq!(wrong_scene.message, "Not here!");

        let mut blocked_grid = open_dungeon_record();
        blocked_grid[dungeon_cell_index(0, 2, 1)] = 0xb0;
        let mut blocked_target = dungeon_state(blocked_grid, 0, 1, 1);
        blocked_target.spell_charges[FIRE_FIELD_SPELL_INDEX] = 1;
        blocked_target.party[0].mana = FIELD_SPELL_COST;
        blocked_target.party[0].level = FIELD_SPELL_COST;
        blocked_target.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut blocked_target, 'C', "1FGI6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(blocked_target.grid[dungeon_cell_index(0, 2, 1)], 0xb0);
        assert_eq!(blocked_target.spell_charges[FIRE_FIELD_SPELL_INDEX], 0);
        assert_eq!(blocked_target.party[0].mana, 0);
        assert_eq!(blocked_target.turn, 1);
        assert_eq!(blocked_target.message, "Failed!");
    }

    #[test]
    fn cast_dispel_field_clears_public_dungeon_field_and_preserves_visit_marker() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x8a;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 1;
        state.party[0].mana = DISPEL_FIELD_COST;
        state.party[0].level = DISPEL_FIELD_COST;
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AG6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x08);
        assert_eq!(state.spell_charges[DISPEL_FIELD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(
            state.message,
            "Dispelled wall of fire at (2, 1) on DUNGEON:0 level 0."
        );
    }

    #[test]
    fn cast_dispel_field_rejections_keep_public_resource_ordering() {
        let mut missing_direction = dungeon_state(open_dungeon_record(), 0, 1, 1);
        missing_direction.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 1;
        missing_direction.party[0].mana = DISPEL_FIELD_COST;
        missing_direction.party[0].level = DISPEL_FIELD_COST;

        assert_eq!(
            handle_play_key_input(&mut missing_direction, 'C', "1AG", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_direction.spell_charges[DISPEL_FIELD_SPELL_INDEX], 1);
        assert_eq!(missing_direction.party[0].mana, DISPEL_FIELD_COST);
        assert_eq!(missing_direction.turn, 0);
        assert_eq!(missing_direction.message, "Direction? Use C1AG6.");

        let mut wrong_scene = test_state(open_grid(), 1, 1);
        wrong_scene.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 1;
        wrong_scene.party[0].mana = DISPEL_FIELD_COST;
        wrong_scene.party[0].level = DISPEL_FIELD_COST;

        assert_eq!(
            handle_play_key_input(&mut wrong_scene, 'C', "1AG6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(wrong_scene.spell_charges[DISPEL_FIELD_SPELL_INDEX], 1);
        assert_eq!(wrong_scene.party[0].mana, DISPEL_FIELD_COST);
        assert_eq!(wrong_scene.turn, 0);
        assert_eq!(wrong_scene.message, "Not here!");
    }

    #[test]
    fn cast_dispel_field_non_field_consumes_cast_and_fails() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x00;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 1;
        state.party[0].mana = DISPEL_FIELD_COST;
        state.party[0].level = DISPEL_FIELD_COST;
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AG6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x00);
        assert_eq!(state.spell_charges[DISPEL_FIELD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Failed!");
    }

    #[test]
    fn cast_dungeon_level_spell_scene_gate_precedes_charge_consumption() {
        let mut state = test_state(open_grid(), 5, 5);
        state.spell_charges[UUS_POR_SPELL_INDEX] = 1;
        state.party[0].mana = 4;
        state.party[0].level = 4;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PU", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[UUS_POR_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, 4);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_des_por_bottom_boundary_consumes_cast_and_fails_without_transition() {
        let scene = DungeonScene::new(33).unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 7, 1, 1);
        state.spell_charges[DES_POR_SPELL_INDEX] = 1;
        state.party[0].mana = 4;
        state.party[0].level = 4;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1DP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 7 });
        assert_eq!(state.active_objects[0].z, 7);
        assert_eq!(state.spell_charges[DES_POR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Failed!");
    }

    #[test]
    fn cast_magic_lock_rewrites_facing_unlocked_magic_lock_sidecar() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_LOCK_TABLE_FILE),
            "CASTLE:0 0 2 1 96 97 MAGIC\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 97;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.spell_charges[MAGIC_LOCK_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AEP", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[32 + 2], 96);
        assert_eq!(state.spell_charges[MAGIC_LOCK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Magic lock!");

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.grid[32 + 2], 96);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Magic lock!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_magic_lock_scene_gate_precedes_charge_consumption() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[MAGIC_LOCK_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AEP", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[MAGIC_LOCK_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, 5);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_magic_lock_non_magic_sidecar_consumes_cast_and_fails() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 97 96\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.spell_charges[MAGIC_LOCK_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AEP", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[32 + 2], 96);
        assert_eq!(state.spell_charges[MAGIC_LOCK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Failed!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_unlock_magic_rewrites_facing_magic_lock_sidecar() {
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
        state.visibility_dirty = false;
        state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1EIP", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[32 + 2], 97);
        assert_eq!(state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Unlocked!");

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::DoorOpened
        );
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.message, "Opened!");
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 97,
                x: 2,
                y: 1,
                turns_remaining: 4,
            })
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_unlock_magic_scene_gate_precedes_charge_consumption() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1EIP", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, 5);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_unlock_magic_non_magic_lock_consumes_cast_and_fails() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 97 96\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 97;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1EIP", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[32 + 2], 97);
        assert_eq!(state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Failed!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_rel_hur_changes_wind_and_consumes_charge_mana_and_turn() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 2;
        state.wind = WindState::Calm;
        state.wind_save_byte = 0x7a;
        state.sail_cadence = 1;
        state.sail_stall_pending = true;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::North);
        assert_eq!(state.wind_save_byte, 0x7a);
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.message, "Wind change! Calm Winds -> North Winds.");
    }

    #[test]
    fn cast_rel_hur_resets_raw_wind_save_byte_when_cycle_returns_to_calm() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 2;
        state.wind = WindState::West;
        state.wind_save_byte = 0x7a;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::Calm);
        assert_eq!(state.wind_save_byte, 0);
        assert_eq!(state.message, "Wind change! West Winds -> Calm Winds.");
    }

    #[test]
    fn cast_rel_hur_scene_gate_precedes_charge_consumption() {
        let mut state = test_state(open_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 2;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, 3);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_rel_hur_requires_mixed_charge_without_spending_turn() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.party[0].mana = 3;
        state.party[0].level = 2;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::Calm);
        assert_eq!(state.party[0].mana, 3);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "None mixed!");
    }

    #[test]
    fn cast_rel_hur_low_mana_loses_charge_and_consumes_turn() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 1;
        state.party[0].level = 2;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::Calm);
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "M.P. too low!");
    }

    #[test]
    fn cast_time_stop_sets_counter_and_consumes_charge_mana_and_turn() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[TIME_STOP_SPELL_INDEX] = 1;
        state.party[0].mana = 8;
        state.party[0].level = 8;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AT", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[TIME_STOP_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(state.time_stop_counter, TIME_STOP_DURATION);
        assert_eq!(state.message, "Time stopped for 10 turns.");
    }

    #[test]
    fn time_stop_freezes_scheduled_npcs_and_active_objects_while_counter_decays() {
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::new(17, 59).unwrap();
        state.time_stop_counter = 2;
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
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 6,
            y: 1,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(state.time_stop_counter, 1);
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        assert_eq!(state.active_objects[2].phase, 0x22);
        assert_eq!(state.active_objects[2].tile, 168);

        state.advance_turn();

        assert_eq!(state.clock, GameClock::new(18, 1).unwrap());
        assert_eq!(state.time_stop_counter, 0);
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        assert_eq!(state.active_objects[2].phase, 0x22);
        assert_eq!(state.active_objects[2].tile, 168);

        state.advance_turn();

        assert_eq!(state.clock, GameClock::new(18, 2).unwrap());
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (3, 1)
        );
        assert_eq!(state.active_objects[2].phase, 0x21);
        assert_eq!(state.active_objects[2].tile, 169);
    }

    #[test]
    fn cast_gate_travel_teleports_to_saved_world_slot_and_consumes_resources() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 5, 5);
        assert_eq!(GATE_TRAVEL_SPELL_INDEX, 46);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.spell_charges[TIME_STOP_SPELL_INDEX] = 9;
        state.party[0].mana = 9;
        state.party[0].level = 8;
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: 0,
        };

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PRV2", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (6, 7));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 0);
        assert_eq!(state.spell_charges[TIME_STOP_SPELL_INDEX], 9);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(state.message, "Gate Travel phase 2 -> BRITANNIA at (6, 7).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn gate_travel_destination_does_not_retrigger_underfoot_plane_transition_same_cast() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 6 7 UNDERWORLD 30 40\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = 9;
        state.party[0].level = 8;
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: 0,
        };

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PRV2", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (6, 7));
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Gate Travel phase 2 -> BRITANNIA at (6, 7).");
        assert!(!state.message.contains("F-A-L-L-S"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_gate_travel_invalid_slot_consumes_cast_without_teleporting() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = 9;
        state.party[0].level = 8;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PRV4", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Gate Travel phase 4 is not set.");
    }

    #[test]
    fn cast_gate_travel_from_town_source_uses_saved_world_slot_and_consumes_resources() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = 9;
        state.party[0].level = 8;
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: 0,
        };

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PRV2", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (6, 7));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Gate Travel phase 2 -> BRITANNIA at (6, 7).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_gate_travel_from_dungeon_source_uses_saved_world_slot_and_consumes_resources() {
        let dir = debug_game_dir();
        let mut state = dungeon_state(open_dungeon_record(), 3, 1, 1);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = 9;
        state.party[0].level = 8;
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: 0,
        };

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PRV2", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (6, 7));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Gate Travel phase 2 -> BRITANNIA at (6, 7).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_gate_travel_shipboard_refuses_before_charge_consumption() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = 9;
        state.party[0].level = 8;
        state.player.transport = TransportState::Ship {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
            skiffs: 1,
        };
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: 0,
        };

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PRV2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, 9);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Cannot Gate Travel shipboard.");
    }

    #[test]
    fn play_input_translates_common_ansi_navigation_sequences() {
        assert_eq!(
            play_input_key_and_suffix("\x1b[A\n"),
            Some(('8', String::new()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[B\r\n"),
            Some(('2', String::new()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[D\n"),
            Some(('4', String::new()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[C\n"),
            Some(('6', String::new()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[H\n"),
            Some(('7', String::new()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[5~\n"),
            Some(('9', String::new()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[F\n"),
            Some(('1', String::new()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[6~\n"),
            Some(('3', String::new()))
        );
    }

    #[test]
    fn translated_ansi_navigation_routes_through_play_movement() {
        let (key, suffix) = play_input_key_and_suffix("\x1b[A\n").unwrap();
        let mut town = test_state(open_grid(), 5, 5);

        assert_eq!(
            handle_play_key_input(&mut town, key, &suffix, Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((town.player.x, town.player.y), (5, 4));
        assert_eq!(town.turn, 1);

        let (key, suffix) = play_input_key_and_suffix("\x1b[A\n").unwrap();
        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert_eq!(
            handle_play_key_input(&mut dungeon, key, &suffix, Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((dungeon.player.x, dungeon.player.y), (2, 1));
        assert_eq!(dungeon.player.facing, Direction::East);
        assert_eq!(dungeon.turn, 1);
    }

    #[test]
    fn typeahead_buffer_toggle_is_no_turn_and_visible_in_status() {
        let mut state = test_state(open_grid(), 1, 1);

        assert_eq!(
            handle_play_script_command(&mut state, "buffer", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.typeahead_buffer_enabled);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Buffer On.");
        assert!(play_script_state_line(&state).contains("typeahead on"));
        assert!(state.z_stats_message().contains("typeahead on"));

        assert_eq!(
            handle_play_script_command(&mut state, "typeahead", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(!state.typeahead_buffer_enabled);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Buffer Off.");
    }

    #[test]
    fn typeahead_input_parser_only_splits_simple_keys() {
        assert_eq!(
            play_input_typeahead_chars("dd.\n"),
            Some(vec!['d', 'd', '.'])
        );
        assert_eq!(
            play_input_typeahead_chars("d d\n"),
            Some(vec!['d', ' ', 'd'])
        );
        assert_eq!(play_input_typeahead_chars("d"), None);
        assert_eq!(play_input_typeahead_chars("TJOB"), None);
        assert_eq!(play_input_typeahead_chars("C1IL"), None);
        assert_eq!(
            play_input_key_and_suffix("buffer\n"),
            Some((PLAY_TYPEAHEAD_TOGGLE_KEY, String::new()))
        );
    }

    #[test]
    fn ansi_function_keys_are_ignored_before_command_dispatch() {
        assert_eq!(ansi_function_key("\x1bOP"), Some(1));
        assert_eq!(ansi_function_key("\x1b[21~"), Some(10));
        assert_eq!(
            play_input_key_and_suffix("\x1bOP\n"),
            Some((PLAY_IGNORED_INPUT_KEY, "function".to_string()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[21~\n"),
            Some((PLAY_IGNORED_INPUT_KEY, "function".to_string()))
        );
        assert_eq!(
            play_input_key_and_suffix("\x1b[A\n"),
            Some(('8', String::new()))
        );
    }

    #[test]
    fn function_key_input_is_no_turn_and_no_idle_tick() {
        let (key, suffix) = play_input_key_and_suffix("\x1bOP\n").unwrap();
        let mut town = test_state(open_grid(), 5, 5);

        assert_eq!(
            handle_play_key_input(&mut town, key, &suffix, Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((town.player.x, town.player.y), (5, 5));
        assert_eq!(town.turn, 0);
        assert_eq!(town.animation.frame, 0);
        assert_eq!(town.message, "Function key ignored.");

        let (key, suffix) = play_input_key_and_suffix("\x1b[21~\n").unwrap();
        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert_eq!(
            handle_play_key_input(&mut dungeon, key, &suffix, Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((dungeon.player.x, dungeon.player.y), (1, 1));
        assert_eq!(dungeon.turn, 0);
        assert_eq!(dungeon.animation.frame, 0);
        assert_eq!(dungeon.message, "Function key ignored.");
    }

    #[test]
    fn unclassified_escape_sequences_are_ignored_without_swallowing_escape_key() {
        assert_eq!(
            play_input_key_and_suffix("\x1b[99~\n"),
            Some((PLAY_IGNORED_INPUT_KEY, "escape".to_string()))
        );
        assert_eq!(play_input_typeahead_chars("\x1b[99~\n"), None);
        assert_eq!(
            play_input_key_and_suffix("\x1b\n"),
            Some(('\x1b', String::new()))
        );
    }

