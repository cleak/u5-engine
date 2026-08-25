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
                phase: 0x22,
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
    fn inline_combat_actor_target_uses_second_one_based_number() {
        assert_eq!(parse_inline_combat_actor_slot("1GP7"), Some(6));
        assert_eq!(parse_inline_combat_actor_slot("1GP32"), Some(31));
        assert_eq!(parse_inline_combat_actor_slot("1GP33"), None);
        assert_eq!(parse_inline_combat_actor_slot("1GP"), None);
        assert_eq!(parse_inline_combat_actor_slot("GP7"), None);
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
    fn parse_codex_urn_entries_accepts_world_rows() {
        let entries = parse_codex_urn_entries(
            "\
UNDERWORLD 10 20 136
BRITANNIA 11 21
",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                CodexUrnEntry {
                    plane: WorldPlane::Underworld,
                    x: 10,
                    y: 20,
                    expected_tile: Some(136),
                },
                CodexUrnEntry {
                    plane: WorldPlane::Britannia,
                    x: 11,
                    y: 21,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_codex_urn_entries("MOON 1 2\n").is_err());
        assert!(parse_codex_urn_entries("BRITANNIA 1 2\nBRITANNIA 1 2 136\n").is_err());
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
    fn mass_charm_active_effect_spell_preserves_combat_only_scene_gate() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.spell_charges[MASS_CHARM_SPELL_INDEX] = 1;
        state.party[0].mana = MASS_CHARM_COST;
        state.party[0].level = MASS_CHARM_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AQW", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[MASS_CHARM_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, MASS_CHARM_COST);
        assert_eq!(state.active_effect_tag, None);
        assert_eq!(state.active_effect_counter, 0);
        assert_eq!(state.message, "Not here!");
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
    fn active_effect_counter_255_is_inert_at_aging_endpoint() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = u8::MAX;

        state.advance_turn();

        assert_eq!(state.active_effect_tag, Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG));
        assert_eq!(state.active_effect_counter, u8::MAX);
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
        let mut state = britannia_state(open_world_grid(), 0x23, 0xaf);
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
        assert_eq!(state.message, "Locate: K'P,C'D\"");
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
    fn cast_scene_absorption_precedes_scene_resources_and_turn() {
        let mut stonegate = test_state(open_grid(), 5, 5);
        stonegate.area = Area::Town {
            scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
            floor: 0,
        };
        stonegate.spell_charges[IN_LOR_SPELL_INDEX] = 1;
        stonegate.party[0].mana = IN_LOR_COST;
        stonegate.party[0].level = IN_LOR_COST;

        assert_eq!(
            handle_play_key_input(&mut stonegate, 'C', "1IL", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(stonegate.spell_charges[IN_LOR_SPELL_INDEX], 1);
        assert_eq!(stonegate.party[0].mana, IN_LOR_COST);
        assert_eq!(stonegate.light_spell_counter, 0);
        assert_eq!(stonegate.turn, 0);
        assert_eq!(stonegate.message, "Absorbed!");

        let mut blackthorn = test_state(open_grid(), 5, 5);
        blackthorn.area = Area::Town {
            scene: Scene::new(LORD_BLACKTHORN_CASTLE_SCENE_BYTE).unwrap(),
            floor: 0,
        };
        let gp = spell_index_from_code("GP").unwrap();
        blackthorn.spell_charges[gp] = 1;
        blackthorn.party[0].mana = 1;
        blackthorn.party[0].level = 1;

        assert_eq!(
            handle_play_key_input(&mut blackthorn, 'C', "1GP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(blackthorn.spell_charges[gp], 1);
        assert_eq!(blackthorn.party[0].mana, 1);
        assert_eq!(blackthorn.turn, 0);
        assert_eq!(blackthorn.message, "Absorbed!");
    }

    #[test]
    fn blackthorn_cast_absorption_clears_after_crown_ownership() {
        let mut state = test_state(open_grid(), 5, 5);
        state.area = Area::Town {
            scene: Scene::new(LORD_BLACKTHORN_CASTLE_SCENE_BYTE).unwrap(),
            floor: 0,
        };
        state.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] = 1;
        state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
        state.party[0].mana = IN_LOR_COST;
        state.party[0].level = IN_LOR_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IL", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.light_spell_counter, IN_LOR_LIGHT_DURATION);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Light!");
    }

    #[test]
    fn cast_absorption_does_not_mask_missing_caster_or_unknown_spell() {
        let mut missing_caster = test_state(open_grid(), 5, 5);
        missing_caster.area = Area::Town {
            scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
            floor: 0,
        };
        missing_caster.spell_charges[IN_LOR_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut missing_caster, 'C', "IL", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_caster.spell_charges[IN_LOR_SPELL_INDEX], 1);
        assert_eq!(
            missing_caster.message,
            "Who casts? Use C1IL for party slot 1."
        );

        let mut unknown = test_state(open_grid(), 5, 5);
        unknown.area = Area::Town {
            scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
            floor: 0,
        };

        assert_eq!(
            handle_play_key_input(&mut unknown, 'C', "1ZZ", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(unknown.message, "No effect!");
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
    fn cast_scene_gate_runs_before_resources_for_implemented_noncombat_spells() {
        let mut peer = test_state(open_grid(), 1, 1);
        peer.combat_active = true;
        peer.spell_charges[PEER_SPELL_INDEX] = 1;
        peer.party[0].mana = PEER_COST;
        peer.party[0].level = PEER_COST;

        assert_eq!(
            handle_play_key_input(&mut peer, 'C', "1IQW", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(peer.spell_charges[PEER_SPELL_INDEX], 1);
        assert_eq!(peer.party[0].mana, PEER_COST);
        assert_eq!(peer.turn, 0);
        assert_eq!(peer.message, "Not here!");

        let mut resurrect = test_state(open_grid(), 1, 1);
        resurrect.combat_active = true;
        resurrect.spell_charges[RESURRECT_SPELL_INDEX] = 1;
        resurrect.party[0].mana = RESURRECT_COST;
        resurrect.party[0].level = RESURRECT_COST;
        resurrect.party.push(PartyMember {
            slot: 1,
            class_byte: b'M',
            status: b'G',
            climb_stat: 10,
            mana: 0,
            hp: 0,
            max_hp: 20,
            level: 1,
        });
        resurrect.party[1].status = b'D';

        assert_eq!(
            handle_play_key_input(&mut resurrect, 'C', "1CIM2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(resurrect.spell_charges[RESURRECT_SPELL_INDEX], 1);
        assert_eq!(resurrect.party[0].mana, RESURRECT_COST);
        assert_eq!(resurrect.party[1].status, b'D');
        assert_eq!(resurrect.turn, 0);
        assert_eq!(resurrect.message, "Not here!");
    }

    #[test]
    fn spell_scene_bits_match_published_mask_values() {
        // `catalogs/spell-list.md §4` corrected legend: `0x01` C,
        // `0x02` D, `0x04` I, `0x08` O. The transposed `0x01` dungeon /
        // `0x02` combat legend this test used to assert is withdrawn.
        assert_eq!(SPELL_SCENE_BIT_COMBAT, 0x01);
        assert_eq!(SPELL_SCENE_BIT_DUNGEON, 0x02);
        assert_eq!(SPELL_SCENE_BIT_INDOOR, 0x04);
        assert_eq!(SPELL_SCENE_BIT_OVERWORLD, 0x08);

        let gp = spell_index_from_code("GP").unwrap();
        assert_eq!(SPELL_SCENE_MASKS[gp], SPELL_SCENE_BIT_COMBAT);
        assert!(!spell_allowed_in_area(gp, test_state(open_grid(), 1, 1).area));

        let fgi = spell_index_from_code("FGI").unwrap();
        assert_eq!(
            SPELL_SCENE_MASKS[fgi],
            SPELL_SCENE_BIT_COMBAT | SPELL_SCENE_BIT_DUNGEON
        );
        assert!(spell_allowed_in_area(
            fgi,
            dungeon_state(open_dungeon_record(), 0, 1, 1).area
        ));
    }

    #[test]
    fn cast_vanish_rewrites_exact_live_tile_without_removing_dynamic_object() {
        let mut grid = open_grid();
        grid[1 * 32 + 2] = 0x90;
        let mut state = test_state(grid, 1, 1);
        let spell_index = spell_index_from_code("AY").unwrap();
        state.spell_charges[spell_index] = 1;
        state.party[0].mana = 1;
        state.party[0].level = 1;
        state.active_objects.push(ActiveObject {
            type_byte: 0x40,
            tile: 0x40,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AY6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.active_objects[1].tile, 0x40);
        assert_eq!(state.grid[1 * 32 + 2], VANISH_CLEARED_TILE);
        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "POOF!");
    }

    #[test]
    fn cast_vanish_spends_before_direction_prompt_and_fails_after_target_miss() {
        let mut missing_direction = test_state(open_grid(), 1, 1);
        let spell_index = spell_index_from_code("AY").unwrap();
        missing_direction.spell_charges[spell_index] = 1;
        missing_direction.party[0].mana = 1;
        missing_direction.party[0].level = 1;

        assert_eq!(
            handle_play_key_input(&mut missing_direction, 'C', "1AY", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_direction.spell_charges[spell_index], 0);
        assert_eq!(missing_direction.party[0].mana, 0);
        assert_eq!(missing_direction.turn, 0);
        assert_eq!(
            missing_direction.message,
            "Direction? Use C1AY8/C1AY6/C1AY2/C1AY4."
        );

        let mut no_object = test_state(open_grid(), 1, 1);
        no_object.spell_charges[spell_index] = 1;
        no_object.party[0].mana = 1;
        no_object.party[0].level = 1;

        assert_eq!(
            handle_play_key_input(&mut no_object, 'C', "1AY6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(no_object.spell_charges[spell_index], 0);
        assert_eq!(no_object.party[0].mana, 0);
        assert_eq!(no_object.turn, 1);
        assert_eq!(no_object.message, "Failed!");
    }

    #[test]
    fn cast_vanish_protects_npcs_vehicles_and_moonstones() {
        let spell_index = spell_index_from_code("AY").unwrap();
        for object in [
            ActiveObject {
                type_byte: 0xc0,
                tile: 0xc0,
                x: 2,
                y: 1,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            },
            ActiveObject {
                type_byte: FIRST_PLAYABLE_SKIFF_TILE,
                tile: FIRST_PLAYABLE_SKIFF_TILE,
                x: 2,
                y: 1,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            },
            ActiveObject::moonstone_pickup(0, 2, 1, 0),
        ] {
            let mut state = test_state(open_grid(), 1, 1);
            state.spell_charges[spell_index] = 1;
            state.party[0].mana = 1;
            state.party[0].level = 1;
            state.active_objects.push(object);

            assert_eq!(
                handle_play_key_input(&mut state, 'C', "1AY6", Path::new("")).unwrap(),
                PlayInputDisposition::Continue
            );

            assert_eq!(state.active_objects[1], object);
            assert_eq!(state.spell_charges[spell_index], 0);
            assert_eq!(state.party[0].mana, 0);
            assert_eq!(state.turn, 1);
            assert_eq!(state.message, "Failed!");
        }
    }

    #[test]
    fn cast_create_food_increases_food_and_consumes_resources() {
        // `cleak/u5-spec#49`: per-cast grant is uniform `1..=3`.
        // Cast once and assert the grant range, spell-list
        // consumption, and turn tick.
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.food = 12;
        state.spell_charges[CREATE_FOOD_SPELL_INDEX] = 1;
        state.party[0].mana = CREATE_FOOD_COST + 1;
        state.party[0].level = CREATE_FOOD_COST;

        let food_before = state.food;
        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IMX", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        let created = state.food - food_before;
        assert!(
            (CREATE_FOOD_MIN_GRANT..=CREATE_FOOD_MAX_GRANT).contains(&created),
            "Create Food roll {created} outside spec range 1..=3"
        );
        assert_eq!(state.spell_charges[CREATE_FOOD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(
            state.message,
            format!("Created {created} food; stock is {}.", food_before + created)
        );
    }

    #[test]
    fn cast_create_food_minimum_roll_still_consumes_resources() {
        let mut min_seed = None;
        for candidate in 0..=u16::MAX {
            let mut prng = candidate;
            if u5_prng_range_u16(&mut prng, CREATE_FOOD_MIN_GRANT, CREATE_FOOD_MAX_GRANT)
                == CREATE_FOOD_MIN_GRANT
            {
                min_seed = Some(candidate);
                break;
            }
        }
        let min_seed = min_seed.expect("PRNG should be able to roll minimum for Create Food");

        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.food = 77;
        state.prng_state = min_seed;
        state.spell_charges[CREATE_FOOD_SPELL_INDEX] = 1;
        state.party[0].mana = CREATE_FOOD_COST;
        state.party[0].level = CREATE_FOOD_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IMX", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.food, 77 + CREATE_FOOD_MIN_GRANT);
        assert_eq!(state.spell_charges[CREATE_FOOD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(
            state.message,
            format!(
                "Created {} food; stock is {}.",
                CREATE_FOOD_MIN_GRANT,
                77 + CREATE_FOOD_MIN_GRANT
            )
        );
    }

    #[test]
    fn cast_create_food_clamps_to_party_food_cap() {
        // `cleak/u5-spec#49`: even at one below the cap, the
        // guaranteed-positive grant must clamp at [`PARTY_FOOD_CAP`]
        // without overflow.
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.food = PARTY_FOOD_CAP - 1;
        state.prng_state = 1;
        state.spell_charges[CREATE_FOOD_SPELL_INDEX] = 1;
        state.party[0].mana = CREATE_FOOD_COST;
        state.party[0].level = CREATE_FOOD_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IMX", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.food, PARTY_FOOD_CAP);
        assert_eq!(state.spell_charges[CREATE_FOOD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn cast_create_food_roll_stays_within_spec_range_across_many_casts() {
        // `cleak/u5-spec#49`: verify the uniform `1..=3` grant by
        // sampling many independent casts and confirming every roll
        // lands in the published range.
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.food = 0;
        state.spell_charges[CREATE_FOOD_SPELL_INDEX] = 200;
        state.party[0].mana = (CREATE_FOOD_COST as u16 * 200).min(255) as u8;
        state.party[0].level = CREATE_FOOD_COST;
        let mut seen = [false; 4];
        for _ in 0..200 {
            let before = state.food;
            state.party[0].mana = CREATE_FOOD_COST + 1;
            state.spell_charges[CREATE_FOOD_SPELL_INDEX] = 1;
            handle_play_key_input(&mut state, 'C', "1IMX", Path::new("")).unwrap();
            let grant = state.food - before;
            assert!(grant >= CREATE_FOOD_MIN_GRANT);
            assert!(grant <= CREATE_FOOD_MAX_GRANT);
            seen[grant as usize] = true;
        }
        // Every value in `1..=3` should appear over 200 samples.
        assert!(seen[1], "PRNG never rolled 1 over 200 casts");
        assert!(seen[2], "PRNG never rolled 2 over 200 casts");
        assert!(seen[3], "PRNG never rolled 3 over 200 casts");
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
        assert!(state.message.is_empty());
        let overlay = state.active_view_overlay.as_ref().unwrap();
        assert_eq!(
            overlay.title,
            "Peer view of CASTLE:0 floor 0 (spell; 32x32 class map)"
        );
        assert_eq!(overlay.kind, ViewOverlayKind::Surface);
        assert_eq!(overlay.mode, ViewOverlayMode::PeerSpell);
        assert!(overlay.text_map.contains('@'));
        let viewport = state
            .render_active_view_overlay(TileGraphicsDepth::Ega16)
            .unwrap();
        assert_eq!(viewport.cells_wide, LOCAL_VIEW_OVERLAY_SIDE);
        assert_eq!(viewport.cells_high, LOCAL_VIEW_OVERLAY_SIDE);
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
        assert!(state.active_view_overlay.is_none());
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
        assert!(state.message.is_empty());
        let overlay = state.active_view_overlay.as_ref().unwrap();
        assert_eq!(
            overlay.title,
            "X-Ray view of CASTLE:0 floor 0 (spell; 32x32 class map)"
        );
        assert_eq!(overlay.kind, ViewOverlayKind::Surface);
        assert_eq!(overlay.mode, ViewOverlayMode::XRaySpell);
        assert!(overlay.text_map.contains('@'));
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
        assert!(state.active_view_overlay.is_none());
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
            "Mixed wrong reagents for AS; no spell charges added.\nAcid trap hit party member 1 for 12 HP."
        );
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP - 12);
    }

    #[test]
    fn shared_trap_index_helpers_match_published_selection_and_damage_ranges() {
        let non_combat = (0..8)
            .map(|index| shared_trap_effect_id_from_index(index, false))
            .collect::<Vec<_>>();
        assert_eq!(non_combat, vec![0, 0, 0, 1, 1, 2, 2, 3]);

        let combat = (0..8)
            .map(|index| shared_trap_effect_id_from_index(index, true))
            .collect::<Vec<_>>();
        assert_eq!(combat, vec![0, 1, 0, 1, 0, 1, 0, 1]);

        assert_eq!(shared_trap_damage_from_index(0, TRAP_ACID_DAMAGE_MAX), 1);
        assert_eq!(
            shared_trap_damage_from_index(29, TRAP_ACID_DAMAGE_MAX),
            30
        );
        assert_eq!(
            shared_trap_damage_from_index(30, TRAP_ACID_DAMAGE_MAX),
            1
        );
        assert_eq!(shared_trap_damage_from_index(7, TRAP_BOMB_DAMAGE_MAX), 8);
    }

    #[test]
    fn shared_trap_effect_resolver_applies_published_effect_families() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 20,
                level: 1,
            },
        ];

        let effects = (0..8)
            .map(|turn| {
                state.turn = turn;
                state.shared_trap_effect_id(0)
            })
            .collect::<Vec<_>>();
        assert_eq!(effects, vec![0, 0, 0, 1, 1, 2, 2, 3]);

        state.turn = 0;
        state.active_player = Some(0);
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(0),
            "Acid trap hit party member 1 for 10 HP."
        );
        assert_eq!(state.party[0].hp, 0);
        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.active_player, None);

        // traps.md §3: effect id 1 is a poison primitive, not a revive.
        // Slot 0 was just killed by the acid roll, so the helper skips it
        // and leaves it Dead rather than rewriting it to Poisoned.
        state.turn = 3;
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(0),
            "Poison trap had no effect on party member 1."
        );
        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.party[0].hp, 0);

        state.turn = 5;
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(1),
            "Bomb trap dealt 7 HP across 1 party member(s)."
        );
        assert_eq!(state.party[1].hp, 3);

        // traps.md §3: effect id 3 poisons every living member of the
        // six-slot band. The two Dead slots (0 and 2) are skipped and
        // stay Dead; only the living slot 1 is rewritten.
        state.turn = 7;
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(0),
            "Gas trap poisoned 1 party member(s)."
        );
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.party[2].status, b'D');

        // A living, in-party member is rewritten to Poisoned, and the
        // helper touches nothing else - hit points are unchanged.
        state.party[0].hp = 10;
        state.party[0].status = b'G';
        state.turn = 3;
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(0),
            "Poison trap poisoned party member 1."
        );
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 10);
        assert_eq!(state.party[0].max_hp, 20);
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
    fn codex_urn_reads_first_ordained_virtue_from_clean_sidecar() {
        let dir = debug_game_dir();
        fs::write(dir.join(CODEX_URN_TABLE_FILE), "BRITANNIA 10 20 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);
        state.shrine_ordained_mask = ShrineVirtue::Honesty.bit() | ShrineVirtue::Justice.bit();
        state.shrine_codex_mask = 0;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.shrine_codex_mask, ShrineVirtue::Honesty.bit());
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Codex page for Honesty"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn codex_urn_suffix_routing_precedes_mix_reagents() {
        let dir = debug_game_dir();
        fs::write(dir.join(CODEX_URN_TABLE_FILE), "BRITANNIA 10 20 136\n").unwrap();
        let mut miscmsg = Vec::new();
        for index in 0..MISCMSG_DAT_RECORDS {
            if index == *MISCMSG_URN_CODEX_RANGE.start() + ShrineVirtue::Justice.index() {
                miscmsg.extend_from_slice(b"JUS@[_");
            } else {
                miscmsg.extend_from_slice(format!("rec{index}").as_bytes());
            }
            miscmsg.push(0);
        }
        fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 1;
        state.shrine_ordained_mask = ShrineVirtue::Justice.bit();

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "IL/0x80/1", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.shrine_codex_mask, ShrineVirtue::Justice.bit());
        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 1);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert!(state.message.contains("Codex page for Justice"));
        assert!(state.message.contains("JUS THER"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn codex_urn_no_ordained_and_completed_branches_do_not_stamp_new_bits() {
        let dir = debug_game_dir();
        fs::write(dir.join(CODEX_URN_TABLE_FILE), "BRITANNIA 10 20 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);
        state.shrine_codex_mask = ShrineVirtue::Valor.bit();

        assert_eq!(
            state.read_codex_urn_at_current_position(&dir).unwrap(),
            Some(MoveOutcome::Observed)
        );
        assert_eq!(state.shrine_codex_mask, ShrineVirtue::Valor.bit());
        assert!(state.message.contains("no ordained virtue"));

        state.shrine_ordained_mask = 0xFF;
        state.shrine_codex_mask = 0xFF;
        assert_eq!(
            state.read_codex_urn_at_current_position(&dir).unwrap(),
            Some(MoveOutcome::Observed)
        );
        assert_eq!(state.shrine_codex_mask, 0xFF);
        assert!(state.message.contains("already been read"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn codex_urn_tile_guard_falls_back_to_mix_prompt() {
        let dir = debug_game_dir();
        fs::write(dir.join(CODEX_URN_TABLE_FILE), "BRITANNIA 10 20 136\n").unwrap();
        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.shrine_codex_mask, 0);
        assert!(state.active_mix.is_some());
        assert!(state.message.contains(MMIX_SPELL_PROMPT_MESSAGE));
        let _ = fs::remove_dir_all(dir);
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
    fn shrine_meditation_sets_ordained_bit_from_native_altar_tile_without_sidecar() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_ALTAR_TILE_FIRST;
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
    fn active_shrine_prompt_uses_native_humility_altar_tile_without_sidecar() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_ALTAR_TILE_LAST;
        let mut state = britannia_state(grid, 10, 20);

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_shrine.is_some());
        assert!(state.active_mix.is_none());
        assert!(state.message.contains("Shrine of Humility mantra?"));

        assert_eq!(
            handle_play_key_input(&mut state, 'L', "um\r", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_shrine.is_none());
        assert_eq!(state.shrine_ordained_mask, ShrineVirtue::Humility.bit());
        assert_eq!(state.shrine_codex_mask, 0);
        assert!(state.message.contains("Shrine of Humility"));
        assert!(state.message.contains("ordained"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_sidecar_takes_precedence_over_native_altar_tile() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HUMILITY 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_ALTAR_TILE_FIRST;
        let mut state = britannia_state(grid, 10, 20);

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Lum", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.shrine_ordained_mask, ShrineVirtue::Humility.bit());
        assert_eq!(
            state.shrine_ordained_mask & ShrineVirtue::Honesty.bit(),
            0
        );
        assert!(state.message.contains("Shrine of Humility"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn native_shrine_altar_tiles_are_britannia_only() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_ALTAR_TILE_FIRST;
        let mut state = britannia_state(grid, 10, 20);
        state.area = Area::World {
            plane: WorldPlane::Underworld,
        };
        state.active_objects[0].z = WorldPlane::Underworld.save_floor();

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Ahm", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.shrine_ordained_mask, 0);
        assert!(state.active_shrine.is_none());
        assert!(!state.message.contains("Shrine of Honesty"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_shrine_prompt_collects_mantra_and_ordains() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_shrine.is_some());
        assert!(state.active_mix.is_none());
        assert!(state.message.contains("Shrine of Honesty mantra?"));

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "hm\r", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_shrine.is_none());
        assert_eq!(state.shrine_ordained_mask, ShrineVirtue::Honesty.bit());
        assert_eq!(state.shrine_codex_mask, 0);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("ordained"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_shrine_prompt_blank_mantra_has_no_effect() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);

        handle_play_key_input(&mut state, 'M', "", &dir).unwrap();
        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_shrine.is_none());
        assert_eq!(state.shrine_ordained_mask, 0);
        assert_eq!(state.shrine_codex_mask, 0);
        assert_eq!(state.message, "No effect!");
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
    fn completed_shrine_offering_costs_gold_and_clamps_moral_standing() {
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
        state.moral_standing = 98;
        state.gold = 350;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Mu/2", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.gold, 150);
        assert_eq!(state.moral_standing, MORAL_STANDING_MAX);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Offered 200 gold"));
        assert!(state.message.contains("moral +1 to 99"));

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Mu/9", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.gold, 150);
        assert_eq!(state.message, "Need 900 gold for offering.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_completed_shrine_prompt_collects_offering_digit() {
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
        state.gold = 350;

        handle_play_key_input(&mut state, 'M', "", &dir).unwrap();
        assert_eq!(
            handle_play_key_input(&mut state, 'M', "u\r", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_shrine.is_some());
        assert!(state.message.contains("Offering at the Shrine of Compassion?"));

        assert_eq!(
            handle_play_key_input(&mut state, '2', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_shrine.is_none());
        assert_eq!(state.gold, 150);
        assert_eq!(state.moral_standing, 2);
        assert_eq!(
            state.message,
            "Offered 200 gold at the Shrine of Compassion; moral +2 to 2."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_completed_shrine_offering_repeats_until_affordable_or_cancelled() {
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
        state.gold = 100;

        handle_play_key_input(&mut state, 'M', "", &dir).unwrap();
        handle_play_key_input(&mut state, 'M', "u\r", &dir).unwrap();
        assert_eq!(
            handle_play_key_input(&mut state, '9', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_shrine.is_some());
        assert_eq!(state.gold, 100);
        assert_eq!(state.moral_standing, 0);
        assert!(state.message.contains("Need 900 gold for offering."));

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_shrine.is_none());
        assert_eq!(state.gold, 0);
        assert_eq!(state.moral_standing, 1);
        assert!(state.message.contains("Offered 100 gold"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_codex_turn_in_raises_shared_moral_standing_by_three() {
        // karma.md §3-4: Codex turn-in adds +3 to the shared moral-standing
        // selector, clamped at 99.
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 JUSTICE 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);
        let bit = ShrineVirtue::Justice.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.moral_standing = 50;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Beh", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.moral_standing, 53);
        assert!(state.message.contains("moral +3 to 53"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_codex_humility_turn_in_raises_shared_moral_standing_by_six() {
        // karma.md §3-4: Humility receives an additional +3 on top of the
        // ordinary +3 Codex turn-in bonus to the shared moral-standing
        // selector.
        let dir = debug_game_dir();
        fs::write(
            dir.join(SHRINE_TABLE_FILE),
            "BRITANNIA 10 20 HUMILITY 136\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);
        let bit = ShrineVirtue::Humility.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.moral_standing = 40;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Lum", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.moral_standing, 46);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_codex_turn_in_clamps_shared_moral_standing_at_ninety_nine() {
        // karma.md §3: Shrine increases clamp the shared selector at 99.
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 JUSTICE 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = britannia_state(grid, 10, 20);
        let bit = ShrineVirtue::Justice.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.moral_standing = 98;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Beh", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.moral_standing, MORAL_STANDING_MAX);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_offering_raises_shared_moral_standing_by_offered_digit() {
        // karma.md §3-4: completed-shrine gold offering adds the offered
        // digit (1..9) to the shared moral-standing selector, clamped at 99.
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
        state.gold = 800;
        state.moral_standing = 20;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Mu/7", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.gold, 100);
        assert_eq!(state.moral_standing, 27);
        assert!(state.message.contains("moral +7 to 27"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn published_sound_boundaries_emit_ordered_typed_frontend_events() {
        let mut state = test_state(open_grid(), 4, 4);
        let serial = state.sound_effect_serial;

        assert!(state.apply_wind_state(WindState::North));
        let _ = state.apply_shared_trap_effect_to_slot(0);

        assert_eq!(
            state.sound_effects_after(serial),
            vec![PlaySoundEffect::WindChange, PlaySoundEffect::TrapSting]
        );
        assert_eq!(state.sound_effect_serial, serial + 2);
    }

