    #[test]
    fn world_waterfall_sidecar_does_not_preempt_transport_passability() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_WATERFALL_TABLE_FILE),
            "UNDERWORLD 1 0 EAST 1 5\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 0)] = 1;
        let mut state = world_state(grid, 0, 0);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 20,
            skiffs: 1,
        };
        state.sync_player_object();

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Blocked"));
        assert!(!state.message.contains("waterfall swept"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn carpet_crosses_clean_lava_sidecar_and_damages_living_party() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 1 0 LAVA 14\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 0)] = 14;
        let mut state = world_state(grid, 0, 0);
        state.player.transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        state.sync_player_object();
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 12,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'D',
                climb_stat: 30,
                mana: 8,
                hp: 9,
                max_hp: 20,
                level: 8,
            },
        ];

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(state.party[0].hp < 12);
        assert_eq!(state.party[1].hp, 9);
        assert!(state.message.contains("lava damage"));
        assert!(state.message.contains("party slot 0"));
        assert!(!state.message.contains("party slot 1"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn foot_steps_on_native_molten_lava_and_takes_damage() {
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 0)] = 0x8f;
        let mut state = world_state(grid, 0, 0);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 12,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'D',
                climb_stat: 30,
                mana: 8,
                hp: 9,
                max_hp: 20,
                level: 8,
            },
        ];

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(state.party[0].hp < 12);
        assert_eq!(state.party[1].hp, 9);
        assert!(state.message.contains("lava damage"));
        assert!(state.message.contains("party slot 0"));
        assert!(!state.message.contains("party slot 1"));
    }

    #[test]
    fn native_molten_lava_blocks_horse_without_spending_turn() {
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 0)] = 0x8f;
        let mut state = world_state(grid, 0, 0);
        state.player.transport = TransportState::Horse {
            type_byte: 18,
            tile: 18,
        };
        state.sync_player_object();

        assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);

        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.turn, 0);
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP);
        assert_eq!(state.message, "Blocked!");
    }

    #[test]
    fn foot_enters_clean_drowning_water_sidecar_and_takes_damage() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 1 0 DROWNING 1\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 12,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'D',
                climb_stat: 30,
                mana: 8,
                hp: 9,
                max_hp: 20,
                level: 8,
            },
        ];

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(state.party[0].hp < 12);
        assert_eq!(state.party[1].hp, 9);
        assert!(state.message.contains("drowning damage"));
        assert!(state.message.contains("party slot 0"));
        assert!(!state.message.contains("party slot 1"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_clean_drowning_water_sidecar_damages_foot() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 1 0 WATER 1\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![1; WORLD_CELLS], 1, 0);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: 30,
            mana: 8,
            hp: 12,
            max_hp: 20,
            level: 8,
        }];

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 1);
        assert!(state.party[0].hp < 12);
        assert!(state.message.contains("drowning damage"));
        assert!(state.message.contains("party slot 0"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_narrative_gate_without_ordained_mask_moves_party_south_after_world_step() {
        let dir = debug_game_dir();
        let mut state = britannia_state(
            open_world_grid(),
            NARRATIVE_GATE_X as usize - 1,
            NARRATIVE_GATE_Y as usize,
        );

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(
            (state.player.x, state.player.y),
            (NARRATIVE_GATE_X as usize, NARRATIVE_GATE_Y as usize + 1)
        );
        assert_eq!(
            (state.active_objects[0].x, state.active_objects[0].y),
            (state.player.x, state.player.y)
        );
        assert_eq!(state.turn, 1);
        assert_eq!(
            state.message,
            "Thou art not upon a Sacred Quest!\nPassage denied!\n"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_narrative_gate_with_ordained_mask_grants_passage_after_world_turn() {
        let dir = debug_game_dir();
        let mut state = britannia_state(
            open_world_grid(),
            NARRATIVE_GATE_X as usize,
            NARRATIVE_GATE_Y as usize,
        );
        state.shrine_ordained_mask = 0b0000_0001;

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );

        assert_eq!(
            (state.player.x, state.player.y),
            (NARRATIVE_GATE_X as usize, NARRATIVE_GATE_Y as usize)
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Pass, Seeker!\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_narrative_gate_coordinate_does_not_fire_in_underworld() {
        let dir = debug_game_dir();
        let mut state = world_state(
            open_world_grid(),
            NARRATIVE_GATE_X as usize,
            NARRATIVE_GATE_Y as usize,
        );

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );

        assert_eq!(
            (state.player.x, state.player.y),
            (NARRATIVE_GATE_X as usize, NARRATIVE_GATE_Y as usize)
        );
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("Pass, Seeker!"));
        assert!(!state.message.contains("Passage denied!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ship_enters_clean_drowning_water_sidecar_without_damage() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 1 0 WATER 1\n",
        )
        .unwrap();
        let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 20,
            skiffs: 1,
        };
        state.sync_player_object();
        state.party[0].hp = 12;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.party[0].hp, 12);
        assert!(!state.message.contains("drowning damage"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn horse_world_movement_ignores_first_cell_waterfall_sidecar() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_WATERFALL_TABLE_FILE),
            "UNDERWORLD 1 0 EAST 2 5\n",
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 0, 0);
        mount_horse(&mut state);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(
            (state.active_objects[0].x, state.active_objects[0].y),
            (1, 0)
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn clean_lava_sidecar_blocks_non_carpet_transport_without_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 1 0 LAVA 5\n",
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 0, 0);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Blocked!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_deeper_transition_table_accepts_clean_scripted_destinations() {
        let entries =
            parse_dungeon_deeper_transition_entries("DUNGEON:6 7 1 1 UNDERWORLD 30 40\n").unwrap();

        assert_eq!(
            entries,
            vec![DungeonDeeperTransitionEntry {
                scene: DungeonScene::from_record(6).unwrap(),
                level: 7,
                x: 1,
                y: 1,
                to_plane: WorldPlane::Underworld,
                to_x: 30,
                to_y: 40,
            }]
        );
    }

    #[test]
    fn dungeon_deeper_transition_table_rejects_invalid_or_duplicate_sources() {
        assert!(
            parse_dungeon_deeper_transition_entries("CASTLE:0 7 1 1 UNDERWORLD 30 40\n").is_err()
        );
        assert!(
            parse_dungeon_deeper_transition_entries("DUNGEON:6 8 1 1 UNDERWORLD 30 40\n").is_err()
        );
        assert!(
            parse_dungeon_deeper_transition_entries("DUNGEON:6 6 1 1 UNDERWORLD 30 40\n").is_err()
        );
        assert!(
            parse_dungeon_deeper_transition_entries("DUNGEON:6 7 8 1 UNDERWORLD 30 40\n").is_err()
        );
        assert!(
            parse_dungeon_deeper_transition_entries(
                "DUNGEON:6 7 1 1 UNDERWORLD 30 40\nDUNGEON:6 7 1 1 BRITANNIA 31 41\n"
            )
            .is_err()
        );
    }

    #[test]
    fn dungeon_teleport_table_accepts_level_destinations_and_optional_cell_guard() {
        let entries =
            parse_dungeon_teleport_entries("DUNGEON:0 0 2 1 3 4 5 0x70\nDUNGEON:1 7 3 4 6 1 2\n")
                .unwrap();

        assert_eq!(
            entries,
            vec![
                DungeonTeleportEntry {
                    scene: DungeonScene::from_record(0).unwrap(),
                    level: 0,
                    x: 2,
                    y: 1,
                    to_level: 3,
                    to_x: 4,
                    to_y: 5,
                    expected_cell: Some(0x70),
                },
                DungeonTeleportEntry {
                    scene: DungeonScene::from_record(1).unwrap(),
                    level: 7,
                    x: 3,
                    y: 4,
                    to_level: 6,
                    to_x: 1,
                    to_y: 2,
                    expected_cell: None,
                },
            ]
        );
    }

    #[test]
    fn dungeon_teleport_table_rejects_invalid_or_duplicate_sources() {
        assert!(parse_dungeon_teleport_entries("CASTLE:0 0 2 1 3 4 5\n").is_err());
        assert!(parse_dungeon_teleport_entries("DUNGEON:0 8 2 1 3 4 5\n").is_err());
        assert!(parse_dungeon_teleport_entries("DUNGEON:0 0 8 1 3 4 5\n").is_err());
        assert!(parse_dungeon_teleport_entries("DUNGEON:0 0 2 1 8 4 5\n").is_err());
        assert!(parse_dungeon_teleport_entries("DUNGEON:0 0 2 1 0 4 5\n").is_err());
        assert!(
            parse_dungeon_teleport_entries("DUNGEON:0 0 2 1 3 4 5\nDUNGEON:0 0 2 1 4 5 6\n")
                .is_err()
        );
    }

    #[test]
    fn dungeon_chest_content_table_accepts_cell_guard_and_multiple_grants() {
        let entries = parse_dungeon_chest_content_entries(
            "DUNGEON:0 0 2 1 0x4c GOLD 7 GEMS 2\nDUNGEON:1 7 3 4 * TORCHES 1\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                DungeonChestContentEntry {
                    scene: DungeonScene::from_record(0).unwrap(),
                    level: 0,
                    x: 2,
                    y: 1,
                    expected_cell: Some(0x4c),
                    grants: vec![
                        ObjectPickupGrant {
                            kind: ObjectPickupKind::Gold,
                            amount: 7,
                        },
                        ObjectPickupGrant {
                            kind: ObjectPickupKind::Gems,
                            amount: 2,
                        },
                    ],
                },
                DungeonChestContentEntry {
                    scene: DungeonScene::from_record(1).unwrap(),
                    level: 7,
                    x: 3,
                    y: 4,
                    expected_cell: None,
                    grants: vec![ObjectPickupGrant {
                        kind: ObjectPickupKind::Torches,
                        amount: 1,
                    }],
                },
            ]
        );
    }

    #[test]
    fn dungeon_chest_content_table_rejects_invalid_or_duplicate_rows() {
        assert!(parse_dungeon_chest_content_entries("CASTLE:0 0 2 1 0x4c GOLD 1\n").is_err());
        assert!(parse_dungeon_chest_content_entries("DUNGEON:0 8 2 1 0x4c GOLD 1\n").is_err());
        assert!(parse_dungeon_chest_content_entries("DUNGEON:0 0 8 1 0x4c GOLD 1\n").is_err());
        assert!(parse_dungeon_chest_content_entries("DUNGEON:0 0 2 1 0x4c GOLD 0\n").is_err());
        assert!(
            parse_dungeon_chest_content_entries("DUNGEON:0 0 2 1 0x4c GOLD 1 GOLD 2\n").is_err()
        );
        assert!(parse_dungeon_chest_content_entries("DUNGEON:0 0 2 1 0x4c GOLD\n").is_err());
        assert!(
            parse_dungeon_chest_content_entries(
                "DUNGEON:0 0 2 1 0x4c GOLD 1\nDUNGEON:0 0 2 1 * GEMS 1\n"
            )
            .is_err()
        );
    }

    #[test]
    fn dungeon_exit_tile_table_accepts_optional_cell_guard() {
        let entries =
            parse_dungeon_exit_tile_entries("DUNGEON:0 0 2 1 0x70\nDUNGEON:1 7 3 4\n").unwrap();

        assert_eq!(
            entries,
            vec![
                DungeonExitTileEntry {
                    scene: DungeonScene::from_record(0).unwrap(),
                    level: 0,
                    x: 2,
                    y: 1,
                    expected_cell: Some(0x70),
                },
                DungeonExitTileEntry {
                    scene: DungeonScene::from_record(1).unwrap(),
                    level: 7,
                    x: 3,
                    y: 4,
                    expected_cell: None,
                },
            ]
        );
    }

    #[test]
    fn dungeon_exit_tile_table_rejects_invalid_or_duplicate_rows() {
        assert!(parse_dungeon_exit_tile_entries("CASTLE:0 0 2 1\n").is_err());
        assert!(parse_dungeon_exit_tile_entries("DUNGEON:0 8 2 1\n").is_err());
        assert!(parse_dungeon_exit_tile_entries("DUNGEON:0 0 8 1\n").is_err());
        assert!(
            parse_dungeon_exit_tile_entries("DUNGEON:0 0 2 1\nDUNGEON:0 0 2 1 0x70\n").is_err()
        );
    }

    #[test]
    fn consumed_top_down_action_on_clean_damage_sidecar_applies_underfoot_damage() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 1 0 DROWNING 5\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 1, 0);
        state.torches = 1;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 12,
            max_hp: 12,
            level: 8,
        }];

        assert!(
            state
                .handle_top_down_key_with_inline('I', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.turn, 1);
        assert!(state.party[0].hp < 12);
        assert!(state.message.contains("Ignited a torch"));
        assert!(state.message.contains("drowning damage"));
        assert!(state.message.contains("party slot 0"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn no_turn_top_down_action_on_clean_damage_sidecar_skips_underfoot_damage() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 1 0 DROWNING 5\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 1, 0);
        state.party[0].hp = 12;

        assert!(
            state
                .handle_top_down_key_with_inline('Z', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.turn, 0);
        assert_eq!(state.party[0].hp, 12);
        assert!(state.message.contains("Player:"));
        assert!(!state.message.contains("drowning damage"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn top_down_movement_on_clean_damage_sidecar_uses_single_landing_damage() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 1 0 DROWNING 1\n",
        )
        .unwrap();
        let mut direct = britannia_state(vec![1; WORLD_CELLS], 0, 0);
        direct.party[0].hp = 12;
        let mut routed = direct.clone();

        assert_eq!(
            direct
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );
        assert!(
            routed
                .handle_top_down_key_with_inline('d', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(routed.turn, direct.turn);
        assert_eq!((routed.player.x, routed.player.y), (1, 0));
        assert_eq!(routed.party[0].hp, direct.party[0].hp);
        assert_eq!(routed.message.matches("drowning damage").count(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn consumed_top_down_action_on_clean_plane_transition_applies_underfoot_transition() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 1 0 UNDERWORLD 30 40 5\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 1, 0);
        state.torches = 1;

        assert!(
            state
                .handle_top_down_key_with_inline('I', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.turn, 1);
        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert!(state.message.contains("Ignited a torch"));
        assert!(state.message.contains("F-A-L-L-S"));
        let _ = fs::remove_dir_all(dir);
    }

