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

