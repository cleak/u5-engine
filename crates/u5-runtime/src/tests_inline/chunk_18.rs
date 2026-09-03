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
    fn cast_des_por_bottom_boundary_exits_at_published_coordinate_on_underworld() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 DUNGEON:0\n",
        )
        .unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 7, 1, 1);
        state.spell_charges[DES_POR_SPELL_INDEX] = 1;
        state.party[0].mana = 4;
        state.party[0].level = 4;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1DP", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.active_objects[0].z, WorldPlane::Underworld.save_floor());
        assert_eq!(state.spell_charges[DES_POR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(
            state.message,
            DUNGEON_EXIT_TO_UNDERWORLD_NARRATION
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_level_spell_refuses_base_and_wall_destination_classes() {
        for destination in [0x00, 0xb0, 0xc0, 0xd0, 0xe0] {
            let mut grid = open_dungeon_record();
            grid[dungeon_cell_index(2, 1, 1)] = destination;
            let mut state = dungeon_state(grid, 3, 1, 1);
            state.spell_charges[UUS_POR_SPELL_INDEX] = 1;
            state.party[0].mana = 4;
            state.party[0].level = 4;

            assert_eq!(
                handle_play_key_input(&mut state, 'C', "1PU", Path::new("")).unwrap(),
                PlayInputDisposition::Continue
            );
            assert_eq!(
                state.area,
                Area::Dungeon {
                    scene: DungeonScene::new(33).unwrap(),
                    level: 3,
                }
            );
            assert_eq!(state.spell_charges[UUS_POR_SPELL_INDEX], 0);
            assert_eq!(state.party[0].mana, 0);
            assert_eq!(state.turn, 1);
            assert_eq!(state.message, "Failed!");
        }
    }

    #[test]
    fn dungeon_level_spells_refuse_outright_inside_doom() {
        for (suffix, spell_index, delta_level) in [
            ("1PU", UUS_POR_SPELL_INDEX, 3),
            ("1DP", DES_POR_SPELL_INDEX, 3),
        ] {
            let doom = DungeonScene::new(DUNGEON_DOOM_SCENE_BYTE).unwrap();
            let mut grid = open_dungeon_record();
            grid[dungeon_cell_index(2, 1, 1)] = 0x10;
            grid[dungeon_cell_index(4, 1, 1)] = 0x10;
            let mut state = dungeon_state(grid, delta_level, 1, 1);
            state.area = Area::Dungeon {
                scene: doom,
                level: delta_level,
            };
            state.sync_player_object();
            state.spell_charges[spell_index] = 1;
            state.party[0].mana = 4;
            state.party[0].level = 4;

            assert_eq!(
                handle_play_key_input(&mut state, 'C', suffix, Path::new("")).unwrap(),
                PlayInputDisposition::Continue
            );
            assert_eq!(
                state.area,
                Area::Dungeon {
                    scene: doom,
                    level: delta_level,
                }
            );
            assert_eq!(state.spell_charges[spell_index], 0);
            assert_eq!(state.party[0].mana, 0);
            assert_eq!(state.turn, 1);
            assert_eq!(state.message, "Failed!");
        }
    }

    #[test]
    fn cast_magic_lock_rewrites_the_exact_live_unlocked_door_tile() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_LOCK_TABLE_FILE),
            "CASTLE:0 0 2 1 151 184 MAGIC\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 0xB8;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::North;
        state.visibility_dirty = false;
        state.spell_charges[MAGIC_LOCK_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AEP6", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[32 + 2], 0x97);
        assert_eq!(state.spell_charges[MAGIC_LOCK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Success!");

        state.player.facing = Direction::East;
        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::LockTried
        );
        assert_eq!(state.grid[32 + 2], 0x97);
        assert_eq!(state.turn, 2);
        assert_eq!(state.message, "Key broke!");
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
    fn lock_utility_spells_spend_resources_before_direction_followup() {
        let mut town_magic_lock = test_state(open_grid(), 5, 5);
        town_magic_lock.spell_charges[MAGIC_LOCK_SPELL_INDEX] = 1;
        town_magic_lock.party[0].mana = MAGIC_LOCK_COST;
        town_magic_lock.party[0].level = MAGIC_LOCK_COST;

        assert_eq!(
            town_magic_lock
                .cast_spell_from_suffix("1AEP", Path::new(""))
                .unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(town_magic_lock.spell_charges[MAGIC_LOCK_SPELL_INDEX], 0);
        assert_eq!(town_magic_lock.party[0].mana, 0);
        assert_eq!(town_magic_lock.turn, 0);
        assert_eq!(
            town_magic_lock.message,
            "Direction? Use C1AEP8/C1AEP6/C1AEP2/C1AEP4."
        );

        let mut town_unlock_magic = test_state(open_grid(), 5, 5);
        town_unlock_magic.spell_charges[UNLOCK_MAGIC_SPELL_INDEX] = 1;
        town_unlock_magic.party[0].mana = UNLOCK_MAGIC_COST;
        town_unlock_magic.party[0].level = UNLOCK_MAGIC_COST;

        assert_eq!(
            town_unlock_magic
                .cast_spell_from_suffix("1EIP", Path::new(""))
                .unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(
            town_unlock_magic.spell_charges[UNLOCK_MAGIC_SPELL_INDEX],
            0
        );
        assert_eq!(town_unlock_magic.party[0].mana, 0);
        assert_eq!(town_unlock_magic.turn, 0);
        assert_eq!(
            town_unlock_magic.message,
            "Direction? Use C1EIP8/C1EIP6/C1EIP2/C1EIP4."
        );

    }

    #[test]
    fn combat_lock_utility_spells_rewrite_adjacent_arena_terrain() {
        let mut magic_lock = britannia_state(open_world_grid(), 5, 5);
        magic_lock.combat_active = true;
        magic_lock.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        magic_lock.combat_terrain[5][6] = 0xB8;
        magic_lock.spell_charges[MAGIC_LOCK_SPELL_INDEX] = 1;
        magic_lock.party[0].mana = MAGIC_LOCK_COST;
        magic_lock.party[0].level = MAGIC_LOCK_COST;
        magic_lock.visibility_dirty = false;

        assert_eq!(
            magic_lock
                .cast_spell_from_suffix("1AEP6", Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(magic_lock.combat_terrain[5][6], 0x97);
        assert_eq!(magic_lock.spell_charges[MAGIC_LOCK_SPELL_INDEX], 0);
        assert_eq!(magic_lock.party[0].mana, 0);
        assert_eq!(magic_lock.turn, 1);
        assert_eq!(magic_lock.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(magic_lock.message, "Success!");

        let mut unlock_magic = britannia_state(open_world_grid(), 5, 5);
        unlock_magic.combat_active = true;
        unlock_magic.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        unlock_magic.combat_terrain[5][6] = 0x97;
        unlock_magic.spell_charges[UNLOCK_MAGIC_SPELL_INDEX] = 1;
        unlock_magic.party[0].mana = UNLOCK_MAGIC_COST;
        unlock_magic.party[0].level = UNLOCK_MAGIC_COST;
        unlock_magic.visibility_dirty = false;

        assert_eq!(
            unlock_magic
                .cast_spell_from_suffix("1EIP6", Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(unlock_magic.combat_terrain[5][6], 0xB8);
        assert_eq!(unlock_magic.spell_charges[UNLOCK_MAGIC_SPELL_INDEX], 0);
        assert_eq!(unlock_magic.party[0].mana, 0);
        assert_eq!(unlock_magic.turn, 1);
        assert_eq!(unlock_magic.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(unlock_magic.message, "Success!");
    }

    #[test]
    fn cast_magic_lock_non_magic_sidecar_consumes_cast_and_fails() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 97 96\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::North;
        state.spell_charges[MAGIC_LOCK_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AEP6", &dir).unwrap(),
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
    fn combat_vanish_without_live_actor_refuses_before_resources() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.combat_active = true;
        state.spell_charges[VANISH_SPELL_INDEX] = 1;
        state.party[0].mana = VANISH_COST;
        state.party[0].level = VANISH_COST;

        assert_eq!(
            state
                .cast_spell_from_suffix("1AY6", Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.spell_charges[VANISH_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, VANISH_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
        assert_eq!(state.message, "Who casts?");
    }

    #[test]
    fn cast_unlock_magic_rewrites_the_exact_live_magic_lock_tile() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_LOCK_TABLE_FILE),
            "CASTLE:0 0 2 1 96 97 MAGIC\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 0x97;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::North;
        state.visibility_dirty = false;
        state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1EIP6", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[32 + 2], 0xB8);
        assert_eq!(state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Success!");
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
        grid[32 + 2] = 0xB9;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::North;
        state.spell_charges[UNLOCK_MAGIC_SPELL_INDEX] = 1;
        state.party[0].mana = 5;
        state.party[0].level = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1EIP6", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[32 + 2], 0xB9);
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

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR4", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::North);
        assert_eq!(state.wind_save_byte, 1);
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(state.sail_cadence, 0);
        assert_eq!(state.message, "Wind change! Calm Winds -> North Winds.");
    }

    #[test]
    fn cast_rel_hur_prompt_direction_maps_to_spec_winds() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 2;
        state.wind = WindState::West;
        state.wind_save_byte = 0x7a;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR8", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::West);
        assert_eq!(state.wind_save_byte, 4);
        assert_eq!(state.message, "Wind change! West Winds -> West Winds.");
    }

    #[test]
    fn cast_rel_hur_missing_direction_prompts_without_spending_spell() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 2;
        state.wind = WindState::East;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::East);
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, 3);
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.message,
            "Direction? Use C1HR8/C1HR6/C1HR2/C1HR4, or C1HR<space>."
        );
    }

    #[test]
    fn cast_rel_hur_pass_consumes_cast_without_changing_wind() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 2;
        state.wind = WindState::East;
        state.wind_save_byte = 3;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR ", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::East);
        assert_eq!(state.wind_save_byte, 3);
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(state.message, "Wind change! Pass.");
    }

    #[test]
    fn cast_rel_hur_scene_gate_precedes_charge_consumption() {
        let mut state = test_state(open_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 2;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR8", Path::new("")).unwrap(),
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
            handle_play_key_input(&mut state, 'C', "1HR8", Path::new("")).unwrap(),
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
            handle_play_key_input(&mut state, 'C', "1HR8", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.wind, WindState::Calm);
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "M.P. too low!");
    }

    #[test]
    fn cast_time_stop_sets_shared_t_counter_and_consumes_charge_mana_and_turn() {
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
        assert_eq!(state.active_effect_tag, Some(NEGATE_TIME_ACTIVE_EFFECT_TAG));
        assert_eq!(state.active_effect_counter, TIME_STOP_DURATION);
        assert_eq!(state.time_stop_counter, 0);
        assert_eq!(state.message, "Negate time!");
    }

    #[test]
    fn cast_time_stop_scene_absorption_uses_special_message_without_resources() {
        let mut stonegate = test_state(open_grid(), 5, 5);
        stonegate.area = Area::Town {
            scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
            floor: 0,
        };
        stonegate.spell_charges[TIME_STOP_SPELL_INDEX] = 1;
        stonegate.party[0].mana = TIME_STOP_COST;
        stonegate.party[0].level = TIME_STOP_COST;

        assert_eq!(
            handle_play_key_input(&mut stonegate, 'C', "1AT", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(stonegate.spell_charges[TIME_STOP_SPELL_INDEX], 1);
        assert_eq!(stonegate.party[0].mana, TIME_STOP_COST);
        assert_eq!(stonegate.turn, 0);
        assert_eq!(stonegate.active_effect_tag, None);
        assert_eq!(stonegate.active_effect_counter, 0);
        assert_eq!(stonegate.message, "Magic absorbed!");

        let mut blackthorn = test_state(open_grid(), 5, 5);
        blackthorn.area = Area::Town {
            scene: Scene::new(LORD_BLACKTHORN_CASTLE_SCENE_BYTE).unwrap(),
            floor: 0,
        };
        blackthorn.spell_charges[TIME_STOP_SPELL_INDEX] = 1;
        blackthorn.party[0].mana = TIME_STOP_COST;
        blackthorn.party[0].level = TIME_STOP_COST;

        assert_eq!(blackthorn.cast_time_stop(0), MoveOutcome::Blocked);
        assert_eq!(blackthorn.spell_charges[TIME_STOP_SPELL_INDEX], 1);
        assert_eq!(blackthorn.party[0].mana, TIME_STOP_COST);
        assert_eq!(blackthorn.turn, 0);
        assert_eq!(blackthorn.active_effect_tag, None);
        assert_eq!(blackthorn.active_effect_counter, 0);
        assert_eq!(blackthorn.message, "Magic absorbed!");
    }

    #[test]
    fn negate_time_t_tag_freezes_minutes_npcs_and_active_objects_while_counter_decays() {
        let mut state = test_state(npc_open_grid(), 1, 1);
        state.clock = GameClock::new(17, 59).unwrap();
        state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 2;
        state.torch_counter = 5;
        state.light_spell_counter = 7;
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
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!(state.clock, GameClock::new(17, 59).unwrap());
        assert_eq!(state.active_effect_tag, Some(NEGATE_TIME_ACTIVE_EFFECT_TAG));
        assert_eq!(state.active_effect_counter, 1);
        assert_eq!(state.torch_counter, 5);
        assert_eq!(state.light_spell_counter, 7);
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        assert_eq!(state.active_objects[2].phase, 0x22);
        assert_eq!(state.active_objects[2].tile, 168);

        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!(state.clock, GameClock::new(17, 59).unwrap());
        assert_eq!(state.active_effect_tag, None);
        assert_eq!(state.active_effect_counter, 0);
        assert_eq!(state.torch_counter, 5);
        assert_eq!(state.light_spell_counter, 7);
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        assert_eq!(state.active_objects[2].phase, 0x22);
        assert_eq!(state.active_objects[2].tile, 168);

        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(state.torch_counter, 4);
        assert_eq!(state.light_spell_counter, 6);
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
    fn cast_gate_travel_shipboard_refusal_spends_the_charge_and_the_mana() {
        // magic.md §5 steps 4-7: the dispatcher decrements the premixed charge
        // "immediately, before any further checks" and debits mana before it
        // "computes the spell's index (0..47) into a forty-eight-entry
        // dispatch table and calls the matching handler". magic.md §8 puts the
        // shipboard test inside the Gate Travel handler, so a shipboard
        // attempt is a committed cast that refunds nothing.
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

        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 9 - GATE_TRAVEL_COST);
        // "A spell cast costs one turn regardless of the spell's power."
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Cannot Gate Travel shipboard.");
        // The party has not moved: the refusal is inside the handler, above
        // the moonstone-slot teleport.
        assert_eq!((state.player.x, state.player.y), (5, 5));
    }
