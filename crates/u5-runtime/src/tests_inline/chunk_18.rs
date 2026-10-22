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

        // `magic.md §5` (issue #277): the handler's prompt is reached with
        // the charge and mana already spent.
        assert_eq!(missing_direction.spell_charges[FIRE_FIELD_SPELL_INDEX], 0);
        assert_eq!(missing_direction.party[0].mana, 0);
        assert_eq!(missing_direction.turn, 0);
        assert_eq!(
            missing_direction.pending_cast_argument,
            Some(CastArgumentRequest::Direction)
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
            DISPEL_FIELD_DESTROYED_LINE
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

        // `magic.md §8` (R513): the dungeon arm raises no direction prompt;
        // with no field in the own or facing cell it is an ordinary failure.
        assert_eq!(missing_direction.spell_charges[DISPEL_FIELD_SPELL_INDEX], 0);
        assert_eq!(missing_direction.party[0].mana, 0);
        assert_eq!(missing_direction.turn, 1);
        assert_eq!(missing_direction.pending_cast_argument, None);
        assert_eq!(missing_direction.message, "Failed!");

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
        // `dungeon-mode.md §13.1`: "Any route that leaves the level stack |
        // the climb word, then the shared exit line of Section 13.2".
        // The word is printed once (playtest E09), and the exit line beneath.
        let rows: Vec<&str> = state
            .message_transcript
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        let word = rows
            .iter()
            .position(|row| *row == DUNGEON_KLIMB_DOWN.trim_end())
            .expect("the climb word prints");
        assert_eq!(
            rows[word..].join("\n").trim_end(),
            format!("{DUNGEON_KLIMB_DOWN}{DUNGEON_EXIT_TO_UNDERWORLD_NARRATION}").trim_end(),
            "{rows:?}"
        );
        assert!(!state.message_slot_needs_flush());
        let _ = fs::remove_dir_all(dir);
    }

    /// `dungeon-mode.md §13.1` (`RETRACTIONS.md` R472): the destination test
    /// accepts only the open-passage class, so the ladder, chest, pit, wall
    /// and door-presentation classes are all refused. The base `0x00` this
    /// test used to list among the refusals is the one class that passes.
    #[test]
    fn dungeon_level_spell_refuses_every_class_but_open_passage() {
        for destination in [0x10, 0x40, 0x60, 0xb0, 0xc0, 0xd0, 0xe0] {
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
            // "the climb word (already printed), then `Failed!\n` with the
            // error tone".
            // Printed once (playtest E09).
            let rows: Vec<&str> = state
                .message_transcript
                .iter()
                .map(|entry| entry.text.as_str())
                .collect();
            assert_eq!(rows[rows.len() - 2..], ["Up!", "Failed!"], "{rows:?}");
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
            town_magic_lock.pending_cast_argument,
            Some(CastArgumentRequest::Direction)
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
            town_unlock_magic.pending_cast_argument,
            Some(CastArgumentRequest::Direction)
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
        assert_eq!(magic_lock.turn, 0); // combat.md §15: no per-command turn in a fight (playtest G08)
        assert_eq!(magic_lock.clock, GameClock::new(12, 0).unwrap());
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
        assert_eq!(unlock_magic.turn, 0); // combat.md §15: no per-command turn in a fight (playtest G08)
        assert_eq!(unlock_magic.clock, GameClock::new(12, 0).unwrap());
        assert_eq!(unlock_magic.message, "Success!");
    }

    /// `stats-panel.md §2.2.1` row 2: combat's Ring of Regeneration effect
    /// defers the panel, the same as the world-turn and camp-step passes.
    #[test]
    fn combat_ring_regeneration_defers_the_stats_panel() {
        let mut base = britannia_state(open_world_grid(), 5, 5);
        base.combat_active = true;
        base.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        base.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
        base.party[0].hp = base.party[0].max_hp - 1;

        let seed = (0u32..256)
            .find(|&seed| {
                let mut probe = base.clone();
                probe.prng_state = seed as u16;
                probe
                    .apply_visible_combat_magic_ring_pass_to_slot(0)
                    .is_some_and(|outcome| outcome.regeneration_applied > 0)
            })
            .expect("some seed rolls the one-in-eight regeneration tick");
        let mut state = base;
        state.prng_state = seed as u16;

        let outcome = state.apply_visible_combat_magic_ring_pass_to_slot(0);

        assert_eq!(outcome.map(|o| o.regeneration_applied), Some(1));
        assert_eq!(state.party[0].hp, state.party[0].max_hp);
        assert_eq!(
            state.stats_panel_command_refresh,
            crate::stats_panel::PanelRefreshMode::Deferred
        );
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
        assert_eq!(
            state.pending_cast_argument,
            Some(CastArgumentRequest::Caster)
        );
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
        // `magic.md §5.1`: Rel Hur has no generic completion line, and
        // `Wind change!` is the scroll's banner; the wind banner repaints.
        assert_eq!(state.message, "");
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
        assert_eq!(state.message, "");
    }

    #[test]
    fn cast_rel_hur_spends_charge_and_mana_before_prompting_for_direction() {
        // `magic.md §5`: "Charge and mana are already spent when a handler
        // prompts", and "No later gate can fail after a prompt opens".
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
        // `magic.md §5` (issue #277): the handler's prompt is reached with
        // the charge and mana already spent.
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.pending_cast_argument,
            Some(CastArgumentRequest::Direction)
        );
    }

    #[test]
    fn cast_rel_hur_low_mana_refuses_before_the_direction_prompt_opens() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
        state.party[0].mana = 1;
        state.party[0].level = 2;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1HR", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.pending_cast_argument, None);
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 0);
        assert_eq!(state.message, "M.P. too low!\nFailed!\n");
    }

    #[test]
    fn cast_rel_hur_interactive_direction_spends_once_and_echoes_only_the_direction() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.spell_charges[REL_HUR_SPELL_INDEX] = 2;
        state.party[0].mana = 4;
        state.party[0].level = 2;
        state.wind = WindState::Calm;

        state.active_player = Some(0);
        assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
        assert!(state.step_active_cast('H', "R", Path::new("")).unwrap().is_none());
        assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_none());
        assert!(state.active_cast_followup.is_some());
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, 2);

        let result = state
            .step_active_cast_followup('6', "", Path::new(""))
            .unwrap()
            .expect("east finishes Rel Hur");
        assert_eq!(result.0, MoveOutcome::Cast);
        assert_eq!(state.wind, WindState::East);
        assert_eq!(state.spell_charges[REL_HUR_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, 2);
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("Wind change"), "{}", state.message);
        assert!(state
            .message_entries()
            .iter()
            .any(|entry| entry.text == format!("{SPELL_DIRECTION_PROMPT_PREFIX}East")));
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
        assert_eq!(state.message, "");
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
        assert_eq!(state.message, "None mixed!\n");
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
        assert_eq!(state.message, "M.P. too low!\nFailed!\n");
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
        // `magic.md §5.1`: An Tym takes no generic completion line and does
        // not acquire its scroll counterpart's banner (`RETRACTIONS.md` R402).
        assert!(state.message.is_empty(), "{}", state.message);
    }

    #[test]
    fn cast_time_stop_absorbed_by_any_shadow_lord_base_tile_record() {
        // `magic.md §8` (added 2026-10-16, u5-spec#298 Q15): the sentinel is
        // an active-object record whose base tile is exactly `0xFC`, anywhere
        // in the table. The variant still plays first; the match prints
        // `Magic absorbed!`, plays the absorption envelope, then the shared
        // `Failed!` and glissando, and the effect is not set.
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.active_objects[29] = ActiveObject {
            type_byte: SHADOWLORD_ACTOR_TILE,
            tile: 0xfe,
            x: 31,
            y: 31,
            ..ActiveObject::empty()
        };
        state.spell_charges[TIME_STOP_SPELL_INDEX] = 1;
        state.party[0].mana = 8;
        state.party[0].level = 8;
        let serial = state.sound_effect_serial;

        assert_eq!(state.cast_time_stop(0), MoveOutcome::Blocked);
        assert_eq!(state.message, "Magic absorbed!\nFailed!");
        assert_eq!(state.active_effect_tag, None);
        assert_eq!(state.active_effect_counter, 0);
        assert_eq!(state.spell_charges[TIME_STOP_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![
                SoundEffect::SharedVariant { variant: 8 },
                SoundEffect::MagicAbsorbed,
                SoundEffect::CastFailure,
            ]
        );

        // Other Shadow Lord frames as the base tile do not match, and the
        // displayed tile is not consulted.
        let mut frame = britannia_state(open_world_grid(), 5, 5);
        frame.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        frame.active_objects[3] = ActiveObject {
            type_byte: 0xfd,
            tile: SHADOWLORD_ACTOR_TILE,
            ..ActiveObject::empty()
        };
        frame.spell_charges[TIME_STOP_SPELL_INDEX] = 1;
        frame.party[0].mana = 8;
        frame.party[0].level = 8;
        assert_eq!(frame.cast_time_stop(0), MoveOutcome::Cast);
        assert_eq!(frame.active_effect_tag, Some(NEGATE_TIME_ACTIVE_EFFECT_TAG));
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
        // `audio.md §6.1` / `RETRACTIONS.md` R462: "The text is
        // `Absorbed!` with a trailing newline." The earlier
        // `Magic absorbed!` label is withdrawn.
        assert_eq!(stonegate.message, SCENE_ABSORBED_CAST_MESSAGE);

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
        assert_eq!(blackthorn.message, SCENE_ABSORBED_CAST_MESSAGE);
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
        // Issue #298 Q29: a countdown pass decrements the delay and leaves the
        // displayed frame alone ("the frame is unchanged").
        assert_eq!(state.active_objects[2].tile, 168);
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
        // `commands.md §8.1` keeps the scene key and the cell out of the
        // message window; they are a diagnostic now.
        assert!(
            state
                .diagnostics
                .join(" ")
                .contains("Gate Travel phase 2 -> BRITANNIA at (6, 7).")
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `overworld.md §9.2` (issue #299 Q26): "Transport therefore survives a
    /// transit unchanged ... a party on a horse or a magic carpet arrives
    /// mounted or airborne", a town destination included; `magic.md §8`'s
    /// Gate travel row: "A party on a horse or carpet teleports mounted".
    #[test]
    fn gate_travel_into_a_town_keeps_the_horse_and_the_carpet() {
        for transport in [
            TransportState::Horse {
                type_byte: HORSE_MOUNTED_FIRST,
                tile: FIRST_PLAYABLE_HORSE_TILE,
            },
            TransportState::Carpet {
                type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
                tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
            },
        ] {
            let dir = debug_game_dir();
            let mut state = britannia_state(open_world_grid(), 5, 5);
            state.player.transport = transport;
            state.sync_player_object();
            state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
            state.party[0].mana = 9;
            state.party[0].level = 8;
            state.moonstone_slots[1] = MoonstoneGateSlot {
                // Scene 17: the first CASTLE-family location, which the
                // synthetic game directory carries.
                scene: 17,
                x: 6,
                y: 7,
                z: 0,
            };

            handle_play_key_input(&mut state, 'C', "1PRV2", &dir).unwrap();

            assert!(matches!(state.area, Area::Town { .. }), "{:?}", state.area);
            assert_eq!((state.player.x, state.player.y), (6, 7));
            assert_eq!(state.player.transport, transport);
            assert_eq!(state.active_objects[0].type_byte, transport.save_marker());
            let _ = fs::remove_dir_all(dir);
        }
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
        // `commands.md §8.1` keeps the scene key and the cell out of the
        // message window; they are a diagnostic now.
        assert!(
            state
                .diagnostics
                .join(" ")
                .contains("Gate Travel phase 2 -> BRITANNIA at (6, 7).")
        );
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
        assert_eq!(state.message, "Failed!");
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
        // `commands.md §8.1` keeps the scene key and the cell out of the
        // message window; they are a diagnostic now.
        assert!(
            state
                .diagnostics
                .join(" ")
                .contains("Gate Travel phase 2 -> BRITANNIA at (6, 7).")
        );
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
        // `commands.md §8.1` keeps the scene key and the cell out of the
        // message window; they are a diagnostic now.
        assert!(
            state
                .diagnostics
                .join(" ")
                .contains("Gate Travel phase 2 -> BRITANNIA at (6, 7).")
        );
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
        assert_eq!(state.message, "Failed!");
        // The party has not moved: the refusal is inside the handler, above
        // the moonstone-slot teleport.
        assert_eq!((state.player.x, state.player.y), (5, 5));
    }
