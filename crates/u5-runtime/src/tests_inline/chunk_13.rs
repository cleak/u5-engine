    #[test]
    fn no_turn_dungeon_action_on_wind_tile_skips_underfoot_wind() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_WIND_TILE_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.torch_counter = 5;
        state.visibility_dirty = false;

        assert!(state.handle_dungeon_key('l', &dir).unwrap());

        assert_eq!(state.turn, 0);
        assert_eq!(state.torch_counter, 5);
        assert!(!state.visibility_dirty);
        assert!(!state.message.contains("breeze blows out the torch"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_dungeon_wind_tile_extinguishes_underfoot_torch_after_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_WIND_TILE_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.torch_counter = 5;
        state.light_spell_counter = 5;
        state.visibility_dirty = false;

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.torch_counter, 0);
        assert_eq!(state.light_spell_counter, 4);
        assert!(state.visibility_dirty);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("breeze blows out the torch"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_wind_tile_sidecar_extinguishes_torch_but_not_light_spell() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_WIND_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.torch_counter = 5;
        state.light_spell_counter = 5;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
        assert_eq!(state.torch_counter, 0);
        assert_eq!(state.light_spell_counter, 4);
        assert!(state.message.contains("breeze blows out the torch"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_wind_tile_cell_guard_mismatch_does_not_extinguish_torch() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_WIND_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x71\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.torch_counter = 5;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.torch_counter, 4);
        assert!(!state.message.contains("breeze blows out the torch"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn consumed_dungeon_turn_on_teleport_sidecar_changes_level_after_turn() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 1 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('a', &dir).unwrap());

        assert_eq!(state.area, Area::Dungeon { scene, level: 3 });
        assert_eq!((state.player.x, state.player.y), (4, 5));
        assert_eq!(state.active_objects[0].z, 3);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Turned to face"));
        assert!(state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn no_turn_dungeon_action_on_teleport_sidecar_skips_underfoot_teleport() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 1 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('l', &dir).unwrap());

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
        assert!(!state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_dungeon_teleport_sidecar_changes_level_after_turn() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 1 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 3 })
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 3 });
        assert_eq!((state.player.x, state.player.y), (4, 5));
        assert_eq!(state.active_objects[0].z, 3);
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_scripted_teleport_sidecar_changes_level_and_position() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 2 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 3 })
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 3 });
        assert_eq!((state.player.x, state.player.y), (4, 5));
        assert_eq!(
            (
                state.active_objects[0].x,
                state.active_objects[0].y,
                state.active_objects[0].z,
            ),
            (4, 5, 3)
        );
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_teleport_cell_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 2 1 3 4 5 0x71\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_tile_sidecar_returns_to_world_location_table() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x70\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "UNDERWORLD 10 20 DUNGEON:0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeon(scene))
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(
            state.active_objects[0].z,
            WorldPlane::Underworld.save_floor()
        );
        assert_eq!(state.grid[world_cell_index(10, 20)], 5);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("dungeon exit tile"));
        assert!(state.message.contains("world-location table point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_dungeon_exit_tile_sidecar_returns_after_turn() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "UNDERWORLD 10 20 DUNGEON:0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeon(scene))
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(
            state.active_objects[0].z,
            WorldPlane::Underworld.save_floor()
        );
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("Triggered dungeon exit tile"));
        assert!(state.message.contains("world-location table point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_tile_missing_return_metadata_stays_in_dungeon() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("dungeon exit tile"));
        assert!(
            state
                .message
                .contains("missing clean return-coordinate metadata")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_tile_sidecar_overrides_blocking_cell() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0xB0\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "UNDERWORLD 12 34 DUNGEON:0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xb0;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeon(scene))
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (12, 34));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("dungeon exit tile"));
        assert!(state.message.contains("world-location table point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_tile_cell_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x71\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(
            state.area,
            Area::Dungeon {
                scene: DungeonScene::new(33).unwrap(),
                level: 0,
            }
        );
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("dungeon exit tile"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_energy_field_marker_variants_keep_subtype_reaction() {
        assert_eq!(dungeon_field_effect(0x88), Some(DungeonFieldEffect::Sleep));
        assert_eq!(
            dungeon_field_effect(0x89),
            Some(DungeonFieldEffect::PoisonGas)
        );
        assert_eq!(dungeon_field_effect(0x8a), Some(DungeonFieldEffect::Fire));
        assert_eq!(
            dungeon_field_effect(0x8b),
            Some(DungeonFieldEffect::Electric)
        );
        assert_eq!(dungeon_field_effect(0x90), Some(DungeonFieldEffect::Energy));
        assert_eq!(dungeon_field_effect(0x70), None);
    }

    #[test]
    fn dungeon_room_trigger_marks_visit_local_helper_state_and_reports_arena() {
        let scene = DungeonScene::new(35).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xf7;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.area = Area::Dungeon { scene, level: 0 };

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa7);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("slot 7"));
        assert!(state.message.contains("selected DUNGEON.CBT arena 23"));
        assert!(!state.message.contains("out of scope"));
    }

    #[test]
    fn dungeon_room_trigger_loads_selected_dungeon_cbt_record_when_available() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(35).unwrap();
        let record = synthetic_combat_arena_record();
        let mut dungeon_cbt = Vec::new();
        for _ in 0..DUNGEON_CBT_RECORDS {
            dungeon_cbt.extend_from_slice(&record);
        }
        fs::write(dir.join(DUNGEON_CBT_FILE), dungeon_cbt).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xf7;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.area = Area::Dungeon { scene, level: 0 };

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa7);
        assert!(state.message.contains("loaded DUNGEON.CBT arena 23"));
        assert!(state.message.contains("terrain[0,0]=0x00"));
        assert!(state.message.contains("16 room source marker(s)"));
        assert!(state.message.contains("1 absorbable-field marker(s)"));
        assert!(state.message.contains("first source 0x30"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_current_room_trigger_fires_before_next_key() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xf3;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('w', Path::new("")).unwrap());

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xa3);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("slot 3"));
    }

    #[test]
    fn doom_final_room_trigger_enters_endgame_without_room_rewrite() {
        let scene = DungeonScene::new(40).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(
            DOOM_FINAL_ROOM_LEVEL,
            DOOM_FINAL_ROOM_X,
            DOOM_FINAL_ROOM_Y,
        )] = 0xf0 | DOOM_FINAL_ROOM_SLOT;
        let mut state = dungeon_state(grid, DOOM_FINAL_ROOM_LEVEL, 4, DOOM_FINAL_ROOM_Y);
        state.area = Area::Dungeon {
            scene,
            level: DOOM_FINAL_ROOM_LEVEL,
        };

        assert_eq!(state.step(Direction::East), MoveOutcome::EndgameEntered);

        assert_eq!((state.player.x, state.player.y), (5, 7));
        assert_eq!(
            state.grid[dungeon_cell_index(
                DOOM_FINAL_ROOM_LEVEL,
                DOOM_FINAL_ROOM_X,
                DOOM_FINAL_ROOM_Y,
            )],
            0xf0 | DOOM_FINAL_ROOM_SLOT
        );
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.endgame,
            Some(EndgameState::awaiting_first_confirmation())
        );
        assert!(state.message.contains("Lord British asks"));
    }

    #[test]
    fn endgame_confirmation_gates_victory_on_final_answer_and_box_flag() {
        let dir = debug_game_dir();
        let mut missing_box = dungeon_state(open_dungeon_record(), 0, 1, 1);
        missing_box.enter_endgame();

        assert_eq!(
            handle_play_key_input(&mut missing_box, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            missing_box.endgame,
            Some(EndgameState::awaiting_final_confirmation(true))
        );

        assert_eq!(
            handle_play_key_input(&mut missing_box, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            missing_box
                .endgame
                .as_ref()
                .and_then(|state| state.outcome),
            Some(EndgameOutcome::MissingBoxOrRefused)
        );
        assert_eq!(missing_box.turn, 0);

        let mut victory = dungeon_state(open_dungeon_record(), 0, 1, 1);
        victory.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 1;
        victory.party_names = vec![*b"MARIA\0\0\0\0"];
        victory.clock = GameClock::with_date(141, 5, 6, 12, 0).unwrap();
        victory.enter_endgame();
        victory.resolve_endgame_confirmation(false);
        victory.resolve_endgame_confirmation(true);

        let endgame = victory.endgame.as_ref().unwrap();
        assert_eq!(endgame.first_confirmation, Some(false));
        assert_eq!(endgame.final_confirmation, Some(true));
        assert_eq!(endgame.outcome, Some(EndgameOutcome::Victory));
        assert!(endgame.certificate.as_ref().unwrap().contains("MARIA"));
        assert!(victory.message.contains("sixth day of the fifth month"));
        assert!(victory.message.contains("one hundred forty-one"));
        assert!(victory.message.contains("2 years, 1 month, 1 day"));
        assert!(victory.message.contains("Report this completed quest to Origin"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn enter_endgame_restores_dead_party_for_tableau() {
        // endgame.md §10: dead party members are mutated into a present /
        // restored state for the ending tableau, with current health restored
        // from the stored maximum.
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 60,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 12,
                max_hp: 30,
                level: 4,
            },
            PartyMember {
                slot: 2,
                class_byte: b'M',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 5,
                max_hp: 25,
                level: 3,
            },
        ];

        state.enter_endgame();

        for member in &state.party {
            assert_eq!(member.status, b'G');
            assert_eq!(member.hp, member.max_hp);
        }
        assert!(state.endgame.is_some());
    }

    #[test]
    fn natural_moongate_counter_step_matches_spec_hour_band() {
        // overworld.md §9: 20..=23 and 0..=4 increase; 5..=19 decrease.
        for h in 20..=23u8 {
            assert_eq!(
                natural_moongate_counter_step(h),
                NaturalMoongateCounterStep::Increase
            );
        }
        for h in 0..=4u8 {
            assert_eq!(
                natural_moongate_counter_step(h),
                NaturalMoongateCounterStep::Increase
            );
        }
        for h in 5..=19u8 {
            assert_eq!(
                natural_moongate_counter_step(h),
                NaturalMoongateCounterStep::Decrease
            );
        }
        // Counter saturation
        assert_eq!(natural_moongate_advance_counter(0, 0), 1);
        assert_eq!(
            natural_moongate_advance_counter(NATURAL_MOONGATE_COUNTER_MAX, 0),
            NATURAL_MOONGATE_COUNTER_MAX
        );
        assert_eq!(natural_moongate_advance_counter(5, 12), 4);
        assert_eq!(natural_moongate_advance_counter(0, 12), 0);
        // Slot eligibility — interior (no chunk window)
        assert!(natural_moongate_slot_eligible(13, 0, 5, 5, 13, 0, None));
        assert!(!natural_moongate_slot_eligible(13, 0, 5, 5, 14, 0, None));
        assert!(!natural_moongate_slot_eligible(13, 0, 5, 5, 13, 1, None));
        // Surface chunk-window
        assert!(natural_moongate_slot_eligible(
            0,
            0,
            10,
            10,
            0,
            0,
            Some((0, 0, 32, 32))
        ));
        assert!(!natural_moongate_slot_eligible(
            0,
            0,
            40,
            10,
            0,
            0,
            Some((0, 0, 32, 32))
        ));
        // Live-gate entry hook outcome
        assert!(natural_moongate_dispatches_meditate(0, 0));
        assert!(natural_moongate_dispatches_meditate(0, 9));
        assert!(!natural_moongate_dispatches_meditate(0, 10));
        assert!(!natural_moongate_dispatches_meditate(1, 0));
        // Cached-glyph slot (before noon = 0, noon onward = 1)
        for h in 0..=11u8 {
            assert_eq!(natural_moongate_cached_glyph_slot(h), 0);
        }
        for h in 12..=23u8 {
            assert_eq!(natural_moongate_cached_glyph_slot(h), 1);
        }
        assert_eq!(NARRATIVE_GATE_X, 233);
        assert_eq!(NARRATIVE_GATE_Y, 235);
    }

    #[test]
    fn town_location_class_and_index_split_per_spec() {
        // town-mode.md §2,§3,§4
        assert_eq!(town_location_class(0), None);
        for s in 1..=8u8 {
            assert_eq!(town_location_class(s), Some(TownLocationClass::Town));
            assert_eq!(town_per_class_index(s), Some(s - 1));
        }
        for s in 9..=16u8 {
            assert_eq!(town_location_class(s), Some(TownLocationClass::Dwelling));
            assert_eq!(town_per_class_index(s), Some(s - 9));
        }
        for s in 17..=24u8 {
            assert_eq!(town_location_class(s), Some(TownLocationClass::Castle));
            assert_eq!(town_per_class_index(s), Some(s - 17));
        }
        for s in 25..=32u8 {
            assert_eq!(town_location_class(s), Some(TownLocationClass::Keep));
            assert_eq!(town_per_class_index(s), Some(s - 25));
        }
        assert_eq!(town_location_class(33), None);
        assert_eq!(town_per_class_index(33), None);
        // Family names
        assert_eq!(TownLocationClass::Town.family_name(), "town");
        assert_eq!(TownLocationClass::Castle.family_name(), "castle");
        // Floor byte signed-eight-bit interpretation
        assert_eq!(town_floor_offset(0), 0);
        assert_eq!(town_floor_offset(1), 1);
        assert_eq!(town_floor_offset(127), 127);
        assert_eq!(town_floor_offset(128), -128);
        assert_eq!(town_floor_offset(255), -1); // basement (one floor below base)
        // Per-location grid + roster constants
        assert_eq!(TOWN_GRID_SIDE, 32);
        assert_eq!(TOWN_GRID_BYTES, 1024);
        assert_eq!(TOWN_NPC_ROSTER_SLOTS, 31);
        assert_eq!(TOWN_NPC_BLOCK_BYTES, 576);
    }

    #[test]
    fn blackthorn_rescue_verdict_bands_match_spec() {
        // blackthorn.md §7
        assert_eq!(BLACKTHORN_RESCUE_HANDOFF_SCENE, 17);
        assert_eq!(BLACKTHORN_RESCUE_HANDOFF_X, 10);
        assert_eq!(BLACKTHORN_RESCUE_HANDOFF_Y, 10);
        assert_eq!(BLACKTHORN_RESCUE_STANDING_FLOOR, 75);
        // Twenty-point bands: 0..19, 20..39, 40..59, 60..79, 80..99
        for s in 0..=19u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 0);
        }
        for s in 20..=39u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 1);
        }
        for s in 40..=59u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 2);
        }
        for s in 60..=79u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 3);
        }
        for s in 80..=99u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 4);
        }
        // Clamps to top band for values above the standing cap.
        assert_eq!(blackthorn_rescue_verdict_record(255), 4);
    }

    #[test]
    fn scene_route_classifies_per_main_loop_table() {
        // main-loop.md §3,§4
        assert_eq!(scene_route(0), SceneRoute::Overworld);
        for v in 1..=32u8 {
            assert_eq!(scene_route(v), SceneRoute::TownFamily);
        }
        for v in [33u8, 40, 50, 100, 127] {
            assert_eq!(scene_route(v), SceneRoute::Dungeon);
        }
        for v in 0x40..=0x42u8 {
            assert_eq!(scene_route(v), SceneRoute::IntroOrPreview);
        }
        assert_eq!(scene_route(0xFF), SceneRoute::CombatTemporary);
        // Outside-the-stock-byte high range routes to combat (high
        // values are treated as combat-class by readers).
        assert_eq!(scene_route(0x80), SceneRoute::CombatTemporary);
        assert_eq!(scene_route(0xFE), SceneRoute::CombatTemporary);

        // Stock-named DUNGEON.DAT record indices (33..=40 -> 0..=7)
        assert_eq!(dungeon_record_index(32), None);
        assert_eq!(dungeon_record_index(33), Some(0));
        assert_eq!(dungeon_record_index(40), Some(7));
        assert_eq!(dungeon_record_index(41), None);

        // Per-mode minute increments
        assert_eq!(mode_minute_increment(SceneRoute::Overworld), Some(2));
        assert_eq!(mode_minute_increment(SceneRoute::TownFamily), Some(1));
        assert_eq!(mode_minute_increment(SceneRoute::Dungeon), Some(1));
        assert_eq!(mode_minute_increment(SceneRoute::IntroOrPreview), None);
        assert_eq!(mode_minute_increment(SceneRoute::CombatTemporary), None);
    }

    #[test]
    fn npc_path_direction_codes_match_spec_table() {
        // npc-schedules.md §8.2
        assert_eq!(NPC_PATH_DIR_WEST, 1);
        assert_eq!(NPC_PATH_DIR_SOUTH, 2);
        assert_eq!(NPC_PATH_DIR_EAST, 3);
        assert_eq!(NPC_PATH_DIR_NORTH, 4);
        // Coordinate effects
        assert_eq!(npc_path_direction_offset(NPC_PATH_DIR_WEST), (-1, 0));
        assert_eq!(npc_path_direction_offset(NPC_PATH_DIR_SOUTH), (0, 1));
        assert_eq!(npc_path_direction_offset(NPC_PATH_DIR_EAST), (1, 0));
        assert_eq!(npc_path_direction_offset(NPC_PATH_DIR_NORTH), (0, -1));
        assert_eq!(npc_path_direction_offset(0), (0, 0));
        assert_eq!(npc_path_direction_offset(5), (0, 0));
        // Opposite-direction reversal
        assert_eq!(
            npc_path_direction_opposite(NPC_PATH_DIR_WEST),
            Some(NPC_PATH_DIR_EAST)
        );
        assert_eq!(
            npc_path_direction_opposite(NPC_PATH_DIR_EAST),
            Some(NPC_PATH_DIR_WEST)
        );
        assert_eq!(
            npc_path_direction_opposite(NPC_PATH_DIR_NORTH),
            Some(NPC_PATH_DIR_SOUTH)
        );
        assert_eq!(
            npc_path_direction_opposite(NPC_PATH_DIR_SOUTH),
            Some(NPC_PATH_DIR_NORTH)
        );
        assert_eq!(npc_path_direction_opposite(0), None);
        assert_eq!(npc_path_direction_opposite(5), None);
        // Other §8 constants
        assert_eq!(NPC_PATHFIND_QUEUE_CAPACITY, 32);
        assert_eq!(NPC_FLOOR_LINK_TILE_C8, 0xC8);
        assert_eq!(NPC_FLOOR_LINK_TILE_C9, 0xC9);
    }

    #[test]
    fn animation_phase_step_classifies_per_spec() {
        // active-objects.md §8
        assert_eq!(ANIMATION_PHASE_STEADY_NIBBLE, 0x0F);
        // Steady marker
        assert_eq!(animation_phase_step(0x0F), AnimationPhaseStep::Steady);
        assert_eq!(animation_phase_step(0xFF), AnimationPhaseStep::Steady);
        // AI-eligible (zero nibble)
        assert_eq!(animation_phase_step(0x00), AnimationPhaseStep::AiEligible);
        assert_eq!(animation_phase_step(0xA0), AnimationPhaseStep::AiEligible);
        // Mid-cycle decrement
        assert_eq!(animation_phase_step(0x01), AnimationPhaseStep::Decrement(0));
        assert_eq!(animation_phase_step(0x05), AnimationPhaseStep::Decrement(4));
        assert_eq!(animation_phase_step(0x0E), AnimationPhaseStep::Decrement(13));
        assert_eq!(animation_phase_step(0xA5), AnimationPhaseStep::Decrement(4));
    }

    #[test]
    fn codex_turn_in_stat_steps_match_spec_table() {
        // karma.md §7
        assert_eq!(ShrineVirtue::Honesty.codex_turn_in_stat_steps(), (0, 0, 1));
        assert_eq!(
            ShrineVirtue::Compassion.codex_turn_in_stat_steps(),
            (0, 1, 0)
        );
        assert_eq!(ShrineVirtue::Valor.codex_turn_in_stat_steps(), (1, 0, 0));
        assert_eq!(
            ShrineVirtue::Justice.codex_turn_in_stat_steps(),
            (0, 1, 1)
        );
        assert_eq!(
            ShrineVirtue::Sacrifice.codex_turn_in_stat_steps(),
            (1, 1, 0)
        );
        assert_eq!(ShrineVirtue::Honor.codex_turn_in_stat_steps(), (1, 0, 1));
        assert_eq!(
            ShrineVirtue::Spirituality.codex_turn_in_stat_steps(),
            (1, 1, 1)
        );
        assert_eq!(
            ShrineVirtue::Humility.codex_turn_in_stat_steps(),
            (0, 0, 0)
        );
        // Humility bonus: +3 only on Humility
        for v in ShrineVirtue::ALL {
            let expected = if matches!(v, ShrineVirtue::Humility) { 3 } else { 0 };
            assert_eq!(v.codex_turn_in_humility_bonus(), expected);
        }
    }

    #[test]
    fn boot_driver_selection_matches_spec() {
        // boot.md §5 explicit selector parsing
        assert_eq!(
            parse_explicit_driver_selector(Some("C")),
            Some(DisplayDriverFamily::Cga)
        );
        assert_eq!(
            parse_explicit_driver_selector(Some("e")),
            Some(DisplayDriverFamily::Ega)
        );
        assert_eq!(
            parse_explicit_driver_selector(Some("Tandy")),
            Some(DisplayDriverFamily::Tandy)
        );
        assert_eq!(
            parse_explicit_driver_selector(Some("h")),
            Some(DisplayDriverFamily::Hercules)
        );
        assert_eq!(parse_explicit_driver_selector(Some("X")), None);
        assert_eq!(parse_explicit_driver_selector(Some("")), None);
        assert_eq!(parse_explicit_driver_selector(None), None);

        // Resolution: explicit wins
        assert_eq!(
            resolve_driver_family(Some(DisplayDriverFamily::Cga), GraphicsCapability::Ega),
            Some(DisplayDriverFamily::Cga)
        );
        // Auto-detect mapping
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::GenericFourColour),
            Some(DisplayDriverFamily::Cga)
        );
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::Ega),
            Some(DisplayDriverFamily::Ega)
        );
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::Tandy),
            Some(DisplayDriverFamily::Tandy)
        );
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::Hercules),
            Some(DisplayDriverFamily::Hercules)
        );
        // EgaSentinel without an explicit selector takes no driver-load
        // path.
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::EgaSentinel),
            None
        );

        // Filenames
        assert_eq!(DisplayDriverFamily::Cga.driver_filename(), "CGA.DRV");
        assert_eq!(DisplayDriverFamily::Ega.driver_filename(), "EGA.DRV");
        assert_eq!(DisplayDriverFamily::Tandy.driver_filename(), "T1K.DRV");
        assert_eq!(DisplayDriverFamily::Hercules.driver_filename(), "HER.DRV");

        // Tandy low-memory downgrade threshold
        assert_eq!(TANDY_LOW_MEMORY_THRESHOLD_KB, 368);
        assert!(tandy_low_memory_downgrades(367));
        assert!(!tandy_low_memory_downgrades(368));
        assert!(!tandy_low_memory_downgrades(640));
    }

    #[test]
    fn wrap_byte_kind_classifies_break_visible_and_control() {
        // text-output.md §6
        assert_eq!(wrap_byte_kind(0x00), WrapByteKind::Break);
        assert_eq!(wrap_byte_kind(b'\n'), WrapByteKind::Break);
        assert_eq!(wrap_byte_kind(b'\r'), WrapByteKind::Break);
        assert_eq!(wrap_byte_kind(b' '), WrapByteKind::Break);
        // Visible: low-ASCII printable except space
        assert_eq!(wrap_byte_kind(b'A'), WrapByteKind::Visible);
        assert_eq!(wrap_byte_kind(b'!'), WrapByteKind::Visible);
        assert_eq!(wrap_byte_kind(b'~'), WrapByteKind::Visible);
        assert_eq!(wrap_byte_kind(b'0'), WrapByteKind::Visible);
        // Control: tab, escape, high-bit, etc.
        assert_eq!(wrap_byte_kind(0x09), WrapByteKind::Control);
        assert_eq!(wrap_byte_kind(0x1B), WrapByteKind::Control);
        assert_eq!(wrap_byte_kind(0x7F), WrapByteKind::Control);
        assert_eq!(wrap_byte_kind(0x80), WrapByteKind::Control);
        assert_eq!(wrap_byte_kind(0xFF), WrapByteKind::Control);
        // Min line-buffer width sanity
        assert!(WRAP_MIN_LINE_BUFFER >= 64);
    }

    #[test]
    fn command_for_letter_covers_full_a_to_z_table() {
        // commands.md §4
        assert_eq!(command_for_letter(b' '), Some(Command::Pass));
        assert_eq!(command_for_letter(b'A'), Some(Command::Attack));
        assert_eq!(command_for_letter(b'B'), Some(Command::Board));
        assert_eq!(command_for_letter(b'C'), Some(Command::Cast));
        assert_eq!(
            command_for_letter(b'D'),
            Some(Command::UnassignedRefusal)
        );
        assert_eq!(command_for_letter(b'E'), Some(Command::Enter));
        assert_eq!(command_for_letter(b'F'), Some(Command::Fire));
        assert_eq!(command_for_letter(b'G'), Some(Command::Get));
        assert_eq!(command_for_letter(b'H'), Some(Command::HoleUp));
        assert_eq!(command_for_letter(b'I'), Some(Command::Ignite));
        assert_eq!(command_for_letter(b'J'), Some(Command::Jimmy));
        assert_eq!(command_for_letter(b'K'), Some(Command::Klimb));
        assert_eq!(command_for_letter(b'L'), Some(Command::Look));
        assert_eq!(command_for_letter(b'M'), Some(Command::Mix));
        assert_eq!(command_for_letter(b'N'), Some(Command::NewOrder));
        assert_eq!(command_for_letter(b'O'), Some(Command::Open));
        assert_eq!(command_for_letter(b'P'), Some(Command::Push));
        assert_eq!(command_for_letter(b'Q'), Some(Command::Quit));
        assert_eq!(command_for_letter(b'R'), Some(Command::Ready));
        assert_eq!(command_for_letter(b'S'), Some(Command::Search));
        assert_eq!(command_for_letter(b'T'), Some(Command::Talk));
        assert_eq!(command_for_letter(b'U'), Some(Command::Use));
        assert_eq!(command_for_letter(b'V'), Some(Command::View));
        assert_eq!(
            command_for_letter(b'W'),
            Some(Command::UnassignedRefusal)
        );
        assert_eq!(command_for_letter(b'X'), Some(Command::Xit));
        assert_eq!(command_for_letter(b'Y'), Some(Command::Yell));
        assert_eq!(command_for_letter(b'Z'), Some(Command::ZStats));
        // Lowercase folded
        assert_eq!(command_for_letter(b'a'), Some(Command::Attack));
        // Outside range
        assert_eq!(command_for_letter(b'0'), None);
        assert_eq!(command_for_letter(0), None);
        // Verb prefix sample
        assert_eq!(Command::Attack.verb_prefix(), "Attack");
        assert_eq!(Command::Cast.verb_prefix(), "Cast");
        assert_eq!(Command::HoleUp.verb_prefix(), "Hole up");
        assert_eq!(Command::NewOrder.verb_prefix(), "New order");
        assert_eq!(Command::Xit.verb_prefix(), "X-it");
        assert_eq!(Command::ZStats.verb_prefix(), "Z-stats");
        assert_eq!(Command::UnassignedRefusal.verb_prefix(), "What?");
    }

    #[test]
    fn intro_menu_action_matches_spec_keys() {
        // intro.md §6
        assert_eq!(intro_menu_action(b'J'), Some(IntroMenuAction::JourneyOnward));
        assert_eq!(
            intro_menu_action(b'C'),
            Some(IntroMenuAction::CreateNewCharacter)
        );
        assert_eq!(
            intro_menu_action(b'T'),
            Some(IntroMenuAction::TransferFromUltimaIv)
        );
        assert_eq!(
            intro_menu_action(b'U'),
            Some(IntroMenuAction::UltimaVIntroduction)
        );
        assert_eq!(
            intro_menu_action(b'A'),
            Some(IntroMenuAction::Acknowledgements)
        );
        assert_eq!(intro_menu_action(b'R'), Some(IntroMenuAction::ReturnToView));
        // Lowercase folded
        assert_eq!(intro_menu_action(b'j'), Some(IntroMenuAction::JourneyOnward));
        assert_eq!(intro_menu_action(b'r'), Some(IntroMenuAction::ReturnToView));
        // Enter / Return -> RepeatCachedSelection
        assert_eq!(
            intro_menu_action(b'\r'),
            Some(IntroMenuAction::RepeatCachedSelection)
        );
        assert_eq!(
            intro_menu_action(b'\n'),
            Some(IntroMenuAction::RepeatCachedSelection)
        );
        // Invalid
        assert_eq!(intro_menu_action(b'B'), None);
        assert_eq!(intro_menu_action(b'X'), None);
        assert_eq!(intro_menu_action(0), None);
        assert_eq!(intro_menu_action(b' '), None);
    }

    #[test]
    fn boardable_family_classifier_matches_spec_table() {
        // vehicles.md §4
        assert_eq!(boardable_family(0x10), Some(BoardableFamily::Horse));
        assert_eq!(boardable_family(0x11), Some(BoardableFamily::Horse));
        // Mounted-horse ranges are not boardable parked objects.
        assert_eq!(boardable_family(0x12), None);
        assert_eq!(boardable_family(0x13), None);
        // Carpet
        assert_eq!(boardable_family(0x1B), Some(BoardableFamily::MagicCarpet));
        assert_eq!(boardable_family(0x14), None);
        // Ship
        for byte in 0x24..=0x27u8 {
            assert_eq!(boardable_family(byte), Some(BoardableFamily::Ship));
        }
        // Skiff
        for byte in 0x28..=0x2Bu8 {
            assert_eq!(boardable_family(byte), Some(BoardableFamily::Skiff));
        }
        assert_eq!(boardable_family(0x2C), None);
        assert_eq!(boardable_family(0x00), None);
        // Mount horse marker
        assert_eq!(mount_horse_marker(0x10), Some(0x12));
        assert_eq!(mount_horse_marker(0x11), Some(0x13));
        assert_eq!(mount_horse_marker(0x12), None);
        assert_eq!(mount_horse_marker(0x1B), None);
        // Ship boarding warning predicate
        assert_eq!(SHIP_BOARDING_HULL_WARNING_THRESHOLD, 10);
        assert!(ship_boarding_warns(0, 2)); // hull below 10
        assert!(ship_boarding_warns(9, 2)); // hull below 10
        assert!(!ship_boarding_warns(10, 2));
        assert!(ship_boarding_warns(50, 0)); // no skiffs
        assert!(!ship_boarding_warns(50, 1));
    }

    #[test]
    fn cast_dispatcher_gate_matches_spec_order_and_messages() {
        // magic.md §7
        // Scene gate first: Not here! before charge consumption.
        let r = cast_dispatcher_gate(false, 0, 0, 0, 0);
        assert_eq!(r, CastGateOutcome::NotHere);
        assert!(!r.consumed_charge());
        assert!(!r.consumed_mana());
        assert_eq!(r.message(), "Not here!");

        // No charges: None mixed!, no charge spent.
        let r = cast_dispatcher_gate(true, 0, 99, 8, 1);
        assert_eq!(r, CastGateOutcome::NoneMixed);
        assert!(!r.consumed_charge());
        assert!(!r.consumed_mana());
        assert_eq!(r.message(), "None mixed!");

        // Mana too low: charge spent, mana not.
        let r = cast_dispatcher_gate(true, 1, 2, 8, 5);
        assert_eq!(r, CastGateOutcome::ManaTooLowChargeOnly);
        assert!(r.consumed_charge());
        assert!(!r.consumed_mana());
        assert_eq!(r.message(), "M.P. too low!");

        // Level too low: charge AND mana spent.
        let r = cast_dispatcher_gate(true, 1, 99, 1, 5);
        assert_eq!(r, CastGateOutcome::LevelTooLowChargeAndMana);
        assert!(r.consumed_charge());
        assert!(r.consumed_mana());
        assert_eq!(r.message(), "M.P. too low!");

        // All gates pass.
        let r = cast_dispatcher_gate(true, 1, 99, 8, 5);
        assert_eq!(r, CastGateOutcome::Cast);
        assert!(r.consumed_charge());
        assert!(r.consumed_mana());

        // Heal amount: 0..=60 roll, halved, zero -> 1.
        assert_eq!(heal_spell_amount_from_raw_roll_u8(0), 1);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(1), 1);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(2), 1);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(3), 1);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(4), 2);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(60), 30);
    }

    #[test]
    fn dungeon_cell_class_of_matches_high_nibble_table() {
        // dungeon-mode.md §3
        assert_eq!(dungeon_cell_class_of(0x00), DungeonCellClass::Passage);
        assert_eq!(dungeon_cell_class_of(0x0F), DungeonCellClass::Passage);
        assert_eq!(dungeon_cell_class_of(0x10), DungeonCellClass::UpLadder);
        assert_eq!(dungeon_cell_class_of(0x20), DungeonCellClass::DownLadder);
        assert_eq!(
            dungeon_cell_class_of(0x30),
            DungeonCellClass::TwoWayLadder
        );
        assert_eq!(dungeon_cell_class_of(0x40), DungeonCellClass::Chest);
        assert_eq!(dungeon_cell_class_of(0x50), DungeonCellClass::Fountain);
        assert_eq!(dungeon_cell_class_of(0x60), DungeonCellClass::PitTrap);
        assert_eq!(dungeon_cell_class_of(0x69), DungeonCellClass::PitTrap);
        assert_eq!(
            dungeon_cell_class_of(0x70),
            DungeonCellClass::PassageVariant
        );
        assert_eq!(
            dungeon_cell_class_of(0x80),
            DungeonCellClass::EnergyField
        );
        assert_eq!(
            dungeon_cell_class_of(0x90),
            DungeonCellClass::EnergyFieldSecondary
        );
        assert_eq!(
            dungeon_cell_class_of(0xA0),
            DungeonCellClass::RoomHelperState
        );
        for high in 0xB..=0xE {
            assert_eq!(dungeon_cell_class_of(high << 4), DungeonCellClass::Wall);
        }
        assert_eq!(
            dungeon_cell_class_of(0xF0),
            DungeonCellClass::HeavyDoorOrRoomTrigger
        );
        // Convenience predicates
        assert!(DungeonCellClass::Wall.is_wall());
        assert!(!DungeonCellClass::Passage.is_wall());
        assert!(DungeonCellClass::UpLadder.is_ladder());
        assert!(DungeonCellClass::DownLadder.is_ladder());
        assert!(DungeonCellClass::TwoWayLadder.is_ladder());
        assert!(!DungeonCellClass::Chest.is_ladder());
        assert!(DungeonCellClass::Passage.is_passage_like());
        assert!(DungeonCellClass::PassageVariant.is_passage_like());
        assert!(!DungeonCellClass::Wall.is_passage_like());
    }

    #[test]
    fn daylight_base_value_matches_spec_table() {
        // time.md §6
        // Underworld / dungeon depth are always dark
        assert_eq!(daylight_base_value(12, 0, true, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(12, 0, false, 1), FULL_DARKNESS);
        // Pre-dawn / post-dusk surface
        assert_eq!(daylight_base_value(0, 0, false, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(4, 59, false, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(20, 0, false, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(23, 0, false, 0), FULL_DARKNESS);
        // Daytime band
        for hour in 6..=18u8 {
            assert_eq!(daylight_base_value(hour, 0, false, 0), FULL_DAYLIGHT);
            assert_eq!(daylight_base_value(hour, 30, false, 0), FULL_DAYLIGHT);
        }
        // Dawn at hour 5
        assert_eq!(daylight_base_value(5, 0, false, 0), 2);
        assert_eq!(daylight_base_value(5, 9, false, 0), 2);
        assert_eq!(daylight_base_value(5, 10, false, 0), 5);
        assert_eq!(daylight_base_value(5, 19, false, 0), 5);
        assert_eq!(daylight_base_value(5, 20, false, 0), 10);
        assert_eq!(daylight_base_value(5, 30, false, 0), 20);
        assert_eq!(daylight_base_value(5, 40, false, 0), 34);
        assert_eq!(daylight_base_value(5, 50, false, 0), 49);
        assert_eq!(daylight_base_value(5, 59, false, 0), 49);
        // Dusk at hour 19 (mirror of dawn)
        assert_eq!(daylight_base_value(19, 0, false, 0), 49);
        assert_eq!(daylight_base_value(19, 9, false, 0), 49);
        assert_eq!(daylight_base_value(19, 10, false, 0), 34);
        assert_eq!(daylight_base_value(19, 20, false, 0), 20);
        assert_eq!(daylight_base_value(19, 30, false, 0), 10);
        assert_eq!(daylight_base_value(19, 40, false, 0), 5);
        assert_eq!(daylight_base_value(19, 50, false, 0), 2);
        assert_eq!(daylight_base_value(19, 59, false, 0), 2);
    }

    #[test]
    fn normalize_disk_prompt_mode_folds_2_and_5_to_1() {
        // screen-mode-dispatch.md §5
        assert_eq!(normalize_disk_prompt_mode(0), 0);
        assert_eq!(normalize_disk_prompt_mode(1), 1);
        assert_eq!(normalize_disk_prompt_mode(2), 1);
        assert_eq!(normalize_disk_prompt_mode(3), 3);
        assert_eq!(normalize_disk_prompt_mode(4), 4);
        assert_eq!(normalize_disk_prompt_mode(5), 1);
        assert_eq!(normalize_disk_prompt_mode(6), 6);
        assert_eq!(normalize_disk_prompt_mode(255), 255);
    }

    #[test]
    fn save_load_disk_swap_and_double_write_predicates() {
        // save-load.md §4.2 step 6: enter the underworld disk-swap loop
        // only when overworld scene + non-zero Z.
        assert_eq!(SAVE_SCENE_OVERWORLD, 0);
        assert!(save_load_needs_underworld_disk_swap(0, 1));
        assert!(save_load_needs_underworld_disk_swap(0, 255));
        assert!(!save_load_needs_underworld_disk_swap(0, 0));
        assert!(!save_load_needs_underworld_disk_swap(13, 1));
        assert!(!save_load_needs_underworld_disk_swap(33, 0));

        // save-load.md §5.2 step 5: defensive UNDER.OOL re-flush.
        assert!(save_flow_double_writes_underworld(0));
        assert!(!save_flow_double_writes_underworld(1));
        assert!(save_flow_double_writes_underworld(2));

        // save-load.md §3.1: file lengths and Z sentinel.
        assert_eq!(SAVED_OOL_FILE_LEN, 512);
        assert_eq!(PER_PLANE_OOL_FILE_LEN, 256);
        assert_eq!(INIT_OOL_FILE_LEN, 256);
        assert_eq!(OOL_NO_Z_SENTINEL, 0xFF);
    }

    #[test]
    fn input_direction_codes_match_spec_table() {
        // input.md §5
        assert_eq!(
            input_code_direction(0xD3),
            Some(InputDirection::Northwest)
        );
        assert_eq!(
            input_code_direction(0xD4),
            Some(InputDirection::Southwest)
        );
        assert_eq!(
            input_code_direction(0xD5),
            Some(InputDirection::Northeast)
        );
        assert_eq!(
            input_code_direction(0xD6),
            Some(InputDirection::Southeast)
        );
        assert_eq!(input_code_direction(0xFB), Some(InputDirection::West));
        assert_eq!(input_code_direction(0xFC), Some(InputDirection::East));
        assert_eq!(input_code_direction(0xFD), Some(InputDirection::North));
        assert_eq!(input_code_direction(0xFE), Some(InputDirection::South));
        // Non-direction bytes
        assert_eq!(input_code_direction(b'A'), None);
        assert_eq!(input_code_direction(0x00), None);
        assert_eq!(input_code_direction(0xFF), None);
        // Cardinal predicate
        assert!(InputDirection::North.is_cardinal());
        assert!(InputDirection::South.is_cardinal());
        assert!(InputDirection::East.is_cardinal());
        assert!(InputDirection::West.is_cardinal());
        assert!(!InputDirection::Northwest.is_cardinal());
        assert!(!InputDirection::Southeast.is_cardinal());

        // input.md §6 case fold
        assert_eq!(input_case_fold(b'a'), b'A');
        assert_eq!(input_case_fold(b'z'), b'Z');
        assert_eq!(input_case_fold(b'A'), b'A');
        assert_eq!(input_case_fold(b'0'), b'0');
        assert_eq!(input_case_fold(0xFC), 0xFC);
    }

    #[test]
    fn reserved_keyword_effect_matches_spec_words() {
        // conversation.md §6
        assert_eq!(TLK_INPUT_MAX_LEN, 15);
        assert_eq!(
            reserved_keyword_effect(b"NAME"),
            Some(ReservedKeywordEffect::NameEntry)
        );
        assert_eq!(
            reserved_keyword_effect(b"JOB"),
            Some(ReservedKeywordEffect::JobEntry)
        );
        assert_eq!(
            reserved_keyword_effect(b"WORK"),
            Some(ReservedKeywordEffect::JobEntry)
        );
        assert_eq!(
            reserved_keyword_effect(b"BYE"),
            Some(ReservedKeywordEffect::ByePath)
        );
        assert_eq!(
            reserved_keyword_effect(b"THANK"),
            Some(ReservedKeywordEffect::ByePath)
        );
        // JOIN and WHO ART THOU are not engine-reserved.
        assert_eq!(reserved_keyword_effect(b"JOIN"), None);
        assert_eq!(reserved_keyword_effect(b"WHO ART THOU"), None);
        // Case sensitivity: caller is responsible for the upper-case fold.
        assert_eq!(reserved_keyword_effect(b"name"), None);
    }

    #[test]
    fn tlk_keyword_match_is_space_boundary_and_bit7_strip() {
        // conversation.md §6
        // Exact match
        assert!(tlk_keyword_matches(b"GRAN", b"GRAN"));
        // Space-boundary match
        assert!(tlk_keyword_matches(b"GRAN", b"GRAN PA"));
        // Not a substring/prefix match without a space boundary
        assert!(!tlk_keyword_matches(b"GRAN", b"GRANDPA"));
        // Bit-7 strip on the keyword side (high-bit obfuscated)
        let obfuscated = [b'G' | 0x80, b'R' | 0x80, b'A' | 0x80, b'N' | 0x80];
        assert!(tlk_keyword_matches(&obfuscated, b"GRAN"));
        // Case insensitive
        assert!(tlk_keyword_matches(b"NAME", b"name"));
        assert!(tlk_keyword_matches(b"name", b"NAME"));
        // Empty keyword never matches
        assert!(!tlk_keyword_matches(b"", b"NAME"));
        // Input shorter than keyword
        assert!(!tlk_keyword_matches(b"NAMEE", b"NAME"));
    }

    #[test]
    fn schedule_floor_state_matches_spec_table() {
        // npc-schedules.md §6
        // both equal -> 2
        assert_eq!(schedule_floor_state(1, 1, 1), NPC_STATE_INPLANE_MOVE);
        // equal/below -> 7 (target floor index > map floor index)
        assert_eq!(schedule_floor_state(1, 2, 1), NPC_STATE_CLIMB_DOWN_OFF_FLOOR);
        // equal/above -> 6
        assert_eq!(schedule_floor_state(1, 0, 1), NPC_STATE_CLIMB_UP_OFF_FLOOR);
        // below/equal -> 5 (npc floor index > map floor index)
        assert_eq!(schedule_floor_state(2, 1, 1), NPC_STATE_ASCEND_TOWARD_TARGET);
        // above/equal -> 4
        assert_eq!(schedule_floor_state(0, 1, 1), NPC_STATE_DESCEND_TOWARD_TARGET);
        // neither/neither -> 8
        assert_eq!(schedule_floor_state(0, 2, 1), NPC_STATE_PARKED_OFF_FLOOR);
        assert_eq!(schedule_floor_state(2, 0, 1), NPC_STATE_PARKED_OFF_FLOOR);
        assert_eq!(schedule_floor_state(2, 3, 1), NPC_STATE_PARKED_OFF_FLOOR);
    }

    #[test]
    fn tlk_scene_branch_mask_does_not_wrap() {
        // quest-flags.md §3
        assert_eq!(tlk_scene_branch_mask(0), 0x0000_0001);
        assert_eq!(tlk_scene_branch_mask(1), 0x0000_0002);
        assert_eq!(tlk_scene_branch_mask(31), 0x8000_0000);
        // No wrap or clamp: bit 32 and beyond produce zero mask.
        assert_eq!(tlk_scene_branch_mask(32), 0);
        assert_eq!(tlk_scene_branch_mask(255), 0);

        // Setter then tester round-trip
        let slot = tlk_scene_branch_set(0, 5);
        assert!(tlk_scene_branch_is_set(slot, 5));
        assert!(!tlk_scene_branch_is_set(slot, 6));
        // Out-of-range setter is a no-op
        assert_eq!(tlk_scene_branch_set(slot, 32), slot);
        assert!(!tlk_scene_branch_is_set(slot, 32));
    }

    #[test]
    fn conversation_letter_action_table_matches_spec() {
        // quest-flags.md §4
        assert_eq!(
            conversation_letter_action(b'A'),
            Some(ConversationLetterAction::GrantFood)
        );
        assert_eq!(
            conversation_letter_action(b'B'),
            Some(ConversationLetterAction::GrantGold)
        );
        assert_eq!(
            conversation_letter_action(b'C'),
            Some(ConversationLetterAction::GrantKeys)
        );
        assert_eq!(
            conversation_letter_action(b'D'),
            Some(ConversationLetterAction::GrantGems)
        );
        assert_eq!(
            conversation_letter_action(b'E'),
            Some(ConversationLetterAction::GrantTorches)
        );
        assert_eq!(
            conversation_letter_action(b'F'),
            Some(ConversationLetterAction::SetGrappleGate)
        );
        assert_eq!(
            conversation_letter_action(b'G'),
            Some(ConversationLetterAction::GrantMagicCarpet)
        );
        assert_eq!(
            conversation_letter_action(b'H'),
            Some(ConversationLetterAction::SetSextant)
        );
        assert_eq!(
            conversation_letter_action(b'I'),
            Some(ConversationLetterAction::SetSpyglass)
        );
        assert_eq!(
            conversation_letter_action(b'J'),
            Some(ConversationLetterAction::SetBlackBadge)
        );
        assert_eq!(
            conversation_letter_action(b'K'),
            Some(ConversationLetterAction::GrantSkullKeys)
        );
        assert_eq!(conversation_letter_action(b'L'), None);
        assert_eq!(conversation_letter_action(b'a'), None);
        assert_eq!(conversation_letter_action(0), None);
    }

    #[test]
    fn visibility_markers_classify_per_spec() {
        // visibility.md §2
        assert_eq!(VIEWPORT_SIDE, 11);
        assert_eq!(VIEWPORT_ROW_STRIDE, 32);
        assert_eq!(TERRAIN_BAND_ROW_STRIDE, 16);
        assert_eq!(VIEWPORT_PLAYER_ROW, 5);
        assert_eq!(VIEWPORT_PLAYER_COL, 5);
        assert_eq!(visibility_marker(0xFF), VisibilityMarker::Hidden);
        assert_eq!(visibility_marker(0x00), VisibilityMarker::UseCompanion);
        assert_eq!(visibility_marker(0xDD), VisibilityMarker::ClearVisible);
        assert_eq!(visibility_marker(0x1C), VisibilityMarker::DimPeriphery);
        assert_eq!(
            visibility_marker(0x87),
            VisibilityMarker::AlreadyRendered
        );
        assert_eq!(
            visibility_marker(0x42),
            VisibilityMarker::DirectTile(0x42)
        );

        // visibility.md §3 light-radius branch (signed)
        assert_eq!(light_radius_branch(0), LightRadiusBranch::PitchDark);
        assert_eq!(light_radius_branch(1), LightRadiusBranch::Carve(1));
        assert_eq!(light_radius_branch(50), LightRadiusBranch::Carve(50));
        assert_eq!(light_radius_branch(127), LightRadiusBranch::Carve(127));
        assert_eq!(light_radius_branch(128), LightRadiusBranch::DebugFullFill);
        assert_eq!(light_radius_branch(255), LightRadiusBranch::DebugFullFill);
    }

    #[test]
    fn active_object_eviction_phase_matches_spec_cascade() {
        // active-objects.md §4
        // Empty slot is always phase 1.
        assert_eq!(active_object_eviction_phase(0x00, true), Some(1));
        assert_eq!(active_object_eviction_phase(0x00, false), Some(1));

        // 0x01..=0x0F low-priority scenery
        assert_eq!(active_object_eviction_phase(0x01, true), Some(2));
        assert_eq!(active_object_eviction_phase(0x0F, true), Some(2));
        assert_eq!(active_object_eviction_phase(0x01, false), Some(6));

        // 0x80..=0xFF monsters/dynamic actors (except 0xB5)
        assert_eq!(active_object_eviction_phase(0x80, true), Some(3));
        assert_eq!(active_object_eviction_phase(0xFF, true), Some(3));
        assert_eq!(active_object_eviction_phase(0x80, false), Some(7));
        assert_eq!(active_object_eviction_phase(0xB5, true), None);
        assert_eq!(active_object_eviction_phase(0xB5, false), None);

        // 0x10..=0x11 door/fixture-like
        assert_eq!(active_object_eviction_phase(0x10, true), Some(4));
        assert_eq!(active_object_eviction_phase(0x11, true), Some(4));
        assert_eq!(active_object_eviction_phase(0x10, false), Some(8));

        // 0x30..=0x7F items/chests
        assert_eq!(active_object_eviction_phase(0x30, true), Some(5));
        assert_eq!(active_object_eviction_phase(0x7F, true), Some(5));
        assert_eq!(active_object_eviction_phase(0x30, false), Some(9));

        // 0x12..=0x1F NPC/person ranges and 0x20..=0x2F vehicle ranges
        // are protected from off-screen phases but eligible for the
        // last-resort phase 10.
        assert_eq!(active_object_eviction_phase(0x12, true), Some(10));
        assert_eq!(active_object_eviction_phase(0x1F, true), Some(10));
        assert_eq!(active_object_eviction_phase(0x20, true), Some(10));
        assert_eq!(active_object_eviction_phase(0x2F, true), Some(10));
        assert_eq!(active_object_eviction_phase(0x20, false), Some(10));
    }

    #[test]
    fn chargen_questionnaire_always_floors_strength_to_twenty() {
        // chargen.md §7: max STR contribution is 2 per question and there
        // are 7 questions, so the floor always fires.
        assert_eq!(CHARGEN_STR_FLOOR, 20);
        assert_eq!(CHARGEN_STARTING_PARTY_SIZE, 3);

        // Empty winners list: STR should still be floored to 20.
        let stats = chargen_stats_from_winners(&[]);
        assert_eq!(stats.strength, CHARGEN_STR_FLOOR);
        assert_eq!(stats.dexterity, 0);
        assert_eq!(stats.intelligence, 0);

        // Worst-case STR contribution (any seven Spirituality wins):
        // chargen_virtue_stat_delta(Spirituality) is INT-only, so STR
        // remains 0 before the floor.
        let all_spirituality = vec![ShrineVirtue::Spirituality; 7];
        let stats = chargen_stats_from_winners(&all_spirituality);
        assert_eq!(stats.strength, CHARGEN_STR_FLOOR);

        // Best-case STR contribution: any seven full-STR virtues should
        // still floor the result, since 7*max delta < 20 only if delta<3.
        // Either way, the result must be >= floor.
        for v in [
            ShrineVirtue::Honesty,
            ShrineVirtue::Compassion,
            ShrineVirtue::Valor,
            ShrineVirtue::Justice,
            ShrineVirtue::Sacrifice,
            ShrineVirtue::Honor,
            ShrineVirtue::Spirituality,
            ShrineVirtue::Humility,
        ] {
            let stats = chargen_stats_from_winners(&vec![v; 7]);
            assert!(stats.strength >= CHARGEN_STR_FLOOR);
        }
    }

    #[test]
    fn trap_effect_classification_matches_spec_table() {
        // traps.md §3
        assert_eq!(trap_effect_for_id(0), Some(TrapEffect::Acid));
        assert_eq!(trap_effect_for_id(1), Some(TrapEffect::Poison));
        assert_eq!(trap_effect_for_id(2), Some(TrapEffect::Bomb));
        assert_eq!(trap_effect_for_id(3), Some(TrapEffect::Gas));
        assert_eq!(trap_effect_for_id(4), None);
        assert_eq!(trap_effect_for_id(255), None);

        assert_eq!(trap_effect_damage_max(TrapEffect::Acid), Some(30));
        assert_eq!(trap_effect_damage_max(TrapEffect::Bomb), Some(8));
        assert_eq!(trap_effect_damage_max(TrapEffect::Poison), None);
        assert_eq!(trap_effect_damage_max(TrapEffect::Gas), None);

        assert!(!trap_effect_targets_whole_party(TrapEffect::Acid));
        assert!(!trap_effect_targets_whole_party(TrapEffect::Poison));
        assert!(trap_effect_targets_whole_party(TrapEffect::Bomb));
        assert!(trap_effect_targets_whole_party(TrapEffect::Gas));

        // The non-combat lookup table publishes 3/2/2/1 weights for the
        // four effect ids.
        let mut counts = [0u32; 4];
        for index in 0..8u8 {
            let id = shared_trap_effect_id_from_index(index, false);
            counts[usize::from(id)] += 1;
        }
        assert_eq!(counts, [3, 2, 2, 1]);

        // In combat the resolver picks only ids 0 and 1.
        for index in 0..8u8 {
            let id = shared_trap_effect_id_from_index(index, true);
            assert!(id == 0 || id == 1);
        }
    }

    #[test]
    fn dungeon_chest_rows_match_spec_table() {
        // containers.md §6
        assert_eq!(DUNGEON_CHEST_ROWS.len(), 7);
        let expected = [
            (2u8, DungeonChestReward::Food),
            (4, DungeonChestReward::Gold),
            (5, DungeonChestReward::Keys),
            (10, DungeonChestReward::Gems),
            (20, DungeonChestReward::Torches),
            (25, DungeonChestReward::Potion),
            (25, DungeonChestReward::Scroll),
        ];
        for (i, row) in DUNGEON_CHEST_ROWS.iter().enumerate() {
            assert_eq!(row.gate_threshold, expected[i].0);
            assert_eq!(row.reward, expected[i].1);
        }
        // Per-row gate max: 4*depth + 4
        assert_eq!(dungeon_chest_row_gate_max(0), 4);
        assert_eq!(dungeon_chest_row_gate_max(7), 32);
        // Awarded when threshold <= roll
        let food = DUNGEON_CHEST_ROWS[0];
        assert!(dungeon_chest_row_awarded(food, 2));
        assert!(dungeon_chest_row_awarded(food, 31));
        assert!(!dungeon_chest_row_awarded(food, 1));
        let scroll = DUNGEON_CHEST_ROWS[6];
        assert!(dungeon_chest_row_awarded(scroll, 25));
        assert!(!dungeon_chest_row_awarded(scroll, 24));
    }

    #[test]
    fn table_food_get_directional_rules_match_spec() {
        // containers.md §7
        assert_eq!(table_food_get_resulting_tile(0x9B, 0, -1), Some(0x95));
        assert_eq!(table_food_get_resulting_tile(0x9B, 0, 1), None);
        assert_eq!(table_food_get_resulting_tile(0x9B, -1, 0), None);
        assert_eq!(table_food_get_resulting_tile(0x9B, 1, 0), None);
        assert_eq!(table_food_get_resulting_tile(0x9B, 1, -1), None);
        assert_eq!(table_food_get_resulting_tile(0x9C, 0, -1), Some(0x9A));
        assert_eq!(table_food_get_resulting_tile(0x9C, 0, 1), Some(0x9B));
        assert_eq!(table_food_get_resulting_tile(0x9C, -1, 0), None);
        assert_eq!(table_food_get_resulting_tile(0x9C, 1, 0), None);
        assert_eq!(table_food_get_resulting_tile(0x95, 0, -1), None);
    }

    #[test]
    fn jimmy_helpers_match_spec_formulas() {
        // doors-and-z-transitions.md §3
        // Door pick: class > roll
        assert_eq!(JIMMY_DOOR_DIE_LOW, 1);
        assert_eq!(JIMMY_DOOR_DIE_HIGH, 29);
        assert!(jimmy_door_succeeds(20, 19));
        assert!(!jimmy_door_succeeds(20, 20));
        assert!(!jimmy_door_succeeds(20, 21));
        assert!(jimmy_door_succeeds(29, 1));

        // Object chest: requires high bit; threshold = (diff - class + 30)/2
        assert_eq!(object_chest_jimmy_threshold(0x40, 10), None);
        // diff=0x10=16, class=10 -> (16-10+30)/2 = 18
        assert_eq!(object_chest_jimmy_threshold(0x90, 10), Some(18));
        // diff=20, class=40 -> (20-40+30)/2 = 5
        assert_eq!(object_chest_jimmy_threshold(0x94, 40), Some(5));
        // Negative raw -> 0
        assert_eq!(object_chest_jimmy_threshold(0x81, 100), Some(0));
        assert!(object_chest_jimmy_succeeds(18, 1));
        assert!(object_chest_jimmy_succeeds(18, 18));
        assert!(!object_chest_jimmy_succeeds(18, 19));

        // Dungeon chest: threshold = (2*depth - class + 30)/2
        // depth=4, class=20 -> (8-20+30)/2 = 9
        assert_eq!(dungeon_chest_jimmy_threshold(4, 20), 9);
        // depth=8, class=10 -> (16-10+30)/2 = 18
        assert_eq!(dungeon_chest_jimmy_threshold(8, 10), 18);
        assert!(dungeon_chest_jimmy_succeeds(9, 9));
        assert!(!dungeon_chest_jimmy_succeeds(9, 10));

        assert_eq!(DOOR_AUTO_CLOSE_TURNS, 4);
    }

    #[test]
    fn lighting_helpers_match_spec_table() {
        // lighting.md §4
        assert_eq!(apply_personal_light(2, 0, 0), 2);
        assert_eq!(apply_personal_light(2, 1, 0), TORCH_LIGHT_FLOOR);
        assert_eq!(apply_personal_light(2, 0, 1), LIGHT_SPELL_FLOOR);
        // Torch dominates spell when both nonzero
        assert_eq!(apply_personal_light(2, 5, 5), TORCH_LIGHT_FLOOR);
        // Ambient already brighter than the floor wins
        assert_eq!(apply_personal_light(FULL_DAYLIGHT, 5, 5), FULL_DAYLIGHT);
        assert_eq!(apply_personal_light(20, 1, 0), 20);

        // lighting.md §6
        assert!(dungeon_blackout(0, 0));
        assert!(!dungeon_blackout(1, 0));
        assert!(!dungeon_blackout(0, 1));

        // lighting.md §5
        assert_eq!(decay_light_counter(10, 1), 9);
        assert_eq!(decay_light_counter(10, 2), 8);
        assert_eq!(decay_light_counter(2, 5), 0);
        assert_eq!(decay_light_counter(0, 1), 0);

        // lighting.md §3
        assert!(!ambient_is_sentinel(50));
        assert!(ambient_is_sentinel(51));
        assert!(ambient_is_sentinel(255));

        // lighting.md §8
        assert_eq!(ignite_torch_surface(), 240);
        assert_eq!(ignite_torch_dungeon(0, 112), 112);
        assert_eq!(ignite_torch_dungeon(100, 127), 227);
        assert_eq!(ignite_torch_dungeon(200, 127), 255);
        assert_eq!(LIGHT_SPELL_DURATION, 100);
        assert_eq!(GREAT_LIGHT_SPELL_DURATION, 255);
    }

    #[test]
    fn player_sail_wait_ticks_matches_weather_table() {
        // weather.md §5
        // Calm never releases.
        for heading in [
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ] {
            assert_eq!(WindState::Calm.player_sail_wait_ticks(heading), None);
        }
        // North wind row: N=2, E=0, S=1, W=0
        assert_eq!(
            WindState::North.player_sail_wait_ticks(Direction::North),
            Some(2)
        );
        assert_eq!(
            WindState::North.player_sail_wait_ticks(Direction::East),
            Some(0)
        );
        assert_eq!(
            WindState::North.player_sail_wait_ticks(Direction::South),
            Some(1)
        );
        assert_eq!(
            WindState::North.player_sail_wait_ticks(Direction::West),
            Some(0)
        );
        // South wind row: N=1, E=0, S=2, W=0
        assert_eq!(
            WindState::South.player_sail_wait_ticks(Direction::North),
            Some(1)
        );
        assert_eq!(
            WindState::South.player_sail_wait_ticks(Direction::South),
            Some(2)
        );
        // East wind row: N=0, E=2, S=0, W=1
        assert_eq!(
            WindState::East.player_sail_wait_ticks(Direction::East),
            Some(2)
        );
        assert_eq!(
            WindState::East.player_sail_wait_ticks(Direction::West),
            Some(1)
        );
        // West wind row: N=0, E=1, S=0, W=2
        assert_eq!(
            WindState::West.player_sail_wait_ticks(Direction::East),
            Some(1)
        );
        assert_eq!(
            WindState::West.player_sail_wait_ticks(Direction::West),
            Some(2)
        );
    }

    #[test]
    fn active_ship_cadence_matches_weather_table() {
        // weather.md §7
        for heading in [
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ] {
            assert_eq!(WindState::Calm.active_ship_cadence(heading), None);
        }
        // North-facing frame row
        assert_eq!(
            WindState::North.active_ship_cadence(Direction::North),
            Some((2, 3))
        );
        assert_eq!(
            WindState::South.active_ship_cadence(Direction::North),
            Some((3, 4))
        );
        assert_eq!(
            WindState::East.active_ship_cadence(Direction::North),
            Some((1, 1))
        );
        assert_eq!(
            WindState::West.active_ship_cadence(Direction::North),
            Some((1, 1))
        );
        // East-facing frame row
        assert_eq!(
            WindState::East.active_ship_cadence(Direction::East),
            Some((2, 3))
        );
        assert_eq!(
            WindState::West.active_ship_cadence(Direction::East),
            Some((3, 4))
        );
        // South-facing frame row
        assert_eq!(
            WindState::South.active_ship_cadence(Direction::South),
            Some((2, 3))
        );
        assert_eq!(
            WindState::North.active_ship_cadence(Direction::South),
            Some((3, 4))
        );
        // West-facing frame row
        assert_eq!(
            WindState::West.active_ship_cadence(Direction::West),
            Some((2, 3))
        );
        assert_eq!(
            WindState::East.active_ship_cadence(Direction::West),
            Some((3, 4))
        );
    }

    #[test]
    fn karma_actions_apply_with_spec_clamps() {
        // karma.md §4
        // Completed-shrine offering adds the digit, capped at MAX
        assert_eq!(
            apply_karma_action(50, KarmaAction::CompletedShrineOffering { digit: 9 }),
            59
        );
        assert_eq!(
            apply_karma_action(95, KarmaAction::CompletedShrineOffering { digit: 9 }),
            MORAL_STANDING_MAX
        );
        // Codex turn-in: +3 normal, +6 for Humility
        assert_eq!(
            apply_karma_action(50, KarmaAction::CodexShrineTurnIn { humility: false }),
            53
        );
        assert_eq!(
            apply_karma_action(50, KarmaAction::CodexShrineTurnIn { humility: true }),
            56
        );
        assert_eq!(
            apply_karma_action(98, KarmaAction::CodexShrineTurnIn { humility: true }),
            MORAL_STANDING_MAX
        );
        // Town chest: -2, floored at 0
        assert_eq!(apply_karma_action(50, KarmaAction::TownChestOpened), 48);
        assert_eq!(apply_karma_action(1, KarmaAction::TownChestOpened), 0);
        assert_eq!(apply_karma_action(0, KarmaAction::TownChestOpened), 0);
        // Crop/table food: -1 when nonzero, no-op at 0
        assert_eq!(apply_karma_action(2, KarmaAction::CropOrTableFoodTaken), 1);
        assert_eq!(apply_karma_action(0, KarmaAction::CropOrTableFoodTaken), 0);
        // Town cannon hit: -5, floored at 0
        assert_eq!(apply_karma_action(10, KarmaAction::TownCannonHit), 5);
        assert_eq!(apply_karma_action(3, KarmaAction::TownCannonHit), 0);
        // Helped NPC thank-you: +2, capped
        assert_eq!(apply_karma_action(50, KarmaAction::HelpedNpcThankYou), 52);
        assert_eq!(
            apply_karma_action(98, KarmaAction::HelpedNpcThankYou),
            MORAL_STANDING_MAX
        );
        // Toll milestone: +1, +3 if left party with zero gold
        assert_eq!(
            apply_karma_action(
                50,
                KarmaAction::TollMilestone {
                    left_party_with_zero_gold: false
                }
            ),
            51
        );
        assert_eq!(
            apply_karma_action(
                50,
                KarmaAction::TollMilestone {
                    left_party_with_zero_gold: true
                }
            ),
            53
        );
        assert_eq!(
            apply_karma_action(
                98,
                KarmaAction::TollMilestone {
                    left_party_with_zero_gold: true
                }
            ),
            MORAL_STANDING_MAX
        );
    }

    #[test]
    fn sleep_ambush_monster_table_matches_spec() {
        // encounters.md §6
        assert_eq!(sleep_ambush_monster(0), Some(SleepAmbushMonster::GiantRat));
        assert_eq!(sleep_ambush_monster(1), Some(SleepAmbushMonster::GiantRat));
        assert_eq!(sleep_ambush_monster(2), Some(SleepAmbushMonster::Troll));
        assert_eq!(sleep_ambush_monster(3), Some(SleepAmbushMonster::Bat));
        assert_eq!(sleep_ambush_monster(4), Some(SleepAmbushMonster::Slime));
        assert_eq!(sleep_ambush_monster(5), Some(SleepAmbushMonster::GiantSpider));
        assert_eq!(sleep_ambush_monster(6), Some(SleepAmbushMonster::Gremlin));
        assert_eq!(sleep_ambush_monster(7), Some(SleepAmbushMonster::Headless));
        assert_eq!(sleep_ambush_monster(8), None);
        assert_eq!(sleep_ambush_monster(255), None);

        // Effective Giant Rat share = 2/8
        let rat_rows = (0..8u8)
            .filter(|r| sleep_ambush_monster(*r) == Some(SleepAmbushMonster::GiantRat))
            .count();
        assert_eq!(rat_rows, 2);

        // Sleep-ambush interruption: only outcome 0 in 0..64 interrupts.
        assert_eq!(SLEEP_AMBUSH_INTERRUPT_DENOMINATOR, 64);
        assert!(sleep_ambush_rest_interrupted(0));
        for roll in 1..SLEEP_AMBUSH_INTERRUPT_DENOMINATOR {
            assert!(!sleep_ambush_rest_interrupted(roll));
        }
    }

    #[test]
    fn random_encounter_threshold_matches_spec_table() {
        // encounters.md §3
        // Underworld: always 3
        for hour in 0..24u8 {
            assert_eq!(random_encounter_threshold(true, 0x05, hour), 3);
            assert_eq!(random_encounter_threshold(true, 0x20, hour), 3);
        }
        // Surface no-encounter band 0x20..=0x26
        assert_eq!(random_encounter_threshold(false, 0x20, 12), 0);
        assert_eq!(random_encounter_threshold(false, 0x26, 18), 0);
        assert_eq!(random_encounter_threshold(false, 0x20, 0), 3);
        assert_eq!(random_encounter_threshold(false, 0x26, 4), 3);
        // Surface tile 0x04 or wilderness 0x09..=0x0F
        assert_eq!(random_encounter_threshold(false, 0x04, 12), 2);
        assert_eq!(random_encounter_threshold(false, 0x09, 12), 2);
        assert_eq!(random_encounter_threshold(false, 0x0F, 12), 2);
        assert_eq!(random_encounter_threshold(false, 0x04, 0), 5);
        assert_eq!(random_encounter_threshold(false, 0x09, 4), 5);
        // Any other surface tile
        assert_eq!(random_encounter_threshold(false, 0x05, 12), 1);
        assert_eq!(random_encounter_threshold(false, 0x06, 18), 1);
        assert_eq!(random_encounter_threshold(false, 0x05, 0), 4);
        assert_eq!(random_encounter_threshold(false, 0x10, 4), 4);
    }

    #[test]
    fn ship_transport_marker_predicates_match_published_ranges() {
        // vehicles.md §6: hoisted 0x20..=0x23, furled 0x24..=0x27.
        for byte in 0x20..=0x23u8 {
            assert!(is_ship_transport_marker(byte));
            assert!(is_ship_transport_hoisted(byte));
            assert!(!is_ship_transport_furled(byte));
        }
        for byte in 0x24..=0x27u8 {
            assert!(is_ship_transport_marker(byte));
            assert!(!is_ship_transport_hoisted(byte));
            assert!(is_ship_transport_furled(byte));
        }
        for byte in [0x1F, 0x28, 0x00, 0xFFu8] {
            assert!(!is_ship_transport_marker(byte));
        }
    }

    #[test]
    fn ship_transport_heading_index_decodes_low_two_bits() {
        // vehicles.md §6: low two bits encode N=0, E=1, S=2, W=3 in both
        // hoisted and furled ranges.
        assert_eq!(ship_transport_heading_index(0x20), Some(0));
        assert_eq!(ship_transport_heading_index(0x21), Some(1));
        assert_eq!(ship_transport_heading_index(0x22), Some(2));
        assert_eq!(ship_transport_heading_index(0x23), Some(3));
        assert_eq!(ship_transport_heading_index(0x24), Some(0));
        assert_eq!(ship_transport_heading_index(0x27), Some(3));
        assert_eq!(ship_transport_heading_index(0x14), None);
    }

    #[test]
    fn active_object_slot_partition_constants_match_section_four() {
        // active-objects.md §4: slot 0 player; ordinary 1..=23; reserved
        // 24..=31; 0xB5 is the universally protected byte-0; off-screen
        // test radius is five cells.
        assert_eq!(ACTIVE_OBJECT_PLAYER_SLOT, 0);
        assert_eq!(ACTIVE_OBJECT_ORDINARY_FIRST, 1);
        assert_eq!(ACTIVE_OBJECT_ORDINARY_LAST, 23);
        assert_eq!(ACTIVE_OBJECT_RESERVED_FIRST, 24);
        assert_eq!(ACTIVE_OBJECT_RESERVED_LAST, 31);
        assert_eq!(ACTIVE_OBJECT_PROTECTED_TYPE_BYTE, 0xB5);
        assert_eq!(ACTIVE_OBJECT_OFF_SCREEN_RADIUS, 5);
    }

    #[test]
    fn tlk_introducer_argument_widths_match_section_seven_six() {
        // conversation.md §7.6: 0x85 GOLD-PAYMENT takes 3 bytes, 0x86
        // ACTION-DISPATCH and 0x8C IF-ELSE take 1 byte, 0xFE IF-ELSE-ALT
        // takes 2 bytes; other codes take none.
        assert_eq!(tlk_introducer_argument_count(TLK_CODE_GOLD_PAYMENT), Some(3));
        assert_eq!(
            tlk_introducer_argument_count(TLK_CODE_ACTION_DISPATCH),
            Some(1)
        );
        assert_eq!(tlk_introducer_argument_count(TLK_CODE_IF_ELSE), Some(1));
        assert_eq!(tlk_introducer_argument_count(TLK_CODE_IF_ELSE_ALT), Some(2));
        for code in [
            TLK_CODE_PRINT_AVATAR_NAME,
            TLK_CODE_END_STREAM,
            TLK_CODE_PAUSE,
            TLK_CODE_WAIT_KEY,
            TLK_CODE_CURSE_CHECK,
            TLK_CODE_PROTECT_RUN,
            TLK_CODE_END_OF_RESPONSE,
        ] {
            assert_eq!(tlk_introducer_argument_count(code), None);
        }
    }

    #[test]
    fn tile_class_partitions_byte_range_per_catalog_section_three() {
        // catalogs/tile-catalog.md §3 coarse class groupings.
        assert_eq!(coarse_tile_class(0x00), TileClass::Sentinel);
        for tile in TILE_WATER_FIRST..=TILE_WATER_LAST {
            assert_eq!(coarse_tile_class(tile), TileClass::Water);
        }
        assert_eq!(coarse_tile_class(0x05), TileClass::Terrain);
        assert_eq!(coarse_tile_class(0x0F), TileClass::Terrain);
        assert_eq!(coarse_tile_class(0x10), TileClass::Path);
        assert_eq!(coarse_tile_class(0x17), TileClass::Path);
        assert_eq!(coarse_tile_class(0x18), TileClass::Wall);
        assert_eq!(coarse_tile_class(0x3F), TileClass::Wall);
        assert_eq!(coarse_tile_class(0x40), TileClass::Furniture);
        assert_eq!(coarse_tile_class(0x5F), TileClass::Furniture);
        assert_eq!(coarse_tile_class(0x60), TileClass::Door);
        assert_eq!(coarse_tile_class(0x67), TileClass::Door);
        assert_eq!(coarse_tile_class(0x68), TileClass::Decoration);
        assert_eq!(coarse_tile_class(0x6F), TileClass::Decoration);
        assert_eq!(coarse_tile_class(0x70), TileClass::Barrier);
        assert_eq!(coarse_tile_class(0x7F), TileClass::Barrier);
        assert_eq!(coarse_tile_class(0x80), TileClass::Special);
        assert_eq!(coarse_tile_class(0x9F), TileClass::Special);
        assert_eq!(coarse_tile_class(0xA0), TileClass::Vehicle);
        assert_eq!(coarse_tile_class(0xBB), TileClass::Vehicle);
        assert_eq!(coarse_tile_class(0xBC), TileClass::VehicleArt);
        assert_eq!(coarse_tile_class(0xBF), TileClass::VehicleArt);
        assert_eq!(coarse_tile_class(0xC0), TileClass::Npc);
        assert_eq!(coarse_tile_class(0xFF), TileClass::Npc);
    }

    #[test]
    fn classify_tlk_byte_partitions_dispatcher_table_per_section_seven() {
        // conversation.md §7 dispatcher classification order: 0x00 NUL,
        // 0x01..=0x7F dictionary, 0x9E..=0x9F GOTO label (precedes the
        // 0x80..=0x9F control band), 0x80..=0x9F control, 0xA0..=0xFD
        // printable, 0xFE IF-ELSE alias, 0xFF end-of-response.
        assert_eq!(classify_tlk_byte(0x00), TlkByteKind::Nul);
        assert_eq!(classify_tlk_byte(0x01), TlkByteKind::DictionaryToken);
        assert_eq!(classify_tlk_byte(0x7F), TlkByteKind::DictionaryToken);
        assert_eq!(classify_tlk_byte(0x80), TlkByteKind::ControlByte);
        assert_eq!(classify_tlk_byte(0x9D), TlkByteKind::ControlByte);
        assert_eq!(classify_tlk_byte(0x9E), TlkByteKind::GotoLabel);
        assert_eq!(classify_tlk_byte(0x9F), TlkByteKind::GotoLabel);
        assert_eq!(classify_tlk_byte(0xA0), TlkByteKind::PrintableText);
        assert_eq!(classify_tlk_byte(0xFD), TlkByteKind::PrintableText);
        assert_eq!(classify_tlk_byte(0xFE), TlkByteKind::IfElseAlias);
        assert_eq!(classify_tlk_byte(0xFF), TlkByteKind::EndOfResponse);
        // Spot-check the control codes resolve to ControlByte (not IfElseAlias).
        for code in [
            TLK_CODE_PRINT_AVATAR_NAME,
            TLK_CODE_GOLD_PAYMENT,
            TLK_CODE_ACTION_DISPATCH,
            TLK_CODE_IF_ELSE,
        ] {
            assert_eq!(classify_tlk_byte(code), TlkByteKind::ControlByte);
        }
    }

    #[test]
    fn tlk_label_byte_classifier_covers_section_seven_seven_range() {
        // conversation.md §7.7: label bytes 0x91..=0x9F, fifteen entries.
        for byte in TLK_LABEL_FIRST..=TLK_LABEL_LAST {
            assert!(is_tlk_label_byte(byte), "byte 0x{byte:02X} should be label");
        }
        assert!(!is_tlk_label_byte(0x90));
        assert!(!is_tlk_label_byte(0xA0));
        assert!(is_tlk_label_byte(TLK_CODE_GOTO_LABEL_FIRST));
        assert!(is_tlk_label_byte(TLK_CODE_GOTO_LABEL_LAST));
    }

    #[test]
    fn chargen_virtue_stat_deltas_match_spec_table() {
        // chargen.md §6: per-virtue (INT, DEX, STR) deltas table.
        let table: &[(ShrineVirtue, u8, u8, u8)] = &[
            (ShrineVirtue::Honesty, 2, 0, 0),
            (ShrineVirtue::Compassion, 0, 2, 0),
            (ShrineVirtue::Valor, 0, 0, 2),
            (ShrineVirtue::Justice, 1, 1, 0),
            (ShrineVirtue::Sacrifice, 0, 1, 1),
            (ShrineVirtue::Honor, 1, 0, 1),
            (ShrineVirtue::Spirituality, 1, 1, 1),
            (ShrineVirtue::Humility, 0, 0, 0),
        ];
        for (virtue, int, dex, str_) in table {
            let delta = chargen_virtue_stat_delta(*virtue);
            assert_eq!(
                (delta.intelligence, delta.dexterity, delta.strength),
                (*int, *dex, *str_),
                "virtue {} mismatch",
                virtue.name()
            );
        }
    }

    #[test]
    fn class_refreshed_mana_covers_default_branch_per_magic_md_section_eight() {
        // magic.md §8 Resurrection: Avatar (A), Mage (M), and the default
        // class branch receive mana equal to Intelligence; Bard (B)
        // receives half Intelligence.
        assert_eq!(class_refreshed_mana(b'A', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'M', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'B', 24), Some(12));
        // Default branch — every other class letter receives full INT.
        assert_eq!(class_refreshed_mana(b'F', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'P', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'R', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'T', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'D', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'S', 24), Some(24));
    }

    #[test]
    fn intro_story_art_placement_for_step_matches_published_table() {
        // intro.md §10: spot-check primary story-art placements at all
        // file-boundary transitions.
        let p = intro_story_art_placement_for_step(0).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 0, top_left_x: 0, top_left_y: 0 });
        let p = intro_story_art_placement_for_step(2).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 0, top_left_x: 136, top_left_y: 0 });
        let p = intro_story_art_placement_for_step(7).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 0, top_left_x: 0, top_left_y: 0 });
        let p = intro_story_art_placement_for_step(13).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 0, top_left_x: 176, top_left_y: 0 });
        let p = intro_story_art_placement_for_step(20).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 4, top_left_x: 0, top_left_y: 87 });
        assert!(intro_story_art_placement_for_step(21).is_none());
    }

    #[test]
    fn intro_story_art_file_for_step_matches_published_boundaries() {
        // intro.md §10: steps 0-1 STORY1, 2-6 STORY2, 7-8 STORY3, 9-10
        // STORY4, 11-12 STORY5, 13-20 STORY6.
        assert_eq!(intro_story_art_file_for_step(0), Some("STORY1.16"));
        assert_eq!(intro_story_art_file_for_step(1), Some("STORY1.16"));
        assert_eq!(intro_story_art_file_for_step(2), Some("STORY2.16"));
        assert_eq!(intro_story_art_file_for_step(6), Some("STORY2.16"));
        assert_eq!(intro_story_art_file_for_step(7), Some("STORY3.16"));
        assert_eq!(intro_story_art_file_for_step(8), Some("STORY3.16"));
        assert_eq!(intro_story_art_file_for_step(9), Some("STORY4.16"));
        assert_eq!(intro_story_art_file_for_step(10), Some("STORY4.16"));
        assert_eq!(intro_story_art_file_for_step(11), Some("STORY5.16"));
        assert_eq!(intro_story_art_file_for_step(12), Some("STORY5.16"));
        assert_eq!(intro_story_art_file_for_step(13), Some("STORY6.16"));
        assert_eq!(intro_story_art_file_for_step(20), Some("STORY6.16"));
        assert_eq!(intro_story_art_file_for_step(21), None);
        assert_eq!(INTRO_STORY_STEP_COUNT, 21);
        assert_eq!(INTRO_AUTO_OPENING_STEP, 0);
        assert_eq!(INTRO_INLINE_DOORWAY_STEP, 6);
    }

    #[test]
    fn chargen_questionnaire_round_structure_matches_spec_section_six() {
        // chargen.md §6: 3 rounds (4 + 2 + 1 = 7 questions), single-elim.
        assert_eq!(CHARGEN_QUESTION_COUNT, 7);
        assert_eq!(CHARGEN_ROUND_COUNT, 3);
        assert_eq!(CHARGEN_QUESTIONS_PER_ROUND, [4, 2, 1]);
        assert_eq!(
            CHARGEN_QUESTIONS_PER_ROUND.iter().sum::<usize>(),
            CHARGEN_QUESTION_COUNT
        );
    }

    #[test]
    fn npc_dynamic_obstacle_radius_matches_published_threshold() {
        // npc-schedules.md §10: occupied cells are blocked only when the
        // occupant is within Manhattan distance less than four from the
        // NPC's runtime destination.
        assert_eq!(NPC_DYNAMIC_OBSTACLE_MANHATTAN_RADIUS, 4);
    }

    #[test]
    fn npc_schedule_state_constants_match_published_state_machine() {
        // npc-schedules.md §7: 0=empty, 1=idle, 2=in-plane move, 3=replay
        // queue, 4=descend, 5=ascend, 6=climb up off, 7=climb down off,
        // 8=parked off-floor.
        assert_eq!(NPC_STATE_EMPTY, 0);
        assert_eq!(NPC_STATE_IDLE, 1);
        assert_eq!(NPC_STATE_INPLANE_MOVE, 2);
        assert_eq!(NPC_STATE_REPLAY_QUEUE, 3);
        assert_eq!(NPC_STATE_DESCEND_TOWARD_TARGET, 4);
        assert_eq!(NPC_STATE_ASCEND_TOWARD_TARGET, 5);
        assert_eq!(NPC_STATE_CLIMB_UP_OFF_FLOOR, 6);
        assert_eq!(NPC_STATE_CLIMB_DOWN_OFF_FLOOR, 7);
        assert_eq!(NPC_STATE_PARKED_OFF_FLOOR, 8);
        assert_eq!(NPC_STUCK_REPLAN_THRESHOLD, 3);
    }

    #[test]
    fn tile_blocks_sight_propagation_matches_spec_classifier() {
        // visibility.md §6: the sight-blocking spec list.
        for tile in [
            0x09u8, 0x0A, 0x0C, 0x0D, 0x4D, 0x4E, 0x4F, 0x5A, 0x97, 0xB8, 0xB9, 0xBC, 0xD0,
            0xD1, 0xD2, 0xD3, 0xF8, 0xFE, 0xFF,
        ] {
            assert!(
                tile_blocks_sight_propagation(tile),
                "tile 0x{tile:02X} should block sight"
            );
        }
        // Non-listed tiles use the ordinary propagation rule.
        for tile in [0x00u8, 0x05, 0x10, 0x4A, 0x4B, 0x98, 0xBA, 0xBB, 0xC0] {
            assert!(
                !tile_blocks_sight_propagation(tile),
                "tile 0x{tile:02X} should not block sight"
            );
        }
    }

    #[test]
    fn tile_propagates_sight_only_when_adjacent_lists_orthogonal_set() {
        // visibility.md §6 orthogonal-only group.
        for tile in [0x4Au8, 0x4B, 0x98, 0xBA, 0xBB] {
            assert!(tile_propagates_sight_only_when_adjacent(tile));
        }
        for tile in [0x09u8, 0x0A, 0x4D, 0x97, 0xB8] {
            assert!(!tile_propagates_sight_only_when_adjacent(tile));
        }
    }

    #[test]
    fn shop_time_of_day_word_partitions_24_hour_clock() {
        // shops.md §4.1: morning for hours 0..12, afternoon for 12..18,
        // evening for 18..24.
        for hour in 0..12u8 {
            assert_eq!(shop_time_of_day_word(hour), "morning");
        }
        for hour in 12..18u8 {
            assert_eq!(shop_time_of_day_word(hour), "afternoon");
        }
        for hour in 18..24u8 {
            assert_eq!(shop_time_of_day_word(hour), "evening");
        }
    }

    #[test]
    fn game_clock_display_hour_and_am_pm_suffix_match_spec() {
        // time.md §2: display hour is 12 when underlying hour is 0; the
        // hour itself when 1..=12; otherwise hour - 12. AM for 0..12, PM
        // otherwise.
        let clock_at = |hour: u8| GameClock::new(hour, 0).unwrap();
        assert_eq!(clock_at(0).display_hour(), 12);
        assert_eq!(clock_at(0).am_pm_suffix(), "A.M.");
        assert_eq!(clock_at(1).display_hour(), 1);
        assert_eq!(clock_at(11).am_pm_suffix(), "A.M.");
        assert_eq!(clock_at(12).display_hour(), 12);
        assert_eq!(clock_at(12).am_pm_suffix(), "P.M.");
        assert_eq!(clock_at(13).display_hour(), 1);
        assert_eq!(clock_at(23).display_hour(), 11);
        assert_eq!(clock_at(23).am_pm_suffix(), "P.M.");
    }

    #[test]
    fn shrine_virtue_companion_table_matches_karma_md_section_nine() {
        // karma.md §9: virtue-to-companion pairing.
        assert_eq!(ShrineVirtue::Honesty.companion(), ("Mariah", "Mage"));
        assert_eq!(ShrineVirtue::Compassion.companion(), ("Iolo", "Bard"));
        assert_eq!(ShrineVirtue::Valor.companion(), ("Geoffrey", "Fighter"));
        assert_eq!(ShrineVirtue::Justice.companion(), ("Jaana", "Druid"));
        assert_eq!(ShrineVirtue::Sacrifice.companion(), ("Julia", "Tinker"));
        assert_eq!(ShrineVirtue::Honor.companion(), ("Dupre", "Paladin"));
        assert_eq!(ShrineVirtue::Spirituality.companion(), ("Shamino", "Ranger"));
        assert_eq!(ShrineVirtue::Humility.companion(), ("Katrina", "Shepherd"));
    }

    #[test]
    fn read_codex_urn_walks_virtues_in_standard_order() {
        // karma.md §8: walk the eight virtues in standard order, stamp the
        // first ordained-and-not-yet-Codex-read virtue, return the chosen
        // virtue. Honesty is index 0 and so should be picked first when
        // ordained.
        let mut codex = 0u8;
        let outcome = read_codex_urn(
            ShrineVirtue::Honesty.bit() | ShrineVirtue::Justice.bit(),
            &mut codex,
        );
        assert_eq!(outcome, CodexUrnReadOutcome::Stamped(ShrineVirtue::Honesty));
        assert_eq!(codex, ShrineVirtue::Honesty.bit());

        // Second read with same ordained mask should pick Justice next
        // because Honesty's Codex-read bit is now set.
        let outcome = read_codex_urn(
            ShrineVirtue::Honesty.bit() | ShrineVirtue::Justice.bit(),
            &mut codex,
        );
        assert_eq!(outcome, CodexUrnReadOutcome::Stamped(ShrineVirtue::Justice));
        assert_eq!(codex, ShrineVirtue::Honesty.bit() | ShrineVirtue::Justice.bit());
    }

    #[test]
    fn read_codex_urn_returns_completed_when_all_codex_bits_set() {
        // karma.md §8: with all eight Codex-read bits set, the reader takes
        // its completed branch and the saved masks are unchanged.
        let mut codex = 0xFFu8;
        let outcome = read_codex_urn(0xFF, &mut codex);
        assert_eq!(outcome, CodexUrnReadOutcome::Completed);
        assert_eq!(codex, 0xFF);
    }

    #[test]
    fn read_codex_urn_no_ordained_branch_when_no_bits_set() {
        // §8: if no virtue is ordained, no virtue can be stamped.
        let mut codex = 0u8;
        let outcome = read_codex_urn(0, &mut codex);
        assert_eq!(outcome, CodexUrnReadOutcome::NoOrdained);
        assert_eq!(codex, 0);
    }

    #[test]
    fn town_tile_predicates_match_published_catalog_ranges() {
        // catalogs/tile-catalog.md §6: door 96..=103, stair 0xC4..=0xC7,
        // chair 0x8C, NPC floor-link markers 0xC8 and 0xC9.
        assert!(is_town_door_tile(96));
        assert!(is_town_door_tile(99));
        assert!(is_town_door_tile(103));
        assert!(!is_town_door_tile(95));
        assert!(!is_town_door_tile(104));

        assert!(is_town_stair_tile(0xC4));
        assert!(is_town_stair_tile(0xC7));
        assert!(!is_town_stair_tile(0xC3));
        assert!(!is_town_stair_tile(0xC8));

        assert!(is_npc_floor_link_tile(0xC8));
        assert!(is_npc_floor_link_tile(0xC9));
        assert!(!is_npc_floor_link_tile(0xC7));
        assert!(!is_npc_floor_link_tile(0xCA));

        assert_eq!(TOWN_CHAIR_TILE, 0x8C);
    }

    #[test]
    fn spell_damage_caps_and_kill_sentinel_match_spec_table() {
        // catalogs/spell-list.md §5: Magic Missile raw 1..16 (id 1),
        // Fireball raw 1..30 (id 13), Kill is single-target instant kill
        // (id 37). combat.md §11 fixes Fire Field raw at 1..21 and §12
        // names the instant-kill sentinel value 99.
        assert_eq!(SPELL_CODES[MAGIC_MISSILE_SPELL_INDEX], "GP");
        assert_eq!(MAGIC_MISSILE_RAW_DAMAGE_MAX, 16);
        assert_eq!(SPELL_CODES[FIREBALL_SPELL_INDEX], "FV");
        assert_eq!(FIREBALL_RAW_DAMAGE_MAX, 30);
        assert_eq!(SPELL_CODES[KILL_SPELL_INDEX], "CX");
        assert_eq!(FIRE_FIELD_RAW_DAMAGE_MAX, 21);
    }

    #[test]
    fn spell_mp_cost_matches_published_per_spell_cost_constants() {
        // combat.md §10 says cost = (id/6)+1. Cross-check the formula
        // against the named per-spell COST constants for several spells.
        assert_eq!(spell_mp_cost(IN_LOR_SPELL_INDEX), Some(IN_LOR_COST));
        assert_eq!(spell_mp_cost(AWAKEN_SPELL_INDEX), Some(AWAKEN_COST));
        assert_eq!(spell_mp_cost(CURE_SPELL_INDEX), Some(CURE_COST));
        assert_eq!(spell_mp_cost(HEAL_SPELL_INDEX), Some(HEAL_COST));
        assert_eq!(spell_mp_cost(REL_HUR_SPELL_INDEX), Some(REL_HUR_COST));
        assert_eq!(spell_mp_cost(IN_WIS_SPELL_INDEX), Some(IN_WIS_COST));
        assert_eq!(spell_mp_cost(CREATE_FOOD_SPELL_INDEX), Some(CREATE_FOOD_COST));
        assert_eq!(spell_mp_cost(VAS_LOR_SPELL_INDEX), Some(VAS_LOR_COST));
        assert_eq!(spell_mp_cost(BLINK_SPELL_INDEX), Some(BLINK_COST));
        assert_eq!(spell_mp_cost(PROTECTION_SPELL_INDEX), Some(PROTECTION_COST));
        assert_eq!(spell_mp_cost(GREAT_HEAL_SPELL_INDEX), Some(GREAT_HEAL_COST));
        assert_eq!(spell_mp_cost(QUICKNESS_SPELL_INDEX), Some(QUICKNESS_COST));
        assert_eq!(spell_mp_cost(MASS_CHARM_SPELL_INDEX), Some(MASS_CHARM_COST));
        assert_eq!(spell_mp_cost(NEGATE_MAGIC_SPELL_INDEX), Some(NEGATE_MAGIC_COST));
        assert_eq!(spell_mp_cost(PEER_SPELL_INDEX), Some(PEER_COST));
        assert_eq!(spell_mp_cost(RESURRECT_SPELL_INDEX), Some(RESURRECT_COST));
        assert_eq!(spell_mp_cost(GATE_TRAVEL_SPELL_INDEX), Some(GATE_TRAVEL_COST));
        assert_eq!(spell_mp_cost(TIME_STOP_SPELL_INDEX), Some(TIME_STOP_COST));
    }

    #[test]
    fn spell_mp_cost_follows_eight_circles_of_six_layout() {
        // combat.md §10: spell MP cost is (spell_id / 6) + 1.
        // Circle 0 (id 0..5) costs 1; circle 1 (6..11) costs 2; ...
        // circle 7 (42..47) costs 8.
        assert_eq!(spell_mp_cost(0), Some(1));
        assert_eq!(spell_mp_cost(5), Some(1));
        assert_eq!(spell_mp_cost(6), Some(2));
        assert_eq!(spell_mp_cost(11), Some(2));
        assert_eq!(spell_mp_cost(12), Some(3));
        assert_eq!(spell_mp_cost(47), Some(8));
        assert_eq!(spell_mp_cost(48), None);

        assert_eq!(spell_circle_index(0), Some(0));
        assert_eq!(spell_circle_index(5), Some(0));
        assert_eq!(spell_circle_index(47), Some(7));
        assert_eq!(spell_circle_index(48), None);
    }

    #[test]
    fn spell_scene_bit_for_scene_byte_matches_published_partition() {
        // catalogs/spell-list.md §4: scene-byte to single-bit mapping.
        // 0 -> overworld, 1..=32 -> indoor, 33..=127 -> dungeon, >=0x80 -> combat.
        assert_eq!(spell_scene_bit_for_scene_byte(0), SPELL_SCENE_OVERWORLD);
        for byte in 1..=32u8 {
            assert_eq!(
                spell_scene_bit_for_scene_byte(byte),
                SPELL_SCENE_INDOOR,
                "byte {byte} should be indoor"
            );
        }
        for byte in [33u8, 40, 100, 127] {
            assert_eq!(
                spell_scene_bit_for_scene_byte(byte),
                SPELL_SCENE_DUNGEON,
                "byte {byte} should be dungeon"
            );
        }
        for byte in [0x80u8, 0x90, 0xC0, 0xFF] {
            assert_eq!(
                spell_scene_bit_for_scene_byte(byte),
                SPELL_SCENE_COMBAT,
                "byte 0x{byte:02X} should be combat"
            );
        }
    }

    #[test]
    fn capped_add_u8_clamps_at_caller_supplied_cap() {
        // stat-arithmetic.md §2: byte capped add stores cap when the result
        // reaches or exceeds the cap; returns actual delta applied.
        let mut field = 90u8;
        let applied = capped_add_u8(&mut field, 5, 99);
        assert_eq!(field, 95);
        assert_eq!(applied, 5);
        let applied = capped_add_u8(&mut field, 10, 99);
        assert_eq!(field, 99);
        assert_eq!(applied, 4);
        let applied = capped_add_u8(&mut field, 50, 99);
        assert_eq!(field, 99);
        assert_eq!(applied, 0);
    }

    #[test]
    fn capped_add_word_uses_signed_comparison_and_returns_delta() {
        // §2: word capped add uses signed comparison; returns actual delta.
        let mut hp: i16 = 50;
        assert_eq!(capped_add_word(&mut hp, 30, 100), 30);
        assert_eq!(hp, 80);
        assert_eq!(capped_add_word(&mut hp, 50, 100), 20);
        assert_eq!(hp, 100);
        // Negative starting field still observes signed cap.
        let mut hp: i16 = -5;
        assert_eq!(capped_add_word(&mut hp, 10, 100), 10);
        assert_eq!(hp, 5);
    }

    #[test]
    fn floor_sub_u8_floors_at_zero_and_returns_actual_subtracted() {
        // §2: byte floor subtract stores zero when the current value is not
        // greater than the amount; returns actual subtracted.
        let mut field = 7u8;
        assert_eq!(floor_sub_u8(&mut field, 3), 3);
        assert_eq!(field, 4);
        assert_eq!(floor_sub_u8(&mut field, 10), 4);
        assert_eq!(field, 0);
        assert_eq!(floor_sub_u8(&mut field, 5), 0);
        assert_eq!(field, 0);
    }

    #[test]
    fn floor_sub_word_clamps_at_zero_in_signed_comparison() {
        // §2: word floor subtract floors at zero with signed comparison.
        let mut hp: i16 = 30;
        assert_eq!(floor_sub_word(&mut hp, 18), 18);
        assert_eq!(hp, 12);
        assert_eq!(floor_sub_word(&mut hp, 100), 12);
        assert_eq!(hp, 0);
        assert_eq!(floor_sub_word(&mut hp, 5), 0);
        assert_eq!(hp, 0);
    }

    #[test]
    fn directed_step_offsets_reduce_wrapped_distance_to_player() {
        // active-objects.md §8: per-axis one-cell step toward the player on
        // the 256-cell torus. Aligned axes return 0; non-wrapped distances
        // pick the obvious direction; wrapped distances pick the shorter way.

        // Same cell: no movement.
        assert_eq!(directed_step_offsets(10, 10, 10, 10), (0, 0));

        // Player one east + two south: step east first.
        assert_eq!(directed_step_offsets(10, 10, 11, 12), (1, 1));

        // Player west + north: negative steps.
        assert_eq!(directed_step_offsets(10, 10, 8, 5), (-1, -1));

        // Wraparound: actor at 250, player at 5 -> shorter forward (wrap).
        assert_eq!(directed_step_offsets(250, 0, 5, 0), (1, 0));
        // Symmetric: actor at 5, player at 250 -> shorter backward (wrap).
        assert_eq!(directed_step_offsets(5, 0, 250, 0), (-1, 0));

        // Equidistant tie (128 each way) prefers forward step.
        assert_eq!(directed_step_offsets(0, 0, 128, 0), (1, 0));
    }

    #[test]
    fn terrain_chance_gate_denominator_matches_spec_outdoor_table() {
        // active-objects.md §8: half-chance for 0x04, 0x06..=0x08,
        // 0x1E..=0x1F; third-chance for 0x09..=0x0F; no gate for everything
        // else in the outdoor mover range.
        assert_eq!(terrain_chance_gate_denominator(0x04), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x06), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x07), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x08), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x1E), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x1F), Some(2));
        for tile in 0x09..=0x0F {
            assert_eq!(terrain_chance_gate_denominator(tile), Some(3));
        }
        assert_eq!(terrain_chance_gate_denominator(0x05), None);
        assert_eq!(terrain_chance_gate_denominator(0x10), None);
        assert_eq!(terrain_chance_gate_denominator(0x1D), None);
        assert_eq!(terrain_chance_gate_denominator(0x20), None);
        assert_eq!(terrain_chance_gate_denominator(0x00), None);
        assert_eq!(terrain_chance_gate_denominator(0xFF), None);
    }

    #[test]
    fn type_bypasses_terrain_chance_gate_lists_water_creatures_and_named_monsters() {
        // active-objects.md §8: ship-like water-creature frames 0x2C..=0x2F
        // and Bat/Daemon/Dragon/Mongbat first-frame type bytes bypass the
        // chance gate.
        for byte in 0x2C..=0x2Fu8 {
            assert!(type_bypasses_terrain_chance_gate(byte));
        }
        assert!(type_bypasses_terrain_chance_gate(0x94));
        assert!(type_bypasses_terrain_chance_gate(0xD8));
        assert!(type_bypasses_terrain_chance_gate(0xDC));
        assert!(type_bypasses_terrain_chance_gate(0xF0));
        // Sibling frames are not part of the bypass set.
        assert!(!type_bypasses_terrain_chance_gate(0x95));
        assert!(!type_bypasses_terrain_chance_gate(0xD9));
        assert!(!type_bypasses_terrain_chance_gate(0xDD));
        assert!(!type_bypasses_terrain_chance_gate(0xF1));
        // Random other bytes are not in the bypass set.
        assert!(!type_bypasses_terrain_chance_gate(0x00));
        assert!(!type_bypasses_terrain_chance_gate(0x80));
    }

    #[test]
    fn axis_first_choice_picks_x_or_y_from_one_bit_roll() {
        // active-objects.md §8: a one-bit random value chooses which axis to
        // try first.
        assert_eq!(axis_first_choice(0), Axis::X);
        assert_eq!(axis_first_choice(2), Axis::X);
        assert_eq!(axis_first_choice(1), Axis::Y);
        assert_eq!(axis_first_choice(3), Axis::Y);
    }

    #[test]
    fn fc_sprite_proximity_mask_matches_spec_six_by_six_table() {
        // active-objects.md §8: `0xFC` sprite class proximity-mask table.
        // Listed cells enter the special branch; the rest fall through.
        let listed = [
            (0u8, 2u8),
            (0, 3),
            (0, 4),
            (1, 3),
            (1, 4),
            (2, 2),
            (2, 3),
            (3, 0),
            (3, 1),
            (3, 2),
            (3, 3),
            (4, 0),
            (4, 1),
        ];
        for (dy, dx) in listed {
            assert!(
                fc_sprite_proximity_mask_hits(dy, dx),
                "({dy},{dx}) should hit"
            );
        }
        // Spot-check non-listed cells from inside the half-window.
        for (dy, dx) in [(0u8, 0u8), (0, 1), (1, 0), (1, 1), (1, 2), (2, 0), (2, 1)] {
            assert!(
                !fc_sprite_proximity_mask_hits(dy, dx),
                "({dy},{dx}) should not hit"
            );
        }
        // Row 5 is entirely outside the special branch.
        for dx in 0..=5u8 {
            assert!(!fc_sprite_proximity_mask_hits(5, dx));
        }
        // Cells outside the 6x6 half-window also miss.
        assert!(!fc_sprite_proximity_mask_hits(6, 0));
        assert!(!fc_sprite_proximity_mask_hits(0, 5));
    }

    #[test]
    fn wrap_text_breaks_at_spaces_within_window_width() {
        // text-output.md §6: only space, LF, CR, and NUL are break bytes.
        // Subsequent lines use the full window width.
        let lines = wrap_text("the quick brown fox", 10, 0);
        assert_eq!(lines, vec!["the quick", "brown fox"]);
    }

    #[test]
    fn wrap_text_first_line_uses_remaining_width_after_cursor() {
        // §6: first emitted line uses `window_width - cursor_x_at_entry`.
        let lines = wrap_text("hello world", 10, 5);
        // First line has 5 cells available, "hello" fits but "hello world"
        // doesn't, so wrap before "world".
        assert_eq!(lines, vec!["hello", "world"]);
    }

    #[test]
    fn wrap_text_terminates_on_nul_and_handles_hard_newlines() {
        // §6: NUL stops reading; LF/CR force a line emit.
        let lines = wrap_text("line one\nline two\0HIDDEN", 40, 0);
        assert_eq!(lines, vec!["line one", "line two"]);
    }

    #[test]
    fn tile_view_class_matches_spec_lookup_table() {
        // systems/view.md §4: per-tile view class lookup. Spot-check
        // representative tiles from each class plus boundary cases.
        // Class 0 (empty/pass-through)
        assert_eq!(tile_view_class(0x00), 0);
        assert_eq!(tile_view_class(0xC0), 0);
        assert_eq!(tile_view_class(0xCF), 0);
        assert_eq!(tile_view_class(0xFF), 0);
        // Class 1
        assert_eq!(tile_view_class(0x05), 1);
        assert_eq!(tile_view_class(0x30), 1);
        assert_eq!(tile_view_class(0x37), 1);
        // Class 2
        assert_eq!(tile_view_class(0x09), 2);
        assert_eq!(tile_view_class(0x2D), 2);
        // Class 3
        assert_eq!(tile_view_class(0x70), 3);
        assert_eq!(tile_view_class(0x7F), 3);
        assert_eq!(tile_view_class(0x44), 3);
        assert_eq!(tile_view_class(0xDD), 3);
        // Class 4
        assert_eq!(tile_view_class(0x5C), 4);
        assert_eq!(tile_view_class(0xBE), 4);
        // Class 5
        assert_eq!(tile_view_class(0x10), 5);
        assert_eq!(tile_view_class(0x1B), 5);
        assert_eq!(tile_view_class(0x4C), 5);
        assert_eq!(tile_view_class(0xFA), 5);
        // Class 6
        assert_eq!(tile_view_class(0xEC), 6);
        assert_eq!(tile_view_class(0xF9), 6);
        assert_eq!(tile_view_class(0xB8), 6);
        // Class 7
        assert_eq!(tile_view_class(0x4D), 7);
        assert_eq!(tile_view_class(0xFE), 7);
        // Class 8
        assert_eq!(tile_view_class(0x0B), 8);
        assert_eq!(tile_view_class(0x0F), 8);
        // Class 9
        assert_eq!(tile_view_class(0x06), 9);
        assert_eq!(tile_view_class(0x2C), 9);
        // Class A
        assert_eq!(tile_view_class(0x60), 0x0A);
        assert_eq!(tile_view_class(0x69), 0x0A);
        // Class B
        assert_eq!(tile_view_class(0xD4), 0x0B);
        assert_eq!(tile_view_class(0xD7), 0x0B);
        // Class C
        assert_eq!(tile_view_class(0x01), 0x0C);
        // Class D
        assert_eq!(tile_view_class(0x04), 0x0D);
        // Class E
        assert_eq!(tile_view_class(0xE0), 0x0E);
        assert_eq!(tile_view_class(0xE3), 0x0E);
        // Class F
        assert_eq!(tile_view_class(0xD8), 0x0F);
        assert_eq!(tile_view_class(0xDC), 0x0F);
        // Class 0x10
        assert_eq!(tile_view_class(0x20), 0x10);
        assert_eq!(tile_view_class(0x26), 0x10);
    }

    #[test]
    fn decode_end_window_strips_layout_markers_and_terminates_on_nul() {
        // formats/end-dat.md §3: `{` paragraph marker and `_` soft hyphen
        // are layout hints; NUL terminates the rendered output.
        let bytes = b"{Avatar_Standing\nat_the_circle\0HIDDEN";
        assert_eq!(decode_end_window(bytes), "AvatarStanding\natthecircle");
    }

    #[test]
    fn end_narrative_window_returns_decoded_subslice() {
        let raw = b"{Hello\nWorld\0".to_vec();
        let narrative = EndNarrative { raw };
        assert_eq!(narrative.full_text(), "Hello\nWorld");
        assert_eq!(narrative.window(1, 6).as_deref(), Some("Hello"));
        // Out-of-range window returns None per spec §5.
        assert!(narrative.window(0, 999).is_none());
    }

    #[test]
    fn parse_story_records_walks_twenty_records_and_strips_markup() {
        // formats/story-dat.md §2-§3: 20 NUL-terminated records driving the
        // intro story sequence; `{` and `_` are layout markup.
        let mut bytes = Vec::new();
        for index in 0..20usize {
            bytes.push(b'{');
            bytes.extend_from_slice(format!("Page{index}_break").as_bytes());
            bytes.push(0x00);
        }
        bytes.push(0x00); // Empty trailer per §2.

        let records = parse_story_records(&bytes).expect("20 records should parse");

        assert_eq!(records.records.len(), 20);
        assert_eq!(records.record(0), Some("Page0break"));
        assert_eq!(records.record(19), Some("Page19break"));
        assert_eq!(records.record(20), None);
    }

    #[test]
    fn parse_story_records_rejects_short_input() {
        let mut bytes = Vec::new();
        for _ in 0..5usize {
            bytes.extend_from_slice(b"x\0");
        }
        assert!(parse_story_records(&bytes).is_err());
    }

    #[test]
    fn parse_question_records_walks_thirty_records_and_strips_markup() {
        // formats/question-dat.md §2-§3: 30 NUL-terminated records;
        // record 0 = gypsy arrival, 1 = gypsy invitation, 2..=29 = dilemmas.
        // `{` is a paragraph marker and `_` is a soft hyphen; both stripped.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"{Arrival_text");
        bytes.push(0x00);
        bytes.extend_from_slice(b"Invitation");
        bytes.push(0x00);
        for _ in 2..30usize {
            bytes.extend_from_slice(b"Dilemma");
            bytes.push(0x00);
        }

        let records = parse_question_records(&bytes).expect("30 records should parse");

        assert_eq!(records.records.len(), 30);
        assert_eq!(records.gypsy_arrival(), Some("Arrivaltext"));
        assert_eq!(records.gypsy_invitation(), Some("Invitation"));
        // Dilemma records start at ordinal 2.
        assert_eq!(records.dilemma(2), Some("Dilemma"));
        assert_eq!(records.dilemma(29), Some("Dilemma"));
        assert_eq!(records.dilemmas().len(), 28);
    }

    #[test]
    fn parse_question_records_rejects_short_input() {
        // §7: fewer than 30 records is a bad asset.
        let mut bytes = Vec::new();
        for _ in 0..10usize {
            bytes.extend_from_slice(b"x\0");
        }
        assert!(parse_question_records(&bytes).is_err());
    }

    #[test]
    fn chargen_question_record_for_pair_matches_spec_table() {
        // formats/question-dat.md §4: spec lists records 2..=29 mapped to
        // virtue pairs. Spot-check several published rows.
        use ShrineVirtue::*;
        assert_eq!(
            chargen_question_record_for_pair(Honesty, Compassion).unwrap(),
            2
        );
        assert_eq!(
            chargen_question_record_for_pair(Honesty, Humility).unwrap(),
            8
        );
        assert_eq!(
            chargen_question_record_for_pair(Compassion, Valor).unwrap(),
            9
        );
        assert_eq!(
            chargen_question_record_for_pair(Valor, Justice).unwrap(),
            15
        );
        assert_eq!(
            chargen_question_record_for_pair(Spirituality, Humility).unwrap(),
            29
        );
        // Symmetric pair (b, a) returns the same record.
        assert_eq!(
            chargen_question_record_for_pair(Humility, Spirituality).unwrap(),
            29
        );
    }

    #[test]
    fn parse_misc_messages_clusters_records_by_consumer() {
        // formats/miscmsg-dat.md §2-§3: 47 NUL-terminated records grouped as
        // 0-11 Blackthorn audience, 12-19 virtue failing text, 20-27 virtue
        // aphorism, 28-35 shrine meditation, 36-46 urn/Codex prophecy.
        let mut bytes = Vec::new();
        for index in 0..47usize {
            let label = format!("rec{index}");
            bytes.extend_from_slice(label.as_bytes());
            bytes.push(0x00);
        }

        let messages = parse_misc_messages(&bytes).expect("47 records should parse");

        assert_eq!(messages.records.len(), 47);
        assert_eq!(messages.blackthorn_audience().len(), 12);
        assert_eq!(messages.virtue_failing_text().len(), 8);
        assert_eq!(messages.virtue_aphorism().len(), 8);
        assert_eq!(messages.shrine_meditation().len(), 8);
        assert_eq!(messages.urn_codex().len(), 11);
        assert_eq!(messages.record(0), Some("rec0"));
        assert_eq!(messages.record(12), Some("rec12"));
        assert_eq!(messages.record(46), Some("rec46"));
        assert_eq!(messages.record(47), None);
    }

    #[test]
    fn parse_misc_messages_preserves_codex_tile_glyph_bytes() {
        // formats/miscmsg-dat.md §4: Codex tile-glyph bytes (`@`, `[`, `]`,
        // `_`) pass through unchanged for the caller to render through the
        // tile-glyph path.
        let mut bytes = Vec::new();
        for _ in 0..36usize {
            bytes.push(b'a');
            bytes.push(0x00);
        }
        bytes.extend_from_slice(b"TRU[");
        bytes.push(0x00);
        for _ in 37..47usize {
            bytes.push(b'b');
            bytes.push(0x00);
        }

        let messages = parse_misc_messages(&bytes).expect("47 records should parse");
        assert_eq!(messages.record(36), Some("TRU["));
    }

    #[test]
    fn parse_misc_messages_rejects_truncated_or_short_input() {
        // §6: missing terminators and short record counts must be rejected.
        let mut short = Vec::new();
        for _ in 0..10usize {
            short.extend_from_slice(b"x\0");
        }
        assert!(parse_misc_messages(&short).is_err());

        let unterminated = b"hello".to_vec();
        assert!(parse_misc_messages(&unterminated).is_err());
    }

    #[test]
    fn parse_endgame_messages_walks_eleven_nul_terminated_records() {
        // formats/endmsg-dat.md §2-§4: eleven NUL-terminated plain-ASCII
        // records consumed by the endgame Lord British dialogue.
        let labels = [
            "Greetings",
            "First box prompt",
            "Second box prompt",
            "Rite 1",
            "Rite 2",
            "Rite 3",
            "Rite 4",
            "Rite 5",
            "Rite 6",
            "Rite 7",
            "Refusal branch",
        ];
        let mut bytes = Vec::new();
        for label in labels {
            bytes.extend_from_slice(label.as_bytes());
            bytes.push(0x00);
        }

        let messages = parse_endgame_messages(&bytes).expect("11 records should parse");

        assert_eq!(messages.records.len(), 11);
        assert_eq!(messages.initial_greeting(), Some("Greetings"));
        assert_eq!(messages.first_box_prompt(), Some("First box prompt"));
        assert_eq!(messages.second_box_prompt(), Some("Second box prompt"));
        assert_eq!(messages.rite_messages().len(), 7);
        assert_eq!(messages.refusal_branch(), Some("Refusal branch"));
    }

    #[test]
    fn parse_endgame_messages_rejects_unterminated_record() {
        // §5: a missing NUL terminator must be rejected as a bad asset.
        let mut bytes = b"Hello\0World".to_vec();
        // 'World' is not NUL-terminated; parser should error.
        assert!(parse_endgame_messages(&bytes).is_err());

        // Also reject when fewer than 11 records.
        bytes = b"only one record\0".to_vec();
        assert!(parse_endgame_messages(&bytes).is_err());
    }

    #[test]
    fn parse_sign_records_decodes_directory_and_payload() {
        // formats/signs-dat.md §2-§4. Build a minimal SIGNS.DAT image with
        // two scene blocks separated by a zero-scene sentinel. Scene 17 has
        // one record at (0, 5, 6); scene 18 has one record at (1, 7, 8)
        // using divider/decoration glyphs.
        let mut bytes = vec![0u8; 33 * 2];
        let scene17_offset = 66u16;
        bytes[17 * 2..17 * 2 + 2].copy_from_slice(&scene17_offset.to_le_bytes());
        // Scene 17 record + payload + NUL + sentinel = 4 + 5 + 1 + 1 = 11
        let scene18_offset = scene17_offset + 4 + 5 + 1 + 1;
        bytes[18 * 2..18 * 2 + 2].copy_from_slice(&scene18_offset.to_le_bytes());
        // Scene 17 block.
        bytes.extend_from_slice(&[17, 0, 5, 6]);
        bytes.extend_from_slice(b"Hello");
        bytes.push(0x00);
        bytes.push(0x00); // end-of-block sentinel
        // Scene 18 block.
        bytes.extend_from_slice(&[18, 1, 7, 8]);
        bytes.extend_from_slice(&[b'A', 0x26, b'B', 0x29, b'C']);
        bytes.push(0x00);
        bytes.push(0x00); // end-of-block sentinel

        let records = parse_sign_records(&bytes).expect("parse should succeed");
        assert_eq!(records.len(), 2);

        let lookup_17 = find_sign(&records, 17, 0, 5, 6).expect("scene 17 record present");
        assert_eq!(lookup_17.body, "Hello");

        let lookup_18 = find_sign(&records, 18, 1, 7, 8).expect("scene 18 record present");
        assert_eq!(lookup_18.body, "A-B*C");

        // No matching record returns None.
        assert!(find_sign(&records, 17, 1, 1, 1).is_none());
    }

    #[test]
    fn parse_sign_records_rejects_short_directory() {
        // Less than the 66-byte scene directory must error per §2 of the format spec.
        assert!(parse_sign_records(&[0u8; 10]).is_err());
    }

    #[test]
    fn decode_sign_payload_handles_pause_and_high_bit() {
        // §4: 0x0D becomes a newline; high-bit text still prints as the
        // low-seven-bit character.
        let bytes = [b'A', 0x0d, b'B' | 0x80, b'C'];
        assert_eq!(decode_sign_payload(&bytes), "A\nBC");
    }

    #[test]
    fn sky_strip_marker_position_matches_spec_visibility_table() {
        // moons.md §2: Fixed hour marker visible 06:00..17:59 at cell `17 -
        // hour`. Trammel visible 00:00..08:59 at `8 - hour` and 21:00..23:59
        // at `32 - hour`. Felucca visible 00:00..02:59 at `2 - hour` and
        // 15:00..23:59 at `26 - hour`. All other hours are below the horizon.

        // Fixed hour marker boundaries.
        assert_eq!(sky_strip_marker_position(5, SkyStripMarker::FixedHour), None);
        assert_eq!(
            sky_strip_marker_position(6, SkyStripMarker::FixedHour),
            Some(11)
        );
        assert_eq!(
            sky_strip_marker_position(12, SkyStripMarker::FixedHour),
            Some(5)
        );
        assert_eq!(
            sky_strip_marker_position(17, SkyStripMarker::FixedHour),
            Some(0)
        );
        assert_eq!(sky_strip_marker_position(18, SkyStripMarker::FixedHour), None);

        // Trammel windows.
        assert_eq!(
            sky_strip_marker_position(0, SkyStripMarker::Trammel),
            Some(8)
        );
        assert_eq!(
            sky_strip_marker_position(8, SkyStripMarker::Trammel),
            Some(0)
        );
        // Hour 9..20 inclusive is below horizon.
        assert_eq!(sky_strip_marker_position(9, SkyStripMarker::Trammel), None);
        assert_eq!(sky_strip_marker_position(20, SkyStripMarker::Trammel), None);
        assert_eq!(
            sky_strip_marker_position(21, SkyStripMarker::Trammel),
            Some(11)
        );
        assert_eq!(
            sky_strip_marker_position(23, SkyStripMarker::Trammel),
            Some(9)
        );

        // Felucca windows.
        assert_eq!(
            sky_strip_marker_position(0, SkyStripMarker::Felucca),
            Some(2)
        );
        assert_eq!(
            sky_strip_marker_position(2, SkyStripMarker::Felucca),
            Some(0)
        );
        assert_eq!(sky_strip_marker_position(3, SkyStripMarker::Felucca), None);
        assert_eq!(sky_strip_marker_position(14, SkyStripMarker::Felucca), None);
        assert_eq!(
            sky_strip_marker_position(15, SkyStripMarker::Felucca),
            Some(11)
        );
        assert_eq!(
            sky_strip_marker_position(23, SkyStripMarker::Felucca),
            Some(3)
        );
    }

    #[test]
    fn endgame_step_toward_target_prefers_axis_with_greater_distance() {
        // endgame.md §7: each call moves one cell toward target along the axis
        // with the greater remaining distance.
        // Pure horizontal: dx > 0, dy = 0
        assert_eq!(endgame_step_toward_target((0, 5), (3, 5)), (1, 5));
        // Pure vertical: dx = 0, dy < 0
        assert_eq!(endgame_step_toward_target((4, 5), (4, 1)), (4, 4));
        // Diagonal with greater dx
        assert_eq!(endgame_step_toward_target((0, 0), (5, 2)), (1, 0));
        // Diagonal with greater dy
        assert_eq!(endgame_step_toward_target((0, 0), (2, 5)), (0, 1));
        // Negative directions
        assert_eq!(endgame_step_toward_target((10, 10), (3, 7)), (9, 10));
        assert_eq!(endgame_step_toward_target((10, 10), (8, 3)), (10, 9));
        // On target: no movement
        assert_eq!(endgame_step_toward_target((4, 4), (4, 4)), (4, 4));
        // Equal-distance ties: prefer X axis
        assert_eq!(endgame_step_toward_target((0, 0), (3, 3)), (1, 0));
    }

    #[test]
    fn endgame_certificate_word_helpers_cover_calendar_range() {
        assert_eq!(endgame_ordinal_word(1).as_deref(), Some("first"));
        assert_eq!(endgame_ordinal_word(21).as_deref(), Some("twenty-first"));
        assert_eq!(endgame_ordinal_word(28).as_deref(), Some("twenty-eighth"));
        assert_eq!(endgame_ordinal_word(29), None);
        assert_eq!(endgame_cardinal_word(139), "one hundred thirty-nine");
        assert_eq!(endgame_cardinal_word(141), "one hundred forty-one");
        assert_eq!(
            endgame_cardinal_word(2026),
            "two thousand twenty-six"
        );
    }

    #[test]
    fn dungeon_current_room_helper_state_fires_before_next_key_without_rewriting() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xa4;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('w', Path::new("")).unwrap());

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xa4);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("slot 4"));
        assert!(state.message.contains("arena 4"));
    }

    #[test]
    fn dungeon_room_helper_state_reports_arena_without_rewriting() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xa4;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa4);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("room-helper state slot 4"));
        assert!(state.message.contains("arena 4"));
    }

    #[test]
    fn dungeon_movement_rejects_diagonals_and_wraps_bounds() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 0, 0);

        assert_eq!(state.step(Direction::NorthWest), MoveOutcome::Blocked);
        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.turn, 0);

        assert_eq!(state.step(Direction::North), MoveOutcome::Moved);
        assert_eq!((state.player.x, state.player.y), (0, 7));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Moved North to (0, 7)"));
    }

    #[test]
    fn dungeon_movement_blocks_active_monster_cell_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
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
        assert_eq!(state.message, "Blocked!");
    }

    #[test]
    fn dungeon_play_keys_use_facing_relative_forward_and_back() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;

        assert!(state.handle_dungeon_key('w', Path::new("")).unwrap());
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 1);

        assert!(state.handle_dungeon_key('s', Path::new("")).unwrap());
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 2);
    }

    #[test]
    fn dungeon_play_keys_turn_without_changing_cell() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;

        assert!(state.handle_dungeon_key('a', Path::new("")).unwrap());
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.facing, Direction::North);
        assert_eq!(state.turn, 1);

        assert!(state.handle_dungeon_key('d', Path::new("")).unwrap());
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 2);
    }

    #[test]
    fn dungeon_l_key_looks_instead_of_turning() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x61;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;

        assert!(state.handle_dungeon_key('l', Path::new("")).unwrap());

        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 0);
        assert_eq!(state.look_dungeon(), MoveOutcome::Observed);
        assert!(state.message.contains("passage"));
    }

    #[test]
    fn dungeon_talk_reports_no_response_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert!(state.handle_dungeon_key('T', Path::new("")).unwrap());

        assert_eq!(state.message, "Funny, no response!");
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_i_key_ignites_and_reveals_forward_view() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.torches = 2;

        assert!(state.handle_dungeon_key('I', Path::new("")).unwrap());

        assert_eq!(state.torches, 1);
        assert!((112..=127).contains(&state.torch_counter));
        assert_eq!(state.turn, 1);
        let view = state.render_text_view(5);
        assert!(view.contains("First-person dungeon view"));
        assert!(!view.contains("darkness"));
    }

    #[test]
    fn dungeon_o_key_routes_to_underfoot_open() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('O', Path::new("")).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7b);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Opened dungeon chest"));
    }

    #[test]
    fn dungeon_v_key_routes_to_gem_map() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.gems = 1;

        assert!(state.handle_dungeon_key('v', Path::new("")).unwrap());

        assert_eq!(state.gems, 0);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Dungeon view"));
        assert!(state.message.contains("centered flood map"));
    }

    #[test]
    fn dungeon_attack_uses_forward_wrapped_probe_without_direction_prompt() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 0, 1);
        state.player.facing = Direction::West;

        assert!(state.handle_dungeon_key('A', Path::new("")).unwrap());

        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Attacked forward at (7, 1)"));
        assert!(state.message.contains("no target"));
    }

    #[test]
    fn dungeon_attack_forward_monster_clears_active_object_and_consumes_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert!(state.handle_dungeon_key('A', Path::new("")).unwrap());

        assert_eq!(state.turn, 1);
        assert!(state.active_objects[1].is_empty());
        assert!(state.message.contains("Attacked dungeon monster tile 192"));
        assert!(state.message.contains("dungeon combat resolution is pending"));
    }

    #[test]
    fn top_down_uppercase_command_letters_preempt_vi_movement() {
        for (key, expected) in [
            ('A', "Attack where?"),
            ('C', "Cast what?"),
            ('D', "What?"),
            ('M', "Mix what?"),
            ('N', "New order?"),
            ('Q', "Save game?"),
            ('U', "Use what?"),
            ('W', "What?"),
            ('Z', "Z-stats:"),
        ] {
            let mut state = test_state(open_grid(), 5, 5);

            assert!(
                state
                    .handle_top_down_key_with_inline(key, Path::new(""), None, None, None, None)
                    .unwrap()
            );

            assert_eq!((state.player.x, state.player.y), (5, 5));
            assert_eq!(state.turn, 0);
            assert!(
                state.message.contains(expected),
                "{key} reported `{}`",
                state.message
            );
        }
    }

    #[test]
    fn top_down_lowercase_vi_and_wasd_movement_still_routes_before_commands() {
        for (key, expected_position) in [
            ('y', (4, 4)),
            ('w', (5, 4)),
            ('u', (6, 4)),
            ('a', (4, 5)),
            ('d', (6, 5)),
            ('b', (4, 6)),
            ('s', (5, 6)),
            ('n', (6, 6)),
            ('c', (6, 6)),
            ('z', (4, 6)),
        ] {
            let mut state = test_state(open_grid(), 5, 5);

            assert!(
                state
                    .handle_top_down_key_with_inline(key, Path::new(""), None, None, None, None)
                    .unwrap()
            );

            assert_eq!(
                (state.player.x, state.player.y),
                expected_position,
                "{key} routed to `{}`",
                state.message
            );
            assert_eq!(state.turn, 1);
        }
    }

    #[test]
    fn top_down_lowercase_x_routes_to_vehicle_exit() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.player.transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        state.sync_player_object();

        assert!(
            state
                .handle_top_down_key_with_inline('x', Path::new(""), None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!((state.player.x, state.player.y), (6, 5));
        assert!(state.active_objects.iter().skip(1).any(|object| {
            object.type_byte == 184
                && object.tile == 184
                && object.x == 5
                && object.y == 5
                && object.z == WorldPlane::Underworld.save_floor()
        }));
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "carpet!");
    }

    #[test]
    fn town_enter_uses_stock_refusal_without_turn() {
        let mut state = test_state(open_grid(), 5, 5);

        assert!(
            state
                .handle_top_down_key_with_inline('E', Path::new(""), None, None, None, None)
                .unwrap()
        );

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn dungeon_turn_does_not_animate_top_down_active_objects() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 3,
            y: 3,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);

        let object = state.active_objects[1];
        assert_eq!(object.phase, 0x22);
        assert_eq!(object.tile, 192);
        assert_eq!((object.x, object.y), (3, 3));
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
    }

    #[test]
    fn dungeon_post_turn_active_monster_greedy_steps_toward_party() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 3,
            y: 1,
            z: 0,
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.pass_turn_with_game_dir(Some(Path::new(""))).unwrap(),
            MoveOutcome::Passed
        );

        let object = state.active_objects[1];
        assert_eq!((object.x, object.y), (2, 1));
        assert_eq!(object.phase, active_object_phase_from_direction(Direction::West, 0));
        assert!(state.message.contains("Dungeon monster tile 192 moved West to (2, 1)"));
    }

    #[test]
    fn dungeon_post_turn_active_monster_rejects_sleep_field_step() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x80;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 3,
            y: 1,
            z: 0,
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.pass_turn_with_game_dir(Some(Path::new(""))).unwrap(),
            MoveOutcome::Passed
        );

        let object = state.active_objects[1];
        assert_eq!((object.x, object.y), (3, 1));
        assert!(!state.message.contains("Dungeon monster tile 192 moved"));
    }

    #[test]
    fn dungeon_post_turn_active_monster_contact_faces_threat_and_consumes_monster() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::North;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 0,
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.pass_turn_with_game_dir(Some(Path::new(""))).unwrap(),
            MoveOutcome::Used
        );

        assert_eq!(state.player.facing, Direction::East);
        assert!(state.active_objects[1].is_empty());
        assert!(state.message.contains("approaches from the East"));
        assert!(state.message.contains("dungeon combat resolution is pending"));
    }

    #[test]
    fn dungeon_idle_tick_does_not_animate_top_down_active_objects() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 3,
            y: 3,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        let object = state.active_objects[1];
        assert_eq!(object.phase, 0x22);
        assert_eq!(object.tile, 192);
        assert_eq!((object.x, object.y), (3, 3));
        assert_eq!(state.animation.frame, 1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
    }

    #[test]
    fn dungeon_mode_refuses_world_vehicle_and_entry_letters_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 0,
            skiffs: 0,
        };

        for (key, expected) in [('B', "Not here!"), ('E', "Not here!"), ('X', "Not here!")] {
            assert!(state.handle_dungeon_key(key, Path::new("")).unwrap());
            assert_eq!(state.message, expected);
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
            assert_eq!((state.player.x, state.player.y), (1, 1));
            assert_eq!(state.turn, 0);
        }

        for key in ['F', 'P'] {
            assert!(state.handle_dungeon_key(key, Path::new("")).unwrap());
            assert_eq!(state.message, "What?");
            assert_eq!(state.turn, 0);
        }

        assert!(state.handle_dungeon_key('Q', Path::new("")).unwrap());
        assert_eq!(
            state.message,
            "Exit to DOS? Use QY to exit or QN to cancel."
        );
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_q_exit_prompt_is_separate_from_save_command() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(33, 0, 1, 1);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join("SAVED.GAM"), &template).unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert_eq!(
            handle_play_key_input(&mut state, 'Q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            state.message,
            "Exit to DOS? Use QY to exit or QN to cancel."
        );
        assert_eq!(state.turn, 0);
        assert!(!dir.join("SAVED.OOL").exists());

        assert_eq!(
            handle_play_key_input(&mut state, 'Q', "N", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "No.");
        assert_eq!(state.turn, 0);
        assert!(!dir.join("SAVED.OOL").exists());

        assert_eq!(
            handle_play_key_input(&mut state, 'Q', "Y", &dir).unwrap(),
            PlayInputDisposition::Quit
        );
        assert_eq!(state.message, "Yes. Exiting to DOS.");
        assert_eq!(state.turn, 0);
        assert!(!dir.join("SAVED.OOL").exists());
        assert_eq!(fs::read(dir.join("SAVED.GAM")).unwrap(), template);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_command_letters_do_not_fall_through_to_diagonal_movement_refusal() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        for (key, expected) in [
            ('C', "Cast what?"),
            ('M', "Mix what?"),
            ('N', "New order?"),
            ('R', "Ready what?"),
            ('U', "Use what?"),
            ('Y', "Yell what?"),
            ('Z', "Z-stats:"),
        ] {
            assert!(state.handle_dungeon_key(key, Path::new("")).unwrap());
            assert!(
                state.message.contains(expected),
                "{key} reported `{}`",
                state.message
            );
            assert_eq!((state.player.x, state.player.y), (1, 1));
            assert_eq!(state.turn, 0);
        }

        for key in ['7', '9', '1', '3'] {
            assert!(state.handle_dungeon_key(key, Path::new("")).unwrap());
            assert!(state.message.contains("forward, back, and turns only"));
            assert_eq!((state.player.x, state.player.y), (1, 1));
            assert_eq!(state.turn, 0);
        }
    }

