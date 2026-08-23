    #[test]
    fn dungeon_get_refuses_chest_room_trigger_or_unrelated_cell_without_turn() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x42;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(state.get_dungeon_underfoot(scene, 0), MoveOutcome::Blocked);

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x42);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Must open it first.");

        state.grid[dungeon_cell_index(0, 1, 1)] = 0xf2;
        assert_eq!(state.get_dungeon_underfoot(scene, 0), MoveOutcome::Blocked);

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xf2);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to get here.");

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
        assert_eq!(state.message, "What?");
    }

    #[test]
    fn dungeon_open_preserves_room_trigger_without_turn() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xf2;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(state.open_facing(), MoveOutcome::Blocked);

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xf2);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "What?");
    }

    #[test]
    fn dungeon_open_underfoot_passage_chest_variant_reports_chest_opened() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x70);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Chest opened");
    }

    #[test]
    fn dungeon_jimmy_preserves_room_trigger_without_turn() {
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
    fn dungeon_e_cells_are_pass_through_visual_variants_not_openable_doors() {
        let mut open_grid = open_dungeon_record();
        open_grid[dungeon_cell_index(0, 1, 1)] = 0xe2;
        let mut open_state = dungeon_state(open_grid, 0, 1, 1);

        assert_eq!(open_state.open_facing(), MoveOutcome::Blocked);
        assert_eq!(open_state.grid[dungeon_cell_index(0, 1, 1)], 0xe2);
        assert_eq!(open_state.turn, 0);
        assert_eq!(open_state.message, "What?");

        let mut jimmy_grid = open_dungeon_record();
        jimmy_grid[dungeon_cell_index(0, 1, 1)] = 0xe2;
        let mut jimmy_state = dungeon_state(jimmy_grid, 0, 1, 1);
        assert_eq!(
            jimmy_state
                .jimmy_facing_with_game_dir_and_member(None, Some(0))
                .unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(jimmy_state.grid[dungeon_cell_index(0, 1, 1)], 0xe2);
        assert_eq!(jimmy_state.turn, 0);
        assert_eq!(jimmy_state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(jimmy_state.message, "No lock!");

        let mut step_grid = open_dungeon_record();
        step_grid[dungeon_cell_index(0, 2, 1)] = 0xe2;
        let mut step_state = dungeon_state(step_grid, 0, 1, 1);
        assert_eq!(step_state.step(Direction::East), MoveOutcome::Moved);
        assert_eq!((step_state.player.x, step_state.player.y), (2, 1));
        assert_eq!(step_state.turn, 1);
        assert!(step_state.message.contains("underfoot heavy-door variant"));
    }

    #[test]
    fn stale_dungeon_door_sidecar_file_does_not_block_f_room_trigger() {
        let dir = debug_game_dir();
        fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 2 1 0x70 0xF2\n")
            .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xF2;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xA2);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("room trigger slot 2"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_open_ignores_stale_f_room_trigger_sidecar_without_turn() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 1 1 0x70 0xF2\n")
            .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xF2;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.visibility_dirty = false;

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xF2);
        assert_eq!(state.turn, 0);
        assert_eq!(state.door_tracker, None);
        assert!(!state.visibility_dirty);
        assert_eq!(state.message, "What?");
        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_jimmy_key_on_f_room_trigger_ignores_stale_sidecar() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 1 1 0x70 0xF2\n")
            .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xF2;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.visibility_dirty = false;

        assert!(state.handle_dungeon_key('J', &dir).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xA2);
        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("room trigger slot 2"));
        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stale_sidecar_open_f_cell_key_runs_underfoot_trigger_first() {
        let dir = debug_game_dir();
        fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 1 1 0xF1 0xF2\n")
            .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xF1;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.visibility_dirty = false;

        assert!(state.handle_dungeon_key('J', &dir).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xA1);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("room trigger slot 1"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stale_sidecar_open_f_cell_still_enters_room_trigger() {
        let dir = debug_game_dir();
        fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 2 1 0xF1 0xF2\n")
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
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xA1);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("room trigger slot 1"));
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
    fn town_look_routes_sign_object_classes_through_sign_records() {
        let dir = debug_game_dir();
        fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
        fs::write(
            dir.join(SIGNS_DAT_FILE),
            signs_dat_bytes_for_test(&[(17, 0, 1, 2, b"Posted notice")]),
        )
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 0xa0,
            tile: 0xa0,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.look_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Observed
        );

        assert_eq!(state.message, "Sign:\nPosted notice");
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn yew_wanted_poster_exception_uses_party_names_without_signs_dat() {
        let mut state = test_state(open_grid(), 16, 21);
        state.area = Area::Town {
            scene: Scene::new(4).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.active_objects.push(ActiveObject {
            type_byte: 0xa0,
            tile: 0xa0,
            x: 17,
            y: 21,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        assert_eq!(
            state.message,
            [
                "abbbbbbbbbbbbbc",
                "g   Wanted:   g",
                "g             g",
                "g   AVATAR    g",
                "g    IOLO     g",
                "g             g",
                "g             g",
                "gDead or Aliveg",
                "deeeeeeeeeeeeeef",
            ]
            .join("\n")
        );
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn town_look_routes_death_vision_object_before_generic_object_description() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.party_intelligence[0] = 31;
        state.active_objects.push(ActiveObject {
            type_byte: DEATH_VISION_OBJECT_CLASS,
            tile: DEATH_VISION_OBJECT_CLASS,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        assert_eq!(
            state.active_direction_prompt.as_ref().map(|session| session.kind),
            Some(DirectionPromptKind::SurfaceDeathVision { x: 2, y: 1 })
        );
        assert!(state.message.contains("death-vision member"));
        assert!(!state.message.contains("actor tile"));
        assert_eq!(state.turn, 0);

        let outcome = state
            .step_active_direction_prompt('1', "", Path::new(""))
            .unwrap();

        assert_eq!(outcome, Some(MoveOutcome::Observed));
        assert!(state.active_direction_prompt.is_none());
        assert!(state.message.contains("Strange vision"));
        assert!(state.active_view_overlay.is_some());
        assert!(!state.message.contains("actor tile"));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn death_vision_failure_has_no_overlay_and_names_member_number() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.party_intelligence[0] = 0;

        assert_eq!(
            state.apply_death_vision_look_for_member(2, 1, 0),
            MoveOutcome::Observed
        );

        assert_eq!(state.message, "Death vision: party member 1.");
        assert!(state.active_view_overlay.is_none());
        assert_eq!(state.turn, 0);
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
    fn town_look_visibility_gate_hides_dark_unlit_target() {
        let table = parse_look2_dat(&look2_bytes(&[(16, "east road")])).unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 16;
        let mut hidden = test_state(grid.clone(), 1, 1);
        hidden.player.facing = Direction::East;
        // `visibility.md §4`: zero light is the pitch-dark branch — the
        // carve is skipped and even the adjacent cell stays hidden.
        // `FULL_DARKNESS` (2) is a threshold that still lights it.
        hidden.ambient_light = 0;
        hidden.message = "previous".to_string();

        assert_eq!(
            hidden.look_direction_with_table(Direction::East, Some(&table)),
            MoveOutcome::Observed
        );

        assert!(hidden.message.is_empty());
        assert_eq!(hidden.turn, 0);

        let mut lit = test_state(grid, 1, 1);
        lit.player.facing = Direction::East;
        lit.ambient_light = FULL_DARKNESS;
        lit.torch_counter = 1;
        lit.recompute_daylight();

        assert_eq!(
            lit.look_direction_with_table(Direction::East, Some(&table)),
            MoveOutcome::Observed
        );

        assert!(lit.message.contains("east road at (2, 1)"));
        assert_eq!(lit.turn, 0);
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
    fn town_look_visibility_gate_precedes_special_surface_prompts() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa1;
        let mut hidden = test_state(grid.clone(), 1, 1);
        hidden.player.facing = Direction::East;
        // `visibility.md §4`: zero light is the pitch-dark branch — the
        // carve is skipped and even the adjacent cell stays hidden.
        // `FULL_DARKNESS` (2) is a threshold that still lights it.
        hidden.ambient_light = 0;
        hidden.gold = 1;

        assert_eq!(hidden.look_facing(), MoveOutcome::Observed);

        assert!(hidden.message.is_empty());
        assert!(hidden.active_wishing_well.is_none());

        let mut lit = test_state(grid, 1, 1);
        lit.player.facing = Direction::East;
        lit.ambient_light = FULL_DARKNESS;
        lit.torch_counter = 1;
        lit.recompute_daylight();

        assert_eq!(lit.look_facing(), MoveOutcome::Observed);

        assert_eq!(
            lit.active_wishing_well
                .as_ref()
                .map(|session| (session.direction, session.coin_accepted)),
            Some((Direction::East, false))
        );
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
        state.area = Area::Town {
            scene: Scene::new(0x16).unwrap(),
            floor: 0,
        };
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
        assert_eq!(state.message, "Wishing well: a horse appears.");
        let horse = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == HORSE_PARKED_FIRST)
            .expect("accepted wishing-well wish should spawn a horse");
        assert_eq!(
            (horse.type_byte, horse.x, horse.y, horse.z, horse.aux1),
            (HORSE_PARKED_FIRST, 2, 1, 0, 0)
        );
        assert_eq!(
            state.boardable_vehicle_slot_at(2, 1).map(|candidate| candidate.transport),
            Some(TransportState::Horse {
                type_byte: HORSE_MOUNTED_FIRST,
                tile: FIRST_PLAYABLE_HORSE_TILE,
            })
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_surface_wishing_well_rejects_unknown_wish_after_coin() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa1;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.gold = 2;
        assert_eq!(state.look_facing(), MoveOutcome::Observed);
        assert_eq!(state.step_active_wishing_well('Y', ""), None);

        assert_eq!(
            state.step_active_wishing_well('A', "vatar"),
            Some(MoveOutcome::Observed)
        );

        assert!(state.active_wishing_well.is_none());
        assert_eq!(state.gold, 1);
        assert_eq!(state.message, "Wishing well: no effect.");
        assert!(
            state
                .active_objects
                .iter()
                .skip(1)
                .all(|object| object.type_byte != HORSE_PARKED_FIRST)
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
    }

    #[test]
    fn town_surface_wishing_well_rejects_accepted_wish_outside_grant_scenes() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa1;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.gold = 2;
        assert_eq!(state.look_facing(), MoveOutcome::Observed);
        assert_eq!(state.step_active_wishing_well('Y', ""), None);

        assert_eq!(
            state.step_active_wishing_well('H', "orse"),
            Some(MoveOutcome::Observed)
        );

        assert_eq!(state.gold, 1);
        assert_eq!(state.message, "Wishing well: no effect.");
        assert!(state.boardable_vehicle_slot_at(2, 1).is_none());
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn town_surface_wishing_well_car_wish_grants_horse_family_object() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa1;
        let mut state = test_state(grid, 1, 1);
        state.area = Area::Town {
            scene: Scene::new(0x1f).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 2;
        assert_eq!(state.look_facing(), MoveOutcome::Observed);
        assert_eq!(state.step_active_wishing_well('Y', ""), None);

        assert_eq!(
            state.step_active_wishing_well('F', "errari"),
            Some(MoveOutcome::Observed)
        );

        assert_eq!(state.message, "Wishing well: a horse appears.");
        assert!(state.boardable_vehicle_slot_at(2, 1).is_some());
    }

    #[test]
    fn town_surface_fountain_drink_refreshes_living_member_without_mutating() {
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

    #[test]
    fn town_surface_fountain_prompt_cancel_prints_no_one_result() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xd8;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        assert_eq!(state.look_facing(), MoveOutcome::Observed);

        let outcome = state
            .step_active_direction_prompt(' ', "", Path::new(""))
            .unwrap();

        assert_eq!(outcome, Some(MoveOutcome::Observed));
        assert!(state.active_direction_prompt.is_none());
        assert_eq!(state.message, "You see: a fountain. No one drinks.");
        assert_eq!(state.turn, 0);
    }

    fn signs_dat_bytes_for_test(records: &[(u8, u8, u8, u8, &[u8])]) -> Vec<u8> {
        let mut bytes = vec![0; SIGNS_DAT_SCENE_DIRECTORY_BYTES];
        if let Some((scene, ..)) = records.first() {
            let offset = SIGNS_DAT_SCENE_DIRECTORY_BYTES as u16;
            bytes[*scene as usize * 2..*scene as usize * 2 + 2]
                .copy_from_slice(&offset.to_le_bytes());
        }
        for (scene, z, y, x, body) in records {
            bytes.extend_from_slice(&[*scene, *z, *x, *y]);
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
        let mut grid = open_grid();
        grid[2 * 32 + 1] = 0x05;
        let mut state = test_state(grid, 1, 1);

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
    fn world_look_visibility_gate_hides_dark_unlit_wrapped_object() {
        let mut hidden = world_state(open_world_grid(), 255, 0);
        hidden.player.facing = Direction::East;
        // `visibility.md §4`: zero light is the pitch-dark branch — the
        // carve is skipped and even the adjacent cell stays hidden.
        // `FULL_DARKNESS` (2) is a threshold that still lights it.
        hidden.ambient_light = 0;
        hidden.message = "previous".to_string();
        hidden.active_objects.push(ActiveObject {
            type_byte: 170,
            tile: 170,
            x: 0,
            y: 0,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(hidden.look_facing(), MoveOutcome::Observed);

        assert!(hidden.message.is_empty());
        assert_eq!(hidden.turn, 0);

        let mut lit = world_state(open_world_grid(), 255, 0);
        lit.player.facing = Direction::East;
        lit.ambient_light = FULL_DARKNESS;
        lit.torch_counter = 1;
        lit.recompute_daylight();
        lit.active_objects.push(ActiveObject {
            type_byte: 170,
            tile: 170,
            x: 0,
            y: 0,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(lit.look_facing(), MoveOutcome::Observed);

        assert!(lit.message.contains("object tile 170"));
        assert_eq!(lit.turn, 0);
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
    fn parsed_tlk_gives_every_id_its_own_blob_including_id_one() {
        // There is no sentinel row and no id-1 alias: each header row's id
        // addresses its own blob. Confirmed against the shipped files, whose
        // ids run exactly 1..=count.
        let dialogue = parse_tlk_bytes(&tlk_bytes(&[
            (1, &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"]),
            (2, &["Bry", "a test baker", "Hello", "I bake bread", "Bye"]),
        ]))
        .unwrap();

        assert_eq!(dialogue.len(), 2);
        assert_eq!(dialogue[&1][0], "Ada");
        assert_eq!(dialogue[&2][0], "Bry");
        assert_ne!(dialogue.get(&1), dialogue.get(&2));
    }

    #[test]
    fn parsed_tlk_caps_each_blob_to_runtime_window() {
        let long_name = "A".repeat(1100);
        let bytes = tlk_bytes(&[(2, &[long_name.as_str()])]);
        let dialogue = parse_tlk_bytes(&bytes).unwrap();

        assert_eq!(dialogue[&2][0].len(), 1024);
    }

    #[test]
    fn shipped_tlk_corpus_parses_and_runner_smokes_sanitized_fields() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join(TOWNE_TLK_FILENAME).exists() {
            return;
        }

        let corpora = [
            (TOWNE_TLK_FILENAME, TlkFileClass::Towne.shipped_npc_count()),
            (
                DWELLING_TLK_FILENAME,
                TlkFileClass::Dwelling.shipped_npc_count(),
            ),
            (
                CASTLE_TLK_FILENAME,
                TlkFileClass::Castle.shipped_npc_count(),
            ),
            (KEEP_TLK_FILENAME, TlkFileClass::Keep.shipped_npc_count()),
        ];
        let inputs = crate::tlk_runner::TlkRunInputs {
            avatar_name: "Avatar",
            moral_standing: 99,
            dictionary: Some(&PUBLISHED_COMMON_WORD_DICTIONARY),
            gold_payment_accepted: true,
            gold_available: Some(9999),
            ..Default::default()
        };
        let mut total_npcs = 0usize;
        let mut total_fields = 0usize;
        let mut dictionary_fields = 0usize;
        let mut control_counts = [(0u8, 0usize); 7];
        for (slot, code) in [0x84, 0x85, 0x86, 0x87, 0x88, 0x8c, 0xfe]
            .into_iter()
            .enumerate()
        {
            control_counts[slot].0 = code;
        }

        for (file_name, expected_npcs) in corpora {
            let bytes = fs::read(game_dir.join(file_name)).unwrap();
            let decoded = parse_tlk_bytes(&bytes).unwrap();
            let raw = parse_tlk_blob_fields_raw(&bytes).unwrap();

            assert_eq!(decoded.len(), expected_npcs, "{file_name}");
            assert_eq!(raw.len(), expected_npcs, "{file_name}");
            total_npcs += raw.len();

            for npc_id in 1..=expected_npcs as u16 {
                let fields = raw
                    .get(&npc_id)
                    .unwrap_or_else(|| panic!("{file_name} missing NPC id {npc_id}"));
                assert!(
                    fields.len() >= 5,
                    "{file_name} NPC id {npc_id} has too few fields"
                );
                total_fields += fields.len();

                if tlk_fields_use_common_word_dictionary(fields) {
                    dictionary_fields += 1;
                }

                for field in fields {
                    for byte in field.iter().copied() {
                        if let Some((_, count)) =
                            control_counts.iter_mut().find(|(code, _)| *code == byte)
                        {
                            *count += 1;
                        }
                    }

                    let output = crate::tlk_runner::run_tlk_stream(field, &inputs);
                    assert!(
                        !matches!(
                            output.stop,
                            crate::tlk_runner::TlkRunStop::MalformedIntroducer(_)
                        ),
                        "{file_name} NPC id {npc_id} stopped at {:?}",
                        output.stop
                    );
                    assert!(
                        !output.text.contains("[w"),
                        "{file_name} NPC id {npc_id} left an unresolved dictionary token"
                    );
                }
            }
        }

        assert_eq!(
            total_npcs,
            TOWNE_TLK_NPCS + DWELLING_TLK_NPCS + CASTLE_TLK_NPCS + KEEP_TLK_NPCS
        );
        assert!(total_fields > total_npcs * 5);
        assert!(dictionary_fields > 0);
        assert!(control_counts.iter().any(|(_, count)| *count > 0));
    }

    #[test]
    fn shipped_tlk_corpus_contains_public_action_payment_and_flag_controls() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join(TOWNE_TLK_FILENAME).exists() {
            return;
        }

        let corpora = [
            (TOWNE_TLK_FILENAME, TlkFileClass::Towne.shipped_npc_count()),
            (
                DWELLING_TLK_FILENAME,
                TlkFileClass::Dwelling.shipped_npc_count(),
            ),
            (
                CASTLE_TLK_FILENAME,
                TlkFileClass::Castle.shipped_npc_count(),
            ),
            (KEEP_TLK_FILENAME, TlkFileClass::Keep.shipped_npc_count()),
        ];
        let inputs = crate::tlk_runner::TlkRunInputs {
            avatar_name: "Avatar",
            moral_standing: 99,
            dictionary: Some(&PUBLISHED_COMMON_WORD_DICTIONARY),
            gold_payment_accepted: true,
            gold_available: Some(9999),
            ..Default::default()
        };

        let mut action_arg_counts = [0usize; 11];
        let mut surfaced_action_count = 0usize;
        let mut gold_payment_controls = 0usize;
        let mut surfaced_gold_payments = 0usize;
        let mut set_flag_controls = 0usize;
        let mut if_else_controls = 0usize;
        let mut ask_party_controls = 0usize;

        for (file_name, expected_npcs) in corpora {
            let bytes = fs::read(game_dir.join(file_name)).unwrap();
            let raw = parse_tlk_blob_fields_raw(&bytes).unwrap();

            for npc_id in 2..=expected_npcs as u16 {
                let fields = raw
                    .get(&npc_id)
                    .unwrap_or_else(|| panic!("{file_name} missing NPC id {npc_id}"));
                for field in fields {
                    let mut idx = 0usize;
                    while idx < field.len() {
                        match field[idx] {
                            TLK_CODE_ACTION_DISPATCH if idx + 1 < field.len() => {
                                let arg = field[idx + 1] & 0x7F;
                                if (b'A'..=b'K').contains(&arg) {
                                    action_arg_counts[usize::from(arg - b'A')] += 1;
                                }
                                idx += 2;
                            }
                            TLK_CODE_GOLD_PAYMENT if idx + 3 < field.len() => {
                                gold_payment_controls += 1;
                                idx += 4;
                            }
                            TLK_CODE_SET_FLAG => {
                                set_flag_controls += 1;
                                idx += 1;
                            }
                            TLK_CODE_ASK_PARTY_NAME => {
                                ask_party_controls += 1;
                                idx += 1;
                            }
                            TLK_CODE_IF_ELSE if idx + 1 < field.len() => {
                                if_else_controls += 1;
                                idx += 2;
                            }
                            TLK_CODE_IF_ELSE_ALT if idx + 2 < field.len() => {
                                if_else_controls += 1;
                                idx += 3;
                            }
                            _ => idx += 1,
                        }
                    }

                    let output = crate::tlk_runner::run_tlk_stream(field, &inputs);
                    surfaced_action_count += output.action_grants.len();
                    surfaced_gold_payments += output
                        .events
                        .iter()
                        .filter(|event| {
                            matches!(event, crate::tlk_runner::TlkRunEvent::GoldPayment { .. })
                        })
                        .count();
                }
            }
        }

        for (letter, description) in [
            (b'A', "food grants"),
            (b'C', "ordinary key grants"),
            (b'F', "Grapple/Klimb gear grants"),
            (b'J', "Black Badge grants"),
            (b'K', "skull/special-key grants"),
        ] {
            let index = usize::from(letter - b'A');
            assert!(
                action_arg_counts[index] > 0,
                "asset TLK corpus should contain public {description}"
            );
        }
        assert!(surfaced_action_count > 0);
        assert!(gold_payment_controls > 0);
        assert!(surfaced_gold_payments > 0);
        assert!(set_flag_controls > 0);
        assert!(if_else_controls > 0);
        assert!(ask_party_controls > 0);
    }

    #[test]
    fn shipped_npc_roster_corpus_matches_public_catalog_counts() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join(TOWNE_NPC_FILENAME).exists() {
            return;
        }

        let corpora = [
            (
                TOWNE_NPC_FILENAME,
                TOWNE_TLK_FILENAME,
                1u8..=8u8,
                107usize,
                47usize,
                (31usize, 1usize, 28usize),
            ),
            (
                DWELLING_NPC_FILENAME,
                DWELLING_TLK_FILENAME,
                9u8..=16u8,
                18usize,
                14usize,
                (3usize, 1usize, 0usize),
            ),
            (
                CASTLE_NPC_FILENAME,
                CASTLE_TLK_FILENAME,
                17u8..=24u8,
                112usize,
                44usize,
                (42usize, 1usize, 25usize),
            ),
            (
                KEEP_NPC_FILENAME,
                KEEP_TLK_FILENAME,
                25u8..=32u8,
                88usize,
                31usize,
                (50usize, 1usize, 6usize),
            ),
        ];
        let expected_tags = [
            0x01, 0x0e, 0x10, 0x11, 0x1b, 0x1e, 0x28, 0x40, 0x44, 0x48, 0x50, 0x54,
            0x58, 0x5c, 0x68, 0x6c, 0x70, 0x78, 0x90, 0x94, 0xb5, 0xb6, 0xb8, 0xd8,
            0xfc,
        ];

        let mut total_occupied = 0usize;
        let mut total_named = 0usize;
        let mut total_dialog_zero = 0usize;
        let mut total_dialog_one = 0usize;
        let mut total_high_special = 0usize;
        let mut distinct_tags = Vec::<u8>::new();

        for (
            npc_file,
            tlk_file,
            scenes,
            expected_occupied,
            expected_named,
            (expected_zero, expected_one, expected_high),
        ) in corpora
        {
            let npc_len = fs::metadata(game_dir.join(npc_file)).unwrap().len() as usize;
            assert_eq!(npc_len, NPC_FILE_LEN, "{npc_file}");

            let tlk = parse_tlk(&game_dir.join(tlk_file)).unwrap();

            let mut family_occupied = 0usize;
            let mut family_named = 0usize;
            let mut family_dialog_zero = 0usize;
            let mut family_dialog_one = 0usize;
            let mut family_high_special = 0usize;

            for scene_byte in scenes {
                let scene = Scene::new(scene_byte).unwrap();
                assert_eq!(npc_roster_filename(scene_byte), Some(npc_file));
                assert_eq!(npc_tlk_filename(scene_byte), Some(tlk_file));

                let slots = parse_npc_block(game_dir, scene, &tlk).unwrap();
                assert_eq!(slots.len(), NPC_SLOTS_PER_SUB_MAP);
                assert_eq!(slots[NPC_SENTINEL_SLOT].slot, NPC_SENTINEL_SLOT);

                for slot in slots.iter().skip(1) {
                    assert!(slot.slot < NPC_SLOTS_PER_SUB_MAP);
                    let occupied = npc_type_byte_occupied(slot.type_byte);
                    assert_eq!(
                        occupied,
                        npc_type_byte_class(slot.type_byte) != NpcTypeByteClass::Empty
                    );
                    if !occupied {
                        continue;
                    }

                    family_occupied += 1;
                    if !distinct_tags.contains(&slot.type_byte) {
                        distinct_tags.push(slot.type_byte);
                    }

                    for waypoint in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
                        let ai = slot.schedule[NPC_SCHEDULE_AI_OFFSET + waypoint];
                        let x = slot.schedule[NPC_SCHEDULE_X_OFFSET + waypoint];
                        let y = slot.schedule[NPC_SCHEDULE_Y_OFFSET + waypoint];
                        let z = slot.schedule[NPC_SCHEDULE_Z_OFFSET + waypoint];
                        assert!(
                            npc_ai_behavior(ai).is_some(),
                            "{npc_file} scene {scene_byte} slot {} waypoint {waypoint} has invalid AI byte {ai}",
                            slot.slot
                        );
                        assert!(
                            x < TOWN_GRID_SIDE as u8 && y < TOWN_GRID_SIDE as u8,
                            "{npc_file} scene {scene_byte} slot {} waypoint {waypoint} has out-of-grid coordinate ({x},{y})",
                            slot.slot
                        );
                        assert!(
                            z <= 7 || z == u8::MAX,
                            "{npc_file} scene {scene_byte} slot {} waypoint {waypoint} has unexpected floor byte {z}",
                            slot.slot
                        );
                    }

                    for boundary in 0..NPC_SCHEDULE_TIME_BOUNDARY_COUNT {
                        let hour = slot.schedule[NPC_SCHEDULE_TIME_OFFSET + boundary];
                        assert!(
                            hour < 24,
                            "{npc_file} scene {scene_byte} slot {} has out-of-day boundary {hour}",
                            slot.slot
                        );
                    }

                    let time = [
                        slot.schedule[NPC_SCHEDULE_TIME_OFFSET],
                        slot.schedule[NPC_SCHEDULE_TIME_OFFSET + 1],
                        slot.schedule[NPC_SCHEDULE_TIME_OFFSET + 2],
                        slot.schedule[NPC_SCHEDULE_TIME_OFFSET + 3],
                    ];
                    for hour in 0..24 {
                        assert!(npc_schedule_waypoint_for_hour(time, hour) < 3);
                    }

                    match npc_dialog_id_kind(slot.dialog_id) {
                        NpcDialogIdKind::NoDialogue => family_dialog_zero += 1,
                        NpcDialogIdKind::TlkHeaderSentinel => family_dialog_one += 1,
                        NpcDialogIdKind::OrdinaryBlobId => {
                            assert!(
                                tlk.contains_key(&(slot.dialog_id as u16)),
                                "{npc_file} scene {scene_byte} slot {} references missing TLK id {}",
                                slot.slot,
                                slot.dialog_id
                            );
                            family_named += 1;
                        }
                        NpcDialogIdKind::HighSpecial => {
                            assert!(
                                slot.dialog_id == NPC_DIALOG_ID_HIGH_FALLBACK
                                    || npc_shop_trigger(slot.dialog_id).is_some()
                                    || (NPC_DIALOG_ID_HIGH_FIRST..=NPC_DIALOG_ID_HIGH_LAST)
                                        .contains(&slot.dialog_id),
                                "{npc_file} scene {scene_byte} slot {} has unexpected high dialog id {}",
                                slot.slot,
                                slot.dialog_id
                            );
                            family_high_special += 1;
                        }
                    }
                }
            }

            assert_eq!(family_occupied, expected_occupied, "{npc_file}");
            assert_eq!(family_named, expected_named, "{npc_file}");
            assert_eq!(family_dialog_zero, expected_zero, "{npc_file}");
            assert_eq!(family_dialog_one, expected_one, "{npc_file}");
            assert_eq!(family_high_special, expected_high, "{npc_file}");

            total_occupied += family_occupied;
            total_named += family_named;
            total_dialog_zero += family_dialog_zero;
            total_dialog_one += family_dialog_one;
            total_high_special += family_high_special;
        }

        distinct_tags.sort_unstable();
        assert_eq!(distinct_tags, expected_tags);
        assert_eq!(total_occupied, 325);
        assert_eq!(total_named, 136);
        assert_eq!(total_dialog_zero, 126);
        assert_eq!(total_dialog_one, 4);
        assert_eq!(total_high_special, 59);
        assert_eq!(
            total_named + total_dialog_zero + total_dialog_one + total_high_special,
            total_occupied
        );
    }

    #[test]
    fn town_talk_dialog_id_one_reads_its_own_blob() {
        // Dialog id 1 is an ordinary NPC with its own blob, not an alias onto
        // the first surviving entry.
        let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
            1,
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
        // Synthetic minimal TLK in the shipped shape: a two-byte count then
        // that many four-byte `(npc id, blob offset)` rows. No sentinel.
        let blob_offset: u16 = 6;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u16.to_le_bytes()); // count
        bytes.extend_from_slice(&0x0042u16.to_le_bytes()); // npc id 0x42
        bytes.extend_from_slice(&blob_offset.to_le_bytes()); // its blob offset
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
        // A zero count is a legal empty file; a count that overruns the file
        // is not.
        assert!(parse_tlk_bytes(&[0, 0]).unwrap().is_empty());
        let mut count_overruns = Vec::new();
        count_overruns.extend_from_slice(&1u16.to_le_bytes());
        count_overruns.extend_from_slice(&1u16.to_le_bytes());
        assert!(parse_tlk_bytes(&count_overruns).is_err());
        assert!(parse_tlk_bytes(&[0]).is_err());

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

        // Shipped header shape: count word, then one (npc id, blob offset)
        // row. No sentinel.
        let mut bytes = vec![0; 6];
        bytes[0..2].copy_from_slice(&1u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&2u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&6u16.to_le_bytes());
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
        assert_eq!(
            state.special_items[SPECIAL_ITEM_SEXTANT_INDEX],
            SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
        );
        assert_eq!(
            state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX],
            SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
        );
        assert_eq!(
            state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX],
            SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
        );
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
        assert_eq!(
            state.special_items[SPECIAL_ITEM_SEXTANT_INDEX],
            SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
        );
        assert_eq!(
            state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX],
            SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
        );
        assert_eq!(
            state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX],
            SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
        );
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
        state.area = Area::Town {
            scene: Scene::new(21).unwrap(),
            floor: 0,
        };
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

        assert!(state.message.contains("The Oaken Oar"));
        assert!(state.active_shop.is_some());
        assert!(!state.message.contains("out of scope"));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn town_raw_tlk_shop_trigger_opens_active_shop_session() {
        let dialogue = HashMap::new();
        let raw = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(21).unwrap(),
            floor: 0,
        };
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
    fn talk_shop_entry_uses_shared_preamble_record_when_shoppe_dat_is_loaded() {
        let dir = debug_game_dir();
        let scene = Scene::new(22).unwrap();
        fs::write(dir.join(format!("{}.TLK", scene.family.stem())), tlk_bytes(&[])).unwrap();
        fs::write(
            dir.join("SHOPPE.DAT"),
            shoppe_dat_with_records(&[(149, b"Guild preamble two.")]),
        )
        .unwrap();

        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town { scene, floor: 0 };
        state.player.facing = Direction::East;
        state.prng_state = (0..=u16::MAX)
            .find(|seed| {
                let mut prng = *seed;
                u5_prng_range_u16(&mut prng, 0, 3) == 1
            })
            .unwrap();
        let mut expected_prng = state.prng_state;
        let _ = u5_prng_range_u16(&mut expected_prng, 0, 3);
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
                dialog_id: 0x86,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Talked
        );

        assert!(state.message.starts_with("Guild preamble two."));
        assert!(state.message.contains("Keys (A), Gems (B), Torches (C)"));
        assert_eq!(state.prng_state, expected_prng);
        assert!(state.active_shop.is_some());
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_raw_tlk_status_tile_filter_runs_before_shop_dispatch() {
        let dialogue = HashMap::new();
        let raw = HashMap::new();

        for (status_tile, expected) in [
            (TALK_STATUS_TILE_SLEEPING, TALK_SLEEPING_MESSAGE),
            (TALK_STATUS_TILE_PRAYING, TALK_NO_RESPONSE_MESSAGE),
        ] {
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
            let object_slot = state.npcs[0]
                .active_object
                .expect("scheduled NPC should link an active object");
            state.active_objects[object_slot].type_byte = 1;
            state.active_objects[object_slot].tile = status_tile;

            assert_eq!(
                state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
                MoveOutcome::Blocked
            );
            assert_eq!(state.message, expected);
            assert!(state.active_shop.is_none());
            assert_eq!(state.turn, 0);
        }
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
    fn town_talk_high_non_shop_dialog_id_uses_funny_look_stub() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0xFF,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "They give thee a funny look.");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn town_raw_tlk_high_non_shop_dialog_id_uses_funny_look_stub() {
        let dialogue = HashMap::new();
        let raw = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0xFF,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "They give thee a funny look.");
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
        assert_eq!(
            state.special_items[SPECIAL_ITEM_SEXTANT_INDEX],
            SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
        );
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
        state.moral_standing = 40;
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
        // This fixture does not pre-seed a toll-progress milestone boundary,
        // so accepted payment debits gold without changing moral standing.
        assert_eq!(state.moral_standing, 40);

        let mut poor_state = state.clone();
        poor_state.gold = 10;
        poor_state.moral_standing = 40;
        poor_state.open_conversation_session(&dialogue, &raw);
        let (text, ended) = poor_state.submit_active_conversation_keyword("pay");
        assert_eq!(text, "");
        assert!(!ended);
        assert_eq!(poor_state.gold, 10);
        let (text, ended) = poor_state.submit_active_conversation_keyword("y");
        assert_eq!(text, "Too poor");
        assert!(!ended);
        assert_eq!(poor_state.gold, 10);
        assert_eq!(poor_state.moral_standing, 40);
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
    fn tlk_numeric_action_dispatch_increments_signal_slots_to_cap() {
        let mut state = test_state(open_grid(), 1, 1);
        state.conversation_signal_flags[5] = TLK_GENERIC_SIGNAL_CAP - 1;
        state.conversation_signal_flags[6] = TLK_GENERIC_SIGNAL_CAP;

        state.record_tlk_signal_flags(&[5, 5, 6, 64]);

        assert_eq!(state.conversation_signal_flags[5], TLK_GENERIC_SIGNAL_CAP);
        assert_eq!(state.conversation_signal_flags[6], TLK_GENERIC_SIGNAL_CAP);
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
        state.conversation_signal_flags[12] = 1;
        state.record_tlk_signal_flags(&[3, 12]);
        let gold_before = state.gold;

        let first = state.run_final_conversation_cleanup().unwrap();
        assert!(first.contains("resource signal"));
        assert_eq!(state.conversation_resource_signals[1], 1);
        assert_eq!(state.conversation_signal_flags[12], 2);
        assert_eq!(state.gold, gold_before);

        state.conversation_resource_signals = [0; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT];
        let second = state.run_final_conversation_cleanup().unwrap();
        assert!(second.contains("Conversation signal 12"));
        assert_eq!(state.conversation_signal_flags[12], 1);
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
        state.area = Area::Town {
            scene: Scene::new(6).unwrap(),
            floor: 0,
        };
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
        let mut grid = open_grid();
        grid[2 * 32 + 1] = 0x05;
        let mut state = test_state(grid, 1, 1);
        state.area = Area::Town {
            scene: Scene::new(20).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 143;
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
        assert!(state.message.contains("143 gold"));

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
    fn horse_trader_purchase_uses_published_adjacent_probe_order_and_skips_occupied() {
        let dialogue = HashMap::new();
        let mut grid = open_grid();
        grid[2 * 32 + 1] = 0x05; // south: first probe, but occupied below
        grid[1] = 0x44; // north: first free accepted marker
        grid[32 + 2] = 0x45; // east: talk target is here
        grid[32] = 0x05; // west: later fallback
        let mut state = test_state(grid, 1, 1);
        state.area = Area::Town {
            scene: Scene::new(20).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 143;
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
        state.active_objects.push(ActiveObject {
            type_byte: 0x42,
            tile: 0x42,
            x: 1,
            y: 2,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.talk_facing_with_dialogue(&dialogue),
            MoveOutcome::Talked
        );
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        let horse = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == HORSE_PARKED_FIRST)
            .copied()
            .expect("horse object was not placed");
        assert_eq!((horse.x, horse.y, horse.z), (1, 0, 0));
    }

    #[test]
    fn horse_trader_purchase_refuses_without_local_marker_and_preserves_gold() {
        let dialogue = HashMap::new();
        let mut grid = open_grid();
        grid[2 * 32 + 1] = 0x01;
        grid[1] = 0x01;
        grid[32 + 2] = 0x01;
        grid[32] = 0x01;
        let mut state = test_state(grid, 1, 1);
        state.area = Area::Town {
            scene: Scene::new(20).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 143;
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
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 143);
        assert!(state.active_shop.is_some());
        assert!(state.message.contains("no room for a horse"));
        assert!(!state
            .active_objects
            .iter()
            .any(|object| object.type_byte == HORSE_PARKED_FIRST));
    }

    #[test]
    fn town_talk_guild_shop_uses_scene_local_prices() {
        let dialogue = HashMap::new();
        let mut grid = open_grid();
        grid[2 * 32 + 1] = 0x05;
        let mut state = test_state(grid, 1, 1);
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
        assert!(state.message.contains("143 gold"));
    }

    #[test]
    fn town_talk_horse_trader_quotes_active_speaker_intelligence() {
        let dialogue = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(20).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = 300;
        state.party_intelligence = vec![30, 10];
        state.active_player = Some(1);
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

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert!(state.message.contains("221 gold"));
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

        // Shipped header shape: count word, then one (npc id, blob offset)
        // row. No sentinel.
        let mut bytes = vec![0; 6];
        bytes[0..2].copy_from_slice(&1u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&2u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&6u16.to_le_bytes());
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
    fn game_dir_talk_uses_published_dictionary_for_tokenized_raw_tlk() {
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

        assert_eq!(
            state.talk_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("the"));
        assert!(!state.message.contains("[w01]"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn game_dir_talk_sidecar_dictionary_overrides_published_token() {
        let dir = debug_game_dir();
        fs::write(dir.join("CASTLE.TLK"), tokenized_tlk_bytes_for_test()).unwrap();
        fs::write(
            dir.join(COMMON_WORD_DICTIONARY_FILE),
            complete_common_word_dictionary_text("custom"),
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

        assert!(state.message.contains("custom"));
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
        state.area = Area::Town {
            scene: Scene::new(2).unwrap(),
            floor: 0,
        };
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
                base_room_rate: 2,
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
        assert!(state.message.contains("Deposit is 33 gold"));
        handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();
        assert!(state.message.contains("party member 2"));
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 67);
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
        assert!(state.message.contains("22 gold"));
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 78);
        assert!(state.inn_registry.is_empty());
        assert_eq!(state.party.len(), 2);
        assert_eq!(state.party[1].status, b'D');
        assert_eq!(state.party[1].hp, 0);
        assert_eq!(state.party_names[1], *b"IOLO\0\0\0\0\0");
        assert!(state.message.contains("has died"));
    }

    #[test]
    fn end_to_end_innkeeper_pickup_bill_uses_stay_units_not_leave_deposit() {
        use crate::shop_runtime::InnkeeperState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.party_intelligence[0] = 25;
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
            stay_counter: 3,
        });
        state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::default()));

        handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
        assert!(state.message.contains("75 gold"));
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 25);
        assert!(state.inn_registry.is_empty());
        assert_eq!(state.party.len(), 2);
        assert!(state.message.contains("Picked up companion 2 for 75 gold"));
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
        handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
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
        handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
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
    fn end_to_end_healer_mission_cure_bypasses_shadowlord_surcharge() {
        use crate::shop_runtime::HealerShopState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(DEFAULT_SHADOWLORD_HIDEOUTS[0]).unwrap(),
            floor: 0,
        };
        state.gold = 50;
        state.party[0].status = b'P';
        state.active_shop = Some(ActiveShopSession::Healer(
            HealerShopState::Greeting,
            Healer::TheHealersMission,
        ));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 50);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.message, "Cured party member 1.");
        assert!(!state.message.contains("Surcharge"));
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
        state.prng_state = 0;
        let expected_prng_state = u5_prng_advance_state(state.prng_state);
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
            Tavern::TheHonestMeal,
        )));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'M', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 78);
        assert_eq!(state.prng_state, expected_prng_state);
        assert!(state.message.contains("served a round for 3 gold"));
        assert!(state.message.contains("Surcharge 19 gold"));
    }

    #[test]
    fn active_shop_surcharge_suppresses_without_zero_shadowlord_sentinel() {
        use crate::shop_runtime::TavernState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.prng_state = 0x1234;
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
            Tavern::TheHonestMeal,
        )));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'M', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 97);
        assert_eq!(state.prng_state, 0x1234);
        assert!(state.message.contains("served a round for 3 gold"));
        assert!(!state.message.contains("Surcharge"));
    }

    #[test]
    fn talk_display_marker_tile_does_not_preempt_real_shop() {
        use crate::shop_session::ActiveShopSession;

        let dialogue = HashMap::new();
        let raw = HashMap::new();
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(21).unwrap(),
            floor: 0,
        };
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
        state.grid[2 * 32 + 1] = 0x05;
        state.active_objects.push(ActiveObject {
            type_byte: 0x05,
            tile: 0x05,
            x: 1,
            y: 2,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.talk_direction_with_dialogue_and_keyword_raw(Direction::East, &dialogue, &raw, None),
            MoveOutcome::Talked
        );

        assert!(state.message.contains("Ship"));
        assert_eq!(state.turn, 1);
        assert!(matches!(
            state.active_shop,
            Some(ActiveShopSession::ShipBroker(_))
        ));
    }

    #[test]
    fn end_to_end_sage_rumour_quotes_confirms_debits_and_renders() {
        use crate::shop_runtime::SageState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 200;
        state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

        handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
        assert_eq!(state.gold, 200);
        assert_eq!(state.message, "That will cost 50 gold. Pay? (Y/N)");
        assert!(state.active_shop.is_some());

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert!(state.gold <= 150);
        assert!(state.message.contains("Malik"));
        assert!(state.message.contains("Moonglow"));
        assert!(state.active_shop.is_none());
    }

    #[test]
    fn end_to_end_tavern_menu_lore_letter_reaches_paid_sage_lookup() {
        use crate::shop_runtime::{SageState, TavernState};
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 200;
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
            Tavern::TheWayfarerTavern,
        )));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert!(state.message.contains("lore"));
        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
        assert!(state.message.contains("tavern branch A"));
        handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();

        assert_eq!(state.message, "Of what wouldst thou hear my lore?");
        assert!(matches!(
            state.active_shop,
            Some(ActiveShopSession::Sage(SageState::Prompt { .. }))
        ));

        handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
        assert_eq!(state.message, "That will cost 50 gold. Pay? (Y/N)");
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert!(state.gold <= 150);
        assert!(state.message.contains("Malik"));
        assert!(state.message.contains("Moonglow"));
        assert!(state.active_shop.is_none());
    }

    #[test]
    fn end_to_end_sage_rumour_prefers_shoppe_record_rendering() {
        use crate::shop_runtime::SageState;
        use crate::shop_session::ActiveShopSession;

        let dir = debug_game_dir();
        let shoppe = shoppe_dat_with_records(&[
            (SAGE_RUMOUR_FEE_QUOTE_RECORD, b"Asset fee: % gold?".as_slice()),
            (85, b"Asset says: seek & below *.".as_slice()),
            (86, b"Asset says: seek & below *.".as_slice()),
            (87, b"Asset says: seek & below *.".as_slice()),
            (88, b"Asset says: seek & below *.".as_slice()),
            (SAGE_RUMOUR_SHORT_FUNDS_RECORD, b"Asset says no credit.".as_slice()),
        ]);
        std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 200;
        state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

        handle_play_key_input(&mut state, 'H', "ONE", &dir).unwrap();
        assert_eq!(state.message, "Asset fee: 50 gold?");
        handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();

        assert!(state.message.contains("Asset says: seek Malik below Moonglow."));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn end_to_end_sage_short_funds_prefers_shoppe_record_rendering() {
        use crate::shop_runtime::SageState;
        use crate::shop_session::ActiveShopSession;

        let dir = debug_game_dir();
        let shoppe = shoppe_dat_with_records(&[
            (SAGE_RUMOUR_FEE_QUOTE_RECORD, b"Asset fee: % gold?".as_slice()),
            (SAGE_RUMOUR_SHORT_FUNDS_RECORD, b"Asset says no credit.".as_slice()),
        ]);
        std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 49;
        state.prng_state = 0x2468;
        state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

        handle_play_key_input(&mut state, 'H', "ONE", &dir).unwrap();
        assert_eq!(state.message, "Asset fee: 50 gold?");
        handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();

        assert_eq!(state.gold, 49);
        assert_eq!(state.prng_state, 0x2468);
        assert_eq!(state.message, "Asset says no credit.");
        assert!(state.active_shop.is_none());
        let _ = fs::remove_dir_all(dir);
    }

    fn shoppe_dat_with_records(records: &[(usize, &[u8])]) -> Vec<u8> {
        assert!(!records.is_empty());
        let payload_len: usize = records.iter().map(|(_, record)| record.len()).sum();
        for (record_id, _) in records {
            assert!(*record_id < SHOPPE_DAT_NONEMPTY_RECORDS);
        }
        let filler_record_count = SHOPPE_DAT_NONEMPTY_RECORDS - 1 - records.len();
        let record_zero_len =
            SHOPPE_DAT_LEN - SHOPPE_DAT_RECORD_SLOTS - filler_record_count - payload_len;
        let mut bytes = Vec::with_capacity(SHOPPE_DAT_LEN);
        bytes.extend(std::iter::repeat_n(b'a', record_zero_len));
        bytes.push(0);
        for id in 1..SHOPPE_DAT_NONEMPTY_RECORDS {
            if let Some((_, record)) = records.iter().find(|(record_id, _)| *record_id == id) {
                bytes.extend_from_slice(record);
            } else {
                bytes.push(b'x');
            }
            bytes.push(0);
        }
        for _ in SHOPPE_DAT_NONEMPTY_RECORDS..SHOPPE_DAT_RECORD_SLOTS {
            bytes.push(0);
        }
        assert_eq!(bytes.len(), SHOPPE_DAT_LEN);
        bytes
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
    fn end_to_end_sage_short_funds_does_not_draw_success_record() {
        use crate::shop_runtime::SageState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 49;
        state.prng_state = 0x2468;
        state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

        handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
        assert!(state.message.contains("50 gold"));
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 49);
        assert_eq!(state.prng_state, 0x2468);
        assert_eq!(state.message, "Beat it!");
        assert!(state.active_shop.is_none());
    }

    #[test]
    fn end_to_end_sage_success_draws_record_after_debit_gate() {
        use crate::shop_runtime::SageState;
        use crate::shop_session::ActiveShopSession;

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 50;
        state.prng_state = 0x2468;
        let expected_prng = u5_prng_advance_state(state.prng_state);
        state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

        handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
        assert_eq!(state.prng_state, 0x2468);
        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 0);
        assert_eq!(state.prng_state, expected_prng);
        assert!(state.message.contains("Malik"));
        assert!(state.message.contains("Moonglow"));
        assert!(state.active_shop.is_none());
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
        assert!(state.message.contains("Mace costs"));
        assert!((0..4).any(|roll| {
            state
                .message
                .contains(crate::shops::arms_buy_confirmation_prompt_for_roll(roll))
        }));

        handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
        assert_eq!(state.equipment_stock[24], 1);
        assert_eq!(state.equipment_stock[1], 0);
        assert!(state.gold < 1000);
        assert!(state.message.contains("Sold!"));
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
        assert!(state.message.contains("a) Short Sword"));
        assert!(matches!(
            state.active_shop,
            Some(ActiveShopSession::ArmsStocked(
                ArmsShopState::BuyPickItem,
                _
            ))
        ));
    }

    #[test]
    fn end_to_end_stocked_arms_shop_escape_exits_buy_and_sell_submenus() {
        use crate::shop_runtime::ArmsShopState;
        use crate::shop_session::ActiveShopSession;
        use crate::shops::ArmsStockTable;

        let mut buy_state = test_state(open_grid(), 1, 1);
        buy_state.gold = 1000;
        buy_state.active_shop = Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::Greeting,
            ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
        ));

        handle_play_key_input(&mut buy_state, 'B', "", Path::new("")).unwrap();
        handle_play_key_input(&mut buy_state, '\x1b', "", Path::new("")).unwrap();
        assert!(buy_state.active_shop.is_none());
        assert_eq!(buy_state.gold, 1000);
        assert!(buy_state.equipment_stock.iter().all(|count| *count == 0));
        assert!(buy_state.message.contains("Farewell"));

        let mut sell_state = test_state(open_grid(), 1, 1);
        sell_state.gold = 1000;
        sell_state.active_shop = Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::Greeting,
            ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
        ));

        handle_play_key_input(&mut sell_state, 'S', "", Path::new("")).unwrap();
        handle_play_key_input(&mut sell_state, ' ', "", Path::new("")).unwrap();
        assert!(sell_state.active_shop.is_none());
        assert_eq!(sell_state.gold, 1000);
        assert!(sell_state.equipment_stock.iter().all(|count| *count == 0));
        assert!(sell_state.message.contains("Farewell"));
    }

    #[test]
    fn end_to_end_stocked_arms_shop_confirmation_ignores_non_yes_no_keys() {
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
        handle_play_key_input(&mut state, 'b', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, 'x', "", Path::new("")).unwrap();

        assert_eq!(state.gold, 1000);
        assert_eq!(state.equipment_stock[24], 0);
        assert!(state.message.contains("Mace costs"));
        assert!(matches!(
            state.active_shop,
            Some(ActiveShopSession::ArmsStocked(
                ArmsShopState::BuyConfirm { item: 24, .. },
                _
            ))
        ));
    }

    #[test]
    fn end_to_end_shipwright_frigate_queues_published_dock_delivery() {
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
                x: 136,
                y: 158,
                skiffs: 2,
            })
        );
    }

    /// `animation.md §6` (spec HEAD `c00bf63`): "These are render
    /// selectors, not map edits; the authored map byte remains the
    /// phase-zero tile id." The grid must never be rewritten for an
    /// animated cell.
    ///
    /// This test used to drive tile `1` (water) and read the change out
    /// of the text view's `~`/`=` glyphs. Both halves are withdrawn:
    /// water does not animate, and every id in every surviving family
    /// shares a text-view glyph with its own frames, so the assertion
    /// reads the resolved selector directly.
    #[test]
    fn render_resolves_static_animation_without_mutating_grid() {
        let mut grid = open_world_grid();
        // 0xD4: first id of the waterfall family.
        grid[world_cell_index(6, 5)] = 0xD4;
        let mut state = britannia_state(grid, 5, 5);
        state.ambient_light = FULL_DAYLIGHT;

        let _ = state.render_text_view(1);
        assert_eq!(state.grid[world_cell_index(6, 5)], 0xD4);
        assert_eq!(state.animation.resolve_static_tile(0xD4), 0xD4);

        for expected in [0xD5u8, 0xD6, 0xD7, 0xD4] {
            state.animation.tick_static_tiles();
            let _ = state.render_text_view(1);
            assert_eq!(
                state.grid[world_cell_index(6, 5)],
                0xD4,
                "the authored map byte must stay the phase-zero tile id"
            );
            assert_eq!(state.animation.resolve_static_tile(0xD4), expected);
        }
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

