    #[test]
    fn dungeon_unhandled_play_input_uses_sleep_idle_visual_tick_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.clock = GameClock::new(12, 34).unwrap();
        state.torch_counter = 3;
        state.light_spell_counter = 2;
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

        assert_eq!(
            handle_play_key_input(&mut state, '?', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Zzzzzz...");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 34).unwrap());
        assert_eq!(state.torch_counter, 3);
        assert_eq!(state.light_spell_counter, 2);
        assert_eq!(state.animation.frame, 1);
        assert_eq!(state.active_objects[1].phase, 0x22);

        let mut town = test_state(open_grid(), 1, 1);
        assert_eq!(
            handle_play_key_input(&mut town, '?', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(town.message, "Unhandled command `?`.");
        assert_eq!(town.turn, 0);
    }

    #[test]
    fn dungeon_j_key_routes_to_jimmy_without_movement_fallback() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('J', Path::new("")).unwrap());

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7b);
        assert_eq!(state.message, "Unlocked!");
        assert!(!state.message.contains("Dungeon movement"));
    }

    #[test]
    fn dungeon_jimmy_prompts_for_picker_before_key_stock_check() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.keys = 0;

        assert_eq!(
            state
                .jimmy_facing_with_game_dir_and_member(None, None)
                .unwrap(),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("Who picks?"));
        assert!(state.active_jimmy.is_some());
        assert_eq!(state.turn, 0);
        assert_eq!(state.keys, 0);
    }

    #[test]
    fn dungeon_jimmy_plain_closed_chest_breaks_key_without_rewrite() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x40;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.keys = 2;

        assert_eq!(
            state.jimmy_facing_with_game_dir_and_member(None, Some(0)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x40);
        assert_eq!(state.keys, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Key broke!");
    }

    #[test]
    fn dungeon_jimmy_marked_chest_roll_success_unlocks_visit_local_cell() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.keys = 2;
        state.visibility_dirty = false;
        state.party[0].class_byte = 0;
        state.prng_state = 0x1234;
        let expected_prng_state = u5_prng_advance_state(state.prng_state);

        assert_eq!(
            state.jimmy_facing_with_game_dir_and_member(None, Some(0)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7b);
        assert_eq!(state.keys, 2);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Unlocked!");
    }

    #[test]
    fn dungeon_jimmy_marked_chest_roll_failure_breaks_key_without_rewrite() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.keys = 2;
        state.party[0].class_byte = 30;
        state.prng_state = 0x1234;
        let expected_prng_state = u5_prng_advance_state(state.prng_state);

        assert_eq!(
            state.jimmy_facing_with_game_dir_and_member(None, Some(0)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x4b);
        assert_eq!(state.keys, 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Key broke!");
    }

    #[test]
    fn jimmy_requires_keys_before_tile_probe() {
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.keys = 0;
        state.prng_state = 0x1234;

        assert_eq!(state.jimmy_facing(), MoveOutcome::Blocked);

        assert_eq!(state.prng_state, 0x1234);
        assert_eq!(state.message, "No keys!");
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[32 + 2], 96);
    }

    #[test]
    fn dungeon_look_reports_darkness_without_personal_light() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x40;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.look_dungeon(), MoveOutcome::Observed);

        assert_eq!(state.message, "You see: darkness.");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_render_blacks_out_without_personal_light() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x40;
        let state = dungeon_state(grid, 0, 1, 1);

        let view = state.render_text_view(5);

        assert!(view.contains("torch 0 spell 0"));
        assert!(view.contains("darkness"));
        assert!(!view.contains('$'));
    }

    #[test]
    fn dungeon_darkness_view_keeps_latest_command_feedback_visible() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.message = "Not here!".to_string();

        let view = state.render_text_view(5);

        assert!(view.contains("darkness"));
        assert!(view.contains("Not here!"));
        assert!(!view.contains('@'));
    }

    #[test]
    fn dungeon_render_uses_facing_relative_forward_view_when_lit() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x40;
        grid[dungeon_cell_index(0, 2, 0)] = 0xb0;
        grid[dungeon_cell_index(0, 2, 2)] = 0x50;
        grid[dungeon_cell_index(0, 3, 1)] = 0x80;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 9;

        let view = state.render_text_view(5);

        assert!(view.contains("First-person dungeon view"));
        assert!(view.contains("0: here passage"));
        assert!(view.contains("1: ahead a wooden chest; left a wall; right a fountain"));
        assert!(view.contains("2: ahead a sleep field"));
        assert!(!view.contains('$'));
    }

    #[test]
    fn dungeon_render_obscures_bands_behind_front_wall() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xb0;
        grid[dungeon_cell_index(0, 3, 1)] = 0x40;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.light_spell_counter = 3;

        let view = state.render_text_view(5);

        assert!(view.contains("1: ahead a wall"));
        assert!(view.contains("2: obscured by front wall"));
        assert!(!view.contains("wooden chest"));
    }

    #[test]
    fn dungeon_raster_frame_respects_light_gate() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.visibility_dirty = true;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert_eq!(viewport.width, 11 * TILE_ATLAS_SIDE);
        assert!(viewport.pixels.iter().all(|&pixel| pixel == 0));
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn dungeon_raster_frame_draws_facing_relative_wall_and_feature_cues() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x40;
        grid[dungeon_cell_index(0, 2, 0)] = 0xb0;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 9;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert!(viewport.pixels.iter().any(|&pixel| pixel == 15));
        assert!(viewport.pixels.iter().any(|&pixel| pixel == 8));
        assert!(viewport.pixels.iter().any(|&pixel| pixel == 6));
    }

    #[test]
    fn dungeon_raster_room_helper_blocks_far_features() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xa0;
        grid[dungeon_cell_index(0, 3, 1)] = 0x40;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 9;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert!(viewport.pixels.iter().any(|&pixel| pixel == 15));
        assert!(viewport.pixels.iter().any(|&pixel| pixel == 14));
        assert!(!viewport.pixels.iter().any(|&pixel| pixel == 6));
    }

    #[test]
    fn dungeon_raster_draws_side_feature_and_field_cues() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 0)] = 0x80;
        grid[dungeon_cell_index(0, 2, 2)] = 0x50;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.light_spell_counter = 9;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert!(viewport.pixels.iter().any(|&pixel| pixel == 12));
        assert!(viewport.pixels.iter().any(|&pixel| pixel == 11));
    }

    #[test]
    fn dungeon_raster_draws_active_monster_overlay_at_visible_depth() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 9;
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
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert!(viewport.pixels.iter().any(|&pixel| pixel == 13));
    }

    #[test]
    fn dungeon_raster_ignores_active_monster_from_other_level() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 9;
        state.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 2,
            y: 1,
            z: 1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert!(!viewport.pixels.iter().any(|&pixel| pixel == 13));
    }

    #[test]
    fn dungeon_look_uses_tile_description_when_lit() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x40;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.light_spell_counter = 3;

        assert_eq!(state.look_dungeon(), MoveOutcome::Observed);

        assert!(state.message.contains("wooden chest"));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_look_reports_marked_trap_variants_as_traps() {
        for tile in [0x69, 0x6a] {
            let mut grid = open_dungeon_record();
            grid[dungeon_cell_index(0, 2, 1)] = tile;
            let mut state = dungeon_state(grid, 0, 1, 1);
            state.player.facing = Direction::East;
            state.torch_counter = 5;

            assert_eq!(state.look_dungeon(), MoveOutcome::Observed);

            assert!(state.message.contains("pit or trap"));
            assert_eq!(state.turn, 0);
        }
    }

    #[test]
    fn dungeon_fountain_look_prompts_without_spending_turn() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x50;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;

        assert_eq!(state.look_dungeon(), MoveOutcome::Observed);

        assert_eq!(state.turn, 0);
        assert!(state.message.contains("You see: a fountain"));
        assert!(state.message.contains("Will you drink?"));
        assert_eq!(
            state.active_yes_no_prompt.as_ref().map(|session| session.kind),
            Some(YesNoPromptKind::DungeonFountainDrink {
                party_index: 0,
                focus: DungeonLookFocus::Ahead,
            })
        );
    }

    #[test]
    fn dungeon_fountain_drink_applies_cure_heal_and_poison_to_selected_member() {
        let mut cure_grid = open_dungeon_record();
        cure_grid[dungeon_cell_index(0, 2, 1)] = 0x50;
        let mut cure = dungeon_state(cure_grid, 0, 1, 1);
        cure.player.facing = Direction::East;
        cure.torch_counter = 5;
        cure.party[0].status = b'P';
        cure.party[0].hp = 7;

        assert_eq!(
            cure.look_dungeon_with_drink(Some(true), Some(0)),
            MoveOutcome::Observed
        );

        assert_eq!(cure.party[0].status, b'G');
        assert_eq!(cure.party[0].hp, 7);
        assert_eq!(cure.turn, 0);
        assert!(cure.message.contains("Cured!"));

        let mut heal_grid = open_dungeon_record();
        heal_grid[dungeon_cell_index(0, 2, 1)] = 0x51;
        let mut heal = dungeon_state(heal_grid, 0, 1, 1);
        heal.player.facing = Direction::East;
        heal.torch_counter = 5;
        heal.party[0].status = b'P';
        heal.party[0].hp = 4;
        heal.party[0].max_hp = 18;

        assert_eq!(
            heal.look_dungeon_with_drink(Some(true), Some(0)),
            MoveOutcome::Observed
        );

        assert_eq!(heal.party[0].status, b'P');
        assert_eq!(heal.party[0].hp, 18);
        assert_eq!(heal.turn, 0);
        assert!(heal.message.contains("Healed!"));

        let mut poison_grid = open_dungeon_record();
        poison_grid[dungeon_cell_index(0, 2, 1)] = 0x52;
        let mut poison = dungeon_state(poison_grid, 0, 1, 1);
        poison.player.facing = Direction::East;
        poison.torch_counter = 5;

        assert_eq!(
            poison.look_dungeon_with_drink(Some(true), Some(0)),
            MoveOutcome::Observed
        );

        assert_eq!(poison.party[0].status, b'P');
        assert_eq!(poison.turn, 0);
        assert!(poison.message.contains("Poisoned!"));
    }

    #[test]
    fn dungeon_fountain_bad_taste_damages_selected_member_without_spending_turn() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x53;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 10,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 10,
                max_hp: 20,
                level: 8,
            },
        ];
        state.prng_state = 0;
        let mut expected_prng = state.prng_state;
        let expected_damage = u5_prng_range_u16(&mut expected_prng, 0, 7) as u16;

        assert_eq!(
            state.look_dungeon_with_drink(Some(true), Some(1)),
            MoveOutcome::Observed
        );

        assert_eq!(state.party[0].hp, 10);
        assert_eq!(state.party[1].hp, 10 - expected_damage);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Bad taste."));
        assert!(state.message.contains("slot 1 took"));
    }

    #[test]
    fn dungeon_fountain_decline_and_invalid_member_do_not_mutate_party() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x52;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;

        assert_eq!(
            state.look_dungeon_with_drink(Some(false), Some(0)),
            MoveOutcome::PromptDeclined
        );

        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.turn, 0);

        assert_eq!(
            state.look_dungeon_with_drink(Some(true), Some(3)),
            MoveOutcome::Observed
        );

        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("party member 4 is unavailable"));
    }

    #[test]
    fn dungeon_l_key_can_inline_fountain_drink_choice_and_party_member() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x51;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 5,
                max_hp: 15,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'P',
                climb_stat: 30,
                mana: 8,
                hp: 4,
                max_hp: 19,
                level: 8,
            },
        ];

        assert!(
            state
                .handle_dungeon_key_with_inline(
                    'l',
                    Path::new(""),
                    None,
                    Some(true),
                    Some(1),
                    None,
                    Some(DungeonLookFocus::Ahead),
                )
                .unwrap()
        );

        assert_eq!(state.party[0].hp, 5);
        assert_eq!(state.party[1].hp, 19);
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Healed!"));
    }

    #[test]
    fn dungeon_view_requires_gem_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.gems = 0;

        assert_eq!(state.view_gem(), MoveOutcome::Blocked);

        assert_eq!(state.message, "No gems!");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_view_decrements_gem_and_reports_centered_flood_map_without_light() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 7, 1)] = 0x20;
        grid[dungeon_cell_index(0, 2, 1)] = 0x40;
        grid[dungeon_cell_index(0, 3, 1)] = 0xb0;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.gems = 2;

        assert_eq!(state.view_gem(), MoveOutcome::Observed);

        assert_eq!(state.gems, 1);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Dungeon view"));
        assert!(state.message.contains("centered flood map"));
        assert!(!state.message.contains("out of scope"));
        let rows: Vec<_> = state.message.lines().skip(1).collect();
        assert_eq!(rows.len(), 11);
        assert!(rows.iter().all(|row| row.chars().count() == 11));
        assert_eq!(rows[5].chars().nth(3), Some('>'));
        assert!(rows[5].contains("@$#"));
    }

    #[test]
    fn dungeon_view_flood_stops_expansion_at_wall_like_cells() {
        let mut grid = vec![0xb0; DUNGEON_RECORD_LEN];
        grid[dungeon_cell_index(0, 1, 1)] = 0x00;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.gems = 1;

        assert_eq!(state.view_gem(), MoveOutcome::Observed);

        let rows: Vec<_> = state.message.lines().skip(1).collect();
        assert_eq!(rows.len(), 11);
        assert_eq!(rows[5].chars().nth(5), Some('@'));
        assert_eq!(rows[5].chars().nth(6), Some('#'));
        assert_eq!(rows[5].chars().nth(7), Some(' '));
        assert_eq!(rows[4].chars().nth(4), Some('#'));
        assert_eq!(rows[4].chars().nth(7), Some(' '));
    }

    #[test]
    fn dungeon_view_flood_expands_through_door_and_room_classes() {
        for blocker in [0xa0, 0xe0, 0xf0] {
            let mut grid = vec![0xb0; DUNGEON_RECORD_LEN];
            grid[dungeon_cell_index(0, 1, 1)] = 0x00;
            grid[dungeon_cell_index(0, 2, 1)] = blocker;
            grid[dungeon_cell_index(0, 3, 1)] = 0x20;
            let mut state = dungeon_state(grid, 0, 1, 1);
            state.gems = 1;

            assert_eq!(state.view_gem(), MoveOutcome::Observed);

            let rows: Vec<_> = state.message.lines().skip(1).collect();
            assert_eq!(rows[5].chars().nth(6), Some('+'));
            assert_eq!(rows[5].chars().nth(7), Some('>'));
        }
    }

    #[test]
    fn dungeon_view_flood_stops_at_wall_presentation_classes() {
        for blocker in [0xb0, 0xc0, 0xd0] {
            let mut grid = vec![0xb0; DUNGEON_RECORD_LEN];
            grid[dungeon_cell_index(0, 1, 1)] = 0x00;
            grid[dungeon_cell_index(0, 2, 1)] = blocker;
            grid[dungeon_cell_index(0, 3, 1)] = 0x20;
            let mut state = dungeon_state(grid, 0, 1, 1);
            state.gems = 1;

            assert_eq!(state.view_gem(), MoveOutcome::Observed);

            let rows: Vec<_> = state.message.lines().skip(1).collect();
            assert_eq!(rows[5].chars().nth(6), Some('#'));
            assert_eq!(rows[5].chars().nth(7), Some(' '));
        }
    }

    #[test]
    fn town_view_decrements_gem_and_reports_full_fill_map_without_turn() {
        let mut state = test_state(open_grid(), 5, 5);
        state.gems = 1;
        state.active_objects.push(ActiveObject {
            type_byte: 0xaa,
            tile: 0xaa,
            x: 6,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.view_gem(), MoveOutcome::Observed);

        assert_eq!(state.gems, 0);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Gem view of CASTLE:0"));
        assert!(state.message.contains("32x32 class map"));
        let rows: Vec<_> = state.message.lines().skip(1).collect();
        assert_eq!(rows.len(), 32);
        assert!(rows.iter().all(|row| row.chars().count() == 32));
        assert_eq!(rows[16].chars().nth(16), Some('@'));
        assert_eq!(rows[16].chars().nth(17), Some('3'));
        assert!(state.active_view_overlay.is_some());
        let atlas = TileAtlas {
            depth: TileGraphicsDepth::Ega16,
            pixels: Vec::new(),
        };
        let viewport = state.render_top_down_frame(5, &atlas).unwrap().unwrap();
        assert_eq!(viewport.cells_wide, LOCAL_VIEW_OVERLAY_SIDE);
        assert_eq!(viewport.cells_high, LOCAL_VIEW_OVERLAY_SIDE);
        assert_eq!(
            viewport.width,
            LOCAL_VIEW_OVERLAY_SIDE * LOCAL_VIEW_CELL_PIXEL_SCALE
        );
        assert_eq!(viewport.pixel(66, 64), Some(15));
    }

    #[test]
    fn world_view_decrements_gem_and_wraps_full_fill_map_without_turn() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.gems = 2;
        state.active_objects.push(ActiveObject {
            type_byte: 170,
            tile: 170,
            x: 255,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.view_gem(), MoveOutcome::Observed);

        assert_eq!(state.gems, 1);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Gem view of UNDERWORLD"));
        assert!(state.message.contains("32x32 class map"));
        let rows: Vec<_> = state.message.lines().skip(1).collect();
        assert_eq!(rows.len(), 32);
        assert!(rows.iter().all(|row| row.chars().count() == 32));
        assert_eq!(rows[16].chars().nth(15), Some('3'));
        assert_eq!(rows[16].chars().nth(16), Some('@'));
        assert_eq!(rows[16].chars().nth(17), Some('1'));
    }

    #[test]
    fn active_view_overlay_clears_on_next_key_without_turn_or_extra_gem() {
        let mut state = test_state(open_grid(), 5, 5);
        state.gems = 2;

        assert_eq!(state.view_gem(), MoveOutcome::Observed);
        assert!(state.active_view_overlay.is_some());
        assert_eq!(state.gems, 1);

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_view_overlay.is_none());
        assert_eq!(state.message, "View closed.");
        assert_eq!(state.gems, 1);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn active_view_overlay_leaves_base_viewport_renderable_for_composed_frames() {
        let mut grid = open_grid();
        grid[5 * 32 + 5] = 5;
        let mut state = test_state(grid, 5, 5);
        state.gems = 1;
        assert_eq!(state.view_gem(), MoveOutcome::Observed);

        let atlas = TileAtlas {
            depth: TileGraphicsDepth::Ega16,
            pixels: vec![2; 512 * TILE_ATLAS_SIDE * TILE_ATLAS_SIDE],
        };
        let replacement = state.render_top_down_frame(5, &atlas).unwrap().unwrap();
        let base = state
            .render_top_down_base_frame(5, &atlas)
            .unwrap()
            .unwrap();

        assert_eq!(replacement.cells_wide, LOCAL_VIEW_OVERLAY_SIDE);
        assert_eq!(base.cells_wide, VIEWPORT_SIDE);
        assert_eq!(base.cells_high, VIEWPORT_SIDE);
        assert!(state.active_view_overlay.is_some());
    }

    #[test]
    fn dungeon_view_overlay_renders_centered_minimap_raster() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x40;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.gems = 1;

        assert_eq!(state.view_gem(), MoveOutcome::Observed);

        let overlay = state.active_view_overlay.as_ref().unwrap();
        assert!(matches!(
            overlay.kind,
            ViewOverlayKind::Dungeon { level: 0 }
        ));
        let viewport = state.render_active_view_overlay(TileGraphicsDepth::Ega16).unwrap();
        assert_eq!(viewport.cells_wide, 11);
        assert_eq!(viewport.cells_high, 11);
        assert_eq!(viewport.width, 11 * LOCAL_VIEW_CELL_PIXEL_SCALE);
        assert_eq!(viewport.pixel(22, 20), Some(15));
    }

    #[test]
    fn dungeon_view_overlay_uses_published_minimap_glyph_ids_for_raster() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x10;
        grid[dungeon_cell_index(0, 0, 1)] = 0x20;
        grid[dungeon_cell_index(0, 1, 0)] = 0x30;
        grid[dungeon_cell_index(0, 1, 2)] = 0x40;
        grid[dungeon_cell_index(0, 2, 0)] = 0x50;
        grid[dungeon_cell_index(0, 0, 0)] = 0x60;
        grid[dungeon_cell_index(0, 2, 2)] = 0x61;
        grid[dungeon_cell_index(0, 0, 2)] = 0xE0;
        let state = dungeon_state(grid, 0, 1, 1);

        let glyphs = state.dungeon_vision_glyphs(0);
        let side = (DUNGEON_GEM_VIEW_RADIUS * 2 + 1) as usize;
        let center = DUNGEON_GEM_VIEW_RADIUS as usize;
        let at = |dx: isize, dy: isize| {
            let x = (center as isize + dx) as usize;
            let y = (center as isize + dy) as usize;
            glyphs[y * side + x]
        };

        assert_eq!(at(0, 0), Some(0xff));
        assert_eq!(at(1, 0), Some(0x2E));
        assert_eq!(at(-1, 0), Some(0x2D));
        assert_eq!(at(0, -1), Some(0x2F));
        assert_eq!(at(0, 1), Some(0x70));
        assert_eq!(at(1, -1), Some(0x12));
        assert_eq!(at(-1, -1), Some(0x19));
        assert_eq!(at(1, 1), Some(0x71));
        assert_eq!(at(-1, 1), Some(0x77));

        let viewport =
            state.render_dungeon_view_overlay_for_mode(0, TileGraphicsDepth::Ega16, ViewOverlayMode::GemView);
        let cell = LOCAL_VIEW_CELL_PIXEL_SCALE;
        let px = |dx: isize, dy: isize, lx: usize, ly: usize| {
            let x = (center as isize + dx) as usize * cell + lx;
            let y = (center as isize + dy) as usize * cell + ly;
            viewport.pixel(x, y)
        };
        assert_eq!(px(1, 0, cell / 2, 0), Some(14));
        assert_eq!(px(0, 1, 0, 0), Some(14));
        assert_eq!(px(1, -1, cell / 2, cell / 2), Some(11));
        assert_eq!(px(-1, 1, cell / 2, cell / 2), Some(14));
    }

    #[test]
    fn surface_view_class_uses_spec_tile_ranges() {
        assert_eq!(surface_view_class(0x00), 0x00);
        assert_eq!(surface_view_class(0x05), 0x01);
        assert_eq!(surface_view_class(0x2d), 0x02);
        assert_eq!(surface_view_class(0x70), 0x03);
        assert_eq!(surface_view_class(0x5a), 0x04);
        assert_eq!(surface_view_class(0x80), 0x05);
        assert_eq!(surface_view_class(0xec), 0x06);
        assert_eq!(surface_view_class(0xfe), 0x07);
        assert_eq!(surface_view_class(0x0e), 0x08);
        assert_eq!(surface_view_class(0x2c), 0x09);
        assert_eq!(surface_view_class(0xe4), 0x0a);
        assert_eq!(surface_view_class(0xd4), 0x0b);
        assert_eq!(surface_view_class(0x01), 0x0c);
        assert_eq!(surface_view_class(0x04), 0x0d);
        assert_eq!(surface_view_class(0xe3), 0x0e);
        assert_eq!(surface_view_class(0xdc), 0x0f);
        assert_eq!(surface_view_class(0x26), 0x10);
        assert_eq!(render_surface_view_class(0x0a), 'A');
        assert_eq!(render_surface_view_class(0x10), 'G');
    }

    #[test]
    fn surface_view_overlay_renders_spec_class_shapes() {
        let mut grid = open_grid();
        grid[5 * 32 + 6] = 0x70;
        grid[5 * 32 + 7] = 0x1D;
        grid[5 * 32 + 8] = 0x04;
        let state = test_state(grid, 5, 5);
        let viewport = state.render_surface_view_overlay_for_mode(
            TileGraphicsDepth::Ega16,
            ViewOverlayMode::GemView,
        );
        let scale = LOCAL_VIEW_CELL_PIXEL_SCALE;
        let center = LOCAL_VIEW_OVERLAY_SIDE / 2;
        let sample = |dx: usize, lx: usize, ly: usize| {
            viewport.pixel((center + dx) * scale + lx, center * scale + ly)
        };

        assert_eq!(sample(1, 0, 0), Some(3));
        assert_eq!(sample(1, scale / 2, scale / 2), Some(3));
        assert_eq!(sample(2, 0, 0), Some(14));
        assert_eq!(sample(2, 0, scale - 1), Some(14));
        assert_eq!(sample(2, 0, scale / 2), Some(0));
        assert_eq!(sample(3, 0, 2), Some(12));
        assert_eq!(sample(3, 1, 1), Some(0));
    }

    #[test]
    fn surface_view_overlay_modes_apply_peer_gem_alternate_bank() {
        let mut grid = open_grid();
        grid[5 * 32 + 6] = 0xDC;
        let state = test_state(grid, 5, 5);

        let gem = state.render_surface_view_overlay_for_mode(
            TileGraphicsDepth::Ega16,
            ViewOverlayMode::GemView,
        );
        let x_ray = state.render_surface_view_overlay_for_mode(
            TileGraphicsDepth::Ega16,
            ViewOverlayMode::XRaySpell,
        );
        let cell_x = LOCAL_VIEW_OVERLAY_SIDE / 2 + 1;
        let cell_y = LOCAL_VIEW_OVERLAY_SIDE / 2;
        let px = cell_x * LOCAL_VIEW_CELL_PIXEL_SCALE;
        let py = cell_y * LOCAL_VIEW_CELL_PIXEL_SCALE;

        assert_eq!(x_ray.pixel(px, py), Some(4));
        assert_eq!(gem.pixel(px, py), Some(14));
    }

    fn surface_view_class_gallery_state() -> PlayState {
        let mut grid = vec![0; TOWN_GRID_SIDE * TOWN_GRID_SIDE];
        for (index, (tile, _class)) in SURFACE_VIEW_CLASS_GALLERY_TILES.iter().enumerate() {
            grid[4 * TOWN_GRID_SIDE + 4 + index] = *tile;
        }
        test_state(grid, TOWN_GRID_SIDE / 2, TOWN_GRID_SIDE / 2)
    }

    const SURFACE_VIEW_CLASS_GALLERY_TILES: [(u8, u8); 17] = [
        (0x00, 0x00),
        (0x05, 0x01),
        (0x09, 0x02),
        (0x70, 0x03),
        (0x1D, 0x04),
        (0x10, 0x05),
        (0x0D, 0x06),
        (0x0C, 0x07),
        (0x0B, 0x08),
        (0x06, 0x09),
        (0x60, 0x0A),
        (0xD4, 0x0B),
        (0x01, 0x0C),
        (0x04, 0x0D),
        (0xE0, 0x0E),
        (0xD8, 0x0F),
        (0x20, 0x10),
    ];

    #[test]
    fn surface_view_overlay_class_gallery_covers_all_published_classes() {
        for (tile, class) in SURFACE_VIEW_CLASS_GALLERY_TILES {
            assert_eq!(surface_view_class(tile), class);
        }

        for mode in [
            ViewOverlayMode::GemView,
            ViewOverlayMode::PeerSpell,
            ViewOverlayMode::XRaySpell,
        ] {
            let state = surface_view_class_gallery_state();
            let viewport = state.render_surface_view_overlay_for_mode(TileGraphicsDepth::Ega16, mode);
            let scale = LOCAL_VIEW_CELL_PIXEL_SCALE;

            for (index, (_tile, class)) in SURFACE_VIEW_CLASS_GALLERY_TILES.iter().enumerate() {
                let cell_x = 4 + index;
                let cell_y = 4;
                let has_colored_pixel = (0..scale).any(|local_y| {
                    (0..scale).any(|local_x| {
                        viewport.pixel(cell_x * scale + local_x, cell_y * scale + local_y)
                            != Some(0)
                    })
                });
                assert_eq!(
                    has_colored_pixel,
                    !matches!(class, 0x00 | 0x0C),
                    "class {class:#04x} mode {mode:?}"
                );
            }
        }
    }

    #[test]
    fn surface_view_overlay_class_gallery_pins_peer_gem_bank_switches() {
        let state = surface_view_class_gallery_state();
        let gem = state.render_surface_view_overlay_for_mode(
            TileGraphicsDepth::Ega16,
            ViewOverlayMode::GemView,
        );
        let peer = state.render_surface_view_overlay_for_mode(
            TileGraphicsDepth::Ega16,
            ViewOverlayMode::PeerSpell,
        );
        let x_ray = state.render_surface_view_overlay_for_mode(
            TileGraphicsDepth::Ega16,
            ViewOverlayMode::XRaySpell,
        );
        let scale = LOCAL_VIEW_CELL_PIXEL_SCALE;
        let sample = |viewport: &TileViewport, index: usize, local_x: usize, local_y: usize| {
            viewport.pixel((4 + index) * scale + local_x, 4 * scale + local_y)
        };

        assert_eq!(sample(&gem, 10, 1, 0), Some(3));
        assert_eq!(sample(&peer, 10, 1, 0), Some(3));
        assert_eq!(sample(&x_ray, 10, 1, 0), Some(11));

        assert_eq!(sample(&gem, 11, 0, 0), Some(11));
        assert_eq!(sample(&peer, 11, 0, 0), Some(11));
        assert_eq!(sample(&x_ray, 11, 0, 0), Some(13));

        assert_eq!(sample(&gem, 15, 0, 0), Some(14));
        assert_eq!(sample(&peer, 15, 0, 0), Some(14));
        assert_eq!(sample(&x_ray, 15, 0, 0), Some(4));
    }

    fn surface_view_audit_mask(class: u8, tile: u8, mode: ViewOverlayMode) -> [[u8; 4]; 4] {
        assert_eq!(LOCAL_VIEW_CELL_PIXEL_SCALE, 4);
        let viewport = PlayState::render_surface_view_class_cell_for_mode(
            TileGraphicsDepth::Ega16,
            class,
            tile,
            false,
            mode,
        );
        let mut mask = [[0; 4]; 4];
        for (y, row) in mask.iter_mut().enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = viewport.pixel(x, y).unwrap();
            }
        }
        mask
    }

    #[test]
    fn surface_view_overlay_audit_masks_cover_public_class_contracts() {
        let gem = ViewOverlayMode::GemView;
        let solid_2 = [[2, 2, 2, 2]; 4];

        assert_eq!(surface_view_audit_mask(0x00, 0x00, gem), [[0; 4]; 4]);
        assert_eq!(
            surface_view_audit_mask(0x01, 0x05, gem),
            [[0, 7, 0, 0], [0, 0, 0, 7], [0, 7, 0, 0], [0, 0, 0, 7]]
        );
        assert_eq!(surface_view_audit_mask(0x02, 0x09, gem), solid_2);
        assert_eq!(
            surface_view_audit_mask(0x03, 0x70, gem),
            [[3, 3, 3, 3], [3, 3, 3, 3], [3, 3, 3, 3], [3, 3, 3, 3]]
        );
        assert_eq!(
            surface_view_audit_mask(0x04, 0x1D, gem),
            [[14, 14, 14, 14], [0, 0, 0, 0], [0, 0, 0, 0], [14, 14, 14, 14]]
        );
        assert_eq!(
            surface_view_audit_mask(0x05, 0x10, gem),
            [[0, 0, 0, 0], [0, 15, 15, 0], [0, 15, 15, 0], [0, 0, 0, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x06, 0x0D, gem),
            [[8, 8, 8, 8], [8, 0, 0, 8], [8, 0, 0, 8], [8, 8, 8, 8]]
        );
        assert_eq!(
            surface_view_audit_mask(0x07, 0x0C, gem),
            [[6, 6, 6, 6], [6, 6, 6, 6], [6, 6, 6, 6], [6, 6, 6, 6]]
        );
        assert_eq!(
            surface_view_audit_mask(0x08, 0x0B, gem),
            [[5, 5, 0, 0], [5, 5, 0, 0], [0, 0, 5, 5], [0, 0, 5, 5]]
        );
        assert_eq!(
            surface_view_audit_mask(0x09, 0x06, gem),
            [[10, 10, 10, 10], [0, 0, 0, 0], [10, 10, 10, 10], [10, 0, 0, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x0A, 0x60, gem),
            [[0, 3, 0, 0], [0, 0, 0, 3], [0, 3, 0, 0], [0, 0, 0, 3]]
        );
        assert_eq!(
            surface_view_audit_mask(0x0B, 0xD4, gem),
            [[11, 0, 0, 0], [0, 0, 0, 0], [0, 0, 11, 0], [0, 0, 0, 0]]
        );
        assert_eq!(surface_view_audit_mask(0x0C, 0x01, gem), [[0; 4]; 4]);
        assert_eq!(
            surface_view_audit_mask(0x0D, 0x04, gem),
            [[0, 2, 0, 0], [0, 0, 0, 2], [12, 0, 0, 0], [0, 0, 12, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x0E, 0xE0, gem),
            [[0, 9, 9, 0], [0, 9, 9, 0], [0, 9, 9, 0], [0, 9, 9, 0]]
        );
        assert_eq!(surface_view_audit_mask(0x0F, 0xD8, gem), [[14, 14, 14, 14]; 4]);
        assert_eq!(
            surface_view_audit_mask(0x10, 0x20, gem),
            [[0, 0, 0, 0], [0, 3, 3, 0], [0, 3, 3, 0], [0, 0, 0, 0]]
        );
    }

    #[test]
    fn surface_view_overlay_audit_covers_fence_bits_and_direct_wall_bank_handler() {
        let gem = ViewOverlayMode::GemView;

        assert_eq!(
            surface_view_audit_mask(0x10, 0x21, gem),
            [[0, 1, 1, 0], [0, 3, 3, 0], [0, 3, 3, 0], [0, 0, 0, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x10, 0x22, gem),
            [[0, 0, 0, 0], [0, 3, 3, 1], [0, 1, 3, 1], [0, 0, 0, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x10, 0x23, gem),
            [[0, 1, 1, 0], [0, 1, 3, 1], [0, 3, 3, 1], [0, 0, 0, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x10, 0x24, gem),
            [[0, 0, 0, 0], [0, 3, 1, 0], [0, 3, 3, 0], [0, 1, 1, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x10, 0x25, gem),
            [[0, 1, 1, 0], [0, 3, 3, 0], [0, 3, 1, 0], [0, 1, 1, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x10, 0x26, gem),
            [[0, 0, 0, 0], [0, 3, 3, 1], [0, 3, 3, 1], [0, 1, 1, 0]]
        );
        assert_eq!(
            surface_view_audit_mask(0x5A, 0x5A, gem),
            [[6, 6, 6, 6], [6, 6, 6, 6], [6, 6, 6, 6], [6, 6, 6, 6]]
        );
    }

    fn dungeon_view_audit_mask(glyph: Option<u8>, mode: ViewOverlayMode) -> [[u8; 4]; 4] {
        assert_eq!(LOCAL_VIEW_CELL_PIXEL_SCALE, 4);
        let viewport = PlayState::render_dungeon_view_glyph_cell_for_mode(
            TileGraphicsDepth::Ega16,
            glyph,
            mode,
        );
        let mut mask = [[0; 4]; 4];
        for (y, row) in mask.iter_mut().enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = viewport.pixel(x, y).unwrap();
            }
        }
        mask
    }

    #[test]
    fn dungeon_view_overlay_audit_masks_cover_public_glyph_families() {
        let gem = ViewOverlayMode::GemView;

        assert_eq!(dungeon_view_audit_mask(None, gem), [[0; 4]; 4]);
        assert_eq!(
            dungeon_view_audit_mask(Some(0xff), gem),
            [[0, 0, 15, 0], [0, 0, 15, 0], [15, 15, 15, 15], [0, 0, 15, 0]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x18), gem),
            [[0, 0, 0, 0], [0, 0, 0, 0], [7, 7, 7, 7], [0, 0, 0, 0]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x2E), gem),
            [[14, 14, 14, 14], [0, 0, 14, 0], [0, 0, 14, 0], [0, 0, 14, 0]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x2D), gem),
            [[0, 0, 14, 0], [0, 0, 14, 0], [0, 0, 14, 0], [14, 14, 14, 14]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x2F), gem),
            [[14, 14, 14, 14], [0, 0, 14, 0], [14, 14, 14, 14], [14, 14, 14, 14]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x70), gem),
            [[14, 14, 14, 14], [14, 6, 6, 14], [14, 6, 6, 14], [14, 14, 14, 14]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x12), gem),
            [[0, 0, 11, 0], [0, 0, 11, 0], [11, 11, 11, 11], [0, 0, 11, 0]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x19), gem),
            [[14, 14, 14, 14], [14, 0, 0, 14], [14, 0, 14, 14], [14, 14, 14, 14]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x71), gem),
            [[12, 0, 0, 12], [0, 12, 12, 0], [0, 12, 12, 0], [12, 0, 0, 12]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x72), gem),
            [[14, 0, 0, 14], [0, 14, 14, 0], [0, 14, 14, 0], [14, 0, 0, 14]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x73), gem),
            [[0, 0, 14, 14], [0, 0, 14, 14], [14, 14, 14, 14], [0, 0, 14, 14]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x74), gem),
            [[13, 13, 13, 13], [13, 0, 0, 13], [13, 0, 0, 13], [13, 13, 13, 13]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x75), gem),
            [[13, 13, 14, 13], [13, 0, 14, 13], [13, 0, 14, 13], [13, 13, 14, 13]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x76), gem),
            [[5, 5, 5, 5], [5, 0, 0, 5], [5, 0, 14, 5], [5, 5, 5, 5]]
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x77), gem),
            [[14, 14, 14, 14], [14, 0, 14, 14], [14, 14, 14, 14], [14, 14, 14, 14]]
        );
        assert_eq!(dungeon_view_audit_mask(Some(0x7F), gem), [[13, 13, 13, 13]; 4]);
    }

    #[test]
    fn dungeon_view_overlay_audit_pins_peer_gem_tint_against_xray_mode() {
        let gem = ViewOverlayMode::GemView;
        let peer = ViewOverlayMode::PeerSpell;
        let x_ray = ViewOverlayMode::XRaySpell;

        assert_eq!(
            dungeon_view_audit_mask(Some(0x2E), gem),
            dungeon_view_audit_mask(Some(0x2E), peer)
        );
        assert_eq!(
            dungeon_view_audit_mask(Some(0x73), gem),
            dungeon_view_audit_mask(Some(0x73), peer)
        );

        assert_eq!(dungeon_view_audit_mask(Some(0x2E), x_ray)[0][0], 15);
        assert_eq!(dungeon_view_audit_mask(Some(0x12), x_ray)[2][0], 9);
        assert_eq!(dungeon_view_audit_mask(Some(0x73), x_ray)[2][0], 11);
        assert_eq!(dungeon_view_audit_mask(Some(0x74), x_ray)[0][0], 8);
        assert_eq!(dungeon_view_audit_mask(Some(0x76), x_ray)[0][0], 13);
    }

    #[test]
    fn ignite_torch_consumes_stock_and_lights_dungeon() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.torches = 2;

        assert_eq!(state.ignite_torch(), MoveOutcome::Ignited);

        assert_eq!(state.torches, 1);
        assert!((112..=127).contains(&state.torch_counter));
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.has_personal_light());
    }

    #[test]
    fn ignite_torch_refuses_without_stock_or_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.torches = 0;

        assert_eq!(state.ignite_torch(), MoveOutcome::Blocked);

        assert_eq!(state.message, "No torches!");
        assert_eq!(state.turn, 0);
        assert_eq!(state.torch_counter, 0);
    }

    #[test]
    fn ignite_torch_sets_surface_duration() {
        let mut state = test_state(open_grid(), 1, 1);
        state.torches = 1;

        assert_eq!(state.ignite_torch(), MoveOutcome::Ignited);

        assert_eq!(state.torches, 0);
        assert_eq!(state.torch_counter, SURFACE_TORCH_DURATION);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
    }

    #[test]
    fn mode_zero_cleanup_recomputes_daylight_without_turn_work() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(5, 30).unwrap();
        state.ambient_light = FULL_DARKNESS;
        state.visibility_dirty = false;
        state.door_tracker = Some(DoorTracker {
            previous_tile: 7,
            x: 1,
            y: 1,
            turns_remaining: 1,
        });
        state.animation.tick_static_tiles();
        let frame = state.animation.frame;

        state.mode_zero_cleanup();

        assert_eq!(state.ambient_light, DAWN_DUSK_LIGHT[3]);
        assert!(state.visibility_dirty);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(5, 30).unwrap());
        assert_eq!(state.animation.frame, frame);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 7,
                x: 1,
                y: 1,
                turns_remaining: 1,
            })
        );
    }

    #[test]
    fn daylight_gradient_matches_public_time_and_lighting_specs() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        for (minute, expected) in [(0, 2), (10, 5), (20, 10), (30, 20), (40, 34), (50, 49)] {
            state.clock = GameClock::new(5, minute).unwrap();
            state.ambient_light = 0;
            state.visibility_dirty = false;
            state.mode_zero_cleanup();
            assert_eq!(state.ambient_light, expected, "dawn minute {minute}");

            state.clock = GameClock::new(19, 59 - minute).unwrap();
            state.ambient_light = 0;
            state.visibility_dirty = false;
            state.mode_zero_cleanup();
            assert_eq!(state.ambient_light, expected, "dusk minute {}", 59 - minute);
        }
    }

    #[test]
    fn daylight_recompute_applies_fixed_dark_floors_and_sentinels() {
        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon.clock = GameClock::new(12, 0).unwrap();
        dungeon.mode_zero_cleanup();
        assert_eq!(dungeon.ambient_light, FULL_DARKNESS);

        dungeon.torch_counter = 3;
        dungeon.visibility_dirty = false;
        dungeon.mode_zero_cleanup();
        assert_eq!(dungeon.ambient_light, TORCH_LIGHT_FLOOR);
        assert!(dungeon.visibility_dirty);

        dungeon.torch_counter = 0;
        dungeon.light_spell_counter = 3;
        dungeon.visibility_dirty = false;
        dungeon.mode_zero_cleanup();
        assert_eq!(dungeon.ambient_light, LIGHT_SPELL_FLOOR);
        assert!(dungeon.visibility_dirty);

        dungeon.ambient_light = DAYLIGHT_SENTINEL_MIN;
        dungeon.visibility_dirty = false;
        dungeon.mode_zero_cleanup();
        assert_eq!(dungeon.ambient_light, DAYLIGHT_SENTINEL_MIN);
        assert!(!dungeon.visibility_dirty);
    }

    #[test]
    fn surface_visibility_radius_follows_cached_ambient_light() {
        let mut state = britannia_state(open_world_grid(), 1, 1);

        state.ambient_light = FULL_DAYLIGHT;
        assert_eq!(state.surface_visibility_radius(5), 5);

        state.ambient_light = DAWN_DUSK_LIGHT[4];
        assert_eq!(state.surface_visibility_radius(5), 4);

        state.ambient_light = DAWN_DUSK_LIGHT[3];
        assert_eq!(state.surface_visibility_radius(5), 3);

        state.ambient_light = TORCH_LIGHT_FLOOR;
        assert_eq!(state.surface_visibility_radius(5), 2);

        state.ambient_light = LIGHT_SPELL_FLOOR;
        assert_eq!(state.surface_visibility_radius(5), 2);

        state.ambient_light = DAWN_DUSK_LIGHT[1];
        assert_eq!(state.surface_visibility_radius(5), 1);

        state.ambient_light = FULL_DARKNESS;
        assert_eq!(state.surface_visibility_radius(5), 0);
    }

    #[test]
    fn town_visibility_uses_local_light_mask_beyond_player_radius() {
        let mut unlit = test_state(open_grid(), 5, 5);
        unlit.ambient_light = DAWN_DUSK_LIGHT[1];

        let unlit_view = unlit.render_text_view(5);
        assert_eq!(unlit_view.lines().nth(10).unwrap().chars().nth(5), Some(' '));

        let mut lit_grid = open_grid();
        lit_grid[8 * TOWN_GRID_SIDE + 5] = 0xDC;
        let mut lit = test_state(lit_grid, 5, 5);
        lit.ambient_light = DAWN_DUSK_LIGHT[1];

        let lit_view = lit.render_text_view(5);
        assert_ne!(lit_view.lines().nth(10).unwrap().chars().nth(5), Some(' '));
        assert!(lit.town_cell_visible_with_light_radius(5, 5, 5, 9, 5, 1));
    }

    #[test]
    fn town_local_light_mask_respects_visibility_blockers() {
        let mut grid = open_grid();
        for x in 0..=10 {
            grid[7 * TOWN_GRID_SIDE + x] = 24;
        }
        grid[8 * TOWN_GRID_SIDE + 5] = 0xDC;
        let state = test_state(grid, 5, 5);

        assert!(!state.town_cell_visible_with_light_radius(5, 5, 5, 9, 5, 1));
    }

    #[test]
    fn town_local_light_uses_source_to_target_carves_not_flood_fill() {
        let mut grid = open_grid();
        grid[8 * TOWN_GRID_SIDE + 8] = 0xDC;
        grid[7 * TOWN_GRID_SIDE + 8] = 24;
        let state = test_state(grid, 8, 3);

        assert!(!state.town_cell_visible_with_light_radius(8, 3, 8, 6, 6, 0));
        assert!(state.town_cell_visible_with_light_radius(8, 3, 7, 6, 6, 0));
    }

    #[test]
    fn town_local_light_all_public_terrain_source_ids_emit_light() {
        for source_tile in [
            0xB0u8, 0xB1, 0xB2, 0xB3, 0xBC, 0xBD, 0xBE, 0xBF, 0xDC, 0xDE,
        ] {
            let mut grid = open_grid();
            grid[8 * TOWN_GRID_SIDE + 8] = source_tile;
            let state = test_state(grid, 8, 3);

            assert!(
                state.town_cell_visible_with_light_radius(8, 3, 8, 6, 6, 0),
                "source tile {source_tile:#04x} should light within radius"
            );
        }
    }

    #[test]
    fn town_local_light_masks_union_multiple_sources() {
        let mut grid = open_grid();
        grid[8 * TOWN_GRID_SIDE + 5] = 0xDC;
        grid[8 * TOWN_GRID_SIDE + 11] = 0xDE;
        let state = test_state(grid, 8, 3);

        assert!(state.town_cell_visible_with_light_radius(8, 3, 5, 6, 6, 0));
        assert!(state.town_cell_visible_with_light_radius(8, 3, 11, 6, 6, 0));
        assert!(!state.town_cell_visible_with_light_radius(8, 3, 11, 4, 6, 0));
    }

    #[test]
    fn town_local_light_can_be_reached_through_open_dark_space() {
        let mut grid = open_grid();
        grid[18 * TOWN_GRID_SIDE + 10] = 0xDC;
        let mut state = test_state(grid, 10, 10);
        state.ambient_light = DAWN_DUSK_LIGHT[1];

        assert!(state.town_cell_visible_with_light_radius(10, 10, 10, 18, 10, 1));
        assert_eq!(
            state
                .render_text_view(10)
                .lines()
                .nth(19)
                .unwrap()
                .chars()
                .nth(10),
            Some('^')
        );
    }

    #[test]
    fn town_local_light_uses_chebyshev_radius_three() {
        let mut grid = open_grid();
        grid[8 * TOWN_GRID_SIDE + 8] = 0xDC;
        let state = test_state(grid, 5, 5);

        assert!(state.town_cell_visible_with_light_radius(5, 5, 11, 11, 8, 0));
        assert!(!state.town_cell_visible_with_light_radius(5, 5, 12, 8, 8, 0));
    }

    #[test]
    fn active_object_local_light_sources_contribute_to_surface_visibility() {
        let mut state = test_state(open_grid(), 10, 10);
        state.ambient_light = DAWN_DUSK_LIGHT[1];
        state.active_objects.push(ActiveObject {
            type_byte: 0xDC,
            tile: 0xDC,
            x: 10,
            y: 18,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert!(state.town_cell_visible_with_light_radius(10, 10, 10, 18, 10, 1));
        assert_eq!(
            state
                .render_text_view(10)
                .lines()
                .nth(19)
                .unwrap()
                .chars()
                .nth(10),
            Some('^')
        );
    }

    #[test]
    fn active_object_flame_local_light_source_contributes_to_surface_visibility() {
        let mut state = test_state(open_grid(), 8, 3);
        state.active_objects.push(ActiveObject {
            type_byte: 0xDE,
            tile: 0xDE,
            x: 8,
            y: 8,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert!(state.town_cell_visible_with_light_radius(8, 3, 8, 6, 6, 0));
    }

    #[test]
    fn world_local_light_mask_wraps_around_britannia_edges() {
        let mut grid = open_world_grid();
        grid[world_cell_index(250, 0)] = 0xDC;
        let mut state = britannia_state(grid, 0, 0);
        state.ambient_light = DAWN_DUSK_LIGHT[1];

        assert!(state.world_cell_visible_with_light_radius(0, 0, -6, 0, 10, 1));
        assert_eq!(
            state
                .render_text_view(10)
                .lines()
                .nth(11)
                .unwrap()
                .chars()
                .nth(4),
            Some('^')
        );
    }

    #[test]
    fn render_text_frame_clears_visibility_dirty_after_redraw() {
        let mut state = test_state(open_grid(), 1, 1);
        state.visibility_dirty = true;

        let view = state.render_text_frame(1);

        assert!(view.contains('@'));
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn render_text_frame_refreshes_player_slot_zero_before_compositing() {
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

        let view = state.render_text_frame(1);

        assert!(view.contains('@'));
        assert_eq!(state.turn, 0);
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
    fn stats_panel_renders_six_fixed_party_rows_and_bottom_block() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 4,
            hp: 87,
            max_hp: 120,
            level: 3,
        });
        state.party_names.push(*b"Julia\0\0\0\0");
        state.active_player = Some(1);
        state.food = 123;
        state.gold = 456;
        state.clock = GameClock::with_date(12, 5, 18, 12, 0).unwrap();

        let panel = state.render_stats_panel_view();
        let lines = panel.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), STATS_PANEL_PARTY_ROWS + 5);
        assert!(lines.iter().all(|line| line.chars().count() == STATS_PANEL_WIDTH));
        assert!(lines[1].contains("Avatar"));
        assert!(lines[2].contains("Julia"));
        assert!(lines[2].contains(">  87P"));
        assert!(lines[7].contains("Food"));
        assert!(lines[7].contains("123"));
        assert!(lines[8].contains("Gold"));
        assert!(lines[8].contains("456"));
        assert!(lines[9].contains("05-18"));
        assert!(lines[10].contains("Sky"));
    }

    #[test]
    fn stats_panel_frame_consumes_visible_active_player_cursor_only() {
        let mut state = test_state(open_grid(), 1, 1);
        state.active_player = Some(0);

        let visible_panel = state.render_stats_panel_frame();

        assert!(visible_panel.lines().nth(1).unwrap().contains(">"));
        assert_eq!(state.active_player, None);

        state.active_player = Some(0);
        state.party[0].status = b'S';
        let sleeping_panel = state.render_stats_panel_frame();

        assert!(!sleeping_panel.lines().nth(1).unwrap().contains(">"));
        assert_eq!(state.active_player, Some(0));
    }

    #[test]
    fn play_text_window_system_paints_message_stats_and_prompt_windows() {
        let mut state = test_state(open_grid(), 1, 1);
        state.message = "Hello Britannia".to_string();
        state.active_player = Some(0);

        let system = render_play_text_window_system(&state, state.active_player, Some("job"));

        assert_eq!(system.active_window_index(), MAIN_TEXT_WINDOW_INDEX);
        assert_eq!(system.cell(0, 0).unwrap().byte, b'H');
        assert_eq!(
            system
                .region_rows(
                    STATS_PANEL_TEXT_LEFT,
                    0,
                    STATS_PANEL_TEXT_RIGHT,
                    0,
                    b' '
                )
                .first()
                .unwrap()
                .trim_end(),
            "STATS"
        );
        assert_eq!(system.cell(STATS_PANEL_TEXT_LEFT, 1).unwrap().byte, b'A');
        assert_eq!(system.cell(0, TEXT_SCREEN_ROWS - 2).unwrap().byte, b'>');
        assert_eq!(system.cell(1, TEXT_SCREEN_ROWS - 2).unwrap().byte, b' ');
        assert_eq!(system.cell(2, TEXT_SCREEN_ROWS - 2).unwrap().byte, b'j');
    }

    #[test]
    fn play_text_window_system_paints_active_shop_modal_summary() {
        let mut state = test_state(open_grid(), 1, 1);
        state.message = "Mace costs 42 gold.".to_string();
        state.active_shop = Some(crate::shop_session::ActiveShopSession::ArmsStocked(
            crate::shop_runtime::ArmsShopState::BuyConfirm {
                item: 1,
                quoted_price: 42,
                quote_record_id: SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST + 1,
            },
            ArmsShop::IolosBows.stock_table(),
        ));

        let system = render_play_text_window_system(&state, state.active_player, None);
        let main = system
            .region_rows(0, 0, MESSAGE_TEXT_WINDOW_RIGHT, 5, b' ')
            .join("\n");

        assert_eq!(system.active_window_index(), TALK_SHOP_TEXT_WINDOW_INDEX);
        assert_eq!(
            system.window(TALK_SHOP_TEXT_WINDOW_INDEX).unwrap().top_left_x,
            0
        );
        assert_eq!(
            system
                .window(TALK_SHOP_TEXT_WINDOW_INDEX)
                .unwrap()
                .bottom_right_x,
            MESSAGE_TEXT_WINDOW_RIGHT
        );
        assert!(system.cell(0, 0).is_none(), "Talk entry newline leaves row 0 untouched");
        assert!(main.contains("Iolo"), "{main}");
        assert!(main.contains("Item 1 costs 42 gold"), "{main}");
        assert!(main.contains("Mace costs 42 gold."), "{main}");
        assert_eq!(
            system
                .region_rows(
                    STATS_PANEL_TEXT_LEFT,
                    0,
                    STATS_PANEL_TEXT_RIGHT,
                    0,
                    b' '
                )
                .first()
                .unwrap()
                .trim_end(),
            "STATS"
        );
    }

    #[test]
    fn inn_pickup_register_uses_window_one_then_restores_talk_shop_window_two() {
        let mut state = test_state(open_grid(), 1, 1);
        state.inn_registry.push(InnGuestRecord {
            scene_marker: 0x11,
            name: *b"IOLO\0\0\0\0\0",
            member: PartyMember {
                slot: 4,
                class_byte: b'B',
                status: b'G',
                climb_stat: 7,
                mana: 3,
                hp: 12,
                max_hp: 28,
                level: 3,
            },
            strength: 17,
            intelligence: 19,
            experience: 700,
            equipment: [1, 2, 3, 4, 5, 6],
            stay_counter: 1,
        });
        state.active_shop = Some(crate::shop_session::ActiveShopSession::Innkeeper(
            crate::shop_runtime::InnkeeperState::PickUpCompanion {
                inn: Inn::TheWayfarerInn,
                guest_indices: [0; INN_REGISTRY_CAP],
                guest_count: 1,
                base_lodging_charge: 22,
            },
        ));

        let system = render_play_text_window_system(&state, state.active_player, None);

        assert_eq!(system.active_window_index(), TALK_SHOP_TEXT_WINDOW_INDEX);
        assert_eq!(
            system.window(INN_PICKUP_REGISTER_TEXT_WINDOW_INDEX).unwrap(),
            TextWindowDescriptor {
                top_left_x: INN_PICKUP_REGISTER_LEFT,
                top_left_y: INN_PICKUP_REGISTER_TOP,
                bottom_right_x: INN_PICKUP_REGISTER_FRAME_RIGHT,
                bottom_right_y: INN_PICKUP_REGISTER_BOTTOM,
                cursor_x: 7,
                cursor_y: 3,
                color: text_window_default_color_byte(),
                flags: 0,
            }
        );
        let register = system
            .region_rows(
                INN_PICKUP_REGISTER_LEFT,
                INN_PICKUP_REGISTER_TOP,
                INN_PICKUP_REGISTER_FRAME_RIGHT,
                INN_PICKUP_REGISTER_BOTTOM,
                b' ',
            )
            .join("\n");
        assert!(register.contains("Pick up"), "{register}");
        assert!(register.contains("Companion"), "{register}");
        assert!(register.contains("1 IOLO"), "{register}");
    }

    #[test]
    fn prompt_text_window_cursor_glyph_paints_in_place() {
        let mut system = TextWindowSystem::new();
        configure_play_text_windows(&mut system);

        paint_prompt_text_window_with_cursor(&mut system, "job", Some(4));

        assert_eq!(system.active_window_index(), PROMPT_TEXT_WINDOW_INDEX);
        assert_eq!(system.cell(0, TEXT_SCREEN_ROWS - 2).unwrap().byte, b'>');
        assert_eq!(system.cell(2, TEXT_SCREEN_ROWS - 2).unwrap().byte, b'j');
        assert_eq!(system.cell(5, TEXT_SCREEN_ROWS - 2).unwrap().byte, 4);
        assert_eq!(system.active_cursor(), (5, 0));

        paint_prompt_text_window_with_cursor(&mut system, "job", None);

        assert!(system.cell(5, TEXT_SCREEN_ROWS - 2).is_none());
        assert_eq!(system.active_cursor(), (5, 0));
    }

    #[test]
    fn play_text_window_frame_consumes_active_cursor_like_stats_panel() {
        let mut state = test_state(open_grid(), 1, 1);
        state.active_player = Some(0);

        let visible_frame = state.render_text_window_frame(None);

        assert!(visible_frame.lines().nth(1).unwrap().contains(">"));
        assert_eq!(state.active_player, None);

        state.active_player = Some(0);
        state.party[0].status = b'S';
        let sleeping_frame = state.render_text_window_frame(None);

        assert!(!sleeping_frame.lines().nth(1).unwrap().contains(">"));
        assert_eq!(state.active_player, Some(0));
    }

    #[test]
    fn stats_panel_derives_combat_row_overlay_from_live_combat_state() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.pending_combat_actor_slot = Some(0);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);

        let overlay = stats_panel_combat_row_overlay(&state, 0);
        assert_eq!(
            overlay,
            StatsPanelCombatRowOverlay {
                highlighted: true,
                status_override: None,
            }
        );
        assert_eq!(
            state
                .render_stats_panel_view()
                .lines()
                .nth(1)
                .unwrap()
                .chars()
                .nth(10),
            Some(' ')
        );
        let system = render_play_text_window_system(&state, None, None);
        for offset in 0..STATS_PANEL_WIDTH {
            assert!(
                system
                    .cell(STATS_PANEL_TEXT_LEFT + offset as u8, 1)
                    .unwrap()
                    .inverse,
                "party row cell {offset} should be inverse-highlighted"
            );
        }
        assert!(
            !system
                .cell(STATS_PANEL_TEXT_LEFT, 2)
                .map(|cell| cell.inverse)
                .unwrap_or(false)
        );

        state.active_cast = Some(CastSession::for_combat_actor(0, true));
        let casting_overlay = stats_panel_combat_row_overlay(&state, 0);

        assert_eq!(casting_overlay.status_override, Some(b'C'));
        assert!(state
            .render_stats_panel_view()
            .lines()
            .nth(1)
            .unwrap()
            .ends_with('C'));
    }

    #[test]
    fn stats_panel_combat_overlay_brackets_active_player_cursor_with_inverse_video() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.active_player = Some(0);
        state.pending_combat_actor_slot = Some(0);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);

        let panel = state.render_stats_panel_view();
        let row = panel.lines().nth(1).unwrap();

        assert!(row.contains('>'));
        let system = render_play_text_window_system(&state, state.active_player, None);
        assert_eq!(system.cell(STATS_PANEL_TEXT_LEFT + 10, 1).unwrap().byte, b'>');
        assert!(system.cell(STATS_PANEL_TEXT_LEFT + 10, 1).unwrap().inverse);
        assert!(system.cell(STATS_PANEL_TEXT_LEFT + 15, 1).unwrap().inverse);
    }

    #[test]
    fn stats_panel_combat_overlay_matches_descriptor_owner_field_not_slot_number() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 4,
            hp: 87,
            max_hp: 120,
            level: 3,
        });
        state.party_names.push(*b"Julia\0\0\0\0");
        state.combat_active = true;
        state.pending_combat_actor_slot = Some(0);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            1,
            0,
            0,
            5,
            5,
        ]);

        assert!(!stats_panel_combat_row_overlay(&state, 0).highlighted);
        assert!(stats_panel_combat_row_overlay(&state, 1).highlighted);

        let system = render_play_text_window_system(&state, None, None);
        assert!(!system.cell(STATS_PANEL_TEXT_LEFT, 1).unwrap().inverse);
        assert!(system.cell(STATS_PANEL_TEXT_LEFT, 2).unwrap().inverse);
    }

    #[test]
    fn stats_panel_ship_marker_middle_counter_renders_hull_instead_of_gold() {
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 999;
        state.player.transport = TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
            tile: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
            sails_hoisted: true,
            hull: 42,
            skiffs: 2,
        };

        let panel = state.render_stats_panel_view();
        let middle = panel.lines().nth(8).unwrap();

        assert!(middle.contains("Ship hull"));
        assert!(middle.contains("42"));
        assert!(!middle.contains("999"));
    }

    #[test]
    fn top_down_viewport_rasterizes_town_tiles_player_and_objects() {
        let mut grid = open_grid();
        grid[1 * 32 + 2] = 17;
        let mut state = test_state(grid, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 18,
            tile: 18,
            x: 0,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();

        assert_eq!(viewport.depth, TileGraphicsDepth::Ega16);
        assert_eq!((viewport.cells_wide, viewport.cells_high), (3, 3));
        assert_eq!((viewport.width, viewport.height), (48, 48));
        // PLAYER_TILE is a sentinel; the renderer resolves it to the
        // actual avatar sprite at PLAYER_SPRITE_TILE.
        assert_eq!(
            viewport.pixel(16, 16),
            Some((PLAYER_SPRITE_TILE as u8) % atlas.depth.pixel_limit())
        );
        assert_eq!(viewport.pixel(0, 16), Some(18 % atlas.depth.pixel_limit()));
        assert_eq!(viewport.pixel(32, 16), Some(17 % atlas.depth.pixel_limit()));
    }

    #[test]
    fn top_down_viewport_suppresses_active_object_on_blocking_compositor_terrain() {
        let mut grid = open_grid();
        grid[32] = 0xEC;
        let mut state = test_state(grid, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 0x40,
            tile: 0x40,
            x: 0,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();

        assert_eq!(viewport.pixel(0, 16), Some(0xEC % atlas.depth.pixel_limit()));
    }

    #[test]
    fn top_down_viewport_applies_active_object_direct_marker_remap() {
        let mut grid = open_grid();
        grid[32] = 0x57;
        let mut state = test_state(grid, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 0x40,
            tile: 0x40,
            x: 0,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();

        assert_eq!(viewport.pixel(0, 16), Some(0x38 % atlas.depth.pixel_limit()));
    }

    #[test]
    fn top_down_viewport_applies_previous_row_compositor_marker() {
        let mut grid = open_grid();
        grid[2 * 32 + 3] = 0x9D;
        grid[3 * 32 + 3] = 0x10;
        let mut state = test_state(grid, 2, 2);
        state.active_objects.push(ActiveObject {
            type_byte: 0x40,
            tile: 0x40,
            x: 3,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_viewport(2, &atlas).unwrap().unwrap();

        assert_eq!(viewport.pixel(3 * 16, 2 * 16), Some(0x9E % atlas.depth.pixel_limit()));
        assert_eq!(viewport.pixel(3 * 16, 3 * 16), Some(0x40 % atlas.depth.pixel_limit()));
    }

    #[test]
    fn top_down_viewport_rasterizes_world_wrapping_moongates_and_visibility() {
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 17;
        let mut state = britannia_state(grid, 255, 0);
        state.ambient_light = FULL_DAYLIGHT;
        // Moongate is a single-frame sprite at 0xDC; keep frame at 0.
        state.animation.moongate_frame = 0;
        state.moongates.push(MoongateEntry {
            x: 254,
            y: 0,
            destination_plane: WorldPlane::Underworld,
            destination_x: 0,
            destination_y: 0,
            active_hours: None,
            expected_tile: None,
        });
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();

        // PLAYER_TILE is a sentinel; the renderer resolves it to the
        // actual avatar sprite at PLAYER_SPRITE_TILE.
        assert_eq!(
            viewport.pixel(16, 16),
            Some((PLAYER_SPRITE_TILE as u8) % atlas.depth.pixel_limit())
        );
        assert_eq!(viewport.pixel(32, 16), Some(17 % atlas.depth.pixel_limit()));
        assert_eq!(
            viewport.pixel(0, 16),
            Some(MOONGATE_TILE_BASE % atlas.depth.pixel_limit())
        );

        let mut dark = state.clone();
        dark.ambient_light = FULL_DARKNESS;
        let dark_viewport = dark.render_top_down_viewport(1, &atlas).unwrap().unwrap();
        assert_eq!(
            dark_viewport.pixel(16, 16),
            Some((PLAYER_SPRITE_TILE as u8) % atlas.depth.pixel_limit())
        );
        assert_eq!(dark_viewport.pixel(32, 16), Some(0));
    }

    #[test]
    fn top_down_viewport_samples_world_live_chunk_buffer() {
        let mut grid = open_world_grid();
        grid[world_cell_index(9, 8)] = 0x16;
        let mut state = britannia_state(grid, 8, 8);
        state.ambient_light = FULL_DAYLIGHT;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();

        assert_eq!(
            viewport.pixel(2 * 16, 16),
            Some(LIVE_CHUNK_SUBSTITUTION_TARGET_DF % atlas.depth.pixel_limit())
        );
        assert_eq!(state.grid[world_cell_index(9, 8)], 0x16);
    }

    #[test]
    fn world_step_refreshes_live_chunk_buffer_scroll_base() {
        let mut state = britannia_state(open_world_grid(), 23, 8);
        assert_eq!(state.world_live_chunks.as_ref().unwrap().scroll_base, (0, 0));

        let outcome = state
            .step_world(Direction::East, 24, 8, WorldPlane::Britannia, None)
            .unwrap();

        assert_eq!(outcome, MoveOutcome::Moved);
        assert_eq!(state.player.x, 24);
        assert_eq!(state.world_live_chunks.as_ref().unwrap().scroll_base, (16, 0));
    }

    #[test]
    fn top_down_frame_repairs_player_object_and_clears_dirty() {
        let mut state = test_state(open_grid(), 4, 5);
        state.visibility_dirty = true;
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
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_frame(1, &atlas).unwrap().unwrap();

        // PLAYER_TILE is a sentinel; the renderer resolves it to the
        // actual avatar sprite at PLAYER_SPRITE_TILE.
        assert_eq!(
            viewport.pixel(16, 16),
            Some((PLAYER_SPRITE_TILE as u8) % atlas.depth.pixel_limit())
        );
        assert!(!state.visibility_dirty);
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
    fn white_potion_sweep_renders_for_counted_frames_then_clears() {
        let mut state = test_state(open_grid(), 1, 1);
        state.visibility_dirty = false;
        state.white_potion_sweep = Some(WhitePotionSweep {
            frames_remaining: 2,
            radius: 0,
            center_x: 1,
            center_y: 1,
        });
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let first = state.render_top_down_frame(1, &atlas).unwrap().unwrap();

        assert_eq!(first.pixel(24, 24), Some(15));
        assert_eq!(
            state.white_potion_sweep.map(|sweep| sweep.frames_remaining),
            Some(1)
        );
        assert!(state.visibility_dirty);

        let second = state.render_top_down_frame(1, &atlas).unwrap().unwrap();

        assert_eq!(second.pixel(24, 24), Some(15));
        assert_eq!(state.white_potion_sweep, None);
        assert!(state.visibility_dirty);

        let third = state.render_top_down_frame(1, &atlas).unwrap().unwrap();

        assert_ne!(third.pixel(24, 24), Some(15));
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn combat_potion_presentation_renders_sleep_and_one_frame_poof_marks() {
        let mut combat = test_state(open_grid(), 1, 1);
        combat.combat_active = true;
        combat.active_objects.push(ActiveObject {
            type_byte: 0x81,
            tile: 0x81,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        });
        combat.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            0,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            1,
            5,
            5,
        ]);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        combat.combat_potion_presentation = Some(CombatPotionPresentation {
            kind: CombatPotionPresentationKind::Sleep,
            actor_slot: 0,
            active_object_slot: 1,
            frames_remaining: COMBAT_POTION_SLEEP_PRESENTATION_FRAMES,
        });
        let sleep = combat.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert_eq!(sleep.pixel(84, 84), Some(11));
        assert_eq!(
            combat.combat_potion_presentation
                .map(|presentation| presentation.kind),
            Some(CombatPotionPresentationKind::Sleep)
        );
        assert!(!combat.visibility_dirty);

        combat.visibility_dirty = false;
        combat.combat_potion_presentation = Some(CombatPotionPresentation {
            kind: CombatPotionPresentationKind::Poof,
            actor_slot: 0,
            active_object_slot: 1,
            frames_remaining: COMBAT_POTION_POOF_PRESENTATION_FRAMES,
        });
        let poof = combat.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert_eq!(poof.pixel(88, 88), Some(13));
        assert_eq!(combat.combat_potion_presentation, None);
        assert!(combat.visibility_dirty);

        let cleared = combat.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert_ne!(cleared.pixel(88, 88), Some(13));
        assert!(!combat.visibility_dirty);
    }

    #[test]
    fn combat_viewport_renders_post_round_cursor_and_secondary_marker_hooks() {
        let mut combat = test_state(open_grid(), 1, 1);
        combat.combat_active = true;
        combat.combat_terrain = [[5; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        combat.active_player = Some(0);
        combat.combat_cursor_blink = true;
        combat.combat_secondary_marker = Some((3, 4));
        combat.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            0,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            1,
            5,
            6,
        ]);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let marked = combat.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert_eq!(marked.pixel(56, 72), Some(11));
        assert_eq!(marked.pixel(80, 96), Some(14));

        combat.combat_cursor_blink = false;
        combat.combat_secondary_marker = Some((99, 99));
        let cleared = combat.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert_ne!(cleared.pixel(56, 72), Some(11));
        assert_ne!(cleared.pixel(80, 96), Some(14));
    }

    #[test]
    fn tile_viewport_to_rgba_matches_dimensions_and_palette() {
        let mut state = test_state(open_grid(), 1, 1);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let viewport = state.render_top_down_frame(1, &atlas).unwrap().unwrap();

        let rgba = viewport.to_rgba();

        assert_eq!(rgba.len(), viewport.width * viewport.height * 4);
        for chunk in rgba.chunks_exact(4) {
            assert_eq!(chunk[3], 0xff, "alpha should be opaque");
        }
        let player_index = ((PLAYER_SPRITE_TILE as u8) % atlas.depth.pixel_limit()) as usize;
        let expected_player_rgb = EGA_PALETTE_RGB[player_index];
        let center_pixel_offset =
            (viewport.height / 2) * viewport.width * 4 + (viewport.width / 2) * 4;
        let center = &rgba[center_pixel_offset..center_pixel_offset + 4];
        assert_eq!(
            [center[0], center[1], center[2]],
            expected_player_rgb,
            "centre cell should display the player tile in EGA RGB"
        );
        assert!(
            rgba.iter().any(|&byte| byte != 0),
            "framebuffer should not be entirely zero",
        );
    }

    #[test]
    fn tile_viewport_to_rgba_uses_cga_palette_for_cga_atlas() {
        let mut state = test_state(open_grid(), 1, 1);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Cga4);
        let viewport = state.render_top_down_frame(1, &atlas).unwrap().unwrap();

        let rgba = viewport.to_rgba();

        assert_eq!(rgba.len(), viewport.width * viewport.height * 4);
        for chunk in rgba.chunks_exact(4) {
            let rgb = [chunk[0], chunk[1], chunk[2]];
            assert!(
                CGA_PALETTE_RGB.contains(&rgb),
                "RGB {rgb:?} should match the CGA palette",
            );
            assert_eq!(chunk[3], 0xff);
        }
    }

    #[test]
    fn text_panel_renderer_produces_bounded_nonblank_rgba() {
        let rgba = render_text_panel_rgba(
            "DUNGEON:0 LEVEL 0\nA VERY LONG DUNGEON STATUS LINE",
            48,
            24,
        )
        .unwrap();

        assert_eq!(rgba.len(), 48 * 24 * 4);
        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| pixel == TEXT_PANEL_HEADER_RGBA)
        );
        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| pixel == TEXT_PANEL_BODY_RGBA)
        );
        assert!(wrap_text_panel_lines("DUNGEON:0 LEVEL 0", 12, 4).contains(&"LEVEL 0".to_string()));
    }

    #[test]
    fn text_panel_renderer_rejects_overflow_dimensions() {
        assert!(render_text_panel_rgba("x", usize::MAX, 2).is_err());
        assert!(render_text_panel_rgba("x", usize::MAX / 2 + 1, 3).is_err());
    }

    #[test]
    fn fixed_cell_font_renderer_paints_text_window_glyphs_with_palette() {
        let mut font_bytes = vec![0; CH_FONT_LEN];
        font_bytes[usize::from(b'A') * CH_CELL_SIDE] = 0b1000_0000;
        let font = parse_ch_font(&font_bytes, IBM_CH_FILE).unwrap();
        let mut system = TextWindowSystem::new();
        system.emit_byte(b'A');

        let rgba = render_text_window_rgba(&system, &font).unwrap();

        assert_eq!(
            rgba.len(),
            TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4
        );
        assert_eq!(&rgba[0..4], &[0xff, 0xff, 0xff, 0xff]);
        assert_eq!(&rgba[4..8], &[0x00, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn fixed_cell_font_renderer_applies_inverse_and_underline_style() {
        let mut font_bytes = vec![0; CH_FONT_LEN];
        font_bytes[usize::from(b'X') * CH_CELL_SIDE] = 0b1000_0000;
        let font = parse_ch_font(&font_bytes, IBM_CH_FILE).unwrap();
        let mut system = TextWindowSystem::new();
        system.set_active_flags(TEXT_WINDOW_FLAG_INVERSE | TEXT_WINDOW_FLAG_UNDERLINE);
        system.emit_byte(b'X');

        let rgba = render_text_window_rgba(&system, &font).unwrap();

        assert_eq!(&rgba[0..4], &[0x00, 0x00, 0x00, 0xff]);
        assert_eq!(&rgba[4..8], &[0xff, 0xff, 0xff, 0xff]);
        let underline_offset =
            ((CH_CELL_SIDE - 1) * TEXT_WINDOW_RENDER_WIDTH + (CH_CELL_SIDE - 1)) * 4;
        assert_eq!(
            &rgba[underline_offset..underline_offset + 4],
            &[0x00, 0x00, 0x00, 0xff]
        );
    }

    #[test]
    fn fixed_cell_font_rejects_wrong_length_assets() {
        assert!(parse_ch_font(&vec![0; CH_FONT_LEN - 1], IBM_CH_FILE).is_err());
        assert!(parse_ch_font(&vec![0; CH_FONT_LEN + 1], IBM_CH_FILE).is_err());
    }




    #[test]
    fn viewport_renders_lit_dungeon_area() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.torch_counter = 9;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();

        assert_eq!(viewport.width, 3 * TILE_ATLAS_SIDE);
        assert!(viewport.pixels.iter().any(|&pixel| pixel != 0));
    }

    #[test]
    fn town_render_visibility_carve_uses_terrain_blockers() {
        let mut grid = open_grid();
        grid[2] = 24;
        grid[32 + 2] = 24;
        grid[64 + 2] = 24;
        grid[32 + 3] = 16;
        let state = test_state(grid, 1, 1);

        let view = state.render_text_view(2);
        let row: Vec<_> = view.lines().nth(3).unwrap().chars().collect();

        assert_eq!(row[2], '@');
        assert_eq!(row[3], '#');
        assert_eq!(row[4], ' ');
    }

    #[test]
    fn town_render_open_door_does_not_block_line_of_sight() {
        let mut grid = open_grid();
        grid[32 + 2] = 16;
        grid[32 + 3] = 16;
        let state = test_state(grid, 1, 1);

        let view = state.render_text_view(2);
        let row: Vec<_> = view.lines().nth(3).unwrap().chars().collect();

        assert_eq!(row[2], '@');
        assert_eq!(row[3], '.');
        assert_eq!(row[4], '.');
    }

    #[test]
    fn town_render_active_object_does_not_block_visibility_carve() {
        let mut state = test_state(open_grid(), 1, 1);
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

        let view = state.render_text_view(2);
        let row: Vec<_> = view.lines().nth(3).unwrap().chars().collect();

        assert_eq!(row[2], '@');
        assert_eq!(row[3], 'n');
        assert_eq!(row[4], '.');
    }

