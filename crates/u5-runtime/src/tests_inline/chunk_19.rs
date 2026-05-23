











    #[test]
    fn pending_moongate_prompt_consumes_quit_key_without_turn() {
        let mut prompted = world_state(open_world_grid(), 4, 5);
        let entry = MoongateEntry {
            x: 4,
            y: 5,
            destination_plane: WorldPlane::Britannia,
            destination_x: 6,
            destination_y: 7,
            active_hours: None,
            expected_tile: None,
        };
        prompted.pending_moongate = Some(entry);

        assert_eq!(
            handle_play_key_input(&mut prompted, 'q', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(prompted.turn, 0);
        assert_eq!((prompted.player.x, prompted.player.y), (4, 5));
        assert_eq!(prompted.pending_moongate, Some(entry));
        assert_eq!(prompted.message, "Enter moongate? (Y/N).");

        let mut unprompted = world_state(open_world_grid(), 4, 5);

        assert_eq!(
            handle_play_key_input(&mut unprompted, 'q', "", Path::new("")).unwrap(),
            PlayInputDisposition::Quit
        );
        assert_eq!(unprompted.turn, 0);
    }

    #[test]
    fn pending_moongate_prompt_suppresses_idle_visual_tick() {
        let mut prompted = world_state(open_world_grid(), 4, 5);
        let entry = MoongateEntry {
            x: 4,
            y: 5,
            destination_plane: WorldPlane::Britannia,
            destination_x: 6,
            destination_y: 7,
            active_hours: None,
            expected_tile: None,
        };
        prompted.pending_moongate = Some(entry);
        prompted.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 6,
            y: 5,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            handle_play_key_input(&mut prompted, '.', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(prompted.turn, 0);
        assert_eq!(prompted.pending_moongate, Some(entry));
        assert_eq!(prompted.animation.frame, 0);
        assert_eq!(prompted.active_objects[1].phase, 0x22);
        assert_eq!(prompted.active_objects[1].tile, 168);
        assert_eq!(prompted.message, "Enter moongate? (Y/N).");

        let mut unprompted = world_state(open_world_grid(), 4, 5);
        unprompted.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 6,
            y: 5,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            handle_play_key_input(&mut unprompted, '.', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(unprompted.turn, 0);
        assert_eq!(unprompted.animation.frame, 1);
        assert_eq!(unprompted.active_objects[1].phase, 0x21);
        assert_eq!(unprompted.active_objects[1].tile, 169);
        assert_eq!(unprompted.message, "Idle animation tick.");
    }

    #[test]
    fn parse_town_rest_bed_entries_accepts_optional_tile_guard() {
        let entries = parse_town_rest_bed_entries("CASTLE:0 0 1 1 55\nCASTLE:0 0 2 1\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownRestBedEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    expected_tile: Some(55),
                },
                TownRestBedEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_rest_bed_entries("CASTLE:0 0 32 1 55\n").is_err());
        assert!(parse_town_rest_bed_entries("DUNGEON:0 0 1 1 55\n").is_err());
    }

    #[test]
    fn parse_town_stair_entries_accepts_direction_and_optional_tile_guard() {
        let entries =
            parse_town_stair_entries("CASTLE:0 0 1 1 UP 55\nCASTLE:0 1 2 1 both\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownStairEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    kind: TownStairKind::Up,
                    expected_tile: Some(55),
                },
                TownStairEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 1,
                    x: 2,
                    y: 1,
                    kind: TownStairKind::Both,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_stair_entries("CASTLE:0 0 32 1 UP 55\n").is_err());
        assert!(parse_town_stair_entries("DUNGEON:0 0 1 1 UP 55\n").is_err());
        assert!(parse_town_stair_entries("CASTLE:0 0 1 1 SIDEWAYS\n").is_err());
        assert!(parse_town_stair_entries("CASTLE:0 0 1 1 UP\nCASTLE:0 0 1 1 DOWN\n").is_err());
    }

    #[test]
    fn parse_town_trap_door_entries_accepts_target_floor_and_optional_tile_guard() {
        let entries =
            parse_town_trap_door_entries("CASTLE:0 0 1 1 -1 55\nCASTLE:0 1 2 1 0\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownTrapDoorEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    to_floor: -1,
                    expected_tile: Some(55),
                },
                TownTrapDoorEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 1,
                    x: 2,
                    y: 1,
                    to_floor: 0,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_trap_door_entries("CASTLE:0 0 32 1 -1 55\n").is_err());
        assert!(parse_town_trap_door_entries("DUNGEON:0 0 1 1 -1 55\n").is_err());
        assert!(parse_town_trap_door_entries("CASTLE:0 0 1 1 0\n").is_err());
        assert!(parse_town_trap_door_entries("CASTLE:0 0 1 1 -1\nCASTLE:0 0 1 1 -2\n").is_err());
    }

    #[test]
    fn parse_town_poison_gas_entries_accepts_optional_tile_guard() {
        let entries =
            parse_town_poison_gas_entries("CASTLE:0 0 2 1 55\nCASTLE:0 -1 4 5\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownPoisonGasEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    expected_tile: Some(55),
                },
                TownPoisonGasEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: -1,
                    x: 4,
                    y: 5,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_poison_gas_entries("CASTLE:0 0 32 1 55\n").is_err());
        assert!(parse_town_poison_gas_entries("DUNGEON:0 0 1 1 55\n").is_err());
        assert!(
            parse_town_poison_gas_entries("CASTLE:0 0 1 1\nCASTLE:0 0 1 1 55\n").is_err()
        );
    }

    #[test]
    fn parse_town_tile_attribute_entries_accepts_hex_values() {
        let entries = parse_town_tile_attribute_entries("0x37 4 0x1C\n56 3 0x1C\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownTileAttributeEntry {
                    tile: 0x37,
                    tile_class: TOWN_POISON_GAS_TILE_CLASS,
                    vehicle_byte: TOWN_POISON_GAS_VEHICLE_BYTE,
                },
                TownTileAttributeEntry {
                    tile: 56,
                    tile_class: 3,
                    vehicle_byte: TOWN_POISON_GAS_VEHICLE_BYTE,
                },
            ]
        );
        assert!(parse_town_tile_attribute_entries("0x37 4\n").is_err());
        assert!(parse_town_tile_attribute_entries("0x37 4 0x1C\n56 4 0x1C\n").is_ok());
        assert!(parse_town_tile_attribute_entries("0x37 4 0x1C\n0x37 5 0x1C\n").is_err());
    }

    #[test]
    fn parse_town_exit_tile_entries_accepts_optional_tile_guard() {
        let entries = parse_town_exit_tile_entries("CASTLE:0 0 1 1 55\nCASTLE:0 1 2 1\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownExitTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    expected_tile: Some(55),
                },
                TownExitTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 1,
                    x: 2,
                    y: 1,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_exit_tile_entries("CASTLE:0 0 32 1 55\n").is_err());
        assert!(parse_town_exit_tile_entries("DUNGEON:0 0 1 1 55\n").is_err());
        assert!(parse_town_exit_tile_entries("CASTLE:0 0 1 1\nCASTLE:0 0 1 1 55\n").is_err());
    }

    #[test]
    fn parse_town_lock_entries_accepts_magic_and_locked_rows() {
        let entries =
            parse_town_lock_entries("CASTLE:0 0 1 1 97 96\nCASTLE:0 1 2 1 98 97 MAGIC\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownLockEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    locked_tile: 97,
                    unlocked_tile: 96,
                    kind: TownLockKind::Locked,
                },
                TownLockEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 1,
                    x: 2,
                    y: 1,
                    locked_tile: 98,
                    unlocked_tile: 97,
                    kind: TownLockKind::Magic,
                },
            ]
        );
        assert!(parse_town_lock_entries("DUNGEON:0 0 1 1 97 96\n").is_err());
        assert!(parse_town_lock_entries("CASTLE:0 0 32 1 97 96\n").is_err());
        assert!(parse_town_lock_entries("CASTLE:0 0 1 1 95 96\n").is_err());
        assert!(parse_town_lock_entries("CASTLE:0 0 1 1 97 97\n").is_err());
        assert!(parse_town_lock_entries("CASTLE:0 0 1 1 97 96\nCASTLE:0 0 1 1 98 97\n").is_err());
    }

    #[test]
    fn town_hole_up_requires_hours_and_clean_bed_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);

        assert_eq!(
            state.hole_up_command(&dir, None).unwrap(),
            MoveOutcome::Observed
        );
        assert!(state.message.contains("how many hours"));
        assert!(state.active_rest.is_some());
        assert_eq!(state.turn, 0);
        state.active_rest = None;

        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 56\n").unwrap();
        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Not here!");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_accepts_only_single_nonzero_duration_digit() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);

        assert_eq!(
            state.hole_up_command(&dir, Some(10)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Rest hours must be in 1..9.");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_runs_initial_schedule_burst_and_ten_minute_cleanup() {
        assert_eq!(PlayState::town_rest_target_hour(17, 2), 19);
        assert_eq!(PlayState::town_rest_target_hour(23, 1), 1);
        assert_eq!(PlayState::town_rest_target_hour(22, 2), 1);

        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(17, 30).unwrap();
        state.torch_counter = 100;
        state.light_spell_counter = 90;
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

        assert_eq!(
            state.hole_up_command(&dir, Some(2)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.clock, GameClock::new(19, 0).unwrap());
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS) + 9
        );
        assert_eq!(state.torch_counter, 10);
        assert_eq!(state.light_spell_counter, 0);
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (4, 1));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (4, 1)
        );
        assert!(state.message.contains("Rested 2 hours"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_stops_when_rest_surface_rejects_after_elapsed_tick() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 0x87;
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(19, 50).unwrap();
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 5,
            max_hp: 10,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(2)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.clock, GameClock::new(20, 0).unwrap());
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS) + 1
        );
        assert_eq!(state.grid[32 + 1], 55 ^ 0xdd);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[0].hp, 5);
        assert_eq!(state.party[0].mana, 0);
        assert!(state.message.contains("thrown out"));
        assert!(state.message.contains("woke 1 asleep member(s)"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_advances_time_without_direct_recovery() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 5,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 5,
                hp: 3,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 10,
                level: 8,
            },
        ];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.clock, GameClock::new(9, 0).unwrap());
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS)
                + u64::from(TOWN_REST_TICKS_PER_HOUR)
        );
        assert_eq!(state.party[0].hp, 5);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[1].hp, 3);
        assert_eq!(state.party[1].mana, 5);
        assert_eq!(state.party[2].status, b'D');
        assert_eq!(state.party[2].hp, 0);
        assert_eq!(state.party[2].mana, 0);
        assert!(state.message.contains("recovered 0 HP"));
        assert!(state.message.contains("and 0 MP"));
        assert!(state.message.contains("woke 2 asleep member(s)"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_poisoned_member_keeps_status_and_skips_hp_recovery() {
        // commands.md §10: poisoned and dead members are not treated like
        // healthy sleepers. The town bed-rest path must skip HP gain for
        // poisoned members while still ticking mana and hourly poison.
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 90,
            hp: 4,
            max_hp: 12,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 3);
        assert_eq!(state.party[0].mana, 90);
        assert!(state.message.contains("recovered 0 HP"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_requires_hours_without_turn() {
        let mut state = britannia_state(open_world_grid(), 1, 1);

        assert_eq!(
            state.hole_up_command(Path::new(""), None).unwrap(),
            MoveOutcome::Observed
        );
        assert!(state.message.contains("how many hours"));
        assert!(state.active_rest.is_some());
        assert_eq!(state.turn, 0);
        state.active_rest = None;

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(0)).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Rest hours must be in 1..9.");
        assert_eq!(state.turn, 0);

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(10)).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Rest hours must be in 1..9.");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn active_rest_prompt_accepts_duration_without_watch_for_single_member() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();

        assert_eq!(
            handle_play_key_input(&mut state, 'H', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_rest.is_some());
        assert!(state.message.contains("Rest- how many hours"));
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_rest.is_none());
        assert_eq!(state.turn, 3);
        assert!(state.message.contains("Party rested 1 hour"));
        assert!(state.message.contains("no watch needed"));
    }

    #[test]
    fn active_rest_prompt_collects_watch_member() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 8,
            max_hp: 12,
            level: 8,
        });

        assert_eq!(
            handle_play_key_input(&mut state, 'H', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_rest.is_some());
        assert!(state.message.contains("Set watch"));
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.message.contains("Who keeps watch"));

        assert_eq!(
            handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_rest.is_none());
        assert_eq!(state.turn, 3);
        assert!(state.message.contains("party slot 2 keeps watch"));
    }

    #[test]
    fn active_rest_prompt_invalid_watcher_rests_without_watch() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 8,
            max_hp: 12,
            level: 8,
        });

        handle_play_key_input(&mut state, 'H', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();

        assert!(state.active_rest.is_none());
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.turn, 3);
        assert!(state.message.contains("no watch set"));
    }

    #[test]
    fn active_town_hole_up_prompt_accepts_duration_digit() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();

        assert_eq!(
            handle_play_key_input(&mut state, 'H', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_rest.is_some());
        assert!(state.message.contains("Hole up- how many hours"));
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_rest.is_none());
        assert_eq!(state.clock, GameClock::new(9, 0).unwrap());
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS)
                + u64::from(TOWN_REST_TICKS_PER_HOUR)
        );
        assert!(state.message.contains("Rested 1 hour"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_accepts_valid_inline_watcher_without_changing_ambush_odds() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 8,
            max_hp: 12,
            level: 8,
        });

        assert_eq!(
            state
                .hole_up_command(
                    Path::new(""),
                    InlineRestRequest {
                        hours: Some(1),
                        watcher: Some(1),
                    },
                )
                .unwrap(),
            MoveOutcome::Rested
        );

        assert!(state.message.contains("party slot 2 keeps watch"));
        assert_eq!(state.turn, 3);
        assert!(!state.combat_active);
    }

    #[test]
    fn rest_with_watch_rejects_non_good_watcher_but_still_rests() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 8,
            max_hp: 12,
            level: 8,
        });

        assert_eq!(
            state
                .hole_up_command(
                    Path::new(""),
                    InlineRestRequest {
                        hours: Some(1),
                        watcher: Some(1),
                    },
                )
                .unwrap(),
            MoveOutcome::Rested
        );

        assert!(state.message.contains("no valid watch set"));
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.turn, 3);
    }

    #[test]
    fn world_rest_with_watch_advances_three_twenty_minute_ticks_per_hour() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(5, 30).unwrap();
        state.ambient_light = FULL_DARKNESS;
        state.visibility_dirty = false;
        state.torch_counter = 80;
        state.light_spell_counter = 70;

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(2)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.clock, GameClock::new(7, 30).unwrap());
        assert_eq!(state.turn, 6);
        assert_eq!(state.torch_counter, 0);
        assert_eq!(state.light_spell_counter, 0);
        // The frame counter advances once per turn and wraps at the LCM
        // of supported cycle lengths (12 covers 3 + 4); after 6 ticks
        // from 0 it sits at 6. The displayed water tile cycles modulo
        // 3 on top of this counter.
        assert_eq!(state.animation.frame, 6);
        assert_eq!(state.ambient_light, FULL_DAYLIGHT);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Party rested 2 hours"));
    }

    #[test]
    fn dangerous_rest_interrupts_on_one_in_sixty_four_predicate() {
        let mut state = britannia_state(open_world_grid(), 0, 15);
        state.clock = GameClock::new(0, 0).unwrap();
        state.prng_state = 0x00f0;
        let mut expected_prng_state = state.prng_state;
        // The rest interruption, ambush-monster row, and combat setup count
        // each consume one resident PRNG advance before combat starts.
        for _ in 0..3 {
            expected_prng_state = u5_prng_advance_state(expected_prng_state);
        }
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 12,
                max_hp: 12,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 2,
                hp: 8,
                max_hp: 12,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 2,
                hp: 8,
                max_hp: 12,
                level: 8,
            },
        ];

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(2)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.clock, GameClock::new(0, 20).unwrap());
        assert!(state.message.contains("Party rested 0 hours 20 minutes"));
        assert!(state.message.contains("Ambushed!"));
        assert!(state.message.contains("sleep ambush entered combat"));
        assert!(!state.message.contains("out of scope"));
        assert!(state.combat_active);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[2].status, b'P');
        assert_eq!(
            state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].z,
            WorldPlane::Britannia.save_floor()
        );
        assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_empty());
    }

    #[test]
    fn world_rest_with_watch_applies_underfoot_damage_sidecar_each_tick() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 1 1 DROWNING 5\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.prng_state = 0x0002;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 50,
            max_hp: 50,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.turn, 3);
        assert!(state.party[0].hp < 50);
        assert!(
            state
                .message
                .contains("Underfoot world damage triggered 3 tick(s)")
        );
        assert!(state.message.contains("drowning damage"));
        assert!(state.message.contains("party slot 0"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sleep_ambush_cleanup_does_not_revive_members_killed_during_rest() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 0 15 DROWNING 5\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 0, 15);
        state.clock = GameClock::new(0, 0).unwrap();
        state.prng_state = 0x0270;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 1,
            max_hp: 1,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Ambushed!"));
        assert!(state.combat_active);
        assert_eq!(state.party[0].hp, 0);
        assert_eq!(state.party[0].status, b'D');
        assert!(state.combat_actors[0].is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_advances_time_and_wakes_initial_sleepers() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 5,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 2,
                hp: 3,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 3,
                class_byte: b'A',
                status: b'A',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 4,
                hp: 6,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 4,
                class_byte: b'A',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 98,
                hp: 7,
                max_hp: 8,
                level: 8,
            },
        ];

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.party[0].hp, 5);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.party[1].hp, 3);
        assert_eq!(state.party[1].mana, 2);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[2].status, b'D');
        assert_eq!(state.party[2].hp, 0);
        assert_eq!(state.party[2].mana, 0);
        assert_eq!(state.party[3].status, b'A');
        assert_eq!(state.party[3].hp, 6);
        assert_eq!(state.party[3].mana, 4);
        assert_eq!(state.party[4].status, b'P');
        assert_eq!(state.party[4].hp, 6);
        assert_eq!(state.party[4].mana, 98);
        assert!(state.message.contains("recovered 0 HP"));
        assert!(state.message.contains("0 MP"));
        assert!(state.message.contains("woke 1 asleep member"));
    }

    #[test]
    fn rest_with_watch_poisoned_members_keep_status_and_skip_hp_recovery() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 98,
            hp: 3,
            max_hp: 12,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 2);
        assert_eq!(state.party[0].mana, 98);
        assert!(state.message.contains("recovered 0 HP"));
        assert!(state.message.contains("MP"));
    }

    #[test]
    fn completed_long_camp_recovery_applies_guarded_hp_and_class_mana() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.avatar_stats.intelligence = 22;
        state.party_intelligence = vec![22, 24, 20, 18, 12, 8];
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 1,
                max_hp: 2,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'M',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 1,
                hp: 4,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'B',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 2,
                hp: 5,
                max_hp: 6,
                level: 8,
            },
            PartyMember {
                slot: 3,
                class_byte: b'F',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 3,
                hp: 5,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 4,
                class_byte: b'A',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 4,
                hp: 5,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 5,
                class_byte: b'M',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 5,
                hp: 0,
                max_hp: 20,
                level: 8,
            },
        ];
        let entry_statuses = [b'G', b'G', b'G', b'G', b'P', b'D'];

        assert_eq!(
            state.apply_completed_long_camp_recovery(5, Some(3), &entry_statuses),
            (0, 0)
        );

        let (hp, mana) = state.apply_completed_long_camp_recovery(6, Some(3), &entry_statuses);

        assert!(hp >= 3);
        assert_eq!(mana, 53);
        assert_eq!(state.party[0].hp, 2);
        assert_eq!(state.party[0].mana, 22);
        assert!((5..=10).contains(&state.party[1].hp));
        assert_eq!(state.party[1].mana, 24);
        assert_eq!(state.party[2].hp, 6);
        assert_eq!(state.party[2].mana, 10);
        assert_eq!(state.party[3].hp, 5);
        assert_eq!(state.party[3].mana, 3);
        assert_eq!(state.party[4].hp, 5);
        assert_eq!(state.party[4].mana, 4);
        assert_eq!(state.party[5].hp, 0);
        assert_eq!(state.party[5].mana, 5);
    }

    #[test]
    fn lord_british_camp_event_recomputes_level_and_prints_karma_verdict() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(KARMA_DAT_FILE),
            karma_bytes(&[
                "low",
                "twenty",
                "forty",
                "sixty",
                "blackthorn-top",
                "camp-top",
            ]),
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 2, 0);
        state.clock = GameClock::new(0, 0).unwrap();
        state.avatar_stats = AvatarStats {
            strength: 20,
            dexterity: 20,
            intelligence: 18,
        };
        state.party[0].level = 1;
        state.party[0].hp = 10;
        state.party[0].max_hp = 30;
        state.party[0].mana = 0;
        state.party[0].climb_stat = 20;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: 20,
            mana: REST_MANA_CAP,
            hp: 10,
            max_hp: 30,
            level: 1,
        });
        state.party_experience = vec![200, 200];
        state.party_strengths = vec![20, 20];
        state.party_intelligence = vec![18, 24];
        state.moral_standing = 80;
        let mut expected_prng_state = state.prng_state;
        for _ in 0..6 {
            expected_prng_state = u5_prng_advance_state(expected_prng_state);
        }

        assert_eq!(state.hole_up_command(&dir, Some(1)).unwrap(), MoveOutcome::Rested);

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.party[0].level, 3);
        assert_eq!(state.party[0].hp, 90);
        assert_eq!(state.party[0].max_hp, 90);
        assert_eq!(state.avatar_stats.dexterity, 21);
        assert_eq!(state.party[0].climb_stat, 21);
        assert_eq!(state.party[0].mana, 18);
        assert_eq!(state.party[1].level, 3);
        assert_eq!(state.party[1].hp, 90);
        assert_eq!(state.party[1].max_hp, 90);
        assert_eq!(state.party[1].mana, REST_MANA_CAP);
        assert_eq!(state.party_strengths[1], 21);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Lord British-in-disguise camp event."));
        assert!(state.message.contains("P1 reached level 3 from 200 XP"));
        assert!(state.message.contains("P2 reached level 3 from 200 XP"));
        assert!(state.message.contains("Dexterity reward"));
        assert!(state.message.contains("Verdict: camp-top"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_h_key_routes_to_rest_with_watch_with_inline_hours() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.clock = GameClock::new(1, 45).unwrap();
        state.torch_counter = 70;
        state.light_spell_counter = 50;

        assert!(
            state
                .handle_dungeon_key_with_inline('h', Path::new(""), Some(1), None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.clock, GameClock::new(2, 45).unwrap());
        assert_eq!(state.turn, 3);
        assert_eq!(state.torch_counter, 10);
        assert_eq!(state.light_spell_counter, 0);
        assert!(state.message.contains("Party rested 1 hour"));
    }

