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
        assert!(state.message.contains("Jimmy checked dungeon chest"));
        assert!(!state.message.contains("Dungeon movement"));
    }

    #[test]
    fn jimmy_requires_keys_before_tile_probe() {
        let mut grid = open_grid();
        grid[32 + 2] = 96;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.keys = 0;

        assert_eq!(state.jimmy_facing(), MoveOutcome::Blocked);

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
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 10,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 10,
                max_hp: 20,
                level: 8,
            },
        ];
        let expected_damage = state.dungeon_fountain_damage_roll(1, 0x53) as u16;

        assert_eq!(
            state.look_dungeon_with_drink(Some(true), Some(1)),
            MoveOutcome::Observed
        );

        assert_eq!(state.party[0].hp, 10);
        assert_eq!(state.party[1].hp, 10 - expected_damage);
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
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 5,
                max_hp: 15,
                level: 8,
            },
            PartyMember {
                slot: 1,
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
    fn town_view_decrements_gem_and_reports_full_fill_map_without_turn() {
        let mut state = test_state(open_grid(), 5, 5);
        state.gems = 1;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
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
        assert!(state.message.contains(".....@n...."));
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
        assert!(state.message.contains(",,,,v@,,,,,"));
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
    fn top_down_viewport_skips_dungeon_area() {
        let state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        assert!(state.render_top_down_viewport(1, &atlas).unwrap().is_none());
    }

    #[test]
    fn town_render_uses_line_of_sight_blockers() {
        let mut grid = open_grid();
        grid[32 + 2] = 24;
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
    fn town_render_active_object_blocks_line_of_sight_behind_it() {
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
        assert_eq!(row[4], ' ');
    }

