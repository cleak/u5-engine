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
        grid[dungeon_cell_index(0, 1, 1)] = 0x7d;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('g', Path::new("")).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x08);
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
        assert_eq!(state.message, "Nothing to open here.");
    }

    #[test]
    fn dungeon_jimmy_preserves_unresolved_heavy_door_subtype_without_turn() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xf2;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .jimmy_facing_with_game_dir_and_member(None, Some(0))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xf2);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "No lock!");
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
    fn parsed_tlk_aliases_sentinel_id_one_to_first_real_blob() {
        let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
            2,
            &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"],
        )]))
        .unwrap();

        assert_eq!(dialogue.get(&1), dialogue.get(&2));
    }

    #[test]
    fn parsed_tlk_caps_each_blob_to_runtime_window() {
        let long_name = "A".repeat(1100);
        let bytes = tlk_bytes(&[(2, &[long_name.as_str()])]);
        let dialogue = parse_tlk_bytes(&bytes).unwrap();

        assert_eq!(dialogue[&2][0].len(), 1024);
    }

    #[test]
    fn town_talk_dialog_id_one_uses_sentinel_alias() {
        let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
            2,
            &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"],
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
                dialog_id: 1,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: Some("Ada".to_string()),
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("Talked to Ada"));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn talk_shop_trigger_maps_public_shop_roles() {
        assert_eq!(
            talk_shop_trigger(0x81),
            Some(("Weaponsmith / armourer", "Arms stock arm"))
        );
        assert_eq!(
            talk_shop_trigger(0x84),
            Some(("Ship broker / shipwright", "Shipwright sale arm"))
        );
        assert_eq!(talk_shop_trigger(0xff), None);
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
    fn talk_keyword_response_resolves_reserved_aliases_and_pairs() {
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

        assert_eq!(talk_keyword_response(&fields, "name"), Some("Ada"));
        assert_eq!(talk_keyword_response(&fields, "job"), Some("I mend gear"));
        assert_eq!(talk_keyword_response(&fields, "work"), Some("I mend gear"));
        assert_eq!(talk_keyword_response(&fields, "bye"), Some("Farewell"));
        assert_eq!(talk_keyword_response(&fields, "thank"), Some("Farewell"));
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
    fn resolve_keyword_response_field_index_matches_reserved_and_pair_keywords() {
        let fields = vec![
            "Ada".to_string(),
            "smith".to_string(),
            "Greetings".to_string(),
            "I mend gear".to_string(),
            "Farewell".to_string(),
            "GRAN".to_string(),
            "Short answer".to_string(),
            "GRANDPA".to_string(),
            "Long answer".to_string(),
        ];

        assert_eq!(resolve_keyword_response_field_index(&fields, "name"), Some(0));
        assert_eq!(resolve_keyword_response_field_index(&fields, "job"), Some(3));
        assert_eq!(resolve_keyword_response_field_index(&fields, "work"), Some(3));
        assert_eq!(resolve_keyword_response_field_index(&fields, "bye"), Some(4));
        assert_eq!(resolve_keyword_response_field_index(&fields, "thank"), Some(4));
        assert_eq!(
            resolve_keyword_response_field_index(&fields, "grandpa"),
            Some(8)
        );
        assert_eq!(
            resolve_keyword_response_field_index(&fields, "gran news"),
            Some(6)
        );
        assert_eq!(
            resolve_keyword_response_field_index(&fields, "granite"),
            None
        );
    }

    #[test]
    fn parse_tlk_blob_fields_raw_round_trips_a_minimal_blob() {
        // Synthetic minimal TLK: header count=2 (so 1 live NPC) at offset 0.
        // Header entry layout: 2 bytes blob_offset, 2 bytes npc_id.
        // Slot 0 is the sentinel; slot 1 is the live NPC. The actual file
        // starts with the count word, then `count - 1` header entries.
        let blob_offset: u16 = 8;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&2u16.to_le_bytes()); // count
        bytes.extend_from_slice(&[0u8; 2]); // slot-0 sentinel padding
        bytes.extend_from_slice(&blob_offset.to_le_bytes()); // blob offset for npc 1
        bytes.extend_from_slice(&0x0042u16.to_le_bytes()); // npc id 0x42
        // Two fields: "Ada\0" then "smith\0" (XOR-encoded).
        let xor = 0x80u8;
        let field_a = b"Ada";
        let field_b = b"smith";
        for b in field_a {
            bytes.push(*b ^ xor);
        }
        bytes.push(0);
        for b in field_b {
            bytes.push(*b ^ xor);
        }
        bytes.push(0);

        let parsed = parse_tlk_blob_fields_raw(&bytes).unwrap();
        let fields = parsed.get(&0x0042).expect("npc 0x42 missing");
        assert!(fields.len() >= 2);
        // Each field's bytes are still XOR-encoded; running through the
        // byte-runner produces "Ada" and "smith".
        let inputs = crate::tlk_runner::TlkRunInputs {
            avatar_name: "X",
            ..Default::default()
        };
        let out0 = crate::tlk_runner::run_tlk_stream(&fields[0], &inputs);
        let out1 = crate::tlk_runner::run_tlk_stream(&fields[1], &inputs);
        assert_eq!(out0.text, "Ada");
        assert_eq!(out1.text, "smith");
    }

    #[test]
    fn talk_response_text_and_actions_strips_action_markers() {
        assert_eq!(
            talk_response_text_and_actions("Take this {ACTION:F} friend"),
            ("Take this friend".to_string(), vec!['F'])
        );
    }

    #[test]
    fn talk_branch_flags_use_32_bit_scene_slot_and_zero_mask_out_of_range() {
        assert_eq!(talk_branch_flag_mask(0), 1);
        assert_eq!(talk_branch_flag_mask(31), 0x8000_0000);
        assert_eq!(talk_branch_flag_mask(32), 0);
        assert_eq!(talk_branch_flag_mask(255), 0);

        let mut slot = 0u32;
        assert!(!talk_branch_flag_is_set(slot, 5));
        assert!(set_talk_branch_flag(&mut slot, 5));
        assert_eq!(slot, 0x20);
        assert!(talk_branch_flag_is_set(slot, 5));
        assert!(!set_talk_branch_flag(&mut slot, 5));
        assert_eq!(slot, 0x20);

        assert!(!set_talk_branch_flag(&mut slot, 32));
        assert_eq!(slot, 0x20);
        assert!(!talk_branch_flag_is_set(0xffff_ffff, 32));
    }

    #[test]
    fn play_state_keeps_talk_branch_flags_per_town_scene() {
        let mut state = test_state(open_grid(), 1, 1);
        let first_scene = match state.area {
            Area::Town { scene, .. } => scene,
            _ => unreachable!(),
        };
        let second_scene = Scene::new(first_scene.byte + 1).unwrap();

        assert_eq!(state.talk_branch_slot_for_scene(first_scene), 0);
        assert!(!state.active_talk_branch_flag_is_set(3));
        assert!(state.set_active_talk_branch_flag(3));
        assert!(!state.set_active_talk_branch_flag(3));
        assert!(state.active_talk_branch_flag_is_set(3));
        assert_eq!(state.talk_branch_slot_for_scene(first_scene), 0x08);
        assert_eq!(state.talk_branch_slot_for_scene(second_scene), 0);

        assert!(state.set_talk_branch_flag_for_scene(second_scene, 5));
        assert_eq!(state.talk_branch_slot_for_scene(first_scene), 0x08);
        assert_eq!(state.talk_branch_slot_for_scene(second_scene), 0x20);

        state.area = Area::Town {
            scene: second_scene,
            floor: 0,
        };
        assert!(!state.active_talk_branch_flag_is_set(3));
        assert!(state.active_talk_branch_flag_is_set(5));
    }

    #[test]
    fn play_state_talk_branch_flags_ignore_non_town_and_out_of_range_bits() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert!(!state.active_talk_branch_flag_is_set(0));
        assert!(!state.set_active_talk_branch_flag(0));

        let scene = Scene::new(0x11).unwrap();
        assert!(!state.set_talk_branch_flag_for_scene(scene, 32));
        assert_eq!(state.talk_branch_slot_for_scene(scene), 0);
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
    fn town_talk_action_dispatch_grants_confirmed_special_item_flags() {
        fn push_text(bytes: &mut Vec<u8>, text: &str) {
            for byte in text.bytes() {
                bytes.push(byte | 0x80);
            }
            bytes.push(0);
        }

        let mut bytes = vec![0; 8];
        bytes[0..2].copy_from_slice(&2u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&8u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&2u16.to_le_bytes());
        for field in ["Ada", "a test smith", "Greetings", "I mend gear", "Bye", "GIFT"] {
            push_text(&mut bytes, field);
        }
        push_text(&mut bytes, "Take this");
        let terminator = bytes.pop().unwrap();
        assert_eq!(terminator, 0);
        bytes.push(0x86);
        bytes.push(b'F' | 0x80);
        bytes.push(0x86);
        bytes.push(b'H' | 0x80);
        bytes.push(0x86);
        bytes.push(b'I' | 0x80);
        bytes.push(0x86);
        bytes.push(b'J' | 0x80);
        bytes.push(0);

        let dialogue = parse_tlk_bytes(&bytes).unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.climbing_gear = 0;
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
            state.talk_facing_with_dialogue_and_keyword(&dialogue, Some("gift")),
            MoveOutcome::Talked
        );

        assert_eq!(state.climbing_gear, 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_SEXTANT_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX], 1);
        assert_eq!(state.message, "Talked to Ada: Take this");
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
    fn play_input_talk_suffix_routes_reserved_aliases() {
        let dir = debug_game_dir();
        fs::write(
            dir.join("CASTLE.TLK"),
            tlk_bytes(&[(
                2,
                &["Ada", "a test smith", "Greetings", "I mend gear", "Farewell"],
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
            handle_play_key_input(&mut state, 'T', "WORK", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "Talked to Ada: I mend gear");

        assert_eq!(
            handle_play_key_input(&mut state, 'T', "THANK", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "Talked to Ada: Farewell");
        assert_eq!(state.turn, 2);
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
    fn town_talk_reports_public_shop_trigger_dispatch_family() {
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

        assert!(state.message.contains("Shipwright"));
        assert!(state.active_shop.is_some());
        assert!(!state.message.contains("out of scope"));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn town_talk_horse_mounted_refuses_non_horse_trader_shops() {
        // shops.md §2: ordinary shop arms refuse before opening their menu when
        // the party is mounted on a horse; only the 0x83 horse trader remains.
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.player.transport = TransportState::Horse {
            type_byte: FIRST_PLAYABLE_HORSE_TILE,
            tile: FIRST_PLAYABLE_HORSE_TILE,
        };
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
                dialog_id: 0x85, // herbalist
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Blocked
        );

        assert!(state.message.contains("Herbalist"));
        assert!(state.message.contains("horseback"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_talk_horse_mounted_still_reaches_horse_trader() {
        // shops.md §2: the 0x83 horse-trader vehicle-sale arm remains
        // available while mounted.
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.player.transport = TransportState::Horse {
            type_byte: FIRST_PLAYABLE_HORSE_TILE,
            tile: FIRST_PLAYABLE_HORSE_TILE,
        };
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
                dialog_id: 0x83, // horse trader
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("Horse Trader"));
        assert!(state.active_shop.is_some());
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn open_conversation_session_renders_greeting_and_stores_session() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
            ],
        );
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0x10,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        let greeting = state.open_conversation_session(&dialogue, &raw);
        assert!(greeting.is_some());
        assert!(state.message.contains("Greetings"));
        assert!(state.active_conversation.is_some());
    }

    #[test]
    fn submit_conversation_keyword_returns_job_response() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
            ],
        );
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0x10,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        state.open_conversation_session(&dialogue, &raw);
        let (text, ended) = state.submit_active_conversation_keyword("job");
        assert!(text.contains("read books"));
        assert!(!ended);
    }

    #[test]
    fn submit_conversation_keyword_bye_ends_session() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a sage".to_string(),
                "Greetings".to_string(),
                "books".to_string(),
                "Farewell".to_string(),
            ],
        );
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a sage"),
                enc("Greetings"),
                enc("books"),
                enc("Farewell"),
            ],
        );
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0x10,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        state.open_conversation_session(&dialogue, &raw);
        let (text, ended) = state.submit_active_conversation_keyword("bye");
        assert!(text.contains("Farewell"));
        assert!(ended);
        assert!(state.active_conversation.is_none());
    }

    #[test]
    fn end_to_end_innkeeper_session_through_input_dispatcher() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.gold = 100;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0x88,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        // Talk opens the inn.
        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );
        assert!(state.active_shop.is_some());
        // First key 'Y' (greeting accept) → ConfirmRoom.
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert!(state.message.contains("room"));
        // 'Y' again to confirm.
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert!(state.message.contains("Rented"));
        assert!(state.gold < 100);
    }

    #[test]
    fn end_to_end_innkeeper_decline_returns_to_greeting_without_charge() {
        use crate::shop_runtime::*;
        use crate::shop_session::ActiveShopSession;
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.active_shop = Some(ActiveShopSession::Innkeeper(
            InnkeeperState::ConfirmRoom { cost: 25 },
        ));
        // Pass 'n' via the suffix so the bare-N New-Order intercept in
        // the outer dispatcher does not eat the key before the shop
        // session sees it.
        handle_play_key_input(&mut state, ' ', "n", Path::new("")).unwrap();
        assert_eq!(state.gold, 100);
        assert!(
            state.message.contains("As you wish")
                || state.message.contains("Farewell"),
            "decline message was: {}",
            state.message
        );
    }

    #[test]
    fn end_to_end_arms_shop_exit_clears_session() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot { slot: 1, type_byte: 1, dialog_id: 0x81, schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20], name: None },
        ]);
        state.talk_facing_with_dialogue(&dialogue);
        assert!(state.active_shop.is_some());
        // Space exits the arms shop greeting.
        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
        assert!(state.active_shop.is_none());
        assert!(state.message.contains("Farewell"));
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
        // Per animation.md Section 6: shared frame selector. Every cell
        // in the water family displays the same frame at each tick,
        // regardless of stored id.
        for frame in 0..4u8 {
            let clock = AnimationClock {
                frame,
                moongate_frame: 0,
            };
            let resolved: Vec<_> = (1u8..=3).map(|t| clock.resolve_static_tile(t)).collect();
            let expected_frame = 1 + (frame % 3);
            assert_eq!(
                resolved,
                vec![expected_frame; 3],
                "all water cells must share frame {expected_frame} at tick {frame}"
            );
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

