    #[test]
    fn town_fire_source_runs_auto_close_before_target_scan() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 3] = 16;
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: 96,
            x: 3,
            y: 1,
            turns_remaining: 1,
        });
        state.record_open_town_door(Scene::new(17).unwrap(), 0, 3, 1);

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], 16);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(Scene::new(17).unwrap(), 0, 3, 1));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("destroyed door tile 96"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_fire_source_ticks_door_tracker_once_on_consumed_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 5] = 16;
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: 96,
            x: 5,
            y: 1,
            turns_remaining: 4,
        });
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

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: 96,
                x: 5,
                y: 1,
                turns_remaining: 3,
            })
        );
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("object tile 192"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_fire_source_destroying_door_clears_unrelated_auto_close_tracker() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 3] = 96;
        grid[32 + 5] = 16;
        let scene = Scene::new(17).unwrap();
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: 97,
            x: 5,
            y: 1,
            turns_remaining: 4,
        });
        state.record_open_town_door(scene, 0, 5, 1);

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], 16);
        assert_eq!(state.grid[32 + 5], 16);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(scene, 0, 3, 1));
        assert!(state.is_recorded_open_town_door(scene, 0, 5, 1));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("destroyed door tile 96"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_fire_source_removes_object_before_farther_door() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 3] = 96;
        let mut state = test_state(grid, 0, 1);
        state.moral_standing = 3;
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

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], 96);
        assert!(state.active_objects[1].is_empty());
        assert!(state.object_at_current_floor(2, 1).is_none());
        assert_eq!(state.moral_standing, 0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("object tile 192"));
        assert!(state.message.contains("moral standing decreased by 3"));
        assert!(!state.message.contains("out of scope"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ship_broadside_depletes_target_aux1_without_destroying_low_result() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::South;
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 100,
            skiffs: 2,
        };
        let target = ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 80,
            aux3: 0,
        };
        state.active_objects.push(target);
        let damage = state.ship_broadside_damage_roll(Direction::East, 1, target);

        assert_eq!(state.fire_ship_broadside(Some(Direction::East)), MoveOutcome::Fired);

        assert!(!state.active_objects[1].is_empty());
        assert_eq!(state.active_objects[1].aux1, 80 - damage);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("durability now"));
        assert!(!state.message.contains("out of scope"));
    }

    #[test]
    fn ship_broadside_clears_target_when_aux1_subtraction_enters_high_bit_range() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::South;
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 100,
            skiffs: 2,
        };
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.fire_ship_broadside(Some(Direction::East)), MoveOutcome::Fired);

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("target destroyed"));
        assert!(!state.message.contains("out of scope"));
    }

    #[test]
    fn whirlpool_engagement_pulls_ship_party_to_underworld_landing() {
        // active-objects.md §8: adjacent whirlpool engagement is a
        // plane-transition effect when the party is not on foot. The party
        // lands at underworld coordinate (34, 18) on foot.
        let dir = debug_game_dir();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 100,
            skiffs: 2,
        };
        state.sync_player_object();
        state.active_objects.push(ActiveObject {
            type_byte: 0xec,
            tile: 0xec,
            x: 5,
            y: 4,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let outcome = state
            .apply_world_whirlpool_engagement(&dir, WorldPlane::Britannia)
            .expect("whirlpool engagement should not error");

        assert_eq!(
            outcome,
            Some(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld,
            })
        );
        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld,
            }
        );
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!((state.player.x, state.player.y), (34, 18));
        assert!(state.message.contains("Whirlpool!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn whirlpool_engagement_no_op_for_on_foot_party() {
        // active-objects.md §8: no-op when the party marker is the ordinary
        // on-foot avatar.
        let dir = debug_game_dir();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();
        // Player is on foot by default in world_state.
        state.active_objects.push(ActiveObject {
            type_byte: 0xec,
            tile: 0xec,
            x: 5,
            y: 4,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let outcome = state
            .apply_world_whirlpool_engagement(&dir, WorldPlane::Britannia)
            .expect("whirlpool engagement should not error");

        assert_eq!(outcome, None);
        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia,
            }
        );
        assert_eq!((state.player.x, state.player.y), (5, 5));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ship_broadside_skips_whirlpool_family_without_depletion() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::South;
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 100,
            skiffs: 2,
        };
        state.active_objects.push(ActiveObject {
            type_byte: 0xec,
            tile: 0xec,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 7,
            aux3: 0,
        });

        assert_eq!(state.fire_ship_broadside(Some(Direction::East)), MoveOutcome::Fired);

        assert_eq!(state.active_objects[1].aux1, 7);
        assert!(!state.active_objects[1].is_empty());
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("no target in range"));
    }

    #[test]
    fn parse_town_pushable_entries_accepts_optional_tile_guard() {
        let entries = parse_town_pushable_entries("CASTLE:0 0 2 1 44\nCASTLE:0 0 3 1\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownPushableEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    expected_tile: Some(44),
                },
                TownPushableEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 3,
                    y: 1,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_pushable_entries("CASTLE:0 0 32 1 44\n").is_err());
        assert!(parse_town_pushable_entries("DUNGEON:0 0 2 1 44\n").is_err());
    }

    #[test]
    fn parse_object_pickup_entries_accepts_world_and_town_rows() {
        let entries = parse_object_pickup_entries(
            "BRITANNIA 0 5 5 GEMS 2 210\nCASTLE:0 0 2 1 KEYS 1\nUNDERWORLD -1 1 2 TORCHES 3\nBRITANNIA 0 6 5 FOOD 4\nBRITANNIA 0 7 5 GOLD 9\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                ObjectPickupEntry {
                    target: PlayTarget::World(WorldPlane::Britannia),
                    floor: 0,
                    x: 5,
                    y: 5,
                    kind: ObjectPickupKind::Gems,
                    amount: 2,
                    expected_tile: Some(210),
                },
                ObjectPickupEntry {
                    target: PlayTarget::Town(Scene::new(17).unwrap()),
                    floor: 0,
                    x: 2,
                    y: 1,
                    kind: ObjectPickupKind::Keys,
                    amount: 1,
                    expected_tile: None,
                },
                ObjectPickupEntry {
                    target: PlayTarget::World(WorldPlane::Underworld),
                    floor: -1,
                    x: 1,
                    y: 2,
                    kind: ObjectPickupKind::Torches,
                    amount: 3,
                    expected_tile: None,
                },
                ObjectPickupEntry {
                    target: PlayTarget::World(WorldPlane::Britannia),
                    floor: 0,
                    x: 6,
                    y: 5,
                    kind: ObjectPickupKind::Food,
                    amount: 4,
                    expected_tile: None,
                },
                ObjectPickupEntry {
                    target: PlayTarget::World(WorldPlane::Britannia),
                    floor: 0,
                    x: 7,
                    y: 5,
                    kind: ObjectPickupKind::Gold,
                    amount: 9,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_object_pickup_entries("DUNGEON:0 0 2 1 GEMS 1\n").is_err());
        assert!(parse_object_pickup_entries("BRITANNIA -1 5 5 GEMS 1\n").is_err());
        assert!(parse_object_pickup_entries("CASTLE:0 0 32 1 KEYS 1\n").is_err());
        assert!(parse_object_pickup_entries("BRITANNIA 0 5 5 GEMS 0\n").is_err());
        assert!(
            parse_object_pickup_entries("BRITANNIA 0 5 5 GEMS 1\nBRITANNIA 0 5 5 KEYS 1\n")
                .is_err()
        );
    }

    #[test]
    fn object_pickup_gold_updates_save_backed_counter_with_saturation() {
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = PARTY_GOLD_CAP - 1;

        state.apply_object_pickup(ObjectPickupKind::Gold, 5);

        assert_eq!(state.gold, PARTY_GOLD_CAP);
    }

    #[test]
    fn world_get_consumes_clean_object_pickup_before_blocking_refusal() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "BRITANNIA 0 5 5 GEMS 2 210\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 5,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.gems, DEFAULT_GEM_STOCK + 2);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got 2 gems"));
        assert!(state.message.contains("active-object tile 210"));
        let overlay = state.world_overlays.get(WorldPlane::Britannia).unwrap();
        assert!(overlay[0].is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_get_consumes_clean_object_pickup_before_blocking_refusal() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "CASTLE:0 0 2 1 KEYS 1 210\n",
        )
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.keys, DEFAULT_KEY_STOCK + 1);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got 1 keys"));
        assert!(state.message.contains("CASTLE:0 floor 0"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_get_table_food_uses_directional_rewrite_without_sidecar() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = 0x9b;
        grid[32 * 3 + 4] = 0x9c;
        let mut state = test_state(grid, 2, 2);
        state.player.facing = Direction::North;
        state.food = 12;
        state.moral_standing = 3;
        state.visibility_dirty = false;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[32 + 2], 0x95);
        assert_eq!(state.food, 13);
        assert_eq!(state.moral_standing, 2);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Ate food from table tile 0x9B"));

        state.player.x = 4;
        state.player.y = 2;
        state.player.facing = Direction::South;
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[32 * 3 + 4], 0x9b);
        assert_eq!(state.food, 14);
        assert_eq!(state.moral_standing, 1);
        assert_eq!(state.turn, 2);
        assert!(state.message.contains("Ate food from table tile 0x9C"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_get_table_food_clamps_to_party_food_cap() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = 0x9b;
        let mut state = test_state(grid, 2, 2);
        state.player.facing = Direction::North;
        state.food = PARTY_FOOD_CAP;
        state.moral_standing = 1;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[32 + 2], 0x95);
        assert_eq!(state.food, PARTY_FOOD_CAP);
        assert_eq!(state.moral_standing, 0);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_get_table_food_invalid_reach_refuses_without_mutation_or_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = 0x9b;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.food = 12;
        state.moral_standing = 3;
        state.visibility_dirty = false;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[32 + 2], 0x9b);
        assert_eq!(state.food, 12);
        assert_eq!(state.moral_standing, 3);
        assert_eq!(state.turn, 0);
        assert!(!state.visibility_dirty);
        assert_eq!(state.message, "The plate cannot be reached.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn object_pickup_tile_guard_mismatch_falls_back_to_blocking_refusal_without_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "BRITANNIA 0 5 5 GEMS 2 211\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 5,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert!(!state.active_objects[1].is_empty());
        assert_eq!(state.gems, DEFAULT_GEM_STOCK);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to get there.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn top_down_g_key_routes_to_object_pickup_sidecar() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "BRITANNIA 0 5 5 TORCHES 3 210\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 5,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert!(
            state
                .handle_top_down_key_with_inline(
                    'G',
                    &dir,
                    Some(Direction::East),
                    None,
                    None,
                    None,
                )
                .unwrap()
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.torches, DEFAULT_TORCH_STOCK + 3);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got 3 torches"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_world_get_tile_entries_accepts_replacement_and_optional_tile_guard() {
        let entries = parse_world_get_tile_entries(
            "UNDERWORLD 2 1 5 55\nBRITANNIA 3 1 5\nBRITANNIA 4 1 5 GOLD 7\nUNDERWORLD 5 1 5 55 FOOD 4\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                WorldGetTileEntry {
                    plane: WorldPlane::Underworld,
                    x: 2,
                    y: 1,
                    replacement_tile: 5,
                    expected_tile: Some(55),
                    grant: None,
                },
                WorldGetTileEntry {
                    plane: WorldPlane::Britannia,
                    x: 3,
                    y: 1,
                    replacement_tile: 5,
                    expected_tile: None,
                    grant: None,
                },
                WorldGetTileEntry {
                    plane: WorldPlane::Britannia,
                    x: 4,
                    y: 1,
                    replacement_tile: 5,
                    expected_tile: None,
                    grant: Some(ObjectPickupGrant {
                        kind: ObjectPickupKind::Gold,
                        amount: 7,
                    }),
                },
                WorldGetTileEntry {
                    plane: WorldPlane::Underworld,
                    x: 5,
                    y: 1,
                    replacement_tile: 5,
                    expected_tile: Some(55),
                    grant: Some(ObjectPickupGrant {
                        kind: ObjectPickupKind::Food,
                        amount: 4,
                    }),
                },
            ]
        );
        assert!(parse_world_get_tile_entries("DUNGEON:0 2 1 5 55\n").is_err());
        assert!(parse_world_get_tile_entries("UNDERWORLD 2 1 55 55\n").is_err());
        assert!(parse_world_get_tile_entries("UNDERWORLD 2 1 5 GOLD 0\n").is_err());
        assert!(parse_world_get_tile_entries("UNDERWORLD 2 1 5 JUNK 1\n").is_err());
        assert!(parse_world_get_tile_entries("UNDERWORLD 2 1 5 55\nUNDER 2 1 5 44\n").is_err());
    }

    #[test]
    fn world_get_wraps_and_rewrites_clean_sidecar_tile() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_GET_TILE_TABLE_FILE),
            "UNDERWORLD 0 0 5 55 GOLD 7\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut state = world_state(grid, 255, 0);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[world_cell_index(0, 0)], 5);
        assert_eq!(state.gold, DEFAULT_GOLD_STOCK + 7);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got world tile 55"));
        assert!(state.message.contains("UNDERWORLD"));
        assert!(state.message.contains("added 7 gold"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_get_food_tile_sidecar_applies_crop_moral_debit() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_GET_TILE_TABLE_FILE),
            "UNDERWORLD 0 0 5 55 FOOD 4\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut state = world_state(grid, 255, 0);
        state.player.facing = Direction::East;
        state.food = 12;
        state.moral_standing = 3;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[world_cell_index(0, 0)], 5);
        assert_eq!(state.food, 16);
        assert_eq!(state.moral_standing, 2);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("added 4 food"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_get_refuses_missing_or_mismatched_sidecar_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut state = world_state(grid, 255, 0);
        state.player.facing = Direction::East;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.turn, 0);

        fs::write(dir.join(WORLD_GET_TILE_TABLE_FILE), "UNDERWORLD 0 0 5 44\n").unwrap();
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.grid[world_cell_index(0, 0)], 55);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to get here.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn top_down_g_key_routes_to_world_get_sidecar() {
        let dir = debug_game_dir();
        fs::write(dir.join(WORLD_GET_TILE_TABLE_FILE), "UNDERWORLD 0 0 5 55\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut state = world_state(grid, 255, 0);
        state.player.facing = Direction::East;

        assert!(
            state
                .handle_top_down_key_with_inline(
                    'G',
                    &dir,
                    Some(Direction::East),
                    None,
                    None,
                    None,
                )
                .unwrap()
        );

        assert_eq!(state.grid[world_cell_index(0, 0)], 5);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got world tile 55"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_town_get_tile_entries_accepts_replacement_and_optional_tile_guard() {
        let entries = parse_town_get_tile_entries(
            "CASTLE:0 0 2 1 16 55\nCASTLE:0 0 3 1 16\nCASTLE:0 0 4 1 16 KEYS 2\nCASTLE:0 0 5 1 16 55 GEMS 3\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                TownGetTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    replacement_tile: 16,
                    expected_tile: Some(55),
                    grant: None,
                },
                TownGetTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 3,
                    y: 1,
                    replacement_tile: 16,
                    expected_tile: None,
                    grant: None,
                },
                TownGetTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 4,
                    y: 1,
                    replacement_tile: 16,
                    expected_tile: None,
                    grant: Some(ObjectPickupGrant {
                        kind: ObjectPickupKind::Keys,
                        amount: 2,
                    }),
                },
                TownGetTileEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 5,
                    y: 1,
                    replacement_tile: 16,
                    expected_tile: Some(55),
                    grant: Some(ObjectPickupGrant {
                        kind: ObjectPickupKind::Gems,
                        amount: 3,
                    }),
                },
            ]
        );
        assert!(parse_town_get_tile_entries("CASTLE:0 0 32 1 16 55\n").is_err());
        assert!(parse_town_get_tile_entries("DUNGEON:0 0 2 1 16 55\n").is_err());
        assert!(parse_town_get_tile_entries("CASTLE:0 0 2 1 55 55\n").is_err());
        assert!(parse_town_get_tile_entries("CASTLE:0 0 2 1 16 KEYS 0\n").is_err());
        assert!(parse_town_get_tile_entries("CASTLE:0 0 2 1 16 JUNK 1\n").is_err());
    }

    #[test]
    fn town_get_uses_clean_sidecar_to_rewrite_facing_tile() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_GET_TILE_TABLE_FILE),
            "CASTLE:0 0 2 1 16 55 KEYS 2\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK + 2);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got tile 55"));
        assert!(state.message.contains("added 2 keys"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_get_food_tile_sidecar_applies_crop_moral_debit() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_GET_TILE_TABLE_FILE),
            "CASTLE:0 0 2 1 16 55 FOOD 1\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.food = PARTY_FOOD_CAP;
        state.moral_standing = 1;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.food, PARTY_FOOD_CAP);
        assert_eq!(state.moral_standing, 0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("added 1 food"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_get_refuses_missing_or_mismatched_sidecar_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.turn, 0);

        fs::write(dir.join(TOWN_GET_TILE_TABLE_FILE), "CASTLE:0 0 2 1 16 44\n").unwrap();
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.grid[32 + 2], 55);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Nothing to get here.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn top_down_g_key_routes_to_town_get_sidecar() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_GET_TILE_TABLE_FILE), "CASTLE:0 0 2 1 16 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert!(
            state
                .handle_top_down_key_with_inline(
                    'G',
                    &dir,
                    Some(Direction::East),
                    None,
                    None,
                    None,
                )
                .unwrap()
        );

        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got tile 55"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_search_live_tile_fallback_reports_published_location_prefixes() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa6;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_secret(&[], None),
            MoveOutcome::Blocked
        );

        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Searched barrel; nothing found.");
    }

    #[test]
    fn town_search_generic_find_marker_skips_moonstone_scan() {
        let mut grid = open_grid();
        grid[32 + 2] = 0xdc;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.moonstone_slots[0] = MoonstoneGateSlot {
            scene: 0x11,
            x: 2,
            y: 1,
            z: 0,
        };

        assert_eq!(
            state.search_facing_secret(&[], None),
            MoveOutcome::Blocked
        );

        assert_eq!(state.turn, 0);
        assert_eq!(state.active_objects.len(), 1);
        assert_eq!(
            state.message,
            "Searched a generic find marker; no Moonstone scan was attempted."
        );
    }

    #[test]
    fn search_world_moonstone_surfaces_highest_matching_phase_and_get_invalidates_slot() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 5,
            y: 5,
            z: 0,
        };
        state.moonstone_slots[3] = MoonstoneGateSlot {
            scene: 0,
            x: 5,
            y: 5,
            z: 0,
        };

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Found a strange rock for Moonstone phase 4.");
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(
            state.active_objects[1],
            ActiveObject::moonstone_pickup(3, 5, 5, WorldPlane::Britannia.save_floor())
        );

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(state.turn, 1);
        assert_eq!(
            state.message,
            "Moonstone phase 4 is already surfaced as a strange rock."
        );

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.moonstone_slots[1].scene, 0);
        assert_eq!(state.moonstone_slots[3], MoonstoneGateSlot::invalid());
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
        assert_eq!(
            state.message,
            "Recovered Moonstone phase 4; Gate Travel slot cleared."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_world_rare_reagent_harvest_requires_midnight_and_daily_cookie() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 181, 54);
        state.player.facing = Direction::East;
        state.clock = GameClock::with_date(139, 4, 5, 0, 17).unwrap();
        state.reagents[REAGENT_MANDRAKE] = 98;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.reagents[REAGENT_MANDRAKE], 99);
        assert_eq!(state.rare_reagent_harvest_days[0], 5);
        assert!(state.message.contains("sprigs of Mandrake Root"));

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.reagents[REAGENT_MANDRAKE], 99);
        assert_eq!(state.message, "Nothing to search here.");

        state.clock = GameClock::with_date(139, 4, 6, 1, 0).unwrap();
        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.rare_reagent_harvest_days[0], 5);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_world_rare_reagent_harvest_uses_fixed_nightshade_point() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 44, 136);
        state.player.facing = Direction::South;
        state.clock = GameClock::with_date(139, 4, 5, 0, 0).unwrap();
        state.reagents[REAGENT_NIGHTSHADE] = 0;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert!((2..=15).contains(&state.reagents[REAGENT_NIGHTSHADE]));
        assert_eq!(state.rare_reagent_harvest_days[2], 5);
        assert!(state.message.contains("sprigs of Nightshade"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_fixed_hidden_treasure_stages_pickup_and_get_grants_inventory() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 4, 8);
        state.area = Area::Town {
            scene: Scene::new(1).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gems = 0;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.turn, 1);
        assert!(state.fixed_hidden_treasure_found(18));
        assert_eq!(state.message, "Found gem.");
        assert_eq!(
            state.active_objects[1],
            ActiveObject::fixed_hidden_treasure_pickup(18, 5, 8, 0)
        );

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.gems, 1);
        assert!(state.active_objects[1].is_empty());
        assert!(state.message.contains("added 1 gems"));

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.turn, 2);
        assert_eq!(state.message, "No secret door found.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_hidden_treasure_table_covers_all_spec_records() {
        assert_eq!(
            PlayState::fixed_hidden_treasure_table_len(),
            FIXED_HIDDEN_TREASURE_COUNT
        );
        assert!(PlayState::fixed_hidden_treasure_table_records_are_sequential());
    }

    #[test]
    fn search_fixed_hidden_treasure_skips_found_duplicate_coordinates() {
        let dir = debug_game_dir();
        let mut state = world_state(open_world_grid(), 232, 233);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(state.fixed_hidden_treasure_found(0));
        assert_eq!(state.active_objects[1].fixed_hidden_treasure_record(), Some(0));

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.equipment_stock[15], 1);

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(state.fixed_hidden_treasure_found(1));
        assert_eq!(state.active_objects[1].fixed_hidden_treasure_record(), Some(1));
        assert_eq!(state.message, "Found weapon.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_fixed_hidden_treasure_reaches_final_duplicate_record() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 11, 12);
        state.area = Area::Town {
            scene: Scene::new(17).unwrap(),
            floor: 2,
        };
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(state.fixed_hidden_treasure_found(111));
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.potion_stock[6], 1);

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(state.fixed_hidden_treasure_found(112));
        assert_eq!(
            state.active_objects[1].fixed_hidden_treasure_record(),
            Some(112)
        );
        assert_eq!(state.message, "Found scroll.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_fixed_hidden_daily_cache_resets_by_day_and_caps_keys() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 2);
        state.area = Area::Town {
            scene: Scene::new(5).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.clock = GameClock::with_date(139, 4, 5, 12, 0).unwrap();
        state.keys = 98;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(state.fixed_hidden_treasure_daily_day, 5);

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.keys, 99);

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.fixed_hidden_treasure_daily_day, 5);

        state.clock = GameClock::with_date(139, 4, 6, 12, 0).unwrap();
        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(state.fixed_hidden_treasure_daily_day, 6);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_hidden_treasure_food_clamps_to_party_food_cap() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 18, 24);
        state.area = Area::Town {
            scene: Scene::new(1).unwrap(),
            floor: 1,
        };
        state.player.facing = Direction::East;
        state.food = PARTY_FOOD_CAP - 1;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.food, PARTY_FOOD_CAP);
        assert!(state.message.contains("added 10 food"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn get_fixed_hidden_treasure_grants_equipment_stock() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 0, 15);
        state.area = Area::Town {
            scene: Scene::new(23).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.equipment_stock[47], 1);
        assert!(state.message.contains("equipment id 47"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_town_moonstone_uses_scene_floor_and_surface_get_clears_slot() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.moonstone_slots[0] = MoonstoneGateSlot {
            scene: 0x11,
            x: 2,
            y: 1,
            z: 0,
        };

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(
            state.active_objects[1],
            ActiveObject::moonstone_pickup(0, 2, 1, 0)
        );
        assert_eq!(state.message, "Found a strange rock for Moonstone phase 1.");

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.moonstone_slots[0], MoonstoneGateSlot::invalid());
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(
            state.message,
            "Recovered Moonstone phase 1; Gate Travel slot cleared."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn use_moonstone_world_records_gate_slot_and_can_drive_gate_travel() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = 9;
        state.party[0].level = 8;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "3", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.moonstone_slots[2],
            MoonstoneGateSlot {
                scene: 0,
                x: 4,
                y: 5,
                z: 0,
            }
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(
            state.message,
            "Buried Moonstone phase 3 at BRITANNIA (4, 5)."
        );

        state.player.x = 8;
        state.player.y = 9;
        state.sync_player_object();

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1PRV3", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (4, 5));
        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 1);
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
        assert_eq!(state.message, "Gate Travel phase 3 -> BRITANNIA at (4, 5).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn natural_moongate_refresh_stamps_and_wanes_world_slots() {
        let gate_idx = world_cell_index(6, 7);
        let stale_idx = world_cell_index(9, 10);
        let unrelated_idx = world_cell_index(11, 10);
        let mut grid = open_world_grid();
        grid[stale_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        grid[unrelated_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 4, 5);
        state.clock = GameClock::new(20, 0).unwrap();
        state.natural_moongate_live_cells.push(stale_idx);
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: WorldPlane::Britannia.save_floor() as u8,
        };

        assert!(state.refresh_natural_moongates());

        assert_eq!(state.natural_moongate_counter, 1);
        assert_eq!(state.grid[gate_idx], NATURAL_MOONGATE_TERRAIN_TILE);
        assert_eq!(state.grid[stale_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert_eq!(state.grid[unrelated_idx], NATURAL_MOONGATE_TERRAIN_TILE);
        assert_eq!(state.natural_moongate_live_cells, vec![gate_idx]);
        assert!(state.visibility_dirty);

        state.visibility_dirty = false;
        state.clock = GameClock::new(12, 0).unwrap();

        assert!(state.refresh_natural_moongates());

        assert_eq!(state.natural_moongate_counter, 0);
        assert_eq!(state.grid[gate_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert!(state.natural_moongate_live_cells.is_empty());
        assert!(state.visibility_dirty);
    }

    #[test]
    fn natural_moongate_refresh_uses_wrapping_world_chunk_window() {
        let wrapped_idx = world_cell_index(250, 250);
        let near_zero_idx = world_cell_index(0, 0);
        let outside_idx = world_cell_index(32, 32);
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.clock = GameClock::new(20, 0).unwrap();
        state.moonstone_slots[0] = MoonstoneGateSlot {
            scene: 0,
            x: 250,
            y: 250,
            z: WorldPlane::Britannia.save_floor() as u8,
        };
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 0,
            y: 0,
            z: WorldPlane::Britannia.save_floor() as u8,
        };
        state.moonstone_slots[2] = MoonstoneGateSlot {
            scene: 0,
            x: 32,
            y: 32,
            z: WorldPlane::Britannia.save_floor() as u8,
        };

        assert_eq!(state.natural_moongate_chunk_window(), Some((240, 240, 32, 32)));
        assert!(state.refresh_natural_moongates());

        assert_eq!(state.grid[wrapped_idx], NATURAL_MOONGATE_TERRAIN_TILE);
        assert_eq!(state.grid[near_zero_idx], NATURAL_MOONGATE_TERRAIN_TILE);
        assert_eq!(state.grid[outside_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert_eq!(
            state.natural_moongate_live_cells,
            vec![wrapped_idx, near_zero_idx]
        );
    }

    #[test]
    fn natural_moongate_refresh_stamps_town_slots_on_matching_floor() {
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::new(23, 0).unwrap();
        state.moonstone_slots[0] = MoonstoneGateSlot {
            scene: 0x11,
            x: 2,
            y: 1,
            z: 0,
        };
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0x11,
            x: 3,
            y: 1,
            z: 1,
        };

        assert!(state.refresh_natural_moongates());

        assert_eq!(state.grid[1 * 32 + 2], NATURAL_MOONGATE_TERRAIN_TILE);
        assert_eq!(state.grid[1 * 32 + 3], 16);
        assert_eq!(state.natural_moongate_counter, 1);
    }

    #[test]
    fn natural_moongate_entry_uses_cached_moon_slot_without_spending_turn() {
        let dir = debug_game_dir();
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(11, 58).unwrap();
        state.set_cached_moon_glyph_slots(Some(1), None);
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: WorldPlane::Britannia.save_floor() as u8,
        };

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (6, 7));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(11, 58).unwrap());
        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert_eq!(state.message, "Gate Travel phase 2 -> BRITANNIA at (6, 7).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn natural_moongate_entry_uses_second_cached_moon_slot_after_noon() {
        let dir = debug_game_dir();
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(12, 0).unwrap();
        state.set_cached_moon_glyph_slots(Some(1), Some(2));
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: WorldPlane::Britannia.save_floor() as u8,
        };
        state.moonstone_slots[2] = MoonstoneGateSlot {
            scene: 0,
            x: 8,
            y: 9,
            z: WorldPlane::Britannia.save_floor() as u8,
        };

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (8, 9));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert_eq!(state.message, "Gate Travel phase 3 -> BRITANNIA at (8, 9).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn natural_moongate_entry_clears_tile_and_reports_missing_glyph_cache() {
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(11, 58).unwrap();
        state.natural_moongate_live_cells.push(origin_idx);

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert!(state.natural_moongate_live_cells.is_empty());
        assert!(state.visibility_dirty);
        assert_eq!(
            state.message,
            "Natural moongate moon-glyph cache is unavailable."
        );
    }

    #[test]
    fn natural_moongate_midnight_window_starts_shrine_prompt_after_clearing_tile() {
        let dir = debug_game_dir();
        let origin_idx = world_cell_index(5, 5);
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 5 5 HONESTY 5\n").unwrap();
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(0, 9).unwrap();

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert!(state.active_shrine.is_some());
        assert!(state.message.contains("Shrine of Honesty mantra?"));
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn natural_moongate_midnight_window_reads_codex_urn_before_shrine_prompt() {
        let dir = debug_game_dir();
        let origin_idx = world_cell_index(5, 5);
        fs::write(dir.join(CODEX_URN_TABLE_FILE), "BRITANNIA 5 5 5\n").unwrap();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 5 5 HONESTY 5\n").unwrap();
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(0, 0).unwrap();
        state.shrine_ordained_mask = ShrineVirtue::Justice.bit();

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert!(state.active_shrine.is_none());
        assert_eq!(state.shrine_codex_mask, ShrineVirtue::Justice.bit());
        assert!(state.message.contains("Codex page for Justice"));
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn natural_moongate_midnight_window_reports_meditation_without_sidecar_match() {
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(0, 9).unwrap();

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert_eq!(
            state.message,
            "Natural moongate opened the shrine meditation path."
        );
    }

    #[test]
    fn use_moonstone_town_records_scene_floor_and_clears_stale_pickup() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = 5;
        let mut state = test_state(grid, 1, 1);
        state.moonstone_slots[7] = MoonstoneGateSlot {
            scene: 0x11,
            x: 2,
            y: 1,
            z: 0,
        };
        state
            .active_objects
            .push(ActiveObject::moonstone_pickup(7, 2, 1, 0));
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "8", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.moonstone_slots[7],
            MoonstoneGateSlot {
                scene: 0x11,
                x: 1,
                y: 1,
                z: 0,
            }
        );
        assert!(state.active_objects[1].is_empty());
        assert!(state.visibility_dirty);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(
            state.message,
            "Buried Moonstone phase 8 at CASTLE:0 (1, 1)."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn use_moonstone_rejects_missing_phase_bad_tile_and_dungeon_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(4, 5)] = 16;
        let mut state = britannia_state(grid, 4, 5);

        assert_eq!(state.use_item_command(None, Some(&dir)).unwrap(), MoveOutcome::Blocked);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, use_prompt_message());

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "1", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.moonstone_slots[0], MoonstoneGateSlot::invalid());
        assert_eq!(state.message, "Cannot bury Moonstone on tile 16.");

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        assert!(dungeon.handle_dungeon_key('U', &dir).unwrap());
        assert_eq!(dungeon.turn, 0);
        assert_eq!(dungeon.message, "No usable items.");
        assert_eq!(
            handle_play_key_input(&mut dungeon, 'U', "1", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(dungeon.turn, 0);
        assert_eq!(dungeon.message, "Not here!");
        let _ = fs::remove_dir_all(dir);
    }

