    #[test]
    fn unclassified_escape_input_is_no_turn_and_no_idle_tick() {
        let (key, suffix) = play_input_key_and_suffix("\x1b[99~\n").unwrap();
        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert_eq!(
            handle_play_key_input(&mut dungeon, key, &suffix, Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((dungeon.player.x, dungeon.player.y), (1, 1));
        assert_eq!(dungeon.turn, 0);
        assert_eq!(dungeon.animation.frame, 0);
        assert_eq!(dungeon.message, "Input ignored.");
    }

    #[test]
    fn play_script_typeahead_replays_simple_movement_queue() {
        let mut state = test_state(open_grid(), 1, 1);

        assert_eq!(
            handle_play_script_command(&mut state, "buffer", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            handle_play_script_command(&mut state, "dd.", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (3, 1));
        assert_eq!(state.turn, 2);
        assert!(state.typeahead_buffer_enabled);
        assert_eq!(state.message, "Idle animation tick.");
    }

    #[test]
    fn play_script_command_label_sanitizes_control_sequences() {
        assert_eq!(play_script_command_label(""), "empty");
        assert_eq!(play_script_command_label("\x1bOP"), "F1");
        assert_eq!(play_script_command_label("\x1b[21~"), "F10");
        assert_eq!(play_script_command_label("d\x01."), "d\\x01.");
    }

    #[test]
    fn pending_prompt_consumes_typeahead_toggle_without_changing_buffer_state() {
        let mut prompted = world_state(open_world_grid(), 4, 5);
        prompted.pending_moongate = Some(MoongateEntry {
            x: 4,
            y: 5,
            destination_plane: WorldPlane::Britannia,
            destination_x: 6,
            destination_y: 7,
            active_hours: None,
            expected_tile: None,
        });

        assert_eq!(
            handle_play_script_command(&mut prompted, "buffer", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(!prompted.typeahead_buffer_enabled);
        assert_eq!(prompted.turn, 0);
        assert_eq!(prompted.animation.frame, 0);
        assert!(prompted.pending_moongate.is_some());
        assert_eq!(prompted.message, "Enter moongate? (Y/N).");
    }

    #[test]
    fn empty_play_input_repeats_pending_moongate_prompt_without_turn() {
        let mut prompted = world_state(open_world_grid(), 4, 5);
        prompted.pending_moongate = Some(MoongateEntry {
            x: 4,
            y: 5,
            destination_plane: WorldPlane::Britannia,
            destination_x: 6,
            destination_y: 7,
            active_hours: None,
            expected_tile: None,
        });

        handle_empty_play_input(&mut prompted, Path::new("")).unwrap();

        assert_eq!(prompted.turn, 0);
        assert_eq!((prompted.player.x, prompted.player.y), (4, 5));
        assert!(prompted.pending_moongate.is_some());
        assert_eq!(prompted.message, "Enter moongate? (Y/N).");

        let mut unprompted = test_state(open_grid(), 1, 1);

        handle_empty_play_input(&mut unprompted, Path::new("")).unwrap();

        assert_eq!(unprompted.turn, 1);
        assert_eq!(unprompted.message, "Passed.");
    }

    #[test]
    fn empty_play_input_resolves_dungeon_room_trigger_before_pass() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xf3;
        let mut state = dungeon_state(grid, 0, 1, 1);

        handle_empty_play_input(&mut state, Path::new("")).unwrap();

        assert_eq!(state.turn, 1);
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xa3);
        assert!(
            state
                .message
                .contains("Entered dungeon room trigger slot 3")
        );
        assert!(!state.message.contains("Passed"));
    }

    #[test]
    fn play_script_command_routes_movement_pass_idle_and_quit() {
        let mut state = test_state(open_grid(), 1, 1);

        assert_eq!(
            handle_play_script_command(&mut state, "d", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);

        assert_eq!(
            handle_play_script_command(&mut state, "empty", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 2);
        assert_eq!(state.message, "Passed.");

        assert_eq!(
            handle_play_script_command(&mut state, "pass", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 3);

        assert_eq!(
            handle_play_script_command(&mut state, ".", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 3);
        assert_eq!(state.message, "Idle animation tick.");

        assert_eq!(
            handle_play_script_command(&mut state, "q", Path::new("")).unwrap(),
            PlayInputDisposition::Quit
        );
        assert_eq!(state.turn, 3);
    }

    #[test]
    fn play_script_idle_count_replays_no_turn_visual_ticks() {
        let mut state = test_state(open_grid(), 1, 1);

        assert_eq!(
            handle_play_script_command(&mut state, "idle:3", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
        assert_eq!(state.animation.frame, 3);
        assert_eq!(state.message, "Idle animation tick.");

        assert_eq!(
            handle_play_script_command(&mut state, "tick", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.animation.frame, 0);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn play_script_idle_count_respects_pending_prompt_freeze() {
        let mut prompted = world_state(open_world_grid(), 4, 5);
        prompted.pending_moongate = Some(MoongateEntry {
            x: 4,
            y: 5,
            destination_plane: WorldPlane::Britannia,
            destination_x: 6,
            destination_y: 7,
            active_hours: None,
            expected_tile: None,
        });

        assert_eq!(
            handle_play_script_command(&mut prompted, "idle:2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(prompted.turn, 0);
        assert_eq!(prompted.animation.frame, 0);
        assert!(prompted.pending_moongate.is_some());
        assert_eq!(prompted.message, "Enter moongate? (Y/N).");
    }

    #[test]
    fn play_script_idle_count_rejects_bad_counts() {
        assert!(play_script_idle_tick_count("idle:0").is_err());
        assert!(play_script_idle_tick_count("idle:nope").is_err());
        assert!(play_script_idle_tick_count("idle:1025").is_err());
        assert_eq!(play_script_idle_tick_count("idle").unwrap(), Some(1));
        assert_eq!(play_script_idle_tick_count("tick:4").unwrap(), Some(4));
        assert_eq!(play_script_idle_tick_count("d").unwrap(), None);
    }

    #[test]
    fn play_script_state_line_hashes_message_without_printing_it() {
        let mut state = test_state(open_grid(), 5, 6);
        state.message = "Talked to Ada: I mend gear".to_string();

        let line = play_script_state_line(&state);

        assert!(line.contains("State: CASTLE:0 floor 0 at (5, 6)"));
        assert!(line.contains("message-bytes 26 hash"));
        assert!(!line.contains("Ada"));
        assert!(!line.contains("mend gear"));
    }

    #[test]
    fn play_script_local_clean_smoke_runs_default_scene_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join("CASTLE.DAT").exists() || !game_dir.join(TILES_EGA_FILE).exists() {
            return;
        }

        let mut state = PlayState::load_scene(game_dir, PlayOptions::default()).unwrap();
        let atlas = load_tile_atlas(game_dir, TileGraphicsDepth::Ega16).unwrap();
        let initial_message = state.message.clone();
        let initial_line = play_script_state_line(&state);

        assert!(initial_line.contains("State: CASTLE:0 floor 0"));
        assert!(initial_line.contains("message-bytes"));
        assert!(!initial_message.is_empty());
        assert!(!initial_line.contains(&initial_message));

        let raster_line = raster_diagnostic_line(&mut state, 5, &atlas).unwrap();
        assert!(raster_line.contains("EGA tile atlas"));
        assert!(raster_line.contains("hash "));

        assert_eq!(
            handle_play_script_command(&mut state, "empty", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 1);

        assert_eq!(
            handle_play_script_command(&mut state, ".", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 1);

        assert_eq!(
            handle_play_script_command(&mut state, "q", game_dir).unwrap(),
            PlayInputDisposition::Quit
        );
    }

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
    fn dungeon_door_table_accepts_open_cell_and_optional_closed_guard() {
        let entries =
            parse_dungeon_door_entries("DUNGEON:0 0 2 1 0x70 0xF2\nDUNGEON:1 7 3 4 0xF1\n")
                .unwrap();

        assert_eq!(
            entries,
            vec![
                DungeonDoorEntry {
                    scene: DungeonScene::from_record(0).unwrap(),
                    level: 0,
                    x: 2,
                    y: 1,
                    open_cell: 0x70,
                    expected_cell: Some(0xF2),
                },
                DungeonDoorEntry {
                    scene: DungeonScene::from_record(1).unwrap(),
                    level: 7,
                    x: 3,
                    y: 4,
                    open_cell: 0xF1,
                    expected_cell: None,
                },
            ]
        );
    }

    #[test]
    fn dungeon_door_table_rejects_invalid_or_duplicate_rows() {
        assert!(parse_dungeon_door_entries("CASTLE:0 0 2 1 0x70 0xF2\n").is_err());
        assert!(parse_dungeon_door_entries("DUNGEON:0 8 2 1 0x70 0xF2\n").is_err());
        assert!(parse_dungeon_door_entries("DUNGEON:0 0 8 1 0x70 0xF2\n").is_err());
        assert!(parse_dungeon_door_entries("DUNGEON:0 0 2 1 0x70 0x70\n").is_err());
        assert!(
            parse_dungeon_door_entries("DUNGEON:0 0 2 1 0x70\nDUNGEON:0 0 2 1 0x71\n").is_err()
        );
    }

    #[test]
    fn town_hole_up_requires_hours_and_clean_bed_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);

        assert_eq!(
            state.hole_up_command(&dir, None).unwrap(),
            MoveOutcome::Blocked
        );
        assert!(state.message.contains("how many hours"));
        assert_eq!(state.turn, 0);

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
    fn town_hole_up_advances_one_schedule_tick_per_hour() {
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

        assert_eq!(state.clock, GameClock::new(19, 30).unwrap());
        assert_eq!(state.turn, 2);
        assert_eq!(state.torch_counter, 0);
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
    fn town_hole_up_heals_living_members_per_rested_hour() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party = vec![
            PartyMember {
                slot: 0,
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 5,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 1,
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 5,
                hp: 3,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 2,
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
        assert_eq!(state.turn, 1);
        assert_eq!(state.party[0].hp, 8);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.party[1].status, b'S');
        assert_eq!(state.party[1].hp, 7);
        assert_eq!(state.party[1].mana, 7);
        assert_eq!(state.party[2].status, b'D');
        assert_eq!(state.party[2].hp, 0);
        assert_eq!(state.party[2].mana, 0);
        assert!(state.message.contains("recovered 7 HP"));
        assert!(state.message.contains("and 3 MP"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_requires_hours_without_turn() {
        let mut state = britannia_state(open_world_grid(), 1, 1);

        assert_eq!(
            state.hole_up_command(Path::new(""), None).unwrap(),
            MoveOutcome::Blocked
        );
        assert!(state.message.contains("how many hours"));
        assert_eq!(state.turn, 0);

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(0)).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Rest hours must be in 1..24.");
        assert_eq!(state.turn, 0);
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
        assert_eq!(state.animation.frame, 2);
        assert_eq!(state.ambient_light, FULL_DAYLIGHT);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Party rested 2 hours"));
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
        state.party = vec![PartyMember {
            slot: 0,
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 12,
            max_hp: 12,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.turn, 3);
        assert!(state.party[0].hp < 12);
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
    fn rest_with_watch_heals_living_members_and_wakes_initial_sleepers() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party = vec![
            PartyMember {
                slot: 0,
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 5,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 1,
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 2,
                hp: 3,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 2,
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 3,
                status: b'A',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 4,
                hp: 6,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 4,
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

        assert!(state.party[0].hp > 5);
        assert!(state.party[0].hp <= 10);
        assert!(state.party[0].mana > 0);
        assert!(state.party[1].hp > 3);
        assert!(state.party[1].hp <= 10);
        assert!(state.party[1].mana > 2);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[2].status, b'D');
        assert_eq!(state.party[2].hp, 0);
        assert_eq!(state.party[2].mana, 0);
        assert_eq!(state.party[3].status, b'A');
        assert_eq!(state.party[3].hp, 6);
        assert_eq!(state.party[3].mana, 4);
        assert_eq!(state.party[4].status, b'P');
        assert!(state.party[4].hp >= 7);
        assert!(state.party[4].hp <= 8);
        assert_eq!(state.party[4].mana, REST_MANA_CAP);
        assert!(state.message.contains("recovered "));
        assert!(state.message.contains(" MP"));
        assert!(state.message.contains("woke 1 asleep member"));
    }

    #[test]
    fn dungeon_h_key_routes_to_rest_with_watch_with_inline_hours() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.clock = GameClock::new(1, 45).unwrap();
        state.torch_counter = 70;
        state.light_spell_counter = 50;

        assert!(
            state
                .handle_dungeon_key_with_inline('h', Path::new(""), Some(1), None, None, None)
                .unwrap()
        );

        assert_eq!(state.clock, GameClock::new(2, 45).unwrap());
        assert_eq!(state.turn, 3);
        assert_eq!(state.torch_counter, 10);
        assert_eq!(state.light_spell_counter, 0);
        assert!(state.message.contains("Party rested 1 hour"));
    }

