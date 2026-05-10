    #[test]
    fn pass_turn_repairs_missing_player_slot_zero() {
        let mut state = test_state(open_grid(), 4, 5);
        state.active_objects[0] = ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 9,
            y: 9,
            z: 3,
            phase: 0x22,
            aux1: 7,
            aux3: 8,
        };

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);

        assert_eq!(
            state.active_objects[0],
            ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: 4,
                y: 5,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
    }

    #[test]
    fn idle_tick_repairs_missing_player_slot_zero_without_turn() {
        let mut state = world_state(open_world_grid(), 4, 5);
        state.active_objects.clear();

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(
            state.active_objects,
            vec![ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: 4,
                y: 5,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }]
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
        assert_eq!(state.animation.frame, 1);
    }

    #[test]
    fn idle_tick_preserves_far_overworld_objects_without_turn() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 40,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 80,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
        assert_eq!(state.animation.frame, 1);
        assert_eq!(state.active_objects[1].type_byte, 192);
        assert_eq!(state.active_objects[1].x, 40);
        assert_eq!(state.active_objects[2].type_byte, 168);
        assert_eq!(state.active_objects[2].x, 80);
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn idle_tick_marks_visibility_dirty_for_active_object_frame_change() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.visibility_dirty = false;
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

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!(object.phase, 0x21);
        assert_eq!(object.tile, 169);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
        assert!(state.visibility_dirty);
    }

    #[test]
    fn space_command_routes_to_pass_in_top_down_and_dungeon_modes() {
        let mut town = test_state(open_grid(), 1, 1);
        town.clock = GameClock::new(17, 59).unwrap();

        assert!(
            town.handle_top_down_key_with_inline(' ', Path::new(""), None, None, None, None)
                .unwrap()
        );

        assert_eq!(town.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(town.turn, 1);
        assert_eq!(town.message, "Passed.");

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon.clock = GameClock::new(17, 59).unwrap();

        assert!(dungeon.handle_dungeon_key(' ', Path::new("")).unwrap());

        assert_eq!(dungeon.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(dungeon.turn, 1);
        assert_eq!(dungeon.message, "Passed.");
    }


    #[test]
    fn inline_spell_code_uses_order_insensitive_selector_letters() {
        assert_eq!(inline_spell_code("1LI"), "IL");
        assert_eq!(inline_spell_code("1SA"), "AS");
        assert_eq!(inline_spell_code("1RVP2"), "PRV");
        assert_eq!(inline_spell_code("1GFI6"), "FGI");
    }

    #[test]
    fn parse_inline_party_swap_uses_first_two_one_based_digits() {
        assert_eq!(parse_inline_party_swap("12"), Some((0, 1)));
        assert_eq!(parse_inline_party_swap("2/1"), Some((1, 0)));
        assert_eq!(parse_inline_party_swap("6 then 4"), Some((5, 3)));
        assert_eq!(parse_inline_party_swap("1"), None);
        assert_eq!(parse_inline_party_swap("01"), None);
        assert_eq!(parse_inline_party_swap(""), None);
    }

    #[test]
    fn inline_restore_target_uses_second_one_based_digit() {
        assert_eq!(parse_inline_target_party_index("1AN2"), Some(1));
        assert_eq!(parse_inline_target_party_index("6M4"), Some(3));
        assert_eq!(parse_inline_target_party_index("1AN"), None);
        assert_eq!(parse_inline_target_party_index("1AN0"), None);
    }

    #[test]
    fn parse_inline_mix_request_accepts_spell_mask_and_quantity() {
        assert_eq!(
            parse_inline_mix_request("IL/0x80/2").unwrap(),
            Some(InlineMixRequest {
                spell_index: Some(IN_LOR_SPELL_INDEX),
                reagent_mask: 0x80,
                amount: 2,
            })
        );
        assert_eq!(
            parse_inline_mix_request("LI/128/1").unwrap(),
            Some(InlineMixRequest {
                spell_index: Some(IN_LOR_SPELL_INDEX),
                reagent_mask: 0x80,
                amount: 1,
            })
        );
        assert_eq!(
            parse_inline_mix_request("ZZ/0x80/1").unwrap(),
            Some(InlineMixRequest {
                spell_index: None,
                reagent_mask: 0x80,
                amount: 1,
            })
        );
        assert_eq!(parse_inline_mix_request("").unwrap(), None);
        assert!(parse_inline_mix_request("IL/0x80").is_err());
        assert!(parse_inline_mix_request("IL/nope/1").is_err());
        assert!(parse_inline_mix_request("IL/0x80/many").is_err());
    }

    #[test]
    fn parse_shrine_entries_accepts_clean_britannia_rows() {
        let entries = parse_shrine_entries(
            "\
BRITANNIA 10 20 HONESTY 136
BRITANNIA 11 21 shrine:humility
",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                ShrineEntry {
                    plane: WorldPlane::Britannia,
                    x: 10,
                    y: 20,
                    virtue: ShrineVirtue::Honesty,
                    expected_tile: Some(136),
                },
                ShrineEntry {
                    plane: WorldPlane::Britannia,
                    x: 11,
                    y: 21,
                    virtue: ShrineVirtue::Humility,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_shrine_entries("UNDERWORLD 1 2 HONESTY\n").is_err());
        assert!(parse_shrine_entries("BRITANNIA 1 2 HONESTY\nBRITANNIA 1 2 VALOR\n").is_err());
        assert!(parse_shrine_entries("BRITANNIA 1 2 HONESTY\nBRITANNIA 3 4 HONESTY\n").is_err());
    }

    #[test]
    fn parse_inline_shrine_request_accepts_mantra_and_offering() {
        assert_eq!(
            parse_inline_shrine_request("Ahm").unwrap(),
            Some(InlineShrineRequest {
                mantra: "Ahm".to_string(),
                offering: None,
            })
        );
        assert_eq!(
            parse_inline_shrine_request("Summ/7").unwrap(),
            Some(InlineShrineRequest {
                mantra: "Summ".to_string(),
                offering: Some(7),
            })
        );
        assert_eq!(parse_inline_shrine_request("").unwrap(), None);
        assert!(parse_inline_shrine_request("Ahm/many").is_err());
        assert!(parse_inline_shrine_request("Ahm/1/2").is_err());
        assert!(inline_mix_candidate("IL/0x80/1"));
        assert!(!inline_mix_candidate("Ahm/1"));
    }

    #[test]
    fn inline_field_direction_uses_last_numeric_cardinal() {
        assert_eq!(
            parse_inline_cardinal_direction("1GIN6"),
            Some(Direction::East)
        );
        assert_eq!(
            parse_inline_cardinal_direction("1GIZ4"),
            Some(Direction::West)
        );
        assert_eq!(
            parse_inline_cardinal_direction("1FGI62"),
            Some(Direction::South)
        );
        assert_eq!(parse_inline_cardinal_direction("1FGI"), None);
        assert_eq!(parse_inline_cardinal_direction("1FGI3"), None);
    }

    #[test]
    fn parse_blink_target_entries_accepts_same_map_targets() {
        let entries = parse_blink_target_entries(
            "\
BRITANNIA 0 5 5 E 7 5 16 16
CASTLE:0 0 1 1 6 3 1 * 16
DUNGEON:0 4 1 1 WEST 0 1 0x00 0x08
",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                BlinkTargetEntry {
                    target: PlayTarget::World(WorldPlane::Britannia),
                    floor: 0,
                    from_x: 5,
                    from_y: 5,
                    direction: Direction::East,
                    to_x: 7,
                    to_y: 5,
                    expected_from_tile: Some(16),
                    expected_to_tile: Some(16),
                },
                BlinkTargetEntry {
                    target: PlayTarget::Town(Scene::new(17).unwrap()),
                    floor: 0,
                    from_x: 1,
                    from_y: 1,
                    direction: Direction::East,
                    to_x: 3,
                    to_y: 1,
                    expected_from_tile: None,
                    expected_to_tile: Some(16),
                },
                BlinkTargetEntry {
                    target: PlayTarget::Dungeon(DungeonScene::new(33).unwrap()),
                    floor: 4,
                    from_x: 1,
                    from_y: 1,
                    direction: Direction::West,
                    to_x: 0,
                    to_y: 1,
                    expected_from_tile: Some(0),
                    expected_to_tile: Some(8),
                },
            ]
        );
        assert!(parse_blink_target_entries("BRITANNIA -1 0 0 E 1 0\n").is_err());
        assert!(parse_blink_target_entries("CASTLE:0 0 32 1 E 3 1\n").is_err());
        assert!(parse_blink_target_entries("DUNGEON:0 8 1 1 E 2 1\n").is_err());
    }

    #[test]
    fn cast_in_lor_sets_light_counter_and_consumes_charge_mana_and_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
        state.party[0].mana = 1;
        state.party[0].level = 1;
        state.ambient_light = FULL_DARKNESS;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.light_spell_counter, IN_LOR_LIGHT_DURATION);
        assert_eq!(state.ambient_light, LIGHT_SPELL_FLOOR);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Light!");
    }

    #[test]
    fn cast_active_effect_spells_set_visible_tag_and_counter() {
        let cases = [
            (
                "1IS",
                PROTECTION_SPELL_INDEX,
                PROTECTION_COST,
                PROTECTION_ACTIVE_EFFECT_TAG,
                PROTECTION_ACTIVE_EFFECT_DURATION,
                "Protection!",
            ),
            (
                "1RT",
                QUICKNESS_SPELL_INDEX,
                QUICKNESS_COST,
                QUICKNESS_ACTIVE_EFFECT_TAG,
                QUICKNESS_ACTIVE_EFFECT_DURATION,
                "Quickness!",
            ),
            (
                "1AI",
                NEGATE_MAGIC_SPELL_INDEX,
                NEGATE_MAGIC_COST,
                NEGATE_MAGIC_ACTIVE_EFFECT_TAG,
                NEGATE_MAGIC_ACTIVE_EFFECT_DURATION,
                "Negate magic!",
            ),
        ];

        for (suffix, spell_index, cost, tag, duration, message) in cases {
            let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
            state.spell_charges[spell_index] = 1;
            state.party[0].mana = cost + 1;
            state.party[0].level = cost;

            assert_eq!(
                handle_play_key_input(&mut state, 'C', suffix, Path::new("")).unwrap(),
                PlayInputDisposition::Continue
            );

            assert_eq!(state.spell_charges[spell_index], 0);
            assert_eq!(state.party[0].mana, 1);
            assert_eq!(state.active_effect_tag, Some(tag));
            assert_eq!(state.active_effect_counter, duration);
            assert_eq!(state.turn, 1);
            assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
            assert_eq!(state.message, message);
            assert!(state.z_stats_message().contains(&format!(
                "effect={}/{}",
                char::from(tag),
                duration
            )));
        }
    }

    #[test]
    fn active_effect_counter_ages_and_clears_on_consumed_turns() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 2;

        state.advance_turn();

        assert_eq!(state.active_effect_tag, Some(PROTECTION_ACTIVE_EFFECT_TAG));
        assert_eq!(state.active_effect_counter, 1);

        state.advance_turn();

        assert_eq!(state.active_effect_tag, None);
        assert_eq!(state.active_effect_counter, 0);
        assert_eq!(state.active_effect_status(), "none");
    }

    #[test]
    fn cast_active_effect_resource_gate_precedes_state_change() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.spell_charges[NEGATE_MAGIC_SPELL_INDEX] = 1;
        state.party[0].mana = NEGATE_MAGIC_COST - 1;
        state.party[0].level = NEGATE_MAGIC_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[NEGATE_MAGIC_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, NEGATE_MAGIC_COST - 1);
        assert_eq!(state.active_effect_tag, None);
        assert_eq!(state.active_effect_counter, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "M.P. too low!");
    }

    #[test]
    fn cast_in_wis_reports_world_location_and_consumes_resources() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.player.facing = Direction::East;
        state.wind = WindState::North;
        state.spell_charges[IN_WIS_SPELL_INDEX] = 1;
        state.party[0].mana = IN_WIS_COST;
        state.party[0].level = IN_WIS_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1WI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[IN_WIS_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(
            state.message,
            "Locate: BRITANNIA at (5, 5), facing East, wind North Winds, time 12:02."
        );
    }

    #[test]
    fn cast_in_wis_scene_gate_precedes_charge_consumption() {
        let mut state = test_state(open_grid(), 5, 5);
        state.spell_charges[IN_WIS_SPELL_INDEX] = 1;
        state.party[0].mana = IN_WIS_COST;
        state.party[0].level = IN_WIS_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IW", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[IN_WIS_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, IN_WIS_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_unimplemented_out_of_scene_spell_reports_not_here_without_resources() {
        let mut state = test_state(open_grid(), 1, 1);
        let spell_index = spell_index_from_code("GP").unwrap();
        state.spell_charges[spell_index] = 1;
        state.party[0].mana = 1;
        state.party[0].level = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1GP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[spell_index], 1);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_unimplemented_allowed_spell_still_reports_no_effect() {
        let mut state = test_state(open_grid(), 1, 1);
        let spell_index = spell_index_from_code("AY").unwrap();
        state.spell_charges[spell_index] = 1;
        state.party[0].mana = 1;
        state.party[0].level = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AY", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[spell_index], 1);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "No effect!");
    }

    #[test]
    fn cast_create_food_increases_food_and_consumes_resources() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.food = 12;
        state.spell_charges[CREATE_FOOD_SPELL_INDEX] = 1;
        state.party[0].mana = CREATE_FOOD_COST + 1;
        state.party[0].level = CREATE_FOOD_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IMX", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.food, 12 + CREATE_FOOD_AMOUNT);
        assert_eq!(state.spell_charges[CREATE_FOOD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(
            state.message,
            format!(
                "Created {CREATE_FOOD_AMOUNT} food; stock is {}.",
                12 + CREATE_FOOD_AMOUNT
            )
        );
    }

    #[test]
    fn cast_create_food_resource_gate_precedes_food_change() {
        let mut state = test_state(open_grid(), 5, 5);
        state.food = 12;
        state.party[0].mana = CREATE_FOOD_COST;
        state.party[0].level = CREATE_FOOD_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IMX", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.food, 12);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "None mixed!");
    }

    #[test]
    fn cast_peer_reports_map_without_requiring_gems() {
        let mut state = test_state(open_grid(), 5, 5);
        state.gems = 0;
        state.spell_charges[PEER_SPELL_INDEX] = 1;
        state.party[0].mana = PEER_COST + 1;
        state.party[0].level = PEER_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1QWI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.gems, 0);
        assert_eq!(state.spell_charges[PEER_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.message.contains("Peer view of CASTLE:0 floor 0"));
        assert!(state.message.contains("spell; full-fill 11x11 map"));
        assert!(state.message.contains('@'));
    }

    #[test]
    fn cast_peer_resource_gate_precedes_map_output() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.gems = 0;
        state.party[0].mana = PEER_COST;
        state.party[0].level = PEER_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IQW", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.gems, 0);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "None mixed!");
    }

    #[test]
    fn cast_x_ray_reports_surface_view_without_requiring_gems() {
        let mut state = test_state(open_grid(), 5, 5);
        state.gems = 0;
        state.spell_charges[X_RAY_SPELL_INDEX] = 1;
        state.party[0].mana = X_RAY_COST + 1;
        state.party[0].level = X_RAY_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AWY", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.gems, 0);
        assert_eq!(state.spell_charges[X_RAY_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.message.contains("X-Ray view of CASTLE:0 floor 0"));
        assert!(state.message.contains("first-playable full-fill 11x11 map"));
        assert!(state.message.contains('@'));
    }

    #[test]
    fn cast_x_ray_scene_gate_precedes_charge_consumption() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.spell_charges[X_RAY_SPELL_INDEX] = 1;
        state.party[0].mana = X_RAY_COST;
        state.party[0].level = X_RAY_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AWY", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[X_RAY_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, X_RAY_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn mix_reagents_exact_recipe_debits_and_caps_charges() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 5;
        state.spell_charges[IN_LOR_SPELL_INDEX] = 98;

        assert_eq!(
            state.mix_reagents_from_suffix("IL/0x80/3"),
            MoveOutcome::Cast
        );

        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 2);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 99);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Mixed 1 IL charge; stock is 99.");
    }

    #[test]
    fn mix_reagents_wrong_recipe_spends_selected_without_charge() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 1;
        state.reagents[REAGENT_BLOOD_MOSS] = 1;

        assert_eq!(
            state.mix_reagents_from_suffix("AS/0x80/1"),
            MoveOutcome::Blocked
        );

        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 0);
        assert_eq!(state.reagents[REAGENT_BLOOD_MOSS], 1);
        assert_eq!(state.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(
            state.message,
            "Mixed wrong reagents for AS; no spell charges added."
        );
    }

    #[test]
    fn mix_reagents_refuses_empty_zero_and_insufficient_without_debit() {
        let mut empty = test_state(open_grid(), 1, 1);
        empty.reagents = [0; REAGENT_COUNT];
        assert_eq!(
            empty.mix_reagents_from_suffix("IL/0x80/1"),
            MoveOutcome::Blocked
        );
        assert_eq!(empty.message, "No reagents owned!");

        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 1;

        assert_eq!(
            state.mix_reagents_from_suffix("IL/0x80/0"),
            MoveOutcome::PromptDeclined
        );
        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 1);
        assert_eq!(state.message, "None!");

        assert_eq!(
            state.mix_reagents_from_suffix("IL/0/1"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 1);
        assert_eq!(state.message, "Nothing to mix!");

        assert_eq!(
            state.mix_reagents_from_suffix("IL/0x80/2"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 1);
        assert_eq!(state.message, "Insufficient reagents!");
    }

    #[test]
    fn handle_play_key_input_routes_inline_mix() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "IL/0x80/1", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 0);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Mixed 1 IL charge; stock is 1.");
    }

    #[test]
    fn shrine_meditation_sets_ordained_bit_from_clean_sidecar() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Ahm", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.shrine_ordained_mask, ShrineVirtue::Honesty.bit());
        assert_eq!(state.shrine_codex_mask, 0);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Shrine of Honesty"));
        assert!(state.message.contains("ordained"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_meditation_wrong_mantra_and_tile_guard_do_not_mutate_state() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Mu", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.shrine_ordained_mask, 0);
        assert_eq!(state.message, "No effect!");

        state.grid[world_cell_index(10, 20)] = 5;
        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Ahm", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.shrine_ordained_mask, 0);
        assert!(state.message.contains("Mix syntax"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_codex_turn_in_clears_ordained_and_rewards_avatar_stats() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 JUSTICE 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);
        let bit = ShrineVirtue::Justice.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.avatar_stats = AvatarStats {
            strength: 20,
            dexterity: 20,
            intelligence: 20,
        };
        state.party[0].climb_stat = 20;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Beh", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.shrine_ordained_mask, 0);
        assert_eq!(state.shrine_codex_mask, bit);
        assert_eq!(state.shrine_standing[ShrineVirtue::Justice.index()], 3);
        assert_eq!(
            state.avatar_stats,
            AvatarStats {
                strength: 20,
                dexterity: 21,
                intelligence: 21,
            }
        );
        assert_eq!(state.party[0].climb_stat, 21);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Completed the Shrine of Justice"));
        assert!(state.message.contains("DEX +1, INT +1"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn completed_shrine_offering_costs_gold_and_clamps_standing() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SHRINE_TABLE_FILE),
            "BRITANNIA 10 20 COMPASSION 136\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);
        let virtue = ShrineVirtue::Compassion;
        state.shrine_codex_mask = virtue.bit();
        state.shrine_standing[virtue.index()] = 98;
        state.gold = 350;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Mu/2", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.gold, 150);
        assert_eq!(state.shrine_standing[virtue.index()], SHRINE_STANDING_MAX);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Offered 200 gold"));
        assert!(state.message.contains("standing +1 to 99"));

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Mu/9", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.gold, 150);
        assert_eq!(state.message, "Need 900 gold for offering.");
        let _ = fs::remove_dir_all(dir);
    }

