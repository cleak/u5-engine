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
        // `vehicles.md §6` (u5-spec#297): a frigate key that changes the
        // facing only turns the ship, so face the step's direction first.
        state.player.transport = state.player.transport.with_facing(Direction::East);
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
        // Same rule as the native tile: the published line in the window,
        // the accounting in the diagnostics.
        assert!(state.message.contains(WORLD_BURNING_LINE));
        assert!(!state.message.contains("lava damage"));
        let report = state
            .diagnostics
            .iter()
            .find(|note| note.contains("lava damage"))
            .expect("the accounting is kept as a diagnostic");
        assert!(report.contains("party slot 0"));
        assert!(!report.contains("party slot 1"));
        let _ = fs::remove_dir_all(dir);
    }

    /// `commands.md §3`: the dispatcher's status is `Acted` by default and
    /// `0` has a closed list of producers, none of which is a cancelled
    /// Search - so the turn's epilogue runs and its underfoot work
    /// happens.
    ///
    /// Measured 2026-09-19 (`qa/paired/hidden-treasure-search.tsv`, beat
    /// `again`): standing on molten lava, `S` cancelled with Space
    /// answers ` Search-Pass` / `Burning!` on the original, where this
    /// engine printed the echo alone.
    #[test]
    fn a_cancelled_outdoor_search_still_costs_its_turn() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 1)] = 0x8f;
        let mut state = world_state(grid, 1, 1);
        let turn_before = state.turn;

        handle_play_key_input(&mut state, 'S', "", &dir).unwrap();
        assert!(state.active_direction_prompt.is_some());
        assert_eq!(state.turn, turn_before);

        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();

        assert!(state.active_direction_prompt.is_none());
        assert_eq!(state.turn, turn_before + 1, "the cancel costs its turn");
        let transcript = state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>()
            .join("|");
        assert!(
            transcript.contains(WORLD_BURNING_LINE),
            "the turn's underfoot line is printed: {transcript}"
        );
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
        // The window gets the burning family's published line; the
        // per-member accounting is a diagnostic. Measured 2026-09-19
        // (`qa/paired/hidden-treasure-search.tsv`), where the original
        // answered a turn on Underworld `(233, 233)` with `Burning!` and
        // this engine wrote `lava damage: party slot 0 took 2 HP (58 HP
        // left).` into the message window.
        assert!(state.message.contains(WORLD_BURNING_LINE));
        assert!(!state.message.contains("lava damage"));
        let report = state
            .diagnostics
            .iter()
            .find(|note| note.contains("lava damage"))
            .expect("the accounting is kept as a diagnostic");
        assert!(report.contains("party slot 0"));
        assert!(!report.contains("party slot 1"));
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
        assert!(
            state
                .diagnostics
                .iter()
                .any(|note| note.contains("drowning damage")),
            "the accounting is a diagnostic, not a game line"
        );
        assert!(
            state
                .diagnostics
                .iter()
                .any(|note| note.contains("party slot 0"))
        );
        assert!(
            !state
                .diagnostics
                .iter()
                .any(|note| note.contains("party slot 1"))
        );
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
        assert!(
            state
                .diagnostics
                .iter()
                .any(|note| note.contains("drowning damage")),
            "the accounting is a diagnostic, not a game line"
        );
        assert!(
            state
                .diagnostics
                .iter()
                .any(|note| note.contains("party slot 0"))
        );
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
            "\"Thou art not upon a Sacred Quest!\nPassage denied!\"\n"
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
        assert_eq!(state.message, "\"Pass, Seeker!\"\n");
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
        // `vehicles.md §6` (u5-spec#297): a frigate key that changes the
        // facing only turns the ship, so face the step's direction first.
        state.player.transport = state.player.transport.with_facing(Direction::East);
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
        assert!(
            !state
                .diagnostics
                .iter()
                .any(|note| note.contains("drowning damage"))
        );
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
        assert!(state.torch_counter > 0, "torch ignited");
        assert!(
            state
                .diagnostics
                .iter()
                .any(|note| note.contains("drowning damage")),
            "the accounting is a diagnostic, not a game line"
        );
        assert!(
            state
                .diagnostics
                .iter()
                .any(|note| note.contains("party slot 0"))
        );
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

        // `commands.md §3` lists `D` and `W` as the two stock-refusal
        // letters that report "no action". `Z` used to stand in here, but
        // §4 makes Z-stats an unconditional "acted", so it can no longer
        // demonstrate a zero-turn command.
        assert!(
            state
                .handle_top_down_key_with_inline('D', &dir, None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.turn, 0);
        // `commands.md §4`: `D` is "Default refusal ... it falls through to
        // the stock \"What?\" response when it reaches this dispatcher",
        // so this pins that the command really ran rather than being
        // swallowed before the dispatcher.
        assert!(state.message.contains("What?"));
        assert_eq!(state.party[0].hp, 12);
        assert!(
            !state
                .diagnostics
                .iter()
                .any(|note| note.contains("drowning damage"))
        );
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
        assert_eq!(
            routed
                .diagnostics
                .iter()
                .filter(|note| note.contains("drowning damage"))
                .count(),
            1
        );
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
        assert!(state.torch_counter > 0, "torch ignited");
        // `RETRACTIONS.md` R320: the sidecar plane transition narrates
        // nothing; the falls banner belongs to the waterfall handler.
        assert!(!state.message.contains("F-A-L-L-S"));
        let _ = fs::remove_dir_all(dir);
    }

    /// `overworld.md §7.1`: the four early exits - wrong plane, wrong
    /// tile, the one-in-eight trigger draw, and a non-foot transport -
    /// each "ends the check silently": no line, no prompt.
    #[test]
    fn troll_bridge_ambush_requires_britannia_bridge_tile_and_foot_transport() {
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = TROLL_BRIDGE_TILE_FIRST;

        // Wrong plane: the tiles "occur only on Britannia".
        let mut underworld = world_state(grid.clone(), 2, 1);
        underworld.apply_troll_bridge_ambush(WorldPlane::Underworld);
        assert!(underworld.message.is_empty());
        assert!(underworld.active_yes_no_prompt.is_none());

        // Off the bridge tile entirely.
        let mut off_tile = britannia_state(open_world_grid(), 1, 1);
        off_tile.apply_troll_bridge_ambush(WorldPlane::Britannia);
        assert!(off_tile.message.is_empty());
        assert!(off_tile.active_yes_no_prompt.is_none());

        // On the tile, on foot, but the trigger draw almost always misses
        // ("any nonzero result ends the check, so one turn in eight
        // proceeds"): a forced-nonzero seed must stay silent.
        let mut on_foot = britannia_state(grid.clone(), 2, 1);
        let seed = (0u32..4096)
            .find(|&seed| {
                let mut probe = on_foot.clone();
                probe.prng_state = seed as u16;
                probe.troll_bridge_trigger_roll() != 0
            })
            .expect("some seed misses the one-in-eight trigger");
        on_foot.prng_state = seed as u16;
        on_foot.apply_troll_bridge_ambush(WorldPlane::Britannia);
        assert!(on_foot.message.is_empty());
        assert!(on_foot.active_yes_no_prompt.is_none());

        // Mounted: "the transport must be on foot ... ends the check
        // silently" - forced through the one-in-eight gate this time.
        let mut mounted = britannia_state(grid, 2, 1);
        mounted.player.transport = TransportState::Horse {
            type_byte: 160,
            tile: 160,
        };
        let seed = (0u32..4096)
            .find(|&seed| {
                let mut probe = mounted.clone();
                probe.prng_state = seed as u16;
                probe.troll_bridge_trigger_roll() == 0
            })
            .expect("some seed hits the one-in-eight trigger");
        mounted.prng_state = seed as u16;
        mounted.apply_troll_bridge_ambush(WorldPlane::Britannia);
        assert!(mounted.message.is_empty());
        assert!(mounted.active_yes_no_prompt.is_none());
    }

    /// §7.1 step 2: "If nobody is caught ... print `Trolls evaded!` -
    /// there is no toll and no fight."
    #[test]
    fn troll_bridge_ambush_evades_when_nobody_is_caught() {
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = TROLL_BRIDGE_TILE_LAST;
        let base = britannia_state(grid, 2, 1);
        let seed = (0u32..8192)
            .find(|&seed| {
                let mut probe = base.clone();
                probe.prng_state = seed as u16;
                probe.apply_troll_bridge_ambush(WorldPlane::Britannia);
                probe.message == TROLL_BRIDGE_EVADED_LINE
            })
            .expect("some seed evades the ambush entirely");
        let mut state = base;
        state.prng_state = seed as u16;
        state.apply_troll_bridge_ambush(WorldPlane::Britannia);

        assert_eq!(state.message, TROLL_BRIDGE_EVADED_LINE);
        assert!(state.active_yes_no_prompt.is_none());
        assert_eq!(state.gold, DEFAULT_GOLD_STOCK);
        assert!(
            state
                .message_entries()
                .iter()
                .any(|entry| entry.text.contains("Thou spieth trolls under the bridge!"))
        );
    }

    /// §7.1 steps 3-4: a caught member raises the `Dost thou pay?` prompt
    /// with the toll computed from "the Strength of the first member able
    /// to act".
    #[test]
    fn troll_bridge_ambush_demands_a_toll_when_a_member_is_caught() {
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = TROLL_BRIDGE_TILE_FIRST;
        let mut base = britannia_state(grid, 2, 1);
        // Dexterity zero cannot save against any `1..30` roll, so the lone
        // party member is caught on the first walked slot.
        base.party[0].climb_stat = 0;
        base.party_strengths[0] = 15;
        let seed = (0u32..8192)
            .find(|&seed| {
                let mut probe = base.clone();
                probe.prng_state = seed as u16;
                probe.apply_troll_bridge_ambush(WorldPlane::Britannia);
                probe.active_yes_no_prompt.is_some()
            })
            .expect("some seed hits the trigger and catches the only member");
        let mut state = base;
        state.prng_state = seed as u16;
        state.apply_troll_bridge_ambush(WorldPlane::Britannia);

        // Strength 15 gives 99 - 3*15 = 54, exactly the published example.
        assert_eq!(
            state.active_yes_no_prompt,
            Some(YesNoPromptSession::new(YesNoPromptKind::TrollBridgeToll {
                toll: 54
            }))
        );
        assert_eq!(state.message, "Caught!\n\nThe trolls demand a 54 gp toll!\n\nDost thou pay?");
    }

    #[test]
    fn troll_bridge_toll_payment_debits_gold_when_affordable() {
        let mut state = britannia_state(open_world_grid(), 2, 1);
        state.gold = 100;
        state.active_yes_no_prompt = Some(YesNoPromptSession::new(
            YesNoPromptKind::TrollBridgeToll { toll: 54 },
        ));
        let dir = debug_game_dir();

        state
            .resolve_troll_bridge_toll_payment(54, &dir)
            .unwrap();

        assert_eq!(state.gold, 46);
        assert!(state.stats_panel_refresh_requested);
        // `stats-panel.md §2.2.1` row 19: the toll's gold debit is one of
        // the twenty-three deferred sites, not an immediate repaint.
        assert_eq!(
            state.stats_panel_command_refresh,
            crate::stats_panel::PanelRefreshMode::Deferred
        );
        assert!(!state.combat_active);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn troll_bridge_toll_payment_rolls_back_and_fights_when_unaffordable() {
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = TROLL_BRIDGE_TILE_FIRST;
        let mut state = britannia_state(grid, 2, 1);
        state.gold = 10;
        state.active_yes_no_prompt = Some(YesNoPromptSession::new(
            YesNoPromptKind::TrollBridgeToll { toll: 54 },
        ));
        let dir = debug_game_dir();
        fs::write(dir.join(BRIT_CBT_FILE), Vec::new()).ok();

        let _ = state.resolve_troll_bridge_toll_payment(54, &dir);

        // "the debit is rolled back but the request stays raised ... and
        // the event continues exactly as a refusal."
        assert_eq!(state.gold, 10);
        assert!(state.stats_panel_refresh_requested);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn troll_bridge_toll_for_strength_matches_the_published_example() {
        assert_eq!(troll_bridge_toll_for_strength(15), 54);
        assert_eq!(troll_bridge_toll_for_strength(30), 9);
        assert_eq!(troll_bridge_toll_for_strength(33), 0);
    }

