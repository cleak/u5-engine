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

