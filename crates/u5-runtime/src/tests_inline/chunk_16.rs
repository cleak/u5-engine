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
                // The repair preserves byte 6, but the turn's animator pass
                // reaches slot zero before it and spends one countdown step:
                // `active-objects.md §3` gate 2 is "decrement it and write the
                // recombined byte back. **This write is unconditional and has
                // no tile-class precondition**, and it applies to every slot
                // the walk reaches, **slot zero included**." A shipped save
                // carries zero here (`PLAYER_ACTIVE_OBJECT_PHASE`), which
                // routes past the decrement; this fixture's `0x22` is part of
                // the deliberately bogus record the repair is being tested on.
                phase: 0x21,
                // `active-objects.md §5`: bytes 5 and 7 round-trip on foot.
                aux1: 7,
                aux3: 8,
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
                phase: PLAYER_ACTIVE_OBJECT_PHASE,
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
        // Issue #298 Q29: class `0x5C` animates without the eligibility
        // coin, and script A's first step is f2, so a decision-point slot
        // deterministically changes frame on this tick.
        state.active_objects.push(ActiveObject {
            type_byte: 0x5c,
            tile: 0x5c,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x00,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 0x5c)
            .unwrap();
        assert_eq!(object.phase, 0x10);
        assert_eq!(object.tile, 0x5d);
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
        assert!(town.message.is_empty());

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon.clock = GameClock::new(17, 59).unwrap();

        assert!(dungeon.handle_dungeon_key(' ', Path::new("")).unwrap());

        assert_eq!(dungeon.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(dungeon.turn, 1);
        assert!(dungeon.message.is_empty());
    }

    #[test]
    fn an_accepted_step_prints_its_direction_echo_and_nothing_beneath_it() {
        // `text-output.md §10.2`: the mode loop emits the prompt marker and
        // the handler's own echo before the key is read, and §10.3 lists the
        // movement echo as complete in itself. The observed original prints
        // `>East` and no second line; the refusal adds the `audio.md §7.4`
        // `Blocked!` beneath it.
        let mut open = test_state(open_grid(), 4, 4);

        assert_eq!(
            handle_play_key_input(&mut open, 'd', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((open.player.x, open.player.y), (5, 4));
        assert!(open.message.is_empty(), "message: {}", open.message);
        let accepted: Vec<(&str, bool)> = open
            .message_entries()
            .iter()
            .map(|entry| (entry.text.as_str(), entry.is_command_echo))
            .collect();
        assert_eq!(accepted, vec![("East", true)]);

        let mut grid = open_grid();
        grid[4 * 32 + 5] = IMPASSABLE_FOOT_TILE;
        let mut blocked = test_state(grid, 4, 4);

        assert_eq!(
            handle_play_key_input(&mut blocked, 'd', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((blocked.player.x, blocked.player.y), (4, 4));
        assert_eq!(blocked.message, MOVEMENT_BLOCKED_REFUSAL);
        let refused: Vec<(&str, bool)> = blocked
            .message_entries()
            .iter()
            .map(|entry| (entry.text.as_str(), entry.is_command_echo))
            .collect();
        assert_eq!(refused, vec![("East", true), ("Blocked!", false)]);
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

    /// `lighting.md §8`: "The ordinary Light spell, *In Lor*, sets the
    /// light-spell counter to 100 counter units."
    ///
    /// The ambient assertion is taken at a night hour on purpose.
    /// `lighting.md §3` withdraws the dungeon-depth forced-dark reading -
    /// "Earlier wording in this document and in `systems/time.md` that
    /// placed "any dungeon depth" inside the forced-dark scope is
    /// **withdrawn**" - so "the ambient value computed while the party is
    /// inside a dungeon is simply whatever the clock produces". At noon
    /// that is the full-daylight 50 and §4's floor would be invisible
    /// ("In daylight, where the ambient value is already 50, lighting a
    /// torch or casting a light spell changes nothing at all"); at 22:00
    /// the clock produces the dark value of 2, so the spell's floor of 18
    /// is the value actually under test. An earlier revision of this test
    /// read the noon-in-a-dungeon ambient as forced dark and is retracted.
    #[test]
    fn cast_in_lor_sets_light_counter_and_consumes_charge_mana_and_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
        state.party[0].mana = 1;
        state.party[0].level = 1;
        state.clock = GameClock::new(22, 0).unwrap();
        state.ambient_light = FULL_DARKNESS;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.light_spell_counter, IN_LOR_LIGHT_DURATION);
        assert_eq!(IN_LOR_LIGHT_DURATION, 100);
        assert_eq!(state.ambient_light, LIGHT_SPELL_FLOOR);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(22, 1).unwrap());
        // Measured: the light *spell* prints no result line; `Light!`
        // is the Great Light scroll's banner.
        assert_eq!(state.message, "");
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
            ),
            (
                "1RT",
                QUICKNESS_SPELL_INDEX,
                QUICKNESS_COST,
                QUICKNESS_ACTIVE_EFFECT_TAG,
                QUICKNESS_ACTIVE_EFFECT_DURATION,
            ),
            (
                "1AI",
                NEGATE_MAGIC_SPELL_INDEX,
                NEGATE_MAGIC_COST,
                NEGATE_MAGIC_ACTIVE_EFFECT_TAG,
                NEGATE_MAGIC_ACTIVE_EFFECT_DURATION,
            ),
        ];

        // `magic.md §5.1`: all three are in the "No generic completion
        // line" row and do not borrow their scroll counterparts' banners
        // (`RETRACTIONS.md` R402).
        for (suffix, spell_index, cost, tag, duration) in cases {
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
            // `magic.md §8`: the casting turn's own post-dispatch cleanup
            // ages the new duration (playtest G09).
            assert_eq!(state.active_effect_counter, duration - 1);
            assert_eq!(state.turn, 1);
            assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
            assert!(state.message.is_empty(), "{}", state.message);
            assert!(state.z_stats_message().contains(&format!(
                "effect={}/{}",
                char::from(tag),
                duration - 1
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
        assert_eq!(state.message, "M.P. too low!\nFailed!\n");
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
        // `magic.md §8`: "Locate uses the shared sextant-style
        // coordinate printer" — both coordinates carry their own
        // closing quote, the pair is joined by comma-space, and a
        // newline is emitted before the pair as well as after it.
        // This assertion previously pinned the retracted
        // single-line, single-quote wording.
        // `RETRACTIONS.md` R413: "In Wis has no separate `Locate` label."
        assert_eq!(state.message, "\nK'P\", C'D\"\n");
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
        // Measured: the light *spell* prints no result line; `Light!`
        // is the Great Light scroll's banner.
        assert_eq!(state.message, "");
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
            missing_caster.pending_cast_argument,
            Some(CastArgumentRequest::Caster)
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
            missing_direction.pending_cast_argument,
            Some(CastArgumentRequest::Direction)
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
        assert_eq!(state.message, SPELL_SUCCESS_LINE);
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
        assert_eq!(state.message, SPELL_SUCCESS_LINE);
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
            // `magic.md §8`: each cast's flash holds the loop; a frontend
            // with no pump runs it out before the next key.
            state.flush_staged_narration();
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
        assert_eq!(state.message, "None mixed!\n");
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
        assert_eq!(state.message, "None mixed!\n");
        assert!(state.active_view_overlay.is_none());
    }

    /// `systems/magic.md §8`: X-Ray (*Wis An Ylem*) "is one of the two callers
    /// of the shared visibility sweep — the other is the White potion".
    /// `catalogs/item-list.md §7.2` says that branch "does not spend a gem,
    /// enter the modal View overlay, set an active-effect tag, or persist a
    /// detector flag", and reveals "the **whole** eleven-by-eleven viewport
    /// window straight from the map".
    ///
    /// R327 replaces the previous body of this test, which asserted the modal
    /// 32x32 class-map overlay: an engine that implemented the producer's
    /// negative-light branch as dead compatibility code "has no working White
    /// potion and no working X-Ray".
    #[test]
    fn cast_x_ray_runs_the_shared_visibility_sweep_without_requiring_gems() {
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
        assert!(
            state.active_view_overlay.is_none(),
            "the sweep branch does not enter the modal View overlay"
        );
        let sweep = state
            .visibility_sweep
            .expect("X-Ray starts the shared visibility sweep");
        assert_eq!(sweep.frames_remaining, POTION_WHITE_SWEEP_FRAMES);
        assert_eq!(sweep.pause_bios_ticks_per_frame, 1);
        assert_eq!((sweep.center_x, sweep.center_y), (5, 5));
        assert_eq!(
            sweep.visible_cells.iter().filter(|cell| **cell).count(),
            VIEWPORT_SIDE * VIEWPORT_SIDE
        );
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
        assert_eq!(
            state.message,
            "Mixing...\n\nDone!"
        );
    }

    #[test]
    fn mix_reagents_wrong_recipe_spends_selected_without_charge() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 1;
        state.reagents[REAGENT_BLOOD_MOSS] = 1;
        state.prng_state = trap_prng_state_for(TrapEffect::Acid);
        let mut expected = state.prng_state;
        let _ = u5_prng_range_u16(&mut expected, 0, 7);
        let damage = u16::from(shared_trap_acid_damage_from_index(
            u5_prng_range_u16(&mut expected, 0, 60) as u8,
        ));

        assert_eq!(
            state.mix_reagents_from_suffix("AS/0x80/1"),
            MoveOutcome::Blocked
        );

        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 0);
        assert_eq!(state.reagents[REAGENT_BLOOD_MOSS], 1);
        assert_eq!(state.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(
            state.message,
            // `magic.md §6` step 7 gives the wrong-recipe branch "a line
            // break" and the trap resolver, and no line of its own. The
            // sentence between these two was this engine's composition.
            // `traps.md §3`: the resolver prints its own word through
        // `emit_message_line`, and what it returns is an engine note. The
        // mixer's slot carries neither once both are where they belong.
        ""
        );
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP.saturating_sub(damage));
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

        // `traps.md §3` effect id 2 (Bomb) keeps the uniform draw.
        assert_eq!(shared_trap_damage_from_index(0, TRAP_BOMB_DAMAGE_MAX), 1);
        assert_eq!(shared_trap_damage_from_index(7, TRAP_BOMB_DAMAGE_MAX), 8);
        assert_eq!(shared_trap_damage_from_index(8, TRAP_BOMB_DAMAGE_MAX), 1);

        // `traps.md §3` effect id 0 (Acid): "The roll is an inclusive
        // `0..60` roll halved with truncation and floored to one ... so
        // it is **not** uniform over `1..30`: low values are markedly
        // more likely." This block previously pinned the withdrawn flat
        // `1 + (index % 30)` draw.
        assert_eq!(shared_trap_acid_damage_from_index(0), 1);
        assert_eq!(shared_trap_acid_damage_from_index(1), 1);
        assert_eq!(shared_trap_acid_damage_from_index(2), 1);
        assert_eq!(shared_trap_acid_damage_from_index(3), 1);
        assert_eq!(shared_trap_acid_damage_from_index(4), 2);
        assert_eq!(shared_trap_acid_damage_from_index(59), 29);
        assert_eq!(shared_trap_acid_damage_from_index(60), 30);
        // The published input domain is the inclusive `0..=60` raw
        // roll, so index 61 folds back to raw roll 0.
        assert_eq!(TRAP_ACID_RAW_ROLL_MAX, 60);
        assert_eq!(shared_trap_acid_damage_from_index(61), 1);
        // Every index stays inside the published `1..30` bound, and
        // damage 1 is markedly more likely than any middle value.
        let counts = (0..=u8::MAX)
            .map(shared_trap_acid_damage_from_index)
            .inspect(|damage| assert!((1..=TRAP_ACID_DAMAGE_MAX).contains(damage)))
            .filter(|damage| *damage == 1)
            .count();
        let middles = (0..=u8::MAX)
            .map(shared_trap_acid_damage_from_index)
            .filter(|damage| *damage == 15)
            .count();
        assert!(counts > middles, "{counts} ones vs {middles} fifteens");
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

        // `traps.md §2`: the effect id is a shared-PRNG draw over the
        // eight-entry table, so the resolver consumes the PRNG rather than
        // reading the turn counter or the party's position.
        let mut draws = Vec::new();
        let mut prng = 0x1234u16;
        for _ in 0..64 {
            state.prng_state = prng;
            let mut expected_state = prng;
            let index = u5_prng_range_u16(&mut expected_state, 0, 7) as u8;
            assert_eq!(state.shared_trap_effect_id(), TRAP_NON_COMBAT_EFFECT_TABLE[usize::from(index)]);
            assert_eq!(state.prng_state, expected_state);
            draws.push(index);
            prng = expected_state;
        }
        assert!((0..8).all(|index| draws.contains(&index)));

        state.turn = 0;
        state.active_player = Some(0);
        // `traps.md §3` effect id 0: the raw acid roll is an inclusive
        // `0..60` shared-PRNG draw, halved and floored to one.
        state.prng_state = trap_prng_state_for(TrapEffect::Acid);
        let mut expected = state.prng_state;
        let _ = u5_prng_range_u16(&mut expected, 0, 7);
        let raw = u5_prng_range_u16(&mut expected, 0, 60) as u8;
        let damage = shared_trap_acid_damage_from_index(raw);
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(0),
            format!("Acid trap hit party member 1 for {damage} HP.")
        );
        assert_eq!(state.prng_state, expected);
        assert_eq!(state.party[0].hp, 10 - damage.min(10) as u16);
        state.party[0].hp = 5;
        state.party[0].status = b'G';
        assert_eq!(state.active_player, Some(0));

        // traps.md §3: effect id 1 is a poison primitive, not a revive.
        // A Dead slot is skipped and left Dead rather than rewritten to
        // Poisoned.
        state.party[0].hp = 0;
        state.party[0].status = b'D';
        state.active_player = None;
        state.prng_state = trap_prng_state_for(TrapEffect::Poison);
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(0),
            "Poison trap had no effect on party member 1."
        );
        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.party[0].hp, 0);

        // Effect id 2: one independent inclusive `1..8` draw per accepted
        // member, after the family draw.
        state.prng_state = trap_prng_state_for(TrapEffect::Bomb);
        let mut expected = state.prng_state;
        let _ = u5_prng_range_u16(&mut expected, 0, 7);
        let bomb = u5_prng_range_u16(&mut expected, 1, 8) as u16;
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(1),
            format!("Bomb trap dealt {bomb} HP across 1 party member(s).")
        );
        assert_eq!(state.prng_state, expected);
        assert_eq!(state.party[1].hp, 10 - bomb);

        // traps.md §3: effect id 3 poisons every living member of the
        // six-slot band. The two Dead slots (0 and 2) are skipped and
        // stay Dead; only the living slot 1 is rewritten.
        state.prng_state = trap_prng_state_for(TrapEffect::Gas);
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
        state.prng_state = trap_prng_state_for(TrapEffect::Poison);
        assert_eq!(
            state.apply_shared_trap_effect_to_slot(0),
            "Poison trap poisoned party member 1."
        );
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 10);
        assert_eq!(state.party[0].max_hp, 20);
    }

    /// A PRNG state whose next non-combat trap draw lands on `family`.
    fn trap_prng_state_for(family: TrapEffect) -> u16 {
        (0..=u16::MAX)
            .find(|seed| {
                let mut state = *seed;
                let index = u5_prng_range_u16(&mut state, 0, 7) as u8;
                shared_trap_effect_family_from_index(index, false) == family
            })
            .unwrap()
    }

    /// `traps.md §2`: combat-class scenes draw uniformly between effect ids
    /// zero and one only, from the shared PRNG.
    #[test]
    fn combat_trap_effect_draw_is_a_two_way_prng_roll() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        let mut prng = 0x0BADu16;
        let mut seen = [false; 2];
        for _ in 0..32 {
            state.prng_state = prng;
            let mut expected = prng;
            let id = u5_prng_range_u16(&mut expected, 0, 1) as u8;
            assert_eq!(state.shared_trap_effect_id(), id);
            assert_eq!(state.prng_state, expected);
            seen[usize::from(id)] = true;
            prng = expected;
        }
        assert_eq!(seen, [true, true]);
    }

    #[test]
    fn wrong_mix_uses_first_good_or_poisoned_and_never_falls_back_to_active_member() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'P',
            climb_stat: 10,
            mana: 0,
            hp: 10,
            max_hp: 10,
            level: 1,
        });
        state.party[0].status = b'S';
        state.active_player = Some(0);
        assert_eq!(state.mixer_trap_target_slot(), Some(1));

        state.party[1].status = b'D';
        assert_eq!(state.mixer_trap_target_slot(), None);
        // Select an Acid family for the safe invalid sentinel used by the
        // forced-call edge. The stale active-player hint must not become an
        // arbitrary roster fallback.
        state.prng_state = trap_prng_state_for(TrapEffect::Acid);
        let before = state.party.clone();
        let _ = state.wrong_mix_trap_message();
        // The out-of-range guard's note is a diagnostic, not a line the
        // mixer prints: `traps.md §3` gives the resolver exactly one of
        // `ACID!`, `POISON!`, `BOMB!` and `GAS!` to print.
        assert!(
            state
                .diagnostics
                .join(" ")
                .contains("Acid trap found no party member")
        );
        assert_eq!(state.party, before);
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
        // `magic.md §6` step 4: "A zero quantity, typed or as a bare Return,
        // prints nothing further". This asserted `None!` until 2026-09-18.
        assert_eq!(state.message, "");

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

    /// `magic.md §6` Step 6: "The handler prints `Mixing...`, pauses
    /// briefly, then subtracts the requested quantity from each selected
    /// raw reagent counter", and Step 7 prints the completion message only
    /// afterwards. `text-output.md` §11 owns how the two land in one
    /// transcript: "**Both lines are emitted, in the order they occur**",
    /// because "Text output is a *stream* into a windowed grid" with no
    /// current-message slot to overwrite. So the mix transcript is the
    /// pause line followed by the completion line, not the completion line
    /// alone. An earlier revision of this test omitted the pause line.
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
        assert_eq!(state.message, "Mixing...\n\nDone!");
    }

    /// A synthetic `MISCMSG.DAT` whose every record reads `rec<index>`, with
    /// the tile-glyph bytes of `formats/miscmsg-dat.md §4` in record `41`.
    fn write_codex_miscmsg(dir: &Path) {
        let mut miscmsg = Vec::new();
        for index in 0..MISCMSG_DAT_RECORDS {
            if index == 41 {
                miscmsg.extend_from_slice(b"[E@CODEX\n_ONE");
            } else {
                miscmsg.extend_from_slice(format!("rec{index}").as_bytes());
            }
            miscmsg.push(0);
        }
        fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();
    }

    fn codex_tile_state() -> PlayState {
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = CODEX_SHRINE_ENTRY_TILE;
        britannia_state(grid, 10, 20)
    }

    /// Run the approach walk out and press one advance key, the way a shell
    /// with no pump does.
    fn press_codex_key(state: &mut PlayState, dir: &Path) {
        state.flush_staged_narration();
        assert_eq!(
            handle_play_key_input(state, ' ', "", dir).unwrap(),
            PlayInputDisposition::Continue
        );
    }

    fn has_row(state: &PlayState, text: &str) -> bool {
        transcript_texts(state).iter().any(|row| row.contains(text))
    }

    /// `karma.md §8` (`cleak/u5-spec#250`): "Use **E-Enter while standing on
    /// live terrain tile `0x11`** to begin the Codex interaction". §8.1
    /// selects "the **first ordained virtue in the standard virtue order**"
    /// and §8.2 stamps it on the third advance key, printing its quoted
    /// aphorism from records `20`-`27`.
    #[test]
    fn codex_entry_stamps_the_first_ordained_virtue_and_prints_its_urn_text() {
        let dir = debug_game_dir();
        write_codex_miscmsg(&dir);
        let mut state = codex_tile_state();
        state.shrine_ordained_mask = ShrineVirtue::Honesty.bit() | ShrineVirtue::Justice.bit();
        state.shrine_codex_mask = 0;

        assert_eq!(
            handle_play_key_input(&mut state, 'E', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(
            has_row(&state, "Enter the Shrine of the Codex!"),
            "the published entry line is missing from {:?}",
            transcript_texts(&state)
        );
        for _ in 0..3 {
            press_codex_key(&mut state, &dir);
        }
        assert_eq!(state.shrine_codex_mask, ShrineVirtue::Honesty.bit());
        assert!(has_row(&state, &format!("\"rec{}\"", 20)));
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §8.2`: "The counts begin after the initial approach record
    /// `46`", and each key prints only its own row of the table - "with no
    /// printing ahead". A no-ordained visit consumes three keys and the
    /// fourth is an ordinary command again (after the exit pacing).
    #[test]
    fn codex_reader_prints_one_record_per_key_and_never_ahead() {
        let dir = debug_game_dir();
        write_codex_miscmsg(&dir);
        let mut state = codex_tile_state();

        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        state.flush_staged_narration();
        assert!(has_row(&state, "rec46"));
        assert!(!has_row(&state, "rec37"), "{:?}", transcript_texts(&state));

        press_codex_key(&mut state, &dir);
        assert!(has_row(&state, "rec37"));
        assert!(!has_row(&state, "rec38"));
        press_codex_key(&mut state, &dir);
        assert!(has_row(&state, "rec38"));
        assert!(!has_row(&state, "rec39"));
        press_codex_key(&mut state, &dir);
        assert!(has_row(&state, "rec39"));
        assert_eq!(state.codex_presentation_steps, 0);
        assert_eq!(state.shrine_codex_mask, 0);
        // The exit pacing runs before the reader hands back the keyboard.
        assert!(state.staged_narration_active());
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §8.1`/`§8.2` completion extension: key 4 runs "three shared
    /// viewport flash/rumble effects" - each preceded by an unrestored
    /// inversion - then record `40`; key 5 prints the ordinary-font lead-in
    /// and runic `41`; keys 6-8 runic `42`-`44`; key 9 finishes. "Records
    /// `41`-`44` use the ordinary message window with the runic font
    /// selected", keeping "the stored glyphs and separators".
    #[test]
    fn codex_completion_flashes_three_times_and_prints_runic_pages_per_key() {
        let dir = debug_game_dir();
        write_codex_miscmsg(&dir);
        let mut state = codex_tile_state();
        state.shrine_ordained_mask = ShrineVirtue::Humility.bit();
        state.shrine_codex_mask = 0x7F;

        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        for _ in 0..3 {
            press_codex_key(&mut state, &dir);
        }
        assert_eq!(state.shrine_codex_mask, 0xFF);
        assert!(!has_row(&state, "rec40"));
        let serial = state.sound_effect_serial;
        press_codex_key(&mut state, &dir);
        let flashes = state
            .sound_effects_after(serial)
            .iter()
            .filter(|effect| matches!(effect, SoundEffect::MajorFlash { .. }))
            .count();
        assert_eq!(flashes, 3);
        assert!(state.presentation_viewport_inverted, "no restoring inversion");
        assert!(has_row(&state, "rec40"));
        assert!(!has_row(&state, "Thou dost read:"));

        press_codex_key(&mut state, &dir);
        assert!(has_row(&state, "Thou dost read:"));
        let runic: Vec<_> = state
            .message_entries()
            .iter()
            .filter(|entry| {
                !entry.glyphs.is_empty()
                    && entry
                        .glyphs
                        .iter()
                        .all(|glyph| glyph.font == TlkGlyphFont::Runic)
            })
            .map(|entry| entry.text.clone())
            .collect();
        assert_eq!(runic, vec!["[E@CODEX".to_string(), "_ONE".to_string()]);
        assert!(!has_row(&state, "rec42"));
        for page in 42..=44 {
            press_codex_key(&mut state, &dir);
            assert!(has_row(&state, &format!("rec{page}")));
        }
        assert_ne!(state.codex_presentation_steps, 0);
        press_codex_key(&mut state, &dir);
        assert_eq!(state.codex_presentation_steps, 0);
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §8`: the Codex walk is "**twelve** animation frames, and
    /// **sixty** world steps" from `(5, 10)` up to `(5, 3)`, never kneeling;
    /// the exit "is the shrine's - the same block": re-stamp at `(5, 3)`,
    /// rows 4 through 10, four blank frames and the closing wait. The display
    /// is held through the reader's key waits and released after the exit.
    #[test]
    fn codex_entry_walks_up_seven_rows_and_back_down_after_the_reader() {
        use crate::shrine_presentation::{PresentationArm, ShrinePose};
        let dir = debug_game_dir();
        let mut state = codex_tile_state();
        state.prng_state = 0x2468;
        let mut reference = state.clone();

        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        let walk = state.shrine_approach_walk.expect("the Codex walk runs");
        assert_eq!(walk.arm(), PresentationArm::Codex);
        state.flush_staged_narration();
        // One opening step plus the flat sixty, each with its wind draw.
        for _ in 0..61 {
            let _ = reference.idle_wind_drift();
        }
        assert_eq!(state.prng_state, reference.prng_state);
        assert_eq!(
            state.shrine_approach_walk.unwrap().pose(),
            ShrinePose::Walking { column: 5, row: 3 }
        );
        // The reader's key waits keep the display up.
        state.apply_exploration_turn_gate(&dir).unwrap();
        assert!(state.shrine_approach_walk.is_some());

        for _ in 0..3 {
            press_codex_key(&mut state, &dir);
        }
        assert!(state.shrine_approach_walk.unwrap().is_exiting());
        let mut cells = Vec::new();
        for _ in 0..PresentationArm::Codex.exit_world_steps() {
            state.advance_presentation_world_step();
            if let ShrinePose::Walking { column, row } = state.shrine_approach_walk.unwrap().pose()
                && cells.last() != Some(&(column, row))
            {
                cells.push((column, row));
            }
        }
        assert_eq!(cells, (3..=10).map(|row| (5, row)).collect::<Vec<_>>());
        state.staged_narration.clear();
        state.apply_exploration_turn_gate(&dir).unwrap();
        assert!(state.shrine_approach_walk.is_none());
        let _ = fs::remove_dir_all(dir);
    }

    /// `miscmsg-dat.md §4`: "For Codex records `41`-`44`, the caller selects
    /// the runic font and uses the ordinary fixed-window message printer ...
    /// Keep the authored glyph stream and line separators intact."
    #[test]
    fn codex_completion_pages_print_their_glyph_stream_in_the_runic_font() {
        let dir = debug_game_dir();
        let mut miscmsg = Vec::new();
        for index in 0..MISCMSG_DAT_RECORDS {
            if crate::miscmsg_io::MISCMSG_CODEX_COMPLETION_PAGES.contains(&index) {
                miscmsg.extend_from_slice(format!("TRU[@_{index}").as_bytes());
            } else {
                miscmsg.extend_from_slice(format!("rec{index}\n").as_bytes());
            }
            miscmsg.push(0);
        }
        fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();

        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = CODEX_SHRINE_ENTRY_TILE;
        let mut state = britannia_state(grid, 10, 20);
        state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
        state.shrine_codex_mask = 0xFF & !ShrineVirtue::Honesty.bit();

        assert_eq!(
            handle_play_key_input(&mut state, 'E', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        // `karma.md §8.2`: the reader prints one record per key; key 5 is
        // the lead-in plus runic `41`.
        for _ in 0..5 {
            press_codex_key(&mut state, &dir);
        }

        let page = state
            .message_entries()
            .iter()
            .find(|entry| entry.text.contains("TRU[@_41"))
            .unwrap_or_else(|| panic!("page 41 missing from {:?}", transcript_texts(&state)));
        assert!(page
            .glyphs
            .iter()
            .all(|glyph| glyph.font == TlkGlyphFont::Runic));
        assert!(!transcript_texts(&state)
            .iter()
            .any(|row| row.contains("TRUTH ST")));
        // The lead-in before the pages stays in the normal font.
        let lead_in = state
            .message_entries()
            .iter()
            .find(|entry| entry.text.contains("Thou dost read:"))
            .unwrap();
        assert!(lead_in
            .glyphs
            .iter()
            .all(|glyph| glyph.font == TlkGlyphFont::Ordinary));
        let _ = fs::remove_dir_all(dir);
    }

    /// The runic pages must not change how the Codex text's own line feeds
    /// land: an embedded feed is a producer blank (`text-output.md §10.4`
    /// still owes the derived turn blank after it), in the ordinary text and
    /// inside a runic page alike.
    #[test]
    fn codex_presentation_blank_rows_stay_producer_blanks() {
        let dir = debug_game_dir();
        let mut miscmsg = Vec::new();
        for index in 0..MISCMSG_DAT_RECORDS {
            if crate::miscmsg_io::MISCMSG_CODEX_COMPLETION_PAGES.contains(&index) {
                miscmsg.extend_from_slice(format!("TRU\n\n[@_{index}\n").as_bytes());
            } else {
                miscmsg.extend_from_slice(format!("rec{index}\n").as_bytes());
            }
            miscmsg.push(0);
        }
        fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();

        for codex_mask in [0, 0xFF & !ShrineVirtue::Honesty.bit()] {
            let mut grid = open_world_grid();
            grid[world_cell_index(10, 20)] = CODEX_SHRINE_ENTRY_TILE;
            let mut state = britannia_state(grid, 10, 20);
            state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
            state.shrine_codex_mask = codex_mask;
            handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
            // `karma.md §8.2`: drive the stepped reader to its finish.
            while state.codex_presentation_steps != 0 {
                press_codex_key(&mut state, &dir);
            }

            let entries = state.message_entries();
            let start = entries
                .iter()
                .position(|entry| entry.text.starts_with("rec"))
                .expect("Codex narration printed");
            let blanks = entries[start..]
                .iter()
                .filter(|entry| entry.explicit_blank)
                .collect::<Vec<_>>();
            assert!(!blanks.is_empty(), "mask {codex_mask:#x}: no blank rows");
            assert!(
                blanks.iter().all(|entry| entry.producer_blank),
                "mask {codex_mask:#x}: {:?}",
                transcript_texts(&state)
            );
            assert!(!state.message_row_open_mid_line);
        }
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §8.1`: a read that completes the Codex mask "continue[s]
    /// with three shared viewport flash/rumble effects, then record `40`";
    /// `audio.md §8.4` prices each at 1,856 gameplay-PRNG draws.
    #[test]
    fn codex_completion_runs_three_major_flashes() {
        let dir = debug_game_dir();
        let mut miscmsg = Vec::new();
        for index in 0..MISCMSG_DAT_RECORDS {
            miscmsg.extend_from_slice(format!("rec{index}").as_bytes());
            miscmsg.push(0);
        }
        fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();

        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
        state.shrine_codex_mask = 0xFF & !ShrineVirtue::Honesty.bit();
        // `karma.md §8.2` (audit A114): the reader is stepped, and the
        // flashes belong to advance key 4. Keys 1-3 print `37`, `38` and the
        // stamped aphorism.
        state.read_codex_urn_after_entry(&dir).unwrap();
        state.flush_staged_narration();
        for _ in 0..3 {
            assert!(state.step_codex_presentation(&dir).unwrap());
        }
        let prng_before = state.prng_state;
        let serial = state.sound_effect_serial;
        assert!(state.step_codex_presentation(&dir).unwrap());

        let flashes = state
            .sound_effects_after(serial)
            .into_iter()
            .filter(|effect| matches!(effect, SoundEffect::MajorFlash { .. }))
            .count();
        assert_eq!(flashes, 3);
        let mut prng = U5Prng::new(prng_before);
        for _ in 0..3 {
            let _ = audio::draw_major_flash_bands(&mut prng);
        }
        assert_eq!(state.prng_state, prng.state());

        // An incomplete resulting mask finishes without the flashes.
        let mut partial = britannia_state(open_world_grid(), 10, 20);
        partial.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
        partial.shrine_codex_mask = 0;
        let serial = partial.sound_effect_serial;
        partial.read_codex_urn_after_entry(&dir).unwrap();
        while partial.codex_presentation_steps != 0 {
            partial.flush_staged_narration();
            assert!(partial.step_codex_presentation(&dir).unwrap());
        }
        // The approach walk's world steps may sound on their own; what must
        // not play is a flash.
        assert!(!partial
            .sound_effects_after(serial)
            .iter()
            .any(|effect| matches!(effect, SoundEffect::MajorFlash { .. })));
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §8` (`RETRACTIONS.md` R451): "`M` remains Mix Reagents at
    /// this location. The earlier statement that the M command owns Codex
    /// entry is retracted." So `M` on the Codex tile mixes and stamps nothing.
    #[test]
    fn mix_reagents_still_runs_on_the_codex_tile() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = CODEX_SHRINE_ENTRY_TILE;
        let mut state = britannia_state(grid, 10, 20);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 1;
        state.shrine_ordained_mask = ShrineVirtue::Justice.bit();

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "IL/0x80/1", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.shrine_codex_mask, 0);
        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 0);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 1);
        let _ = fs::remove_dir_all(dir);
    }

    /// §8's other two branches stamp nothing: a party with no ordained virtue,
    /// and one whose eight Codex-read bits are already set - "If all
    /// Codex-read bits are already set, the reader takes its completed branch
    /// instead of stamping another virtue." Neither branch's wording is
    /// published, so neither prints engine prose.
    #[test]
    fn codex_no_ordained_and_completed_branches_do_not_stamp_new_bits() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = CODEX_SHRINE_ENTRY_TILE;
        let mut state = britannia_state(grid, 10, 20);
        state.shrine_codex_mask = ShrineVirtue::Valor.bit();

        assert_eq!(
            handle_play_key_input(&mut state, 'E', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.shrine_codex_mask, ShrineVirtue::Valor.bit());

        state.shrine_ordained_mask = 0xFF;
        state.shrine_codex_mask = 0xFF;
        assert_eq!(
            handle_play_key_input(&mut state, 'E', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.shrine_codex_mask, 0xFF);
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

    /// `karma.md §7`: "`M` always selects Mix Reagents, including at a
    /// shrine. The earlier M-Meditate entry rule is withdrawn
    /// (`RETRACTIONS.md` R411)." A typed mantra after `M` is a mix line, on
    /// the shrine marker and on the native altar tiles alike.
    #[test]
    fn m_on_a_shrine_mixes_reagents_and_never_meditates() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
        for tile in [SHRINE_MARKER_TILE, SHRINE_ALTAR_TILE_FIRST] {
            let mut grid = open_world_grid();
            grid[world_cell_index(10, 20)] = tile;
            let mut state = britannia_state(grid, 10, 20);
            let bit = ShrineVirtue::Honesty.bit();
            state.shrine_ordained_mask = bit;
            state.shrine_codex_mask = bit;

            assert_eq!(
                handle_play_key_input(&mut state, 'M', "Ahm", &dir).unwrap(),
                PlayInputDisposition::Continue
            );

            assert_eq!(state.shrine_ordained_mask, bit, "tile {tile:#04x}");
            assert!(state.active_shrine.is_none());
            assert!(state.pending_shrine_third_mantra.is_none());
            // The unparseable inline Mix suffix prints nothing (playtest N07).
            assert!(state.message.is_empty(), "{:?}", state.message);
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_shrine_prompt_uses_native_humility_altar_tile_without_sidecar() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_ALTAR_TILE_LAST;
        let mut state = britannia_state(grid, 10, 20);

        // `M` is Mix Reagents even here (measured); the altar tile's virtue
        // derivation is reached through the internal dispatch `magic.md §9.2`
        // gives the natural moongate.
        assert_eq!(
            handle_play_key_input(&mut state, 'M', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_shrine.is_none());
        assert!(state.active_mix.is_some());
        state.active_mix = None;

        state
            .start_shrine_prompt_at_current_position(&dir)
            .unwrap()
            .expect("the altar tile names its own shrine");
        assert!(state.active_shrine.is_some());
        assert_eq!(state.message, SHRINE_MANTRA_PROMPT);

        // `karma.md §12`: three `Mantra:` asks before the quest state machine.
        for _ in 0..3 {
            assert_eq!(
                handle_play_key_input(&mut state, 'L', "um\r", &dir).unwrap(),
                PlayInputDisposition::Continue
            );
            // §12: "waiting twelve world ticks after each nonblank answer."
            // No pump here, so resync the same way production code does.
            state.flush_staged_narration();
            state.adopt_shrine_prompt_row();
        }
        // `karma.md §12`: the third mantra answer has its own twelve-tick
        // hold (`cleak/u5-engine#23`) before the closing branch runs.
        state
            .run_shrine_third_mantra_resolution_to_completion(&dir)
            .unwrap();

        // Measured: the third accepted mantra opens the altar announcement,
        // which waits for a command key.
        assert!(state.active_shrine.is_some());
        assert_eq!(state.shrine_ordained_mask, ShrineVirtue::Humility.bit());
        assert_eq!(state.shrine_codex_mask, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_sidecar_takes_precedence_over_native_altar_tile() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HUMILITY 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_ALTAR_TILE_FIRST;
        let mut state = britannia_state(grid, 10, 20);

        state
            .start_shrine_prompt_at_current_position(&dir)
            .unwrap()
            .expect("the sidecar row names the shrine");
        for _ in 0..3 {
            handle_play_key_input(&mut state, 'L', "um\r", &dir).unwrap();
            state.flush_staged_narration();
            state.adopt_shrine_prompt_row();
        }
        state
            .run_shrine_third_mantra_resolution_to_completion(&dir)
            .unwrap();

        assert_eq!(state.shrine_ordained_mask, ShrineVirtue::Humility.bit());
        assert_eq!(
            state.shrine_ordained_mask & ShrineVirtue::Honesty.bit(),
            0
        );
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
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
        let mut state = britannia_state(grid, 10, 20);

        assert_eq!(
            handle_play_key_input(&mut state, 'E', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        // `karma.md §12` stages the entry narration; the virtue question is
        // its third beat. With no pump in these tests, print it now - until
        // it is on screen the prompt is not open and keys are buffered.
        state.flush_staged_narration();
        assert!(state.active_shrine.is_some());
        assert!(state.active_mix.is_none());
        assert!(state.message.contains(SHRINE_VIRTUE_PROMPT) || state.message == ":");

        assert_eq!(
            handle_play_key_input(&mut state, 'H', "ONESTY\r", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        // `karma.md §12`: "wait six world ticks" before the first `Mantra:`
        // row opens. No pump here either, so flush it the same way the
        // entry narration above was; the hold itself prints nothing, so
        // `self.message` needs the same explicit resync production code
        // gives it on entry (`adopt_shrine_prompt_row`).
        state.flush_staged_narration();
        state.adopt_shrine_prompt_row();
        assert_eq!(state.message, SHRINE_MANTRA_PROMPT);

        // `karma.md §12`: the session asks `Mantra:` three times, and only
        // the third submission reaches the quest state machine. Measured: the
        // first two are silent and re-open the row.
        for _ in 0..2 {
            assert_eq!(
                handle_play_key_input(&mut state, 'A', "hm\r", &dir).unwrap(),
                PlayInputDisposition::Continue
            );
            // §12: "waiting twelve world ticks after each nonblank answer."
            state.flush_staged_narration();
            state.adopt_shrine_prompt_row();
            assert!(state.active_shrine.is_some());
            assert_eq!(state.shrine_ordained_mask, 0);
            assert_eq!(state.message, SHRINE_MANTRA_PROMPT);
        }
        assert_eq!(
            handle_play_key_input(&mut state, 'A', "hm\r", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        // `karma.md §12`: the third mantra answer has its own twelve-tick
        // hold (`cleak/u5-engine#23`) before the closing branch runs; resync
        // past it the same way the first two asks did above.
        state.flush_staged_narration();
        state
            .run_shrine_third_mantra_resolution_to_completion(&dir)
            .unwrap();

        // The third answer opens the altar's announcement, which waits for a
        // command key before the quest sentence.
        assert!(state.active_shrine.is_some());
        assert_eq!(state.shrine_ordained_mask, ShrineVirtue::Honesty.bit());
        assert_eq!(state.shrine_codex_mask, 0);
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_enter_echo_puts_the_virtue_name_on_its_own_row() {
        // `karma.md §7` entry table, Enter prefix: "`E`-Enter prints
        // `Enter `, then `the shrine of\n`, then the virtue's name from the
        // table above, then one line feed".
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
        let mut state = britannia_state(grid, 10, 20);

        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();

        let rows = transcript_texts(&state);
        let at = rows
            .iter()
            .position(|row| row == "Enter the shrine of")
            .unwrap_or_else(|| panic!("no `Enter the shrine of` row in {rows:?}"));
        assert_eq!(rows[at + 1], "Honesty");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn escape_at_the_virtue_prompt_erases_the_answer_and_keeps_the_prompt() {
        // `karma.md §7`: "Escape erases the whole current answer and keeps
        // the same prompt open. At an empty answer, Escape changes nothing
        // visible."
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
        let mut state = britannia_state(grid, 10, 20);

        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        state.flush_staged_narration();
        handle_play_key_input(&mut state, 'X', "YZ", &dir).unwrap();
        assert_eq!(state.message, ":XYZ");
        handle_play_key_input(&mut state, '\u{1b}', "", &dir).unwrap();
        assert!(state.active_shrine.is_some(), "Escape must not end the meditation");
        assert_eq!(state.message, ":");
        handle_play_key_input(&mut state, '\u{1b}', "", &dir).unwrap();
        assert!(state.active_shrine.is_some());
        assert_eq!(state.message, ":");

        handle_play_key_input(&mut state, 'H', "onesty\r", &dir).unwrap();
        state.flush_staged_narration();
        state.adopt_shrine_prompt_row();
        assert_eq!(state.message, SHRINE_MANTRA_PROMPT);
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §7` "Random-stream cost": "Each world step in the hold
    /// takes the unconditional 1-in-64 wind draw ... A shrine entry as far
    /// as the virtue prompt therefore consumes **fifty-seven** draws, and an
    /// entry abandoned at that prompt consumes **one hundred and eight**."
    #[test]
    fn shrine_entry_takes_one_wind_draw_per_counted_world_step() {
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Honesty);
        state.prng_state = 0x1357;
        let mut reference = state.clone();

        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        state.flush_staged_narration();
        for _ in 0..57 {
            let _ = reference.idle_wind_drift();
        }
        assert_eq!(state.prng_state, reference.prng_state, "57 draws to the prompt");
        assert!(state.active_shrine.is_some());

        // A blank answer abandons the entry; the exit pacing is fifty-one
        // more counted steps.
        handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
        assert!(state.active_shrine.is_none());
        assert!(state.staged_narration_active());
        state.flush_staged_narration();
        for _ in 0..51 {
            let _ = reference.idle_wind_drift();
        }
        assert_eq!(state.prng_state, reference.prng_state, "108 draws in all");
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §7.2`'s window table: "World steps | 61" on the ordination
    /// arm, from the key that dismisses the quest page to the next prompt,
    /// each with its wind draw.
    #[test]
    fn shrine_ordination_window_is_sixty_one_counted_steps() {
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Honesty);
        meditate_through_shrine_session(&mut state, &dir, "Honesty", "Ahm");
        // Two key waits: the quest sentence, then the closing record.
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
        let mut reference = state.clone();
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
        assert!(state.active_shrine.is_none());
        state.flush_staged_narration();
        for _ in 0..61 {
            let _ = reference.idle_wind_drift();
        }
        assert_eq!(state.prng_state, reference.prng_state);
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §7` "Exit pacing" runs "after the meditation handler
    /// returns" on every arm: the abandoned entry walks back down and clears
    /// the slot before the presentation is released.
    #[test]
    fn an_abandoned_shrine_entry_still_walks_back_down() {
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Honesty);
        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        state.flush_staged_narration();
        handle_play_key_input(&mut state, '\r', "", &dir).unwrap();

        let walk = state.shrine_approach_walk.expect("the presentation is still up");
        assert!(walk.is_exiting());
        // Nothing has stepped yet: the figure still kneels at the altar.
        assert!(matches!(
            walk.pose(),
            crate::shrine_presentation::ShrinePose::Kneeling { row: 6, .. }
        ));
        // A pump that has not yet run the hold keeps the display.
        state.apply_exploration_turn_gate(&dir).unwrap();
        assert!(state.shrine_approach_walk.is_some());

        let mut cells = Vec::new();
        for _ in 0..51 {
            state.advance_presentation_world_step();
            if let crate::shrine_presentation::ShrinePose::Walking { column, row } =
                state.shrine_approach_walk.unwrap().pose()
                && cells.last() != Some(&(column, row))
            {
                cells.push((column, row));
            }
        }
        assert_eq!(cells, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]);
        assert_eq!(
            state.shrine_approach_walk.unwrap().pose(),
            crate::shrine_presentation::ShrinePose::None
        );
        state.staged_narration.clear();
        state.apply_exploration_turn_gate(&dir).unwrap();
        assert!(state.shrine_approach_walk.is_none());
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §7`: on the ordaining arm the walking pose is re-stamped at
    /// `(5, 6)` before record `31`, so the avatar stands through both key
    /// waits; the wrong-answer arm keeps kneeling until the exit.
    #[test]
    fn ordination_restores_the_standing_pose_and_unfocused_keeps_kneeling() {
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Honesty);
        meditate_through_shrine_session(&mut state, &dir, "Honesty", "Ahm");
        let walk = state.shrine_approach_walk.unwrap();
        assert!(walk.is_standing());
        assert_eq!(
            walk.pose(),
            crate::shrine_presentation::ShrinePose::Walking { column: 5, row: 6 }
        );

        let mut state = shrine_marker_state(&dir, ShrineVirtue::Honesty);
        meditate_through_shrine_session(&mut state, &dir, "Honesty", "Mu");
        let walk = state.shrine_approach_walk.unwrap();
        assert!(!walk.is_standing());
        assert!(walk.is_exiting());
        assert!(matches!(
            walk.pose(),
            crate::shrine_presentation::ShrinePose::Kneeling { row: 6, .. }
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_shrine_prompt_blank_mantra_has_no_effect() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
        let mut state = britannia_state(grid, 10, 20);

        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        // `karma.md §12` stages the entry narration; the virtue question is
        // its third beat. With no pump in these tests, print it now - until
        // it is on screen the prompt is not open and keys are buffered.
        state.flush_staged_narration();
        handle_play_key_input(&mut state, 'H', "ONESTY\r", &dir).unwrap();
        // §12: "wait six world ticks" before the `Mantra:` row opens; until
        // then the row is not live and a key would just buffer.
        state.flush_staged_narration();
        state.adopt_shrine_prompt_row();
        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        // Measured: Return on an empty mantra row closes the prompt and
        // prints nothing of its own.
        assert!(state.active_shrine.is_none());
        assert_eq!(state.shrine_ordained_mask, 0);
        assert_eq!(state.shrine_codex_mask, 0);
        assert_eq!(state.message, SHRINE_MANTRA_PROMPT);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_meditation_wrong_mantra_and_tile_guard_do_not_mutate_state() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
        let mut state = britannia_state(grid, 10, 20);

        // `karma.md §7`: a wrong mantra reaches the unfocused result after
        // the third answer, with no quest-state change.
        meditate_through_shrine_session(&mut state, &dir, "Honesty", "Mu");
        assert_eq!(state.shrine_ordained_mask, 0);
        assert!(state.active_shrine.is_none());
        state.flush_staged_narration();

        state.grid[world_cell_index(10, 20)] = 5;
        assert_eq!(
            handle_play_key_input(&mut state, 'M', "Ahm", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.shrine_ordained_mask, 0);
        // The unparseable inline Mix suffix prints nothing (playtest N07).
        assert!(state.message.is_empty(), "{:?}", state.message);
        let _ = fs::remove_dir_all(dir);
    }

    /// Drive one E-Enter meditation through the session: the virtue answer,
    /// the three mantra asks, each hold flushed the way a shell with no pump
    /// does, and the third-mantra resolution.
    fn meditate_through_shrine_session(
        state: &mut PlayState,
        dir: &Path,
        virtue_answer: &str,
        mantra: &str,
    ) {
        handle_play_key_input(state, 'E', "", dir).unwrap();
        state.flush_staged_narration();
        let mut chars = virtue_answer.chars();
        let first = chars.next().expect("nonblank virtue answer");
        handle_play_key_input(state, first, &format!("{}\r", chars.as_str()), dir).unwrap();
        let mut chars = mantra.chars();
        let first = chars.next().expect("nonblank mantra");
        let rest = format!("{}\r", chars.as_str());
        for _ in 0..3 {
            state.flush_staged_narration();
            state.adopt_shrine_prompt_row();
            handle_play_key_input(state, first, &rest, dir).unwrap();
        }
        state
            .run_shrine_third_mantra_resolution_to_completion(dir)
            .unwrap();
    }

    /// A Britannia state standing on the shrine marker, with a sidecar row
    /// naming `virtue`.
    fn shrine_marker_state(dir: &Path, virtue: ShrineVirtue) -> PlayState {
        fs::write(
            dir.join(SHRINE_TABLE_FILE),
            format!("BRITANNIA 10 20 {} 25\n", virtue.name().to_ascii_uppercase()),
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
        britannia_state(grid, 10, 20)
    }

    #[test]
    fn shrine_codex_turn_in_clears_ordained_and_rewards_avatar_stats() {
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Justice);
        let bit = ShrineVirtue::Justice.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.avatar_stats = AvatarStats {
            strength: 20,
            dexterity: 20,
            intelligence: 20,
        };
        state.party[0].climb_stat = 20;

        meditate_through_shrine_session(&mut state, &dir, "Justice", "Beh");

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
        assert!(state.active_shrine.is_none());
        // `karma.md §7.2`: "the attribute lines print after the whole sound,
        // not during it" - they are staged behind the swell's hold.
        state.flush_staged_narration();
        let rows = transcript_texts(&state);
        let dex = rows
            .iter()
            .position(|row| row == crate::commands::SHRINE_REWARD_DEXTERITY_LINE)
            .unwrap_or_else(|| panic!("no Dexterity line in {rows:?}"));
        assert_eq!(rows[dex + 1], crate::commands::SHRINE_REWARD_INTELLIGENCE_LINE);
        assert!(
            state
                .diagnostics
                .iter()
                .any(|note| note.contains("shrine of Justice completed"))
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// Playtest N04: `karma.md §7` awards the Avatar attributes at once, so
    /// the live Strength/Intelligence mirrors that Z-stats and the Ready
    /// burden gate read (`inventory.md §5`) must carry the award without a
    /// save/reload.
    #[test]
    fn shrine_codex_turn_in_strength_and_intelligence_reach_live_party_mirrors() {
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Honor);
        let bit = ShrineVirtue::Honor.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.avatar_stats = AvatarStats {
            strength: 20,
            dexterity: 20,
            intelligence: 20,
        };
        state.party_strengths = vec![20; state.party.len()];
        state.party_intelligence = vec![20; state.party.len()];

        meditate_through_shrine_session(&mut state, &dir, "Honor", "Summ");

        assert_eq!(state.avatar_stats.strength, 21);
        assert_eq!(state.avatar_stats.intelligence, 21);
        assert_eq!(state.party_strengths[0], 21);
        assert_eq!(state.party_intelligence[0], 21);
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §7` quest-state rows: the turn-in arm is reached by "an
    /// accepted virtue answer plus all three accepted mantras" under the
    /// shared substring matcher, so a longer answer containing the mantra
    /// still turns the quest in. It then prints record `36` and plays the
    /// §7.2 beat: inversion, swell, then the shared flash.
    #[test]
    fn shrine_turn_in_accepts_a_substring_mantra_and_prints_record_thirty_six() {
        let dir = debug_game_dir();
        let mut miscmsg = Vec::new();
        for index in 0..MISCMSG_DAT_RECORDS {
            miscmsg.extend_from_slice(format!("rec{index}\n").as_bytes());
            miscmsg.push(0);
        }
        fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Honesty);
        let bit = ShrineVirtue::Honesty.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        let serial = state.sound_effect_serial;

        meditate_through_shrine_session(&mut state, &dir, "Honesty", "ahmm/x:y");

        assert_eq!(state.shrine_ordained_mask, 0);
        assert!(
            transcript_texts(&state)
                .iter()
                .any(|row| row == &format!("rec{MISCMSG_SHRINE_TURN_IN_CONGRATULATION}"))
        );
        assert!(!transcript_texts(&state).iter().any(|row| row == "No effect!"));
        assert!(state.presentation_viewport_inverted);
        let effects = state.sound_effects_after(serial);
        let swell = effects
            .iter()
            .position(|effect| *effect == SoundEffect::ShrineTurnInSwell)
            .expect("the turn-in swell plays");
        assert!(
            matches!(effects.get(swell + 1), Some(SoundEffect::MajorFlash { .. })),
            "the shared flash follows the swell: {effects:?}"
        );
        state.flush_staged_narration();
        assert!(!state.presentation_viewport_inverted);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn completed_shrine_offering_costs_gold_and_clamps_moral_standing() {
        let dir = debug_game_dir();
        let virtue = ShrineVirtue::Compassion;
        let mut state = shrine_marker_state(&dir, virtue);
        state.shrine_codex_mask = virtue.bit();
        state.moral_standing = 98;
        state.gold = 350;

        meditate_through_shrine_session(&mut state, &dir, "Compassion", "Mu");
        assert_eq!(
            handle_play_key_input(&mut state, '9', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.gold, 350);
        // The digit's own echo is the last row: `karma.md §12` checks
        // affordability *after* it. Record 35 and the repeated record 34 log
        // nothing in this fixture, which ships no `MISCMSG.DAT`.
        assert!(state.message.contains("900 gp"));
        assert!(state.active_shrine.is_some());

        assert_eq!(
            handle_play_key_input(&mut state, '2', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.gold, 150);
        assert_eq!(state.moral_standing, MORAL_STANDING_MAX);
        assert_eq!(state.turn, 0);
        // `karma.md §12`: the affordable arm prints `ALAKAZAM` in the runic
        // font and appends `!` in the normal one.
        assert_eq!(state.message, "ALAKAZAM!");
        // `karma.md §7.2`: one unrestored inversion precedes the swell.
        assert!(state.presentation_viewport_inverted);
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §12`: "Print `ALAKAZAM` using the runic font, restore the
    /// normal font and append `!\n`" - the `!` lands on the runic word's own
    /// row, not on a row of its own.
    #[test]
    fn completed_shrine_offering_appends_the_bang_to_the_runic_row() {
        let dir = debug_game_dir();
        let virtue = ShrineVirtue::Compassion;
        let mut state = shrine_marker_state(&dir, virtue);
        state.shrine_codex_mask = virtue.bit();
        state.gold = 350;

        meditate_through_shrine_session(&mut state, &dir, "Compassion", "Mu");
        handle_play_key_input(&mut state, '1', "", &dir).unwrap();

        let rows = transcript_texts(&state);
        assert_eq!(rows.last().map(String::as_str), Some("ALAKAZAM!"), "{rows:?}");
        assert!(!state.message_row_open_mid_line, "`!\\n` closes the row");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_offering_and_turn_in_queue_one_unrestored_viewport_inversion() {
        // `audio.md §5.4.7` / `karma.md §7.2`: both shrine swell callers
        // precede the sweep with one unrestored viewport inversion; the
        // refusal arms reach none. Driven through the interactive session
        // (`karma.md §12`; `M` always mixes, audit A112), with the inversion
        // carried by `presentation_viewport_inverted` (audit A107).
        let dir = debug_game_dir();
        let virtue = ShrineVirtue::Justice;
        let bit = virtue.bit();

        let mut state = shrine_marker_state(&dir, virtue);
        state.shrine_codex_mask = bit;
        state.gold = 900;
        meditate_through_shrine_session(&mut state, &dir, "Justice", "Beh");
        handle_play_key_input(&mut state, '0', "", &dir).unwrap();
        assert!(!state.presentation_viewport_inverted);

        let mut state = shrine_marker_state(&dir, virtue);
        state.shrine_codex_mask = bit;
        state.gold = 900;
        meditate_through_shrine_session(&mut state, &dir, "Justice", "Beh");
        handle_play_key_input(&mut state, '1', "", &dir).unwrap();
        assert_eq!(state.message, "ALAKAZAM!");
        assert!(state.presentation_viewport_inverted);
        // Never restored: the following world steps paint over it.
        state.flush_staged_narration();
        assert!(!state.presentation_viewport_inverted);

        let mut state = shrine_marker_state(&dir, virtue);
        state.shrine_codex_mask = bit;
        state.shrine_ordained_mask = bit;
        meditate_through_shrine_session(&mut state, &dir, "Justice", "Beh");
        assert_eq!(state.shrine_ordained_mask, 0);
        assert!(state.presentation_viewport_inverted);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_completed_shrine_prompt_collects_offering_digit() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SHRINE_TABLE_FILE),
            "BRITANNIA 10 20 COMPASSION 25\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
        let mut state = britannia_state(grid, 10, 20);
        let virtue = ShrineVirtue::Compassion;
        state.shrine_codex_mask = virtue.bit();
        state.gold = 350;

        // Measured: the shrine is entered with `E` and answers a virtue
        // question before the mantra row (`qa/paired/shrine-flow.tsv`).
        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        // `karma.md §12` stages the entry narration; the virtue question is
        // its third beat. With no pump in these tests, print it now - until
        // it is on screen the prompt is not open and keys are buffered.
        state.flush_staged_narration();
        handle_play_key_input(&mut state, 'C', "OMPASSION\r", &dir).unwrap();
        for _ in 0..3 {
            // §12: holds six ticks before the first ask and twelve after
            // each nonblank answer; resync the same way the entry did.
            state.flush_staged_narration();
            state.adopt_shrine_prompt_row();
            assert_eq!(
                handle_play_key_input(&mut state, 'M', "u\r", &dir).unwrap(),
                PlayInputDisposition::Continue
            );
        }
        // `karma.md §12`: the third mantra answer has its own twelve-tick
        // hold (`cleak/u5-engine#23`) before the closing branch runs.
        state.flush_staged_narration();
        state
            .run_shrine_third_mantra_resolution_to_completion(&dir)
            .unwrap();
        assert!(state.active_shrine.is_some());

        assert_eq!(
            handle_play_key_input(&mut state, '2', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_shrine.is_none());
        assert_eq!(state.gold, 150);
        assert_eq!(state.moral_standing, 2);
        assert_eq!(state.message, "ALAKAZAM!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_completed_shrine_offering_repeats_until_affordable_or_cancelled() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SHRINE_TABLE_FILE),
            "BRITANNIA 10 20 COMPASSION 25\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
        let mut state = britannia_state(grid, 10, 20);
        let virtue = ShrineVirtue::Compassion;
        state.shrine_codex_mask = virtue.bit();
        state.gold = 100;

        handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
        // `karma.md §12` stages the entry narration; the virtue question is
        // its third beat. With no pump in these tests, print it now - until
        // it is on screen the prompt is not open and keys are buffered.
        state.flush_staged_narration();
        handle_play_key_input(&mut state, 'C', "OMPASSION\r", &dir).unwrap();
        for _ in 0..3 {
            // §12: holds six ticks before the first ask and twelve after
            // each nonblank answer; resync the same way the entry did.
            state.flush_staged_narration();
            state.adopt_shrine_prompt_row();
            handle_play_key_input(&mut state, 'M', "u\r", &dir).unwrap();
        }
        // `karma.md §12`: the third mantra answer has its own twelve-tick
        // hold (`cleak/u5-engine#23`) before the closing branch runs.
        state
            .run_shrine_third_mantra_resolution_to_completion(&dir)
            .unwrap();
        assert_eq!(
            handle_play_key_input(&mut state, '9', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_shrine.is_some());
        assert_eq!(state.gold, 100);
        assert_eq!(state.moral_standing, 0);
        assert!(state.message.contains("900 gp"));

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_shrine.is_none());
        assert_eq!(state.gold, 0);
        assert_eq!(state.moral_standing, 1);
        assert_eq!(state.message, "ALAKAZAM!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_codex_turn_in_raises_shared_moral_standing_by_three() {
        // karma.md §3-4/§7: Codex turn-in adds +3 to the shared
        // moral-standing selector (Humility another +3), clamped at 99.
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Justice);
        let bit = ShrineVirtue::Justice.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.moral_standing = 50;

        meditate_through_shrine_session(&mut state, &dir, "Justice", "Beh");

        assert_eq!(state.moral_standing, 53);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_codex_humility_turn_in_raises_shared_moral_standing_by_six() {
        // karma.md §3-4/§7: Codex turn-in adds +3 to the shared
        // moral-standing selector (Humility another +3), clamped at 99.
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Humility);
        let bit = ShrineVirtue::Humility.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.moral_standing = 40;

        meditate_through_shrine_session(&mut state, &dir, "Humility", "Lum");

        assert_eq!(state.moral_standing, 46);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_codex_turn_in_clamps_shared_moral_standing_at_ninety_nine() {
        // karma.md §3-4/§7: Codex turn-in adds +3 to the shared
        // moral-standing selector (Humility another +3), clamped at 99.
        let dir = debug_game_dir();
        let mut state = shrine_marker_state(&dir, ShrineVirtue::Justice);
        let bit = ShrineVirtue::Justice.bit();
        state.shrine_ordained_mask = bit;
        state.shrine_codex_mask = bit;
        state.moral_standing = 98;

        meditate_through_shrine_session(&mut state, &dir, "Justice", "Beh");

        assert_eq!(state.moral_standing, MORAL_STANDING_MAX);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shrine_offering_raises_shared_moral_standing_by_offered_digit() {
        // karma.md §3-4: completed-shrine gold offering adds the offered
        // digit (1..9) to the shared moral-standing selector, clamped at 99.
        let dir = debug_game_dir();
        let virtue = ShrineVirtue::Compassion;
        let mut state = shrine_marker_state(&dir, virtue);
        state.shrine_codex_mask = virtue.bit();
        state.gold = 800;
        state.moral_standing = 20;

        meditate_through_shrine_session(&mut state, &dir, "Compassion", "Mu");
        assert_eq!(
            handle_play_key_input(&mut state, '7', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.gold, 100);
        assert_eq!(state.moral_standing, 27);
        assert_eq!(state.message, "ALAKAZAM!");
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
            vec![
                SoundEffect::SharedVariant { variant: 2 },
                SoundEffect::TrapRumble,
            ]
        );
        assert_eq!(state.sound_effect_serial, serial + 2);
    }

    #[test]
    fn dungeon_stage_five_decoration_emits_its_depth_band_sweep_once() {
        let mut grid = open_dungeon_record();
        // Facing east, the north-adjacent cell is the visible band-0
        // left wall. A flavour-wall class routes through the decoration pass.
        let index = dungeon_cell_index(0, 4, 3);
        grid[index] = 0xc5;
        let mut state = dungeon_state(grid, 0, 4, 4);
        state.area = Area::Dungeon {
            scene: DungeonScene::from_record(1).unwrap(),
            level: 0,
        };
        state.torch_counter = 10;
        let serial = state.sound_effect_serial;

        state
            .render_dungeon_viewport(4, TileGraphicsDepth::Ega16, None, None)
            .unwrap();

        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::DungeonWallDrip { band: 0 }]
        );
        assert_eq!(state.grid[index], 0xc0);

        state
            .render_dungeon_viewport(4, TileGraphicsDepth::Ega16, None, None)
            .unwrap();
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::DungeonWallDrip { band: 0 }],
            "clearing stage five prevents the presentation cue from replaying on redraw"
        );
    }


#[test]
fn dungeon_flavour3_flourish_draws_on_four_in_sixty_five_and_always_consumes() {
    // `dungeon-mode.md §6.4` (u5-spec#298 Q12): a `0xC?` wall one cell
    // ahead in a flavour-3 dungeon takes one gameplay draw over `0..64`,
    // hit or miss; below 4 it paints ten pixels in pen 12.
    let lit = |seed: u16, record: u8| {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 5, 4)] = 0xc0;
        let mut state = dungeon_state(grid, 0, 4, 4);
        state.area = Area::Dungeon {
            scene: DungeonScene::from_record(record).unwrap(),
            level: 0,
        };
        state.player.facing = Direction::East;
        state.torch_counter = 10;
        state.prng_state = seed;
        let viewport = state
            .render_dungeon_viewport(4, TileGraphicsDepth::Ega16, None, None)
            .unwrap();
        let pixel = |x: i32, y: i32| {
            viewport.pixels[(y as usize - VIEWPORT_ORIGIN_Y) * viewport.width
                + (x as usize - VIEWPORT_ORIGIN_X)]
        };
        let eyes = DUNGEON_FLOURISH_RUNS
            .iter()
            .flat_map(|&(y, first, last)| (first..=last).map(move |x| (x, y)))
            .filter(|&(x, y)| pixel(x, y) == DUNGEON_FLOURISH_PEN)
            .count();
        (eyes, state.prng_state)
    };
    let hit = (1..=u16::MAX)
        .find(|&seed| u5_prng_range_u16(&mut seed.clone(), 0, 64) < 4)
        .unwrap();
    let miss = (1..=u16::MAX)
        .find(|&seed| u5_prng_range_u16(&mut seed.clone(), 0, 64) >= 4)
        .unwrap();
    for (seed, expected_eyes) in [(hit, 10), (miss, 0)] {
        let mut after = seed;
        u5_prng_range_u16(&mut after, 0, 64);
        // Scene record 0 is Deceit, a flavour-3 dungeon.
        assert_eq!(lit(seed, 0), (expected_eyes, after), "seed {seed:#06x}");
    }
    // Record 1 (Despise) is not flavour 3: no flourish (its own wall
    // decoration pass owns any draws there).
    assert_eq!(lit(hit, 1).0, 0);
}
