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
    fn town_look_direction_samples_selected_direction_without_turn_or_turning() {
        let table = parse_look2_dat(&look2_bytes(&[(16, "east road"), (17, "south road")]))
            .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 16;
        grid[2 * 32 + 1] = 17;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::South;

        assert_eq!(
            state.look_direction_with_table(Direction::East, Some(&table)),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("east road at (2, 1)"));
        assert!(!state.message.contains("south road"));
        assert_eq!(state.player.facing, Direction::South);
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
    fn town_look_at_surface_fountain_prompts_for_drinker_without_spending_turn() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xd8;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        assert_eq!(
            state.active_direction_prompt.as_ref().map(|session| session.kind),
            Some(DirectionPromptKind::SurfaceFountainDrink {
                direction: Direction::East
            })
        );
        assert!(state.message.contains("choose fountain drinker"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_look_at_surface_wishing_well_prompts_for_coin_without_spending_turn() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa1;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        assert_eq!(
            state
                .active_wishing_well
                .as_ref()
                .map(|session| (session.direction, session.coin_accepted)),
            Some((Direction::East, false))
        );
        assert_eq!(state.message, "Wishing well: toss a coin? (Y/N)");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_surface_wishing_well_decline_or_empty_purse_has_no_effect() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa1;
        let mut decline = test_state(grid.clone(), 1, 1);
        decline.player.facing = Direction::East;
        decline.gold = 7;
        assert_eq!(decline.look_facing(), MoveOutcome::Observed);

        assert_eq!(
            decline.step_active_wishing_well('N', ""),
            Some(MoveOutcome::Observed)
        );
        assert!(decline.active_wishing_well.is_none());
        assert_eq!(decline.gold, 7);
        assert_eq!(decline.message, "Wishing well: no effect.");
        assert_eq!(decline.turn, 0);

        let mut empty = test_state(grid, 1, 1);
        empty.player.facing = Direction::East;
        empty.gold = 0;
        assert_eq!(empty.look_facing(), MoveOutcome::Observed);

        assert_eq!(
            empty.step_active_wishing_well('Y', ""),
            Some(MoveOutcome::Observed)
        );
        assert!(empty.active_wishing_well.is_none());
        assert_eq!(empty.gold, 0);
        assert_eq!(empty.message, "Wishing well: no effect.");
        assert_eq!(empty.turn, 0);
    }

    #[test]
    fn town_surface_wishing_well_coin_then_wish_consumes_coin_without_turn() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa1;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.gold = 2;
        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        assert_eq!(state.step_active_wishing_well('Y', ""), None);
        assert_eq!(state.gold, 1);
        assert_eq!(
            state
                .active_wishing_well
                .as_ref()
                .map(|session| (session.direction, session.coin_accepted)),
            Some((Direction::East, true))
        );
        assert_eq!(state.message, "Wishing well: make a wish.");

        assert_eq!(
            state.step_active_wishing_well('H', "orse"),
            Some(MoveOutcome::Observed)
        );
        assert!(state.active_wishing_well.is_none());
        assert_eq!(state.gold, 1);
        assert_eq!(state.message, "Wishing well: no effect.");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_surface_fountain_drink_refreshes_without_mutating_member() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xd9;
        let mut state = test_state(grid, 1, 1);
        state.party[0].status = CharacterStatus::PoisonedOrRevived.save_byte();
        state.party[0].hp = 12;
        state.party[0].max_hp = 90;
        let before = state.party[0];

        assert_eq!(
            state.look_surface_fountain_with_drinker(Direction::East, 0),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("feels refreshed"));
        assert_eq!(state.party[0], before);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_surface_fountain_drink_refuses_incapacitated_member_without_mutating() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xda;
        let mut state = test_state(grid, 1, 1);
        state.party[0].status = CharacterStatus::Sleeping.save_byte();
        state.party[0].hp = 12;
        let before = state.party[0];

        assert_eq!(
            state.look_surface_fountain_with_drinker(Direction::East, 0),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("incapacitated"));
        assert_eq!(state.party[0], before);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_surface_fountain_prompt_digit_routes_refresh_result() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xdb;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        let outcome = state
            .step_active_direction_prompt('1', "", Path::new(""))
            .unwrap();

        assert_eq!(outcome, Some(MoveOutcome::Observed));
        assert!(state.active_direction_prompt.is_none());
        assert!(state.message.contains("feels refreshed"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    fn signs_dat_bytes_for_test(records: &[(u8, u8, u8, u8, &[u8])]) -> Vec<u8> {
        let mut bytes = vec![0; SIGNS_DAT_SCENE_DIRECTORY_BYTES];
        if let Some((scene, ..)) = records.first() {
            let offset = SIGNS_DAT_SCENE_DIRECTORY_BYTES as u16;
            bytes[*scene as usize * 2..*scene as usize * 2 + 2]
                .copy_from_slice(&offset.to_le_bytes());
        }
        for (scene, z, y, x, body) in records {
            bytes.extend_from_slice(&[*scene, *z, *y, *x]);
            bytes.extend_from_slice(body);
            bytes.push(0);
        }
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn town_look_renders_matching_signs_dat_record_without_spending_turn() {
        let dir = debug_game_dir();
        fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[(0x5a, "a sign")])).unwrap();
        fs::write(
            dir.join(SIGNS_DAT_FILE),
            signs_dat_bytes_for_test(&[(17, 0, 1, 2, b"North Road")]),
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 0x5a;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.look_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(state.message, "Sign:\nNorth Road");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_look_special_map_tile_routes_to_britannia_overview() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(LOOK2_DAT_FILE),
            look2_bytes(&[(BRITANNIA_CHUNK_MAP_LOOK_TRIGGER_TILE as usize, "map trigger")]),
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = BRITANNIA_CHUNK_MAP_LOOK_TRIGGER_TILE;
        let mut state = britannia_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.look_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Observed
        );

        assert!(state.message.starts_with("Britannia overview from Look"));
        assert_eq!(state.message.lines().skip(1).count(), BRITANNIA_CHUNK_MAP_ROWS as usize);
        assert!(!state.message.contains("map trigger"));
        assert_eq!(
            state.active_view_overlay.as_ref().map(|overlay| overlay.kind),
            Some(ViewOverlayKind::BritanniaChunkMap)
        );
        let viewport = state
            .render_active_view_overlay(TileGraphicsDepth::Ega16)
            .expect("look-triggered Britannia overview should install a renderable overlay");
        assert_eq!(viewport.cells_wide, BRITANNIA_CHUNK_MAP_COLUMNS as usize);
        assert_eq!(viewport.cells_high, BRITANNIA_CHUNK_MAP_ROWS as usize);
        assert!(viewport.pixels.iter().any(|pixel| *pixel != 0));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_view_overlay.is_none());
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "View closed.");
        let _ = fs::remove_dir_all(dir);
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
    fn look_shrine_altar_tiles_append_virtue_context() {
        let table =
            parse_look2_dat(&look2_bytes(&[(SHRINE_ALTAR_TILE_FIRST as usize, "an altar")]))
                .unwrap();
        let state = test_state(open_grid(), 1, 1);

        assert_eq!(
            state.look_description(SHRINE_ALTAR_TILE_FIRST, Some(&table)),
            "an altar (Shrine of Honesty)"
        );
        assert_eq!(
            state.look_description(SHRINE_ALTAR_TILE_LAST, None),
            "special (Shrine of Humility)"
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
        fs::write(
            dir.join(LOOK2_DAT_FILE),
            look2_bytes(&[
                (170, "terrain frigate"),
                (LOOK2_DAT_TERRAIN_ENTRIES + 170, "object frigate"),
            ]),
        )
        .unwrap();
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

        assert!(state.message.contains("object frigate"));
        assert!(!state.message.contains("terrain frigate"));
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
    fn world_look_shrine_table_appends_clean_virtue_name() {
        let dir = debug_game_dir();
        fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[(0x80, "a shrine")])).unwrap();
        fs::write(
            dir.join(SHRINE_TABLE_FILE),
            "BRITANNIA 2 1 COMPASSION 0x80\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = 0x80;
        let mut state = britannia_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.look_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("a shrine (Shrine of Compassion)"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_look_shrine_altar_avoids_duplicate_virtue_context() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(LOOK2_DAT_FILE),
            look2_bytes(&[(SHRINE_ALTAR_TILE_FIRST as usize, "an altar")]),
        )
        .unwrap();
        fs::write(
            dir.join(SHRINE_TABLE_FILE),
            "BRITANNIA 2 1 HONESTY 0x88\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = SHRINE_ALTAR_TILE_FIRST;
        let mut state = britannia_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.look_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Observed
        );

        assert!(state.message.contains("an altar (Shrine of Honesty)"));
        assert_eq!(state.message.matches("Shrine of Honesty").count(), 1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
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
        bytes.extend_from_slice(&1u16.to_le_bytes()); // leading sentinel npc id
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
    fn parse_tlk_rejects_malformed_headers() {
        assert!(parse_tlk_bytes(&[0, 0, 0]).is_err());

        let mut empty_sentinel_only = Vec::new();
        empty_sentinel_only.extend_from_slice(&1u16.to_le_bytes());
        empty_sentinel_only.extend_from_slice(&1u16.to_le_bytes());
        assert!(parse_tlk_bytes(&empty_sentinel_only).unwrap().is_empty());
        assert!(parse_tlk_bytes(&[1, 0, 0, 0]).unwrap().is_empty());

        let mut zero_count = Vec::new();
        zero_count.extend_from_slice(&0u16.to_le_bytes());
        zero_count.extend_from_slice(&1u16.to_le_bytes());
        assert!(parse_tlk_bytes(&zero_count).is_err());

        let mut bad_sentinel = Vec::new();
        bad_sentinel.extend_from_slice(&2u16.to_le_bytes());
        bad_sentinel.extend_from_slice(&0u16.to_le_bytes());
        bad_sentinel.extend_from_slice(&8u16.to_le_bytes());
        bad_sentinel.extend_from_slice(&2u16.to_le_bytes());
        bad_sentinel.push(0);
        assert!(parse_tlk_bytes(&bad_sentinel).is_err());

        let mut bad_offset = Vec::new();
        bad_offset.extend_from_slice(&2u16.to_le_bytes());
        bad_offset.extend_from_slice(&1u16.to_le_bytes());
        bad_offset.extend_from_slice(&4u16.to_le_bytes());
        bad_offset.extend_from_slice(&2u16.to_le_bytes());
        bad_offset.push(0);
        assert!(parse_tlk_bytes(&bad_offset).is_err());

        let mut unsorted_ids = Vec::new();
        unsorted_ids.extend_from_slice(&3u16.to_le_bytes());
        unsorted_ids.extend_from_slice(&1u16.to_le_bytes());
        unsorted_ids.extend_from_slice(&12u16.to_le_bytes());
        unsorted_ids.extend_from_slice(&3u16.to_le_bytes());
        unsorted_ids.extend_from_slice(&13u16.to_le_bytes());
        unsorted_ids.extend_from_slice(&2u16.to_le_bytes());
        unsorted_ids.extend_from_slice(&[0, 0]);
        assert!(parse_tlk_blob_fields_raw(&unsorted_ids).is_err());
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
    fn tlk_action_dispatch_grants_use_published_caps_and_slots() {
        let mut state = test_state(open_grid(), 1, 1);
        state.food = PARTY_FOOD_CAP;
        state.gold = PARTY_GOLD_CAP;
        state.keys = PARTY_BYTE_STOCK_CAP;
        state.gems = PARTY_BYTE_STOCK_CAP;
        state.torches = PARTY_BYTE_STOCK_CAP;
        state.climbing_gear = 0;
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = PARTY_BYTE_STOCK_CAP;
        state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] = PARTY_BYTE_STOCK_CAP;
        state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 0;
        state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 0;
        state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] = 0;

        state.apply_tlk_action_grants(&[
            TlkActionDispatchVerb::RaiseFood,
            TlkActionDispatchVerb::RaiseGold,
            TlkActionDispatchVerb::RaiseKeys,
            TlkActionDispatchVerb::RaiseGems,
            TlkActionDispatchVerb::RaiseTorches,
            TlkActionDispatchVerb::SetGrappleGate,
            TlkActionDispatchVerb::RaiseCarpets,
            TlkActionDispatchVerb::SetSextantCarried,
            TlkActionDispatchVerb::SetSpyglassCarried,
            TlkActionDispatchVerb::SetBlackBadgeCarried,
            TlkActionDispatchVerb::RaiseSkullKeys,
        ]);

        assert_eq!(state.food, PARTY_FOOD_CAP);
        assert_eq!(state.gold, PARTY_GOLD_CAP);
        assert_eq!(state.keys, PARTY_BYTE_STOCK_CAP);
        assert_eq!(state.gems, PARTY_BYTE_STOCK_CAP);
        assert_eq!(state.torches, PARTY_BYTE_STOCK_CAP);
        assert_eq!(state.climbing_gear, 1);
        assert_eq!(
            state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX],
            PARTY_BYTE_STOCK_CAP
        );
        assert_eq!(
            state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX],
            PARTY_BYTE_STOCK_CAP
        );
        assert_eq!(state.special_items[SPECIAL_ITEM_SEXTANT_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX], 1);
    }

    #[test]
    fn object_pickup_parser_accepts_extended_inventory_grants() {
        let entries = parse_object_pickup_entries(
            "CASTLE:0 0 1 2 POTION:3 2 0x42\n\
             CASTLE:0 0 2 2 SCROLL_4 1\n\
             CASTLE:0 0 3 2 EQUIP-27 1\n\
             CASTLE:0 0 4 2 SHARD:2 1\n\
             CASTLE:0 0 5 2 SANDALWOOD_BOX 1\n",
        )
        .unwrap();

        assert_eq!(entries[0].kind, ObjectPickupKind::Potion(3));
        assert_eq!(entries[0].amount, 2);
        assert_eq!(entries[0].expected_tile, Some(0x42));
        assert_eq!(entries[1].kind, ObjectPickupKind::Scroll(4));
        assert_eq!(entries[2].kind, ObjectPickupKind::Equipment(EQUIPMENT_ID_ARROWS));
        assert_eq!(entries[3].kind, ObjectPickupKind::ShadowlordShard(2));
        assert_eq!(entries[4].kind, ObjectPickupKind::SandalwoodBox);
    }

    #[test]
    fn object_pickup_inventory_grants_cover_caps_equipment_and_story_items() {
        let mut state = test_state(open_grid(), 1, 1);
        state.food = PARTY_FOOD_CAP - 1;
        state.gold = PARTY_GOLD_CAP - 1;
        state.keys = PARTY_BYTE_STOCK_CAP - 1;
        state.gems = PARTY_BYTE_STOCK_CAP - 1;
        state.torches = PARTY_BYTE_STOCK_CAP - 1;
        state.potion_stock[3] = PARTY_BYTE_STOCK_CAP - 1;
        state.scroll_stock[4] = PARTY_BYTE_STOCK_CAP - 1;
        state.equipment_stock[EQUIPMENT_ID_ARROWS] = PARTY_BYTE_STOCK_CAP - 3;
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = PARTY_BYTE_STOCK_CAP - 1;

        state.apply_object_pickup(ObjectPickupKind::Food, 5);
        state.apply_object_pickup(ObjectPickupKind::Gold, 5);
        state.apply_object_pickup(ObjectPickupKind::Keys, 5);
        state.apply_object_pickup(ObjectPickupKind::Gems, 5);
        state.apply_object_pickup(ObjectPickupKind::Torches, 5);
        state.apply_object_pickup(ObjectPickupKind::Potion(3), 5);
        state.apply_object_pickup(ObjectPickupKind::Scroll(4), 5);
        state.apply_object_pickup(ObjectPickupKind::Equipment(EQUIPMENT_ID_ARROWS), 1);
        state.apply_object_pickup(ObjectPickupKind::MagicCarpet, 5);
        state.apply_object_pickup(ObjectPickupKind::SkullKeys, 2);
        state.apply_object_pickup(ObjectPickupKind::HmsCapePlans, 1);
        state.apply_object_pickup(ObjectPickupKind::SandalwoodBox, 1);
        state.apply_object_pickup(ObjectPickupKind::CrownOfLordBritish, 1);
        state.apply_object_pickup(ObjectPickupKind::SceptreOfLordBritish, 1);
        state.apply_object_pickup(ObjectPickupKind::AmuletOfLordBritish, 1);
        state.apply_object_pickup(ObjectPickupKind::ShadowlordShard(2), 1);

        assert_eq!(state.food, PARTY_FOOD_CAP);
        assert_eq!(state.gold, PARTY_GOLD_CAP);
        assert_eq!(state.keys, PARTY_BYTE_STOCK_CAP);
        assert_eq!(state.gems, PARTY_BYTE_STOCK_CAP);
        assert_eq!(state.torches, PARTY_BYTE_STOCK_CAP);
        assert_eq!(state.potion_stock[3], PARTY_BYTE_STOCK_CAP);
        assert_eq!(state.scroll_stock[4], PARTY_BYTE_STOCK_CAP);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_ARROWS], PARTY_BYTE_STOCK_CAP);
        assert_eq!(
            state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX],
            PARTY_BYTE_STOCK_CAP
        );
        assert_eq!(state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 2);
        assert_eq!(state.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_CROWN_LB_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_AMULET_LB_INDEX], 1);
        assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_COWARDICE_INDEX], 1);
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
    fn play_input_talk_without_suffix_opens_keyword_session() {
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
            handle_play_key_input(&mut state, 'T', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "Talk-");
        assert!(state.active_direction_prompt.is_some());
        assert!(state.active_conversation.is_none());
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.message.contains("Greetings"));
        assert!(state.active_direction_prompt.is_none());
        assert!(state.active_conversation.is_some());
        assert_eq!(state.turn, 1);

        assert_eq!(
            handle_play_key_input(&mut state, 'J', "OB", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "I mend gear\nYour interest?\n:");
        assert!(state.active_conversation.is_some());
        assert_eq!(state.turn, 1);

        assert_eq!(
            handle_play_key_input(&mut state, 'X', "YZZY", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            state.message,
            format!("{TLK_NO_KEYWORD_MATCH_MESSAGE}{TLK_KEYWORD_PROMPT}")
        );
        assert!(state.active_conversation.is_some());
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn play_input_conversation_empty_line_emits_bye_envelope_and_closes() {
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

        handle_play_key_input(&mut state, 'T', "", &dir).unwrap();
        handle_play_key_input(&mut state, '6', "", &dir).unwrap();
        assert!(state.active_conversation.is_some());

        assert_eq!(
            handle_play_key_input(&mut state, '\n', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, format!("{TLK_EMPTY_INPUT_BYE_MESSAGE}Farewell"));
        assert!(state.active_conversation.is_none());
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
    fn town_talk_liveness_gate_blocks_before_lookup_without_printing() {
        let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
            2,
            &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"],
        )]))
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.message = "previous line".to_string();
        state.player.facing = Direction::East;
        state.active_player = Some(0);
        state.party[0].status = b'S';
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
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "previous line");
        assert_eq!(state.turn, 0);
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
    fn town_raw_tlk_shop_trigger_opens_active_shop_session() {
        let dialogue = HashMap::new();
        let raw = HashMap::new();
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
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("Shipwright"));
        assert!(state.message.contains("now open"));
        assert!(state.active_shop.is_some());
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn town_raw_tlk_shop_trigger_horseback_refusal_does_not_open_session() {
        let dialogue = HashMap::new();
        let raw = HashMap::new();
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
                dialog_id: 0x85,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Blocked
        );

        assert!(state.message.contains("Herbalist"));
        assert!(state.message.contains("horseback"));
        assert!(state.active_shop.is_none());
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn town_raw_tlk_no_keyword_opens_runner_backed_conversation_session() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
                "GIFT".to_string(),
                "Take this gift".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut gift_response = enc("Take this gift");
        gift_response.push(0x86);
        gift_response.push(b'H' | 0x80);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
                enc("GIFT"),
                gift_response,
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

        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("Thou seest a quiet sage"));
        assert!(state.message.contains("Greetings"));
        assert!(state.message.ends_with(TLK_KEYWORD_PROMPT));
        assert!(state.active_conversation.is_some());
        assert_eq!(state.turn, 1);

        let (text, ended) = state.submit_active_conversation_keyword("gift");
        assert_eq!(text, "Take this gift");
        assert!(!ended);
        assert_eq!(state.special_items[SPECIAL_ITEM_SEXTANT_INDEX], 1);
    }

    #[test]
    fn town_raw_tlk_opening_runs_description_stream_before_greeting() {
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

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut description = enc("a sage watching ");
        description.push(TLK_CODE_PRINT_AVATAR_NAME);
        description.push(TLK_CODE_ACTION_DISPATCH);
        description.push(6);
        description.push(TLK_CODE_END_OF_RESPONSE);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                description,
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.party_names = vec![*b"AVATAR\0\0\0"];
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

        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("Thou seest a sage watching AVATAR"));
        assert!(state.message.contains("Greetings\nYour interest?\n:"));
        assert_eq!(state.conversation_signal_flags[6], 1);
        assert!(state.active_conversation.is_some());
    }

    #[test]
    fn active_conversation_ask_party_name_consumes_next_line_as_answer() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
                "JOIN".to_string(),
                "Name thy companion.".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut join_response = enc("Name thy companion.");
        join_response.push(TLK_CODE_ASK_PARTY_NAME);
        join_response.extend(enc(" Accepted."));
        join_response.push(TLK_CODE_END_OF_RESPONSE);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
                enc("JOIN"),
                join_response,
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: 10,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        });
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
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
        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Talked
        );

        handle_play_key_input(&mut state, 'J', "OIN", Path::new("")).unwrap();
        assert_eq!(state.message, "Name thy companion.\nName?");
        handle_play_key_input(&mut state, 'i', "olo", Path::new("")).unwrap();
        assert_eq!(state.message, " Accepted.\nYour interest?\n:");
        assert!(state.active_conversation.is_some());
    }

    #[test]
    fn active_conversation_ask_who_consumes_next_line_as_answer() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
                "WHO".to_string(),
                "Name the keeper.".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut who_response = enc("Name the keeper.");
        who_response.push(TLK_CODE_ASK_WHO);
        who_response.extend(enc(" Accepted."));
        who_response.push(TLK_CODE_END_OF_RESPONSE);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
                enc("WHO"),
                who_response,
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: 10,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        });
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
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
        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Talked
        );

        handle_play_key_input(&mut state, 'W', "HO", Path::new("")).unwrap();
        assert_eq!(state.message, "Name the keeper.\nWho?");
        handle_play_key_input(&mut state, 'i', "olo", Path::new("")).unwrap();
        assert_eq!(state.message, " Accepted.\nYour interest?\n:");
        assert!(state.active_conversation.is_some());
        assert_eq!(state.active_conversation_join_candidate, None);
    }

    fn conversation_test_roster_record(
        slot: u8,
        name: &[u8; SAVE_CHARACTER_NAME_LEN],
        class_byte: u8,
    ) -> PartyRosterRecord {
        PartyRosterRecord {
            member: PartyMember {
                slot,
                class_byte,
                status: b'G',
                climb_stat: 10 + slot,
                mana: slot,
                hp: 20 + u16::from(slot),
                max_hp: 30 + u16::from(slot),
                level: 1 + slot,
            },
            name: *name,
            experience: u16::from(slot) * 100,
            stay_counter: slot,
            strength: 15 + slot,
            intelligence: 18 + slot,
            equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
        }
    }

    #[test]
    fn conversation_join_adds_inactive_roster_companion() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Gwenno".to_string(),
                "a bard".to_string(),
                "Greetings".to_string(),
                "I sing".to_string(),
                "Farewell".to_string(),
                "JOIN".to_string(),
                "Name thy companion.".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut join_response = enc("Name thy companion.");
        join_response.push(TLK_CODE_ASK_PARTY_NAME);
        join_response.extend(enc(" Accepted."));
        join_response.push(TLK_CODE_END_OF_RESPONSE);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Gwenno"),
                enc("a bard"),
                enc("Greetings"),
                enc("I sing"),
                enc("Farewell"),
                enc("JOIN"),
                join_response,
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.party_roster = vec![
            conversation_test_roster_record(0, b"AVATAR\0\0\0", b'A'),
            conversation_test_roster_record(1, b"GWENNO\0\0\0", b'B'),
        ];
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
        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Talked
        );

        let (prompt, ended) = state.submit_active_conversation_keyword("JOIN");
        assert_eq!(prompt, "Name thy companion.");
        assert!(!ended);
        assert_eq!(
            state.active_conversation_join_candidate.as_deref(),
            Some("Gwenno")
        );
        let (text, ended) = state.submit_active_conversation_keyword("Avatar");

        assert!(text.contains("Accepted."));
        assert!(text.contains("joined."));
        assert!(!ended);
        assert_eq!(state.party.len(), 2);
        assert_eq!(state.party_names[1], *b"GWENNO\0\0\0");
    }

    #[test]
    fn conversation_join_can_be_driven_by_non_join_keyword_ask_prompt() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Gwenno".to_string(),
                "a bard".to_string(),
                "Greetings".to_string(),
                "I sing".to_string(),
                "Farewell".to_string(),
                "HELP".to_string(),
                "Name thy companion.".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut help_response = enc("Name thy companion.");
        help_response.push(TLK_CODE_ASK_PARTY_NAME);
        help_response.extend(enc(" Accepted."));
        help_response.push(TLK_CODE_END_OF_RESPONSE);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Gwenno"),
                enc("a bard"),
                enc("Greetings"),
                enc("I sing"),
                enc("Farewell"),
                enc("HELP"),
                help_response,
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.party_roster = vec![
            conversation_test_roster_record(0, b"AVATAR\0\0\0", b'A'),
            conversation_test_roster_record(1, b"GWENNO\0\0\0", b'B'),
        ];
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
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);

        let (prompt, ended) = state.submit_active_conversation_keyword("help");
        assert_eq!(prompt, "Name thy companion.");
        assert!(!ended);
        assert_eq!(
            state.active_conversation_join_candidate.as_deref(),
            Some("Gwenno")
        );
        let (text, ended) = state.submit_active_conversation_keyword("Avatar");

        assert!(text.contains("Accepted."));
        assert!(text.contains("joined."));
        assert!(!ended);
        assert_eq!(state.party.len(), 2);
        assert_eq!(state.party_names[1], *b"GWENNO\0\0\0");
    }

    #[test]
    fn conversation_ask_party_name_for_non_roster_npc_does_not_seed_join() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a sage".to_string(),
                "Greetings".to_string(),
                "I teach".to_string(),
                "Farewell".to_string(),
                "HELP".to_string(),
                "Name thy companion.".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut help_response = enc("Name thy companion.");
        help_response.push(TLK_CODE_ASK_PARTY_NAME);
        help_response.extend(enc(" Accepted."));
        help_response.push(TLK_CODE_END_OF_RESPONSE);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a sage"),
                enc("Greetings"),
                enc("I teach"),
                enc("Farewell"),
                enc("HELP"),
                help_response,
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.party_roster = vec![conversation_test_roster_record(0, b"AVATAR\0\0\0", b'A')];
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
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);

        let (prompt, ended) = state.submit_active_conversation_keyword("help");
        assert_eq!(prompt, "Name thy companion.");
        assert!(!ended);
        assert_eq!(state.active_conversation_join_candidate, None);
        let (text, ended) = state.submit_active_conversation_keyword("Avatar");

        assert_eq!(text, " Accepted.");
        assert!(!ended);
        assert_eq!(state.party.len(), 1);
        assert_eq!(state.active_conversation_join_candidate, None);
    }

    #[test]
    fn conversation_join_full_party_replaces_answered_companion() {
        let names: [[u8; SAVE_CHARACTER_NAME_LEN]; SAVE_PARTY_SIZE_MAX as usize] = [
            *b"AVATAR\0\0\0",
            *b"IOLO\0\0\0\0\0",
            *b"SHAMINO\0\0",
            *b"MARIAH\0\0\0",
            *b"JULIA\0\0\0\0",
            *b"GEOFFREY\0",
        ];
        let mut state = test_state(open_grid(), 1, 1);
        state.party = (0..SAVE_PARTY_SIZE_MAX)
            .map(|slot| conversation_test_roster_record(slot, &names[slot as usize], b'B').member)
            .collect();
        state.party_names = names.to_vec();
        state.party_experience = (0..SAVE_PARTY_SIZE_MAX)
            .map(|slot| u16::from(slot) * 100)
            .collect();
        state.party_stay_counters = (0..SAVE_PARTY_SIZE_MAX).collect();
        state.party_strengths = (0..SAVE_PARTY_SIZE_MAX).map(|slot| 15 + slot).collect();
        state.party_intelligence = (0..SAVE_PARTY_SIZE_MAX).map(|slot| 18 + slot).collect();
        state.party_equipment = default_party_equipment(SAVE_PARTY_SIZE_MAX as usize);
        state.party_roster = (0..SAVE_PARTY_SIZE_MAX)
            .map(|slot| conversation_test_roster_record(slot, &names[slot as usize], b'B'))
            .collect();
        state.party_roster.push(conversation_test_roster_record(
            SAVE_PARTY_SIZE_MAX,
            b"GWENNO\0\0\0",
            b'D',
        ));
        state.active_player = Some(1);

        let text = state
            .apply_conversation_join_candidate("Gwenno", 2)
            .unwrap();

        assert_eq!(text, "GWENNO joined; IOLO left.");
        assert_eq!(state.party.len(), SAVE_PARTY_SIZE_MAX as usize);
        assert_eq!(state.party_names[1], *b"GWENNO\0\0\0");
        assert_eq!(state.party_roster[SAVE_PARTY_SIZE_MAX as usize].name, *b"IOLO\0\0\0\0\0");
        assert_eq!(state.active_player, None);
    }

    #[test]
    fn town_raw_tlk_gold_payment_debits_only_affordable_accepted_payment() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
                "PAY".to_string(),
                "placeholder".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut pay_response = vec![0x85, b'0', b'2', b'5'];
        pay_response.push(0x9E);
        pay_response.extend(enc("Paid"));
        pay_response.push(0xFF);
        pay_response.push(0x9F);
        pay_response.extend(enc("Too poor"));
        pay_response.push(0xFF);

        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
                enc("PAY"),
                pay_response,
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.gold = 30;
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
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);
        let (text, ended) = state.submit_active_conversation_keyword("pay");
        assert_eq!(text, "");
        assert!(!ended);
        assert_eq!(state.gold, 30);
        assert!(matches!(
            state
                .active_conversation
                .as_ref()
                .map(|session| session.prompt_message()),
            Some(prompt) if prompt == "Pay 25 gold? (Y/N)"
        ));
        let (text, ended) = state.submit_active_conversation_keyword("y");
        assert_eq!(text, "Paid");
        assert!(!ended);
        assert_eq!(state.gold, 5);

        let mut poor_state = state.clone();
        poor_state.gold = 10;
        poor_state.open_conversation_session(&dialogue, &raw);
        let (text, ended) = poor_state.submit_active_conversation_keyword("pay");
        assert_eq!(text, "");
        assert!(!ended);
        assert_eq!(poor_state.gold, 10);
        let (text, ended) = poor_state.submit_active_conversation_keyword("y");
        assert_eq!(text, "Too poor");
        assert!(!ended);
        assert_eq!(poor_state.gold, 10);
    }

    #[test]
    fn town_raw_tlk_one_shot_keyword_records_numeric_signal_flag() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
                "MARK".to_string(),
                "Marked".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut mark_response = enc("Marked");
        mark_response.push(TLK_CODE_ACTION_DISPATCH);
        mark_response.push(5);
        mark_response.push(TLK_CODE_END_OF_RESPONSE);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
                enc("MARK"),
                mark_response,
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

        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, Some("mark")),
            MoveOutcome::Talked
        );

        assert_eq!(state.message, "Talked to Maris: Marked");
        assert_eq!(state.conversation_signal_flags[5], 1);
        assert!(state.active_conversation.is_none());
    }

    #[test]
    fn active_conversation_records_numeric_signal_and_cleanup_reconciles_on_bye() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
                "MARK".to_string(),
                "Marked".to_string(),
            ],
        );

        let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
        let mut mark_response = enc("Marked");
        mark_response.push(TLK_CODE_ACTION_DISPATCH);
        mark_response.push(5);
        mark_response.push(TLK_CODE_END_OF_RESPONSE);
        let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
        raw.insert(
            0x10,
            vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
                enc("MARK"),
                mark_response,
            ],
        );

        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(DEFAULT_SHADOWLORD_HIDEOUTS[0]).unwrap(),
            floor: 0,
        };
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
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);

        let (text, ended) = state.submit_active_conversation_keyword("mark");
        assert_eq!(text, "Marked");
        assert!(!ended);
        assert_eq!(state.conversation_signal_flags[5], 1);

        let (text, ended) = state.submit_active_conversation_keyword("bye");
        assert!(ended);
        assert!(state.active_conversation.is_none());
        assert_eq!(state.conversation_signal_flags[5], 0);
        assert_eq!(
            text,
            "BYE\n\nFarewell Stolen-action warning. Conversation signal 5 reconciled."
        );
    }

    #[test]
    fn final_conversation_cleanup_suppresses_on_nonzero_shared_sentinel() {
        let mut state = test_state(open_grid(), 1, 1);
        state.record_tlk_signal_flags(&[7]);
        let gold_before = state.gold;

        assert_eq!(state.shared_town_conversation_sentinel(), CONVERSATION_SHARED_NO_SLOT_SENTINEL);
        assert_eq!(state.run_final_conversation_cleanup(), None);
        assert_eq!(state.conversation_signal_flags[7], 1);
        assert_eq!(state.gold, gold_before);
    }

    #[test]
    fn final_conversation_cleanup_prioritizes_resource_then_generic_then_gold() {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(DEFAULT_SHADOWLORD_HIDEOUTS[0]).unwrap(),
            floor: 0,
        };
        state.conversation_resource_signals[1] = 2;
        state.record_tlk_signal_flags(&[3, 12]);
        let gold_before = state.gold;

        let first = state.run_final_conversation_cleanup().unwrap();
        assert!(first.contains("resource signal"));
        assert_eq!(state.conversation_resource_signals[1], 1);
        assert_eq!(state.conversation_signal_flags[12], 1);
        assert_eq!(state.gold, gold_before);

        state.conversation_resource_signals = [0; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT];
        let second = state.run_final_conversation_cleanup().unwrap();
        assert!(second.contains("Conversation signal 12"));
        assert_eq!(state.conversation_signal_flags[12], 0);
        assert_eq!(state.conversation_signal_flags[3], 1);
        assert_eq!(state.gold, gold_before);

        state.conversation_signal_flags = [0; TLK_GENERIC_SIGNAL_COUNT];
        state.gold = 10;
        let third = state.run_final_conversation_cleanup().unwrap();
        assert!(third.contains("Gold -"));
        assert!(state.gold < 10);
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

        assert!(state.message.contains("Horse & Rider"));
        assert!(state.active_shop.is_some());
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn end_to_end_horse_trader_purchase_places_boardable_horse() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(20).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 130;
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
                dialog_id: 0x83,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert!(state.message.contains("130 gold"));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 0);
        assert!(state.active_shop.is_none());
        assert!(state.message.contains("Thy horse awaits outside"));
        let horse = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == HORSE_PARKED_FIRST)
            .copied()
            .expect("horse object was not placed");
        assert_eq!((horse.x, horse.y, horse.z), (1, 2, 0));
        assert!(matches!(
            state
                .boardable_vehicle_slot_at(1, 2)
                .map(|candidate| candidate.transport),
            Some(TransportState::Horse { .. })
        ));
    }

    #[test]
    fn town_talk_guild_shop_uses_scene_local_prices() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(24).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 500;
        state.keys = 0;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0x86,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );
        assert!(state.message.contains("The Nemesis"));

        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
        assert!(state.message.contains("The Nemesis sells keys for 185 gold each"));
        handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 130);
        assert_eq!(state.keys, 2);
        assert!(state.message.contains("The Nemesis sold 2 keys for 370 gold"));
    }

    #[test]
    fn town_talk_herbalist_uses_scene_local_reagent_menu() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(23).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 100;
        state.reagents = [0; REAGENT_COUNT];
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0x85,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );
        assert!(state.message.contains("Mysticism"));

        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
        assert!(state.message.contains("Mysticism sells Spider Silk for 6 gold each"));
        handle_play_key_input(&mut state, '3', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 82);
        assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 3);
        assert!(state.message.contains("Mysticism sold 3 Spider Silk for 18 gold"));
    }

    #[test]
    fn town_talk_horse_trader_uses_scene_local_stable_price() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(20).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 200;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0x83,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );
        assert!(state.message.contains("The Stablehouse"));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert!(state.message.contains("130 gold"));
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
    fn raw_conversation_session_expands_loaded_common_word_dictionary() {
        let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
        dialogue.insert(
            0x10,
            vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "fallback greeting".to_string(),
                "fallback job".to_string(),
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
                vec![0x01],
                enc("I read books"),
                enc("Farewell"),
            ],
        );

        let mut dictionary = std::array::from_fn(|_| String::new());
        dictionary[0] = "Greetings".to_string();

        let mut state = test_state(open_grid(), 1, 1);
        state.common_word_dictionary = Some(dictionary);
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
                dialog_id: 0x10,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Talked
        );
        assert!(state.message.contains("Greetings"));
        assert!(!state.message.contains("[w00]"));
    }

    fn tokenized_tlk_bytes_for_test() -> Vec<u8> {
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
        push_text(&mut bytes, "Ada");
        push_text(&mut bytes, "a test speaker");
        bytes.push(0x01);
        bytes.push(0);
        push_text(&mut bytes, "I speak");
        push_text(&mut bytes, "Bye");
        bytes
    }

    fn complete_common_word_dictionary_text(first_word: &str) -> String {
        (0..COMMON_WORD_DICTIONARY_ENTRIES)
            .map(|index| {
                let word = if index == 0 {
                    first_word.to_string()
                } else {
                    format!("word{index}")
                };
                format!("{index}\t{word}\n")
            })
            .collect()
    }

    #[test]
    fn game_dir_talk_requires_dictionary_for_tokenized_raw_tlk() {
        let dir = debug_game_dir();
        fs::write(dir.join("CASTLE.TLK"), tokenized_tlk_bytes_for_test()).unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 2,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        let err = state.talk_facing_with_game_dir(&dir).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains(COMMON_WORD_DICTIONARY_FILE));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn game_dir_talk_expands_loaded_dictionary_token_without_placeholder() {
        let dir = debug_game_dir();
        fs::write(dir.join("CASTLE.TLK"), tokenized_tlk_bytes_for_test()).unwrap();
        fs::write(
            dir.join(COMMON_WORD_DICTIONARY_FILE),
            complete_common_word_dictionary_text("the"),
        )
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 2,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("the"));
        assert!(!state.message.contains("[w01]"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn tlk_and_shoppe_tokens_share_dictionary_entry_zero() {
        let mut dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
        dict[0] = "the";
        let tlk = crate::tlk_runner::run_tlk_stream(
            &[0x01],
            &crate::tlk_runner::TlkRunInputs {
                dictionary: Some(&dict),
                ..Default::default()
            },
        );
        let shoppe = crate::shoppe_bark::render_shoppe_bark(
            &[0x80],
            &crate::shoppe_bark::ShoppeBarkContext {
                dictionary: Some(&dict),
                ..Default::default()
            },
        );

        assert_eq!(tlk.text, "the");
        assert_eq!(shoppe, "the");
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
        assert!(text.starts_with(TLK_EMPTY_INPUT_BYE_MESSAGE));
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
        // First key 'R' selects inn rest.
        handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
        assert!(state.message.contains("room"));
        // 'Y' again to confirm.
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert!(state.message.contains("Rested"));
        assert!(state.gold < 100);
    }

    #[test]
    fn end_to_end_innkeeper_decline_returns_to_greeting_without_charge() {
        use crate::shop_runtime::*;
        use crate::shop_session::ActiveShopSession;
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.active_shop = Some(ActiveShopSession::Innkeeper(
            InnkeeperState::ConfirmRest {
                inn: Inn::TheWayfarerInn,
                adjusted_room_rate: 2,
                total_price: 2,
            },
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
    fn end_to_end_innkeeper_leave_companion_moves_roster_to_registry() {
        use crate::shop_runtime::InnkeeperState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 8,
                hp: 30,
                max_hp: 30,
                level: 5,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'G',
                climb_stat: 7,
                mana: 3,
                hp: 12,
                max_hp: 28,
                level: 3,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.party_stay_counters = vec![8, 9];
        state.party_strengths = vec![30, 17];
        state.party_intelligence = vec![30, 19];
        state.party_experience = vec![0, 700];
        state.party_equipment = default_party_equipment(2);
        state.active_shop = Some(ActiveShopSession::Innkeeper(
            InnkeeperState::for_inn(Inn::HotelBrittany),
        ));

        handle_play_key_input(&mut state, 'L', "", Path::new("")).unwrap();
        assert!(state.message.contains("Deposit is 30 gold"));
        handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();
        assert!(state.message.contains("party member 2"));
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 70);
        assert_eq!(state.party.len(), 1);
        assert_eq!(state.party_names, vec![*b"AVATAR\0\0\0"]);
        assert_eq!(state.inn_registry.len(), 1);
        assert_eq!(state.inn_registry[0].scene_marker, 0x11);
        assert_eq!(state.inn_registry[0].name, *b"IOLO\0\0\0\0\0");
        assert!(state.message.contains("Left companion 2"));
    }

    #[test]
    fn end_to_end_innkeeper_pickup_restores_matching_guest() {
        use crate::shop_runtime::InnkeeperState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.inn_registry.push(InnGuestRecord {
            scene_marker: 0x11,
            name: *b"IOLO\0\0\0\0\0",
            member: PartyMember {
                slot: 4,
                class_byte: b'B',
                status: b'P',
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
            stay_counter: 0,
        });
        state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::default()));

        handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
        assert!(state.message.contains("20 gold"));
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 80);
        assert!(state.inn_registry.is_empty());
        assert_eq!(state.party.len(), 2);
        assert_eq!(state.party[1].status, b'D');
        assert_eq!(state.party[1].hp, 0);
        assert_eq!(state.party_names[1], *b"IOLO\0\0\0\0\0");
        assert!(state.message.contains("has died"));
    }

    #[test]
    fn end_to_end_tavern_blue_boar_fixed_drink_debits_gold() {
        use crate::shop_runtime::TavernState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 200;
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
            Tavern::TheBlueBoarTavern,
        )));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert!(state.message.contains("Blue Boar"));
        handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
        assert!(state.message.contains("A-F"));
        handle_play_key_input(&mut state, 'F', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 102);
        assert!(state.message.contains("98 gold"));
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn end_to_end_tavern_provisions_partially_fill_to_food_cap() {
        use crate::shop_runtime::TavernState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 50;
        state.food = SHOP_FOOD_STOCK_CAP - 1;
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
            Tavern::TheWayfarerTavern,
        )));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
        assert!(state.message.contains("15 gold each"));
        handle_play_key_input(&mut state, '5', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 35);
        assert_eq!(state.food, SHOP_FOOD_STOCK_CAP);
        assert!(state.message.contains("sold 1/5"));
    }

    #[test]
    fn end_to_end_tavern_provisions_accept_multi_digit_inline_quantity() {
        use crate::shop_runtime::TavernState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 1000;
        state.food = 0;
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
            Tavern::TheWayfarerTavern,
        )));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '1', "2", Path::new("")).unwrap();

        assert_eq!(state.gold, 820);
        assert_eq!(state.food, 12);
        assert!(state.message.contains("sold 12/12"));
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn end_to_end_healer_mission_cure_bypasses_gold_path() {
        use crate::shop_runtime::HealerShopState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 0;
        state.party[0].status = b'P';
        state.party[0].hp = 7;
        state.party[0].max_hp = 20;
        state.active_shop = Some(ActiveShopSession::Healer(
            HealerShopState::Greeting,
            Healer::TheHealersMission,
        ));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 0);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[0].hp, 7);
        assert_eq!(state.message, "Cured party member 1.");
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn end_to_end_paid_healer_uses_local_fee_and_play_state_treatment() {
        use crate::shop_runtime::HealerShopState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 60;
        state.party[0].status = b'P';
        state.party[0].hp = 5;
        state.party[0].max_hp = 22;
        state.active_shop = Some(ActiveShopSession::Healer(
            HealerShopState::Greeting,
            Healer::TheShieldOfTruth,
        ));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'H', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();
        assert!(state.message.contains("60 gold"));
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 0);
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 22);
        assert_eq!(state.message, "Healed party member 1 to 22/22.");
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn active_shop_surcharge_applies_only_for_zero_shadowlord_sentinel() {
        use crate::shop_runtime::TavernState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(DEFAULT_SHADOWLORD_HIDEOUTS[0]).unwrap(),
            floor: 0,
        };
        state.gold = 100;
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
            Tavern::TheHonestMeal,
        )));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'M', "", Path::new("")).unwrap();

        assert!(state.gold < 97);
        assert!(state.message.contains("served a round for 3 gold"));
        assert!(state.message.contains("Surcharge"));
    }

    #[test]
    fn active_shop_surcharge_suppresses_without_zero_shadowlord_sentinel() {
        use crate::shop_runtime::TavernState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
            Tavern::TheHonestMeal,
        )));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'M', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 97);
        assert!(state.message.contains("served a round for 3 gold"));
        assert!(!state.message.contains("Surcharge"));
    }

    #[test]
    fn end_to_end_stationary_display_purchase_removes_display_and_grants_item() {
        use crate::shop_runtime::StationaryDisplayState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.shadowlord_hideouts = [250, 251, 252];
        state.gold = 100;
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: 0x90,
            tile: 0x90,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_shop = Some(ActiveShopSession::StationaryDisplay(
            StationaryDisplayState::new(EQUIPMENT_ID_BOW as u8, 75, 0, Some(1)),
        ));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert_eq!(state.gold, 100);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 0);
        assert!(!state.active_objects[1].is_empty());
        assert!(state.active_shop.is_some());
        assert!(state.message.contains("Bow costs 75 gold"));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 25);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 1);
        assert!(state.active_objects[1].is_empty());
        assert!(state.visibility_dirty);
        assert!(state.active_shop.is_none());
        assert!(state.message.contains("Party member 1 bought Bow"));
    }

    #[test]
    fn end_to_end_stationary_display_refusal_keeps_display_and_inventory() {
        use crate::shop_runtime::StationaryDisplayState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 10;
        state.active_objects.push(ActiveObject {
            type_byte: 0x90,
            tile: 0x90,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_shop = Some(ActiveShopSession::StationaryDisplay(
            StationaryDisplayState::new(30, 70, 0, Some(1)),
        ));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 10);
        assert_eq!(state.equipment_stock[30], 0);
        assert!(!state.active_objects[1].is_empty());
        assert!(state.active_shop.is_none());
        assert!(state.message.contains("Thou lackest the 70 gold"));

        state.active_shop = Some(ActiveShopSession::StationaryDisplay(
            StationaryDisplayState::new(30, 70, 0, Some(1)),
        ));
        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
        assert_eq!(state.message, "Farewell.");
        assert!(state.active_shop.is_none());
        assert!(!state.active_objects[1].is_empty());
    }

    #[test]
    fn end_to_end_sage_rumour_quotes_confirms_and_debits_gold() {
        use crate::shop_runtime::SageState;
        use crate::shop_session::ActiveShopSession;

        static TOPICS: [SageTopic; 1] = [SageTopic {
            topic: "codex",
            subject: "the Codex",
            destination: "the Underworld",
            fee: 17,
            template: SageRumourTemplate::SeekSubjectInDestination,
        }];

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 20;
        state.active_shop = Some(ActiveShopSession::Sage(SageState::for_topics(&TOPICS)));

        handle_play_key_input(&mut state, 'C', "ODEX", Path::new("")).unwrap();
        assert_eq!(state.gold, 20);
        assert!(state.message.contains("17 gold"));
        assert!(state.active_shop.is_some());

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 3);
        assert_eq!(state.message, "Seek ye the Codex in the Underworld!");
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn end_to_end_reagent_vendor_uses_compact_herbalist_letter_menu() {
        use crate::shop_runtime::ReagentShopState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.reagents = [0; REAGENT_COUNT];
        state.active_shop = Some(ActiveShopSession::Reagent(
            ReagentShopState::for_herbalist(Herbalist::Mysticism),
        ));

        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
        assert!(state.message.contains("Spider Silk"));
        handle_play_key_input(&mut state, '3', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 82);
        assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 3);
        assert!(state.message.contains("18 gold"));
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn end_to_end_reagent_vendor_accepts_multi_digit_inline_quantity() {
        use crate::shop_runtime::ReagentShopState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.reagents = [0; REAGENT_COUNT];
        state.active_shop = Some(ActiveShopSession::Reagent(
            ReagentShopState::for_herbalist(Herbalist::Mysticism),
        ));

        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '1', "2", Path::new("")).unwrap();

        assert_eq!(state.gold, 28);
        assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 12);
        assert!(state.message.contains("72 gold"));
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn end_to_end_guildmaster_uses_shop_letter_prices() {
        use crate::shop_runtime::GuildShopState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 500;
        state.keys = 0;
        state.active_shop = Some(ActiveShopSession::Guild(GuildShopState::for_shop(
            GuildShop::TheDen,
        )));

        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
        assert!(state.message.contains("keys"));
        handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 120);
        assert_eq!(state.keys, 2);
        assert!(state.message.contains("380 gold"));
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn end_to_end_guildmaster_accepts_multi_digit_inline_quantity() {
        use crate::shop_runtime::GuildShopState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 2000;
        state.keys = 0;
        state.active_shop = Some(ActiveShopSession::Guild(GuildShopState::for_shop(
            GuildShop::TheDen,
        )));

        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '1', "0", Path::new("")).unwrap();

        assert_eq!(state.gold, 100);
        assert_eq!(state.keys, 10);
        assert!(state.message.contains("1900 gold"));
        assert!(state.active_shop.is_some());
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
    fn end_to_end_stocked_arms_shop_buys_by_menu_letter() {
        use crate::shop_runtime::ArmsShopState;
        use crate::shop_session::ActiveShopSession;
        use crate::shops::ArmsStockTable;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 1000;
        state.party_intelligence[0] = 10;
        state.active_shop = Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::Greeting,
            ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
        ));

        handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();
        assert!(state.message.contains("a) Short Sword"));
        assert!(state.message.contains("b) Mace"));

        handle_play_key_input(&mut state, 'b', "", Path::new("")).unwrap();
        assert!(state.message.contains("Item 24 costs"));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert_eq!(state.equipment_stock[24], 1);
        assert_eq!(state.equipment_stock[1], 0);
        assert!(state.gold < 1000);
        assert!(state.message.contains("Bought item 24"));
        assert!(state.active_shop.is_some());
    }

    #[test]
    fn end_to_end_stocked_arms_shop_rejects_empty_stock_letters() {
        use crate::shop_runtime::ArmsShopState;
        use crate::shop_session::ActiveShopSession;
        use crate::shops::ArmsStockTable;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 1000;
        state.active_shop = Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::Greeting,
            ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
        ));

        handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'd', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 1000);
        assert!(state.equipment_stock.iter().all(|count| *count == 0));
        assert_eq!(state.message, "I do not understand.");
        assert!(matches!(
            state.active_shop,
            Some(ActiveShopSession::ArmsStocked(
                ArmsShopState::BuyPickItem,
                _
            ))
        ));
    }

    #[test]
    fn end_to_end_shipwright_frigate_queues_return_world_delivery() {
        use crate::shop_runtime::ShipBrokerState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 3, 4);
        state.gold = 700;
        state.active_shop = Some(ActiveShopSession::ShipBroker(
            ShipBrokerState::for_shipwright(Shipwright::TheRustyBucket),
        ));
        state.return_world = Some(WorldReturn {
            plane: WorldPlane::Britannia,
            x: 12,
            y: 21,
            transport: TransportState::Foot,
            timing_status: TimingStatusTag::Normal,
            sail_cadence: 0,
            sail_stall_pending: false,
            grid: open_world_grid(),
            active_objects: vec![ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: 12,
                y: 21,
                z: WorldPlane::Britannia.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }],
            pending_vehicle: None,
        });

        handle_play_key_input(&mut state, 'F', "", Path::new("")).unwrap();
        assert!(state.message.contains("700 gold"));
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 0);
        assert!(state.message.contains("Delivery is queued"));
        assert_eq!(
            state.return_world.as_ref().and_then(|world| world.pending_vehicle),
            Some(PendingVehicleAcquisition::Frigate {
                x: 12,
                y: 21,
                skiffs: 2,
            })
        );
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

