    #[test]
    fn dungeon_get_refuses_door_or_unrelated_cell_without_turn() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xf2;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(state.get_dungeon_underfoot(scene, 0), MoveOutcome::Blocked);

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xf2);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Must open it first.");

        state.grid[dungeon_cell_index(0, 1, 1)] = 0x00;
        assert_eq!(state.get_dungeon_underfoot(scene, 0), MoveOutcome::Blocked);

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x00);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to get here.");
    }

    #[test]
    fn dungeon_g_key_routes_to_underfoot_get() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4d;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('g', Path::new("")).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7d);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got dungeon chest"));
    }

    #[test]
    fn dungeon_open_unrelated_cell_is_not_a_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert_eq!(state.open_facing(), MoveOutcome::Blocked);

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x00);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to open here.");
    }

    #[test]
    fn dungeon_open_preserves_unresolved_heavy_door_subtype() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xf2;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(state.open_facing(), MoveOutcome::Blocked);

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xf2);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("subtypes are still open"));
    }

    #[test]
    fn dungeon_door_sidecar_blocks_closed_door_without_room_trigger() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_DOOR_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x70 0xF2\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xF2;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Blocked!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_open_uses_sidecar_to_toggle_heavy_door_without_autoclose() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_DOOR_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70 0xF2\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xF2;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.visibility_dirty = false;

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::DoorOpened
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x70);
        assert_eq!(state.turn, 1);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Opened!");
        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_jimmy_uses_sidecar_to_toggle_heavy_door_without_autoclose() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_DOOR_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70 0xF2\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xF2;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.visibility_dirty = false;

        assert!(state.handle_dungeon_key('J', &dir).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x70);
        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Unlocked!");
        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_sidecar_open_door_cell_can_receive_commands_without_room_trigger() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_DOOR_TABLE_FILE),
            "DUNGEON:0 0 1 1 0xF1 0xF2\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xF1;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.visibility_dirty = false;

        assert!(state.handle_dungeon_key('J', &dir).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xF1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "It's open!");
        assert!(!state.message.contains("room trigger"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_sidecar_open_door_cell_is_walkable_even_with_f_nibble() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_DOOR_TABLE_FILE),
            "DUNGEON:0 0 2 1 0xF1 0xF2\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xF1;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("open dungeon door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_look_reports_facing_actor_without_spending_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        assert!(state.message.contains("actor tile 192"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_look_uses_look2_description_when_available() {
        let dir = debug_game_dir();
        fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[(16, "stone path")])).unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.look_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("stone path"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn look_clock_tiles_append_twelve_hour_time_context() {
        let table = parse_look2_dat(&look2_bytes(&[(0xfa, "a clock")])).unwrap();
        let mut state = test_state(open_grid(), 1, 1);

        state.clock = GameClock::new(0, 7).unwrap();
        assert_eq!(
            state.look_description(0xfa, Some(&table)),
            "a clock (12:07 A.M.)"
        );

        state.clock = GameClock::new(12, 0).unwrap();
        assert_eq!(
            state.look_description(0xfa, Some(&table)),
            "a clock (12:00 P.M.)"
        );

        state.clock = GameClock::new(23, 59).unwrap();
        assert_eq!(
            state.look_description(0xfa, Some(&table)),
            "a clock (11:59 P.M.)"
        );
    }

    #[test]
    fn world_look_wraps_and_reports_facing_object_without_spending_turn() {
        let mut state = world_state(open_world_grid(), 255, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 170,
            tile: 170,
            x: 0,
            y: 0,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        assert!(state.message.contains("object tile 170"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn world_look_uses_look2_description_for_wrapped_object() {
        let dir = debug_game_dir();
        fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[(170, "frigate")])).unwrap();
        let mut state = world_state(open_world_grid(), 255, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 170,
            tile: 170,
            x: 0,
            y: 0,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.look_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("frigate"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn world_look_dungeon_mouth_appends_clean_location_name() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(LOOK2_DAT_FILE),
            look2_bytes(&[(0xdf, "a dungeon mouth")]),
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 2 1 DUNGEON:3 0xdf\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = 0xdf;
        let mut state = britannia_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.look_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("a dungeon mouth (Wrong)"));
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn overworld_talk_reports_no_response_without_tlk_lookup_or_turn() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state
                .talk_facing_with_game_dir(Path::new(r"C:\missing-u5-clean-room-test"))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Funny, no response!");
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_talk_reports_facing_npc_envelope_and_consumes_turn() {
        let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
            2,
            &["Ada", "a test smith", "Greetings", "JOB", "Bye"],
        )]))
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
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
                dialog_id: 2,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: Some("Ada".to_string()),
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("Ada"));
        assert!(state.message.contains("Greetings"));
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
    }

    #[test]
    fn talk_keyword_match_respects_space_boundary() {
        assert!(talk_keyword_matches("JOB", "job"));
        assert!(talk_keyword_matches("JOB", "job news"));
        assert!(!talk_keyword_matches("JOB", "jobber"));
        assert!(talk_keyword_matches("WHO ART THOU", "who art thou friend"));
        assert!(!talk_keyword_matches("WHO", "whom"));
    }

    #[test]
    fn talk_keyword_response_resolves_job_bye_and_pairs() {
        let fields = vec![
            "Ada".to_string(),
            "a test smith".to_string(),
            "Greetings".to_string(),
            "I mend gear".to_string(),
            "Farewell".to_string(),
            "GRAN".to_string(),
            "Short answer".to_string(),
            "GRANDPA".to_string(),
            "Long answer".to_string(),
        ];

        assert_eq!(talk_keyword_response(&fields, "job"), Some("I mend gear"));
        assert_eq!(talk_keyword_response(&fields, "bye"), Some("Farewell"));
        assert_eq!(
            talk_keyword_response(&fields, "grandpa"),
            Some("Long answer")
        );
        assert_eq!(
            talk_keyword_response(&fields, "gran news"),
            Some("Short answer")
        );
        assert_eq!(talk_keyword_response(&fields, "granite"), None);
    }

    #[test]
    fn town_talk_inline_keyword_uses_decoded_tlk_response() {
        let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
            2,
            &[
                "Ada",
                "a test smith",
                "Greetings",
                "I mend gear",
                "Bye",
                "TRADE",
                "Bring iron",
            ],
        )]))
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
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
                dialog_id: 2,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: Some("Ada".to_string()),
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword(&dialogue, Some("trade now")),
            MoveOutcome::Talked
        );

        assert_eq!(state.message, "Talked to Ada: Bring iron");
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
    }

    #[test]
    fn play_input_talk_suffix_routes_to_one_shot_keyword_lookup() {
        let dir = debug_game_dir();
        fs::write(
            dir.join("CASTLE.TLK"),
            tlk_bytes(&[(
                2,
                &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"],
            )]),
        )
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
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
                dialog_id: 2,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: Some("Ada".to_string()),
            },
        ]);

        assert_eq!(
            handle_play_key_input(&mut state, 'T', "JOB", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Talked to Ada: I mend gear");
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_talk_can_reach_npc_behind_counter_tile() {
        let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
            2,
            &["Ada", "a test smith", "Greetings", "JOB", "Bye"],
        )]))
        .unwrap();
        let mut grid = open_grid();
        grid[1 * 32 + 2] = 64;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
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
                dialog_id: 2,
                schedule: [0, 0, 0, 3, 3, 3, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: Some("Ada".to_string()),
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("Ada"));
        assert!(state.message.contains("Greetings"));
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
    }

    #[test]
    fn town_talk_reports_nobody_without_spending_turn() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Nobody's here!");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_talk_reports_shop_trigger_without_keyword_loop() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
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
                dialog_id: 0x84,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("shop trigger 0x84"));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn animation_clock_cycles_public_static_four_frame_families() {
        // Only water actually animates (3 frames). Mountains, bookshelves,
        // doors, tables in the 0x0a, 0x5c, 0x98, 0x9c bands are static
        // terrain/furniture per LOOK2.DAT, not animation cycles.
        let (base, cycle) = (1u8, 3u8);
        let mut clock = AnimationClock::default();
        let frames: Vec<_> = (0..cycle)
            .map(|_| {
                let tile = clock.resolve_static_tile(base);
                clock.tick_static_tiles();
                tile
            })
            .collect();
        let expected: Vec<u8> = (0..cycle).map(|i| base + i).collect();
        assert_eq!(frames, expected);

        // Static-only tiles remain unchanged across the same cycle.
        let static_ids: [u8; 6] = [10, 11, 12, 13, 92, 152];
        for tid in static_ids {
            for frame in 0..4u8 {
                let clk = AnimationClock {
                    frame,
                    moongate_frame: 0,
                };
                assert_eq!(
                    clk.resolve_static_tile(tid),
                    tid,
                    "tile 0x{tid:02x} should not animate at frame {frame}"
                );
            }
        }

        let clock = AnimationClock {
            frame: 3,
            moongate_frame: 7,
        };
        assert_eq!(clock.resolve_static_tile(16), 16);
    }

    #[test]
    fn static_tile_animation_uses_family_wide_frame_selector() {
        // Only the water family (1..=3) animates: each cell preserves
        // its stored identity-offset within the 3-frame cycle.
        for frame in 0..4u8 {
            let clock = AnimationClock {
                frame,
                moongate_frame: 0,
            };
            let resolved: Vec<_> = (1u8..=3).map(|t| clock.resolve_static_tile(t)).collect();
            let expected: Vec<u8> = (0u8..3).map(|i| 1 + ((i + frame) % 3)).collect();
            assert_eq!(resolved, expected, "frame {frame}");
        }
    }

    #[test]
    fn render_resolves_static_animation_without_mutating_grid() {
        let mut grid = open_world_grid();
        grid[world_cell_index(6, 5)] = 1;
        let mut state = britannia_state(grid, 5, 5);
        state.ambient_light = FULL_DAYLIGHT;

        let frame_zero = state.render_text_view(1);
        assert!(frame_zero.lines().nth(2).unwrap().contains("@~"));
        assert_eq!(state.grid[world_cell_index(6, 5)], 1);

        state.animation.tick_static_tiles();
        let frame_one = state.render_text_view(1);
        assert!(frame_one.lines().nth(2).unwrap().contains("@="));
        assert_eq!(state.grid[world_cell_index(6, 5)], 1);
    }

    #[test]
    fn animation_clock_cycles_moongate_through_animation_frames() {
        // Moongate cycles through MOONGATE_ANIMATION_FRAMES sprite frames
        // starting at MOONGATE_TILE_BASE. The full ring is verified per
        // u5-spec/catalogs/tile-catalog.md and the LOOK2.DAT moongate
        // labelling of tile 0xDC.
        let mut clock = AnimationClock::default();
        let frames: Vec<_> = (0..MOONGATE_ANIMATION_FRAMES)
            .map(|_| {
                let tile = clock.resolve_moongate_tile();
                clock.tick_moongate();
                tile
            })
            .collect();

        assert_eq!(
            frames,
            (MOONGATE_TILE_BASE..MOONGATE_TILE_BASE + MOONGATE_ANIMATION_FRAMES)
                .collect::<Vec<_>>()
        );
        assert_eq!(clock.resolve_moongate_tile(), MOONGATE_TILE_BASE);
    }

    #[test]
    fn active_object_phase_respects_steady_countdown_and_decision() {
        let mut steady = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 0,
            y: 0,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        assert_eq!(steady.tick_phase(), PhaseTick::Steady);
        assert_eq!(steady.phase, STEADY_PHASE);

        let mut animated = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 0,
            y: 0,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        };
        assert_eq!(animated.tick_phase(), PhaseTick::Countdown);
        assert_eq!(animated.phase, 0x21);

        let mut decision = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 0,
            y: 0,
            z: 0,
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        };
        assert_eq!(decision.tick_phase(), PhaseTick::DecisionPoint);
        assert_eq!(decision.phase, 0x20);
    }

    #[test]
    fn active_object_countdown_updates_vehicle_frame_tile() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!(object.phase, 0x21);
        assert_eq!(object.tile, 169);
    }

    #[test]
    fn active_object_decision_point_returns_to_base_frame_tile() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 171,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!(object.phase, 0x20);
        assert_eq!(object.tile, 168);
    }

    #[test]
    fn active_ship_drifts_with_matching_wind() {
        let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
        state.wind = WindState::East;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!((object.x, object.y), (6, 5));
        assert_eq!(object.phase, 0x20);
        assert_eq!(object.tile, 168);
    }

    #[test]
    fn active_ship_stalls_without_wind() {
        let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));
        assert_eq!(object.phase, 0x20);
    }

    #[test]
    fn active_ship_against_wind_uses_phase_countdown_cadence() {
        let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
        state.wind = WindState::East;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();
        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));
        assert_eq!(object.phase, 0x61);

        state.advance_turn();
        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!((object.x, object.y), (4, 5));
        assert_eq!(object.phase, 0x60);
    }

    #[test]
    fn active_ship_drift_respects_water_and_player_collision() {
        let mut terrain_blocked = world_state(open_world_grid(), 10, 10);
        terrain_blocked.wind = WindState::East;
        terrain_blocked.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        terrain_blocked.advance_turn();
        let object = terrain_blocked
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));

        let mut player_blocked = world_state(vec![1; WORLD_CELLS], 6, 5);
        player_blocked.wind = WindState::East;
        player_blocked.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        player_blocked.advance_turn();
        let object = player_blocked
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));
    }

