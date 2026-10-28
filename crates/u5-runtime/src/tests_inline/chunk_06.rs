    #[test]
    fn town_fire_source_runs_auto_close_before_target_scan() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_FIRE_SOURCE_TABLE_FILE),
            "CASTLE:0 0 1 1 EAST\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 3] = TOWN_DOOR_CLEARED_TILE;
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
            x: 3,
            y: 1,
            turns_remaining: 1,
        });
        state.record_open_town_door(Scene::new(17).unwrap(), 0, 3, 1);

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(Scene::new(17).unwrap(), 0, 3, 1));
        assert_eq!(state.turn, 1);
        assert!(state.diagnostics.iter().any(|note| note.contains("destroyed door tile 184")));
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
        grid[32 + 5] = TOWN_DOOR_CLEARED_TILE;
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
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
                previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                x: 5,
                y: 1,
                turns_remaining: 3,
            })
        );
        assert_eq!(state.turn, 1);
        assert!(state.diagnostics.iter().any(|note| note.contains("object tile 192")));
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
        grid[32 + 3] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
        grid[32 + 5] = TOWN_DOOR_CLEARED_TILE;
        let scene = Scene::new(17).unwrap();
        let mut state = test_state(grid, 0, 1);
        state.door_tracker = Some(DoorTracker {
            previous_tile: TOWN_DOOR_WINDOWED_UNLOCKED_TILE,
            x: 5,
            y: 1,
            turns_remaining: 4,
        });
        state.record_open_town_door(scene, 0, 5, 1);

        assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);

        assert_eq!(state.grid[32 + 3], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.grid[32 + 5], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(scene, 0, 3, 1));
        assert!(state.is_recorded_open_town_door(scene, 0, 5, 1));
        assert_eq!(state.turn, 1);
        assert!(state.diagnostics.iter().any(|note| note.contains("destroyed door tile 184")));
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
        assert!(state.diagnostics.iter().any(|note| note.contains("object tile 192")));
        assert!(state.diagnostics.iter().any(|note| note.contains("moral standing -3")));
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
        state.prng_state = 0;
        let mut expected_prng = state.prng_state;
        let damage = u5_prng_range_u16(
            &mut expected_prng,
            SHIP_BROADSIDE_DAMAGE_MIN.into(),
            SHIP_BROADSIDE_DAMAGE_MAX.into(),
        ) as u8;

        assert_eq!(state.fire_ship_broadside(Some(Direction::East)), MoveOutcome::Fired);

        assert!(!state.active_objects[1].is_empty());
        assert_eq!(state.active_objects[1].aux1, 80 - damage);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(!state.message.contains("BOOOM"));
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
        assert!(!state.message.contains("BOOOM"));
        assert!(!state.message.contains("out of scope"));
    }

    #[test]
    fn whirlpool_engagement_pulls_ship_party_to_underworld_landing() {
        // active-objects.md §8: adjacent whirlpool engagement is a
        // plane-transition effect when the party is not on foot. The party
        // lands at underworld coordinate (34, 18) still aboard: `vehicles.md
        // §2` lists no whirlpool writer of the durable marker.
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
        let prng_before = state.prng_state;
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
            Some(MoveOutcome::Transition(
                AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld,
                },
            ))
        );
        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld,
            }
        );
        assert!(
            matches!(
                state.player.transport,
                TransportState::Ship { type_byte: 168, skiffs: 2, hull, .. } if hull < 100
            ),
            "the frigate survives the swallow with its hull roll applied: {:?}",
            state.player.transport
        );
        assert_eq!((state.player.x, state.player.y), (34, 18));
        // `overworld.md §8.1`: the banner is the first and only text on the
        // path; the coordinate narration is gone.
        let lines: Vec<&str> = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        assert!(lines.contains(&"WHIRLPOOL!"));
        assert!(!state.message.contains("Sucked into the underworld"));
        assert_ne!(state.prng_state, prng_before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn whirlpool_engagement_damages_on_foot_party_without_transition() {
        // The old on-foot no-op is withdrawn. Foot reaches the whole-party
        // impact pass, but skips the transition and keeps the whirlpool slot.
        let dir = debug_game_dir();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();
        state.party = six_member_party(40);
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

        assert_eq!(outcome, Some(MoveOutcome::Used));
        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia,
            }
        );
        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert!(state.party.iter().all(|member| member.hp < 40));
        assert_eq!(state.active_objects[1].type_byte, 0xec);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn post_turn_adjacent_sand_trap_runs_silent_shared_impact_payload() {
        // `active-objects.md §8`: `0xE0..=0xE3` is the Sand Trap run. Its
        // adjacent arm is not combat, prints no narration, keeps the slot,
        // and reaches the same transport-dependent payload as ranged attacks.
        let dir = debug_game_dir();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();
        state.party = six_member_party(40);
        state.message = String::new();
        state.turn = 1;
        state.active_objects.push(ActiveObject {
            type_byte: 0xe0,
            tile: 0xe0,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.pending_outdoor_reaction_slots.push(1);
        // `encounters.md §2.1`: the three effect gates "sit ahead of the
        // encounter probe *and* ahead of the outdoor creature walker". The
        // walker is the only thing that queues a pending reaction slot, so a
        // unit test that seeds one by hand has to seed the same turn's gate
        // decision with it; production reaches both through the clock.
        state.world_walkers_ran_this_turn = true;

        let outcome = state
            .apply_world_post_turn_effects_after_turn(0, &dir)
            .expect("sand-trap reaction should not require optional data");

        assert_eq!(outcome, Some(MoveOutcome::Used));
        assert!(state.message.is_empty());
        assert!(state.party.iter().all(|member| member.hp < 40));
        assert_eq!(state.active_objects[1].type_byte, 0xe0);
        let _ = fs::remove_dir_all(dir);
    }

    /// rc3 residual: a post-turn whirlpool swallow that ends the turn in a
    /// plane transition keeps the command's result and the effect's lines
    /// as separate rows (`overworld.md §8.1`, `text-output.md §11`), never
    /// one space-joined sentence in the slot.
    #[test]
    fn post_turn_whirlpool_swallow_keeps_command_and_banner_on_separate_rows() {
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
        state.party = six_member_party(40);
        state.turn = 1;
        state.message = "South".to_string();
        state.message_flushed.clear();
        state.message_row_open_mid_line = true;
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
        state.pending_outdoor_reaction_slots.push(1);
        state.world_walkers_ran_this_turn = true;

        let outcome = state
            .apply_world_post_turn_effects_after_turn(0, &dir)
            .expect("whirlpool reaction should not require optional data");

        assert!(matches!(outcome, Some(MoveOutcome::Transition(_))));
        assert!(
            !state.message.starts_with("South "),
            "the effect was joined onto the command's row: {:?}",
            state.message
        );
        let lines: Vec<&str> = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        let south = lines.iter().position(|line| *line == "South");
        let banner = lines.iter().position(|line| *line == "WHIRLPOOL!");
        assert!(
            matches!((south, banner), (Some(s), Some(b)) if s < b),
            "command row then banner row: {lines:?}"
        );
        assert_eq!(lines.iter().filter(|line| **line == "South").count(), 1);
        // Nothing is left in the slot for a later flush to print again.
        assert!(!state.flush_message_slot());
        let _ = fs::remove_dir_all(dir);
    }

    /// The silent arm: an effect that prints nothing (or only clears the
    /// slot) leaves the command's row and the row state the command set.
    #[test]
    fn a_silent_post_turn_transition_keeps_the_command_row_and_its_row_state() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.message.clear();
        state.message_row_open_mid_line = false;
        state.keep_post_turn_effect_rows_separate("Klimb-", true);
        assert_eq!(state.message, "Klimb-");
        assert!(state.message_row_open_mid_line);

        state.message_row_open_mid_line = false;
        state.keep_post_turn_effect_rows_separate("Klimb-", true);
        assert_eq!(state.message, "Klimb-");
        assert!(state.message_row_open_mid_line);
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
        assert!(state.diagnostics.iter().any(|line| line.contains("with no target")));
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
        assert!(state.diagnostics.iter().any(|note| note.contains("Got 2 gems") || note.contains("got 2 gems")));
        assert!(state.diagnostics.iter().any(|note| note.contains("active-object tile 210") || note.contains("active-object tile 210")));
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
        assert!(state.diagnostics.iter().any(|note| note.contains("Got 1 keys") || note.contains("got 1 keys")));
        assert!(state.diagnostics.iter().any(|note| note.contains("CASTLE:0 floor 0") || note.contains("cASTLE:0 floor 0")));
        let _ = fs::remove_dir_all(dir);
    }

    /// `RETRACTIONS.md` R477: "Surface Search's object branch accepts **only**
    /// the container class and hands it to the trap narrator, which contains no
    /// inventory grant at all. Search grants nothing from the object table."
    /// This test used to assert the opposite - that Search consumed the record
    /// and credited the stock ahead of the live-tile scans.
    #[test]
    fn town_search_grants_nothing_from_the_object_table() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "CASTLE:0 0 2 1 KEYS 1 210\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 0x4e;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 2,
            y: 1,
            z: 0,
            phase: 0x34,
            aux1: 0,
            aux3: 0x78,
        });
        let keys_before = state.keys;

        // The container class reaches the trap narrator, which grants nothing.
        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        // The record is untouched and nothing is credited.
        assert_eq!(state.active_objects[1].type_byte, 210);
        assert_eq!(state.active_objects[1].tile, 210);
        assert_eq!(state.keys, keys_before);

        // A following Get is what grants, and it is the Get path that prints.
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.keys, keys_before + 1);
        assert_eq!(state.message, "\n1 key!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_get_native_object_pickup_uses_visual_filter_and_class_code_without_sidecar() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: 0x08,
            tile: GETTABLE_LOOSE_OBJECT_VISUAL_FIRST + 2,
            x: 5,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 4,
            aux3: 0,
        });

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.gems, DEFAULT_GEM_STOCK + 4);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "\n4 gems!");
        let _ = fs::remove_dir_all(dir);
    }

    /// `RETRACTIONS.md` R476: "There is one byte. The record's inventory
    /// class byte is both the acceptance test and the grant dispatcher's
    /// selector." The separate visual filter is withdrawn, so the handler
    /// takes the record it finds at the cell and lets its class decide -
    /// it does not look past a record whose class refuses to one underneath.
    /// This test used to assert the opposite.
    #[test]
    fn town_get_stops_at_the_first_record_whose_class_refuses() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
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
        state.active_objects.push(ActiveObject {
            type_byte: 0x02,
            tile: GETTABLE_LOOSE_OBJECT_VISUAL_FIRST,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 7,
            aux3: 0,
        });
        let gold_before = state.gold;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        // Class `0xc0` reaches the dispatcher's default arm, so nothing is
        // consumed and nothing is credited - the gold under it stays put.
        assert_eq!(state.message, GET_NOTHING_REFUSAL);
        assert!(!state.active_objects[1].is_empty());
        assert!(!state.active_objects[2].is_empty());
        assert_eq!(state.gold, gold_before);
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn native_object_pickup_rejects_unknown_inventory_class_without_clearing_slot() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 0xff,
            tile: GETTABLE_LOOSE_OBJECT_VISUAL_FIRST,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert!(!state.active_objects[1].is_empty());
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, GET_NOTHING_REFUSAL);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn native_object_pickup_sets_sandalwood_box_story_flag() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 0;
        state.active_objects.push(ActiveObject {
            type_byte: 0x0e,
            tile: GETTABLE_LOOSE_OBJECT_VISUAL_FIRST,
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
        // `inventory.md §4.5` "Which U-Use rows omit quantity": a normally
        // acquired Wooden Box is an uncounted row, so the grant writes the
        // no-quantity marker 255, not one (playtest P13-09).
        assert_eq!(
            state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX],
            crate::commands::USE_PICKER_NO_QUANTITY
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "\nA sandalwood box!");
        let _ = fs::remove_dir_all(dir);
    }

    /// `commands.md §5.8` "Eating and borrowing are Get cases, not movement
    /// cases": the Get-tile cascade's two published granting arms and the
    /// laden-table arms, each with the row it prints after the leading feed.
    #[test]
    fn the_published_get_tile_cascade_prints_its_own_rows() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_GET_TILE_TABLE_FILE),
            &format!(
                "CASTLE:0 0 2 1 {} {} FOOD 1\nCASTLE:0 0 2 3 {} {}\n",
                GET_TILE_CROPS_REPLACEMENT,
                GET_TILE_CROPS_SOURCE,
                GET_TILE_WALL_TORCH_REPLACEMENT,
                GET_TILE_WALL_TORCH_SOURCE_FIRST,
            ),
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = GET_TILE_CROPS_SOURCE;
        grid[32 * 3 + 2] = GET_TILE_WALL_TORCH_SOURCE_FIRST;

        // | Crops `0x2D`, any direction | `Crops picked!` | `0x2C` |
        let mut state = test_state(grid.clone(), 1, 1);
        state.player.facing = Direction::East;
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.message, GET_TILE_CROPS_PICKED_LINE);
        assert_eq!(state.grid[32 + 2], GET_TILE_CROPS_REPLACEMENT);

        // | Wall torch `0xB0` or `0xB1`, any direction | `Borrowed!` | `0x44` |
        let mut state = test_state(grid, 1, 3);
        state.player.facing = Direction::East;
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.message, GET_TILE_BORROWED_LINE);
        assert_eq!(state.grid[32 * 3 + 2], GET_TILE_WALL_TORCH_REPLACEMENT);

        // | every other tile | `Nothing to get!` | unchanged |
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, GET_NOTHING_REFUSAL);
        let _ = fs::remove_dir_all(dir);
    }

    /// `commands.md §5.8` / `containers.md §7` (audit A056): crops and the
    /// wall-torch pair are native tile cases. They answer from any direction
    /// on any map with no per-coordinate sidecar row present.
    #[test]
    fn crops_and_wall_torches_are_native_get_tile_cases() {
        let dir = debug_game_dir();
        let _ = fs::remove_file(dir.join(TOWN_GET_TILE_TABLE_FILE));
        let mut grid = open_grid();
        grid[32 + 2] = GET_TILE_CROPS_SOURCE;
        grid[32 * 3 + 2] = GET_TILE_WALL_TORCH_SOURCE_SECOND;

        let mut state = test_state(grid.clone(), 2, 2);
        state.player.facing = Direction::North;
        state.food = 9999;
        state.moral_standing = 5;
        assert_eq!(state.get_facing_with_game_dir(&dir).unwrap(), MoveOutcome::Got);
        assert_eq!(state.message, GET_TILE_CROPS_PICKED_LINE);
        assert_eq!(state.grid[32 + 2], GET_TILE_CROPS_REPLACEMENT);
        assert_eq!(state.food, 9999, "the 9999 cap");
        assert!(state.moral_standing < 5, "crop pick debits moral standing");
        // `stats-panel.md §2.2.1` row 22: a crop Get is a deferred site.
        assert_eq!(
            state.stats_panel_command_refresh,
            crate::stats_panel::PanelRefreshMode::Deferred
        );

        let mut state = test_state(grid, 2, 2);
        state.player.facing = Direction::South;
        state.moral_standing = 5;
        assert_eq!(state.get_facing_with_game_dir(&dir).unwrap(), MoveOutcome::Got);
        assert_eq!(state.message, GET_TILE_BORROWED_LINE);
        assert_eq!(state.grid[32 * 3 + 2], GET_TILE_WALL_TORCH_REPLACEMENT);
        assert_eq!(state.moral_standing, 5, "no debit for the borrow");
        // `lighting.md §5`: the borrow turn's cleanup ages the new counter (Q09).
        assert_eq!(state.torch_counter, BORROWED_FIXTURE_TORCH_DURATION - 1);

        let mut world_grid = open_world_grid();
        world_grid[world_cell_index(6, 5)] = GET_TILE_CROPS_SOURCE;
        let mut state = world_state(world_grid, 5, 5);
        state.player.facing = Direction::East;
        assert_eq!(state.get_facing_with_game_dir(&dir).unwrap(), MoveOutcome::Got);
        assert_eq!(state.message, GET_TILE_CROPS_PICKED_LINE);
        assert_eq!(state.grid[world_cell_index(6, 5)], GET_TILE_CROPS_REPLACEMENT);
        let _ = fs::remove_dir_all(dir);
    }

    /// `commands.md §3` and §7 (audit A054): `G`, `S`, `O` and `J` report
    /// the default "acted" on every exit - a miss and a cancel included -
    /// so the per-turn epilogue runs.
    #[test]
    fn tile_commands_report_acted_on_misses_and_cancels() {
        let dir = debug_game_dir();
        for letter in ['G', 'O', 'J'] {
            for second in [char::from(INPUT_CODE_EAST), ' '] {
                let mut state = test_state(open_grid(), 5, 5);
                state.keys = 1;
                assert_eq!(
                    handle_play_key_input(&mut state, letter, "", &dir).unwrap(),
                    PlayInputDisposition::Continue
                );
                assert_eq!(state.turn, 0, "{letter} prompt is free");
                assert!(state.active_direction_prompt.is_some(), "{letter} prompt");
                assert_eq!(
                    handle_play_key_input(&mut state, second, "", &dir).unwrap(),
                    PlayInputDisposition::Continue
                );
                assert!(
                    state.active_direction_prompt.is_none(),
                    "{letter} then {second:?}: {:?}",
                    state.message
                );
                assert_eq!(state.turn, 1, "{letter} then {second:?} must be acted");
            }
        }
        let mut state = world_state(open_world_grid(), 5, 5);
        assert_eq!(
            handle_play_key_input(&mut state, 'G', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, GET_NOTHING_REFUSAL);
        assert_eq!(state.turn, 1, "overworld Get miss is acted");
        let _ = fs::remove_dir_all(dir);
    }

    /// `commands.md §5.8`: the `0x9A` arm is the one eat arm that sets the turn
    /// sentinel, and it is reached one step south.
    #[test]
    fn the_laden_table_south_arm_eats_and_charges_the_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 * 3 + 2] = 0x9a;
        let mut state = test_state(grid, 2, 2);
        state.player.facing = Direction::South;
        state.food = 12;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.message, GET_TABLE_EATEN_LINE);
        assert_eq!(state.grid[32 * 3 + 2], 0x95);
        assert_eq!(state.food, 13);
        assert_eq!(state.turn, 1);

        // "Laden table `0x9A`/`0x9B` from any other direction ... `Can't reach
        // plate!` | unchanged"
        let mut grid = open_grid();
        grid[32 * 2 + 3] = 0x9a;
        let mut state = test_state(grid, 2, 2);
        state.player.facing = Direction::East;
        state.food = 12;
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, GET_TABLE_CANT_REACH_PLATE_LINE);
        assert_eq!(state.grid[32 * 2 + 3], 0x9a);
        assert_eq!(state.food, 12);
        assert_eq!(state.turn, 0);
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
        // `commands.md §5.8`: the three eat arms share the grant tail "but
        // **not** the turn sentinel: only the crop arm and the `0x9A` arm set
        // it".
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, GET_TABLE_EATEN_LINE);
        assert!(state.visibility_dirty);
        assert!(state.diagnostics.iter().any(|note| note.contains("Ate food from table tile 0x9B") || note.contains("ate food from table tile 0x9B")));

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
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, GET_TABLE_EATEN_LINE);
        assert!(state.diagnostics.iter().any(|note| note.contains("Ate food from table tile 0x9C") || note.contains("ate food from table tile 0x9C")));
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
        assert_eq!(state.turn, 0);
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
        assert_eq!(state.message, GET_TABLE_CANT_REACH_PLATE_LINE);
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
        assert_eq!(state.message, GET_NOTHING_REFUSAL);
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
        assert!(state.diagnostics.iter().any(|note| note.contains("Got 3 torches") || note.contains("got 3 torches")));
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
        assert!(state.diagnostics.iter().any(|note| note.contains("Got world tile 55")));
        assert!(state.diagnostics.iter().any(|note| note.contains("UNDERWORLD")));
        assert!(state.diagnostics.iter().any(|note| note.contains("added 7 gold")));
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
        assert!(state.diagnostics.iter().any(|note| note.contains("added 4 food")));
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
        assert_eq!(state.message, GET_NOTHING_REFUSAL);
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
        assert!(state.diagnostics.iter().any(|note| note.contains("Got world tile 55")));
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
        assert!(state.diagnostics.iter().any(|note| note.contains("Got tile 55")));
        assert!(state.diagnostics.iter().any(|note| note.contains("added 2 keys")));
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
        assert!(state.diagnostics.iter().any(|note| note.contains("added 1 food")));
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
        assert_eq!(state.message, GET_NOTHING_REFUSAL);
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
        assert!(state.diagnostics.iter().any(|note| note.contains("Got tile 55")));
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
        // `commands.md §5.8` (u5-spec#298 Q4, R544): the location phrase,
        // then a lower-case `thou dost find` row.
        assert_eq!(state.message, "\nIn the barrel\nthou dost find\nnothing of note.\n");
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
            SEARCH_NOTHING_FOUND
        );
    }

    /// `containers.md §3`: the active-object coordinate branch of Search is
    /// for class `0x1F` corpses only, and "none of [Search's result shapes]
    /// is a slot-indexed treasure grant" - "the later Get path owns the
    /// inventory transfer". Searching a staged fixed-treasure cell again
    /// grants nothing and prints nothing of note (playtest G26).
    #[test]
    fn search_of_a_staged_fixed_treasure_grants_nothing() {
        let mut grid = open_grid();
        grid[8 * 32 + 5] = 0x4e;
        let mut state = test_state(grid, 4, 8);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state
            .active_objects
            .push(ActiveObject::fixed_hidden_treasure_pickup(18, 5, 8, 0));
        state
            .active_objects
            .push(ActiveObject::fixed_hidden_treasure_pickup(13, 5, 8, 0));

        assert_eq!(state.search_active_object_treasure_marker_at(5, 8), None);

        assert!(!state.active_objects[1].is_empty());
        assert!(!state.active_objects[2].is_empty());
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.gems, DEFAULT_GEM_STOCK);
        assert_eq!(state.turn, 0);
        assert!(!state.message.contains("Found"));
    }

    #[test]
    fn town_search_surface_object_trap_preserves_unsigned_threshold_wraparound() {
        let mut grid = open_grid();
        grid[32 + 2] = 0x4e;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: TILE_FURNITURE_FIRST,
            tile: TILE_FURNITURE_FIRST,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x85,
            aux3: 0,
        });
        // `containers.md §5`: the stat is the acting member's Intelligence;
        // above thirty the unsigned halving wraps and the roll always fails.
        state.party_intelligence[0] = 60;

        assert_eq!(
            state.search_facing_secret(&[], None),
            MoveOutcome::Searched
        );

        assert_eq!(state.active_objects[1].type_byte, TILE_FURNITURE_FIRST);
        assert_eq!(state.active_objects[1].aux1, 0x85);
        assert_eq!(state.grid[32 + 2], 0x4e);
        assert_eq!(state.turn, 1);
        // `commands.md §5.8`: the container branch reports the assessed trap
        // state under the shared preamble.
        assert_eq!(
            state.message,
            format!("{SEARCH_PREAMBLE}{SEARCH_TRAP_NONE_LINE}")
        );
        assert!(
            state
                .diagnostics
                .iter()
                .any(|note| note.contains("searched active-object tile 64 at (2, 1); no trap"))
        );
    }

    /// `commands.md §5.8` "Successful Get, Search and eat results": a Get of
    /// a staged fixed hidden treasure prints the class dispatcher's row -
    /// `1_gem!`, `A_<colour>_potion!`, the equipment row name - not a debug
    /// sentence (playtest G25).
    #[test]
    fn get_of_a_staged_fixed_treasure_prints_the_published_pickup_row() {
        // Record 18 Gem 1, record 21 Potion 5, record 28 Ring 42.
        for (record, expected) in [
            (18usize, "\n1 gem!".to_string()),
            (21, format!("\nA {} potion!", potion_colour_display_word(5))),
            (28, format!("\n{}!", equipment_name(42))),
        ] {
            let mut state = test_state(open_grid(), 4, 8);
            state
                .active_objects
                .push(ActiveObject::fixed_hidden_treasure_pickup(record, 5, 8, 0));
            let gems = state.gems;
            let potions = state.potion_stock[5];
            let rings = state.equipment_stock[42];

            assert_eq!(
                state.get_fixed_hidden_treasure_pickup_at(5, 8),
                Some(MoveOutcome::Got)
            );
            assert_eq!(state.message, expected);
            assert!(state.active_objects[1].is_empty());
            assert_eq!(state.turn, 1);
            match record {
                18 => assert_eq!(state.gems, gems + 1),
                21 => assert_eq!(state.potion_stock[5], potions + 1),
                _ => assert_eq!(state.equipment_stock[42], rings + 1),
            }
        }
    }

    /// `containers.md §8` key row: a marked odd-key subtype strips the marker
    /// for the displayed quantity and credits the skull-key counter, through
    /// the same dispatcher a staged ring of keys reaches on Get.
    #[test]
    fn get_of_a_staged_odd_key_ring_credits_skull_keys() {
        let mut state = test_state(open_grid(), 4, 8);
        state
            .active_objects
            .push(ActiveObject::fixed_hidden_treasure_pickup(14, 5, 8, 0));
        let keys = state.keys;
        let skull = state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX];
        assert_eq!(
            state.get_fixed_hidden_treasure_pickup_at(5, 8),
            Some(MoveOutcome::Got)
        );
        assert_eq!(state.message, "\n5 odd keys!");
        assert_eq!(state.keys, keys);
        assert_eq!(state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], skull + 5);
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
        assert_eq!(state.message, format!("{SEARCH_PREAMBLE}{SEARCH_STRANGE_ROCK_LINE}"));
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
        // `commands.md §5.8`: "searching the same cell again before
        // collecting prints `nothing_of_note.`, because the duplicate guard
        // sees the staged object and creates no second one."
        assert_eq!(state.message, SEARCH_NOTHING_FOUND);

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.moonstone_slots[1].scene, 0);
        assert_eq!(state.moonstone_slots[3], MoonstoneGateSlot::invalid());
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
        assert!(state.diagnostics.iter().any(|n| n.contains("recovered Moonstone phase 4; Gate Travel slot cleared")), "{:?}", state.diagnostics);
        // `commands.md §5.8`: the Get prints `A_moonstone!` (playtest E60).
        assert_eq!(state.message, "\nA moonstone!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_search_generic_find_marker_skips_moonstone_but_keeps_later_scans() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[5 * WORLD_SIDE + 5] = 0xdc;
        let mut state = britannia_state(grid, 4, 5);
        state.player.facing = Direction::East;
        state.moonstone_slots[0] = MoonstoneGateSlot {
            scene: 0,
            x: 5,
            y: 5,
            z: 0,
        };

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.turn, 0);
        assert_eq!(state.active_objects.len(), 1);
        assert_eq!(
            state.message,
            SEARCH_NOTHING_FOUND
        );

        state.player.x = 181;
        state.player.y = 54;
        state.clock = GameClock::with_date(139, 4, 5, 0, 17).unwrap();
        state.reagents[REAGENT_MANDRAKE] = 0;
        state.rare_reagent_harvest_days[0] = 0;
        state.grid[54 * WORLD_SIDE + 182] = 0xdc;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert!(state.diagnostics.iter().any(|n| n.contains("sprigs of Mandrake Root")));
        assert_eq!(state.rare_reagent_harvest_days[0], 5);
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
        state.prng_state = 0;
        let mut expected_prng = state.prng_state;
        let expected_amount = u5_prng_range_u16(
            &mut expected_prng,
            RARE_REAGENT_HARVEST_QUANTITY_MIN.into(),
            RARE_REAGENT_HARVEST_QUANTITY_MAX.into(),
        ) as u8;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.reagents[REAGENT_MANDRAKE], 99);
        assert_eq!(state.rare_reagent_harvest_days[0], 5);
        assert_eq!(state.prng_state, expected_prng);
        assert!(state.message.contains(&format!("{expected_amount} sprigs")));
        assert!(state.diagnostics.iter().any(|n| n.contains("sprigs of Mandrake Root")));
        assert_eq!(
            state.message,
            format!("{SEARCH_PREAMBLE}{expected_amount}{SEARCH_REAGENT_SPRIGS_FRAGMENT}{SEARCH_REAGENT_MANDRAKE_LINE}")
        );

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.reagents[REAGENT_MANDRAKE], 99);
        assert_eq!(state.message, SEARCH_NOTHING_FOUND);

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
        assert!(state.diagnostics.iter().any(|n| n.contains("sprigs of Nightshade")));
        assert!(state.message.ends_with(SEARCH_REAGENT_NIGHTSHADE_LINE));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_fixed_hidden_treasure_stages_pickup_and_get_grants_inventory() {
        let dir = debug_game_dir();
        // `hidden-treasures.md §3` (u5-spec#300 Q1, R584): record 18 is at
        // X 8, Y 5 (the transposed X 5, Y 8 is retracted).
        let mut state = test_state(open_grid(), 7, 5);
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
        assert_eq!(state.message, format!("{SEARCH_PREAMBLE}a gem!"));
        assert_eq!(
            state.active_objects[1],
            ActiveObject::fixed_hidden_treasure_pickup(18, 8, 5, 0)
        );

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.gems, 1);
        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.message, "\n1 gem!");

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.turn, 2);
        assert_eq!(state.message, SEARCH_NOTHING_FOUND);
        let _ = fs::remove_dir_all(dir);
    }

    /// `containers.md §5`: "Repeating the same Search later skips the
    /// already-found record and continues scanning", and the staged object
    /// is left for Get, which "owns the inventory transfer". A second Search
    /// before Get grants nothing and leaves the staged slot in place
    /// (playtest G26).
    #[test]
    fn second_search_of_a_staged_fixed_treasure_before_get_grants_nothing() {
        let dir = debug_game_dir();
        // `hidden-treasures.md §3` (u5-spec#300 Q1, R584): record 18 is at
        // X 8, Y 5 (the transposed X 5, Y 8 is retracted).
        let mut state = test_state(open_grid(), 7, 5);
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
        let staged = ActiveObject::fixed_hidden_treasure_pickup(18, 8, 5, 0);
        assert_eq!(state.active_objects[1], staged);

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, SEARCH_NOTHING_FOUND);
        assert_eq!(state.gems, 0);
        assert_eq!(state.active_objects[1], staged);

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.gems, 1);
        assert_eq!(state.message, "\n1 gem!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_hidden_treasure_table_covers_all_spec_records() {
        assert_eq!(
            PlayState::fixed_hidden_treasure_table_len(),
            FIXED_HIDDEN_TREASURE_COUNT
        );
        assert!(PlayState::fixed_hidden_treasure_table_records_are_sequential());
        assert_eq!(
            // u5-spec#300 Q1 (R584): the corrected, untransposed X/Y columns.
            PlayState::fixed_hidden_treasure_table_fingerprint(),
            0xbecc_378a_39f3_cbb3
        );
        assert_eq!(
            PlayState::fixed_hidden_treasure_table_pickup_counts(),
            [12, 16, 21, 19, 4, 7, 3, 7, 5, 7, 2, 2, 8]
        );
        assert_eq!(
            PlayState::fixed_hidden_treasure_table_rule_counts(),
            [110, 1, 1, 1]
        );
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
        assert_eq!(state.message, format!("{SEARCH_PREAMBLE}a weapon!"));
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
        assert_eq!(state.message, format!("{SEARCH_PREAMBLE}a scroll!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_fixed_hidden_daily_cache_resets_by_day_and_grants_odd_keys() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 2);
        state.area = Area::Town {
            scene: Scene::new(5).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.clock = GameClock::with_date(139, 4, 5, 12, 0).unwrap();
        state.keys = 98;
        state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] = 97;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(state.fixed_hidden_treasure_daily_day, 5);

        // Record 14's state `133` is a marked odd-key subtype: the
        // `containers.md §8` key arm strips the marker, credits the
        // skull/special-key counter capped at 99, and leaves skeleton keys.
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.keys, 98);
        assert_eq!(state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 99);
        assert_eq!(state.message, "\n5 odd keys!");

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
    fn search_fixed_hidden_daily_cache_ignores_found_bitmap_bit() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 2);
        state.area = Area::Town {
            scene: Scene::new(5).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.clock = GameClock::with_date(139, 4, 5, 12, 0).unwrap();
        state.fixed_hidden_treasure_daily_day = FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY;
        state.set_fixed_hidden_treasure_found(14);

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(state.fixed_hidden_treasure_found(14));
        assert_eq!(state.fixed_hidden_treasure_daily_day, 5);
        assert_eq!(
            state.active_objects[1].fixed_hidden_treasure_record(),
            Some(14)
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `hidden-treasures.md` §2, record 15: the gate is "The
    /// equipment-inventory counter for the item it grants ... The scan
    /// never writes the counter; the ordinary inventory grant for the item
    /// does, and that is exactly what makes the record single-use."
    /// `formats/saved-gam.md` §10 names the byte: "This is the
    /// **equipment-inventory counter for item id `39` (Glass Sword)** ...
    /// An engine that gives record 15 a separate never-written cookie
    /// yields an infinitely repeatable Glass Sword."
    ///
    /// So the second Search after a successful Get is refused, because the
    /// Get incremented the Glass Sword counter. An earlier revision of this
    /// test searched, got, and searched again expecting a second stage -
    /// the "infinitely repeatable" behaviour the spec names as the error -
    /// and that expectation is retracted. Record 15 still never sets a
    /// found-bitmap bit, which is what the remaining
    /// `fixed_hidden_treasure_found(15)` assertions pin.
    #[test]
    fn search_fixed_hidden_record_15_uses_glass_sword_counter_not_found_bitmap() {
        let dir = debug_game_dir();
        let mut state = world_state(open_world_grid(), 63, 80);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.player.facing = Direction::East;
        state.equipment_stock[EQUIPMENT_ID_GLASS_SWORD] = 0;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(!state.fixed_hidden_treasure_found(15));
        assert_eq!(
            state.active_objects[1].fixed_hidden_treasure_record(),
            Some(15)
        );

        // The Get performs the inventory transfer, which increments the
        // Glass Sword counter; the record is now closed even though no
        // found-bitmap bit was ever set.
        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert!(state.equipment_stock[EQUIPMENT_ID_GLASS_SWORD] > 0);
        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert!(!state.fixed_hidden_treasure_found(15));

        // "a party that discards or loses its Glass Sword makes the record
        // available again - that is original behaviour, not a defect."
        state.equipment_stock[EQUIPMENT_ID_GLASS_SWORD] = 0;
        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(!state.fixed_hidden_treasure_found(15));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_fixed_hidden_record_13_requires_zero_keys() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 7, 6);
        state.area = Area::Town {
            scene: Scene::new(18).unwrap(),
            floor: -1,
        };
        state.player.facing = Direction::East;
        state.keys = 1;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert!(!state.fixed_hidden_treasure_found(13));

        state.keys = 0;
        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        // `hidden-treasures.md §2`: record 13 sets no found-bitmap bit.
        assert!(!state.fixed_hidden_treasure_found(13));
        assert_eq!(
            state.active_objects[1].fixed_hidden_treasure_record(),
            Some(13)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_hidden_special_records_ignore_non_npc_active_object_on_target() {
        let dir = debug_game_dir();
        let mut state = world_state(open_world_grid(), 63, 80);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.player.facing = Direction::East;
        state.equipment_stock[EQUIPMENT_ID_GLASS_SWORD] = 0;
        state.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 64,
            y: 80,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(
            state.active_objects[2].fixed_hidden_treasure_record(),
            Some(15)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_hidden_special_records_block_on_town_npc_at_target() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 7, 6);
        state.area = Area::Town {
            scene: Scene::new(18).unwrap(),
            floor: -1,
        };
        state.player.facing = Direction::East;
        state.keys = 0;
        state.npcs.push(RuntimeNpc {
            slot: 1,
            // Kind 1 is the chest object class (`magic.md`, An Sanct).
            type_byte: 0x50,
            dialog_id: 0,
            schedule: [0; NPC_SCHEDULE_RECORD_LEN],
            state: NPC_STATE_IDLE,
            x: 8,
            y: 6,
            z: 0xff,
            cached_wp: 0,
            move_queue: Vec::new(),
            move_queue_pos: 0,
            stuck_counter: 0,
            active_object: Some(1),
        });
        state.active_objects.push(npc_active_object(0x50, 8, 6, 0xff));

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert!(!state.fixed_hidden_treasure_found(13));
        assert_eq!(state.message, SEARCH_NOTHING_FOUND);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_hidden_treasure_food_clamps_to_party_food_cap() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 23, 19);
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
        assert_eq!(state.message, "\n10 food!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn get_fixed_hidden_treasure_grants_equipment_stock() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 14, 1);
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
        assert_eq!(state.message, format!("\n{}!", equipment_name(47)));
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
        assert_eq!(state.message, format!("{SEARCH_PREAMBLE}{SEARCH_STRANGE_ROCK_LINE}"));

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.moonstone_slots[0], MoonstoneGateSlot::invalid());
        assert_eq!(state.turn, 2);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert!(state.diagnostics.iter().any(|n| n.contains("recovered Moonstone phase 1; Gate Travel slot cleared")), "{:?}", state.diagnostics);
        // `commands.md §5.8`: the Get prints `A_moonstone!` (playtest E60).
        assert_eq!(state.message, "\nA moonstone!");
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
        assert!(state.diagnostics.iter().any(|n| n.contains("buried Moonstone phase 3 at BRITANNIA (4, 5)")), "{:?}", state.diagnostics);
        assert_eq!(state.message, MOONSTONE_BURY_SUCCESS);

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
        // `commands.md §8.1`: the scene key and the cell are a diagnostic.
        assert!(
            state
                .diagnostics
                .join(" ")
                .contains("Gate Travel phase 3 -> BRITANNIA at (4, 5).")
        );
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
    fn mode_zero_cleanup_reapplies_natural_moongates_without_advancing_counter() {
        let gate_idx = world_cell_index(6, 7);
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.clock = GameClock::new(23, 0).unwrap();
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: WorldPlane::Britannia.save_floor() as u8,
        };

        state.mode_zero_cleanup();

        assert_eq!(state.natural_moongate_counter, 0);
        assert_eq!(state.grid[gate_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert!(state.natural_moongate_live_cells.is_empty());

        state.natural_moongate_counter = 2;
        state.mode_zero_cleanup();

        assert_eq!(state.natural_moongate_counter, 2);
        assert_eq!(state.grid[gate_idx], NATURAL_MOONGATE_TERRAIN_TILE);
        assert_eq!(state.natural_moongate_live_cells, vec![gate_idx]);
    }

    #[test]
    fn natural_moongate_entry_uses_cached_moon_slot_without_spending_turn() {
        let dir = debug_game_dir();
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(11, 58).unwrap();
        state.set_cached_moon_glyph_slots(1, 0);
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: WorldPlane::Britannia.save_floor() as u8,
        };

        // The hook runs at the loop head, before any key is read
        // (`overworld.md §9.2`; playtest E25).
        run_overworld_loop_head_hooks(&mut state, &dir).unwrap();

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
        // `commands.md §8.1`: the scene key and the cell are a diagnostic.
        assert!(
            state
                .diagnostics
                .join(" ")
                .contains("Gate Travel phase 2 -> BRITANNIA at (6, 7).")
        );
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
        state.set_cached_moon_glyph_slots(1, 2);
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

        // The hook runs at the loop head, before any key is read
        // (`overworld.md §9.2`; playtest E25).
        run_overworld_loop_head_hooks(&mut state, &dir).unwrap();

        assert_eq!((state.player.x, state.player.y), (8, 9));
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        // `commands.md §8.1`: the scene key and the cell are a diagnostic.
        assert!(
            state
                .diagnostics
                .join(" ")
                .contains("Gate Travel phase 3 -> BRITANNIA at (8, 9).")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn natural_moongate_entry_uses_the_felucca_day_row_from_noon_onward() {
        // `moons.md §2.2`: the glyph identity is "indexed by the
        // calendar day of the month ... It is not indexed by the hour",
        // and `overworld.md §9` / `moons.md §2.2` say the entry hook
        // reads the *second* cached glyph from noon onward. The fixture
        // clock is on day 5, whose Felucca row is `'3'` -> slot 3. This
        // test previously pinned the retracted 24-entry hour table's
        // hour-19 row.
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(19, 0).unwrap();
        state.refresh_cached_moon_glyphs();
        state.natural_moongate_live_cells.push(origin_idx);

        // The hook runs at the loop head, before any key is read
        // (`overworld.md §9.2`; playtest E25).
        run_overworld_loop_head_hooks(&mut state, Path::new("")).unwrap();

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert!(state.natural_moongate_live_cells.is_empty());
        assert!(state.visibility_dirty);
        assert!(state.message.is_empty());
    }

    #[test]
    fn natural_moongate_entry_uses_published_day_table_for_empty_slot() {
        // `moons.md §2.2`: before noon the entry hook reads the first
        // cached glyph, whose identity comes from the calendar day. With no destination wired
        // into slot 6, the gate hook still clears the tile and reports
        // the published "phase N is not set" message — confirming the
        // cached byte rather than recomputing the hour table at entry.
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(11, 58).unwrap();
        // An arbitrary non-digit fixture pair, named here rather than
        // taken from a reserved constant: `moons.md §2.2` says an
        // implementation that reserves a high-bit "off horizon" value
        // "is modelling something the tables do not contain".
        state.set_cached_moon_glyph_bytes(0xAA, 0xBB);
        state.refresh_cached_moon_glyphs();
        state.natural_moongate_live_cells.push(origin_idx);

        // The hook runs at the loop head, before any key is read
        // (`overworld.md §9.2`; playtest E25).
        run_overworld_loop_head_hooks(&mut state, Path::new("")).unwrap();

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        // `moons.md §2.2`: the glyph is chosen by the calendar day, not
        // by the hour. The fixture clock starts on day 5, whose Trammel
        // row is `'2'` — the spec's own worked example ("Trammel's
        // **glyph** comes from day 5, not hour 8 ... gives `'2'`").
        assert!(state.message.is_empty());
    }

    #[test]
    fn natural_moongate_entry_uses_cached_byte_not_current_hour_table() {
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(11, 58).unwrap();
        // The clock is before noon, so only the first cached byte is
        // read (`moons.md §2.2`); the second is any legal phase digit.
        state.set_cached_moon_glyph_bytes(b'1', b'0');
        state.natural_moongate_live_cells.push(origin_idx);

        // The hook runs at the loop head, before any key is read
        // (`overworld.md §9.2`; playtest E25).
        run_overworld_loop_head_hooks(&mut state, Path::new("")).unwrap();

        assert!(state.message.is_empty());
    }

    #[test]
    /// `moons.md §3` refresh cadence: the strip renderer "is **not**
    /// driven by ordinary stats-panel redraws, and an earlier statement
    /// in this document that it should be refreshed on every stats-panel
    /// redraw is retracted." What this test pins is that negative and the
    /// hour-change hook's own row of the census - "Only when the pass
    /// observes the hour changing, and only in a scene that shows the
    /// surface/town status strip and on a floor at or above the entry
    /// floor."
    ///
    /// *Re-anchored (issue #190).* This comment used to carry the model
    /// `RETRACTIONS.md` R343 withdrew - that the renderer "runs from
    /// **exactly one place**: the per-turn cleanup pass, and only when
    /// that pass observes the hour changing". §3 now publishes a
    /// seven-row caller census. The other callers are pinned in
    /// `chunk_13.rs`; this test is about the redraw paths that are *not*
    /// callers.
    fn moon_glyph_cache_refreshes_on_hour_change_but_not_on_status_redraw() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.clock = GameClock::new(10, 58).unwrap();
        // An arbitrary non-digit fixture pair; see the note on
        // `natural_moongate_entry_uses_published_day_table_for_empty_slot`.
        state.set_cached_moon_glyph_bytes(0xAA, 0xBB);

        state.advance_turn_with_minutes(2);

        assert_eq!(state.clock.hour, 11);
        // `moons.md §3`: the hour change is the refresh *trigger*, but
        // the cached glyph identity comes from the calendar day.
        assert_eq!(
            state.cached_moon_glyph_bytes,
            cached_moon_glyph_bytes_for_day(state.clock.day).unwrap()
        );

        // A redraw with no hour change must leave the cache exactly as
        // the caller parked it: the natural-moongate entry hook reads
        // these two bytes to pick the destination Moonstone slot, so a
        // repaint must not silently re-derive them from the current day.
        state.clock = GameClock::new(12, 0).unwrap();
        state.set_cached_moon_glyph_bytes(0xAA, 0xBB);
        let _ = state.render_text_window_frame(None);
        assert_eq!(state.cached_moon_glyph_bytes, [0xAA, 0xBB]);

        let _ = state.render_stats_panel_frame();
        assert_eq!(state.cached_moon_glyph_bytes, [0xAA, 0xBB]);
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

        // The hook runs at the loop head, before any key is read
        // (`overworld.md §9.2`; playtest E25).
        run_overworld_loop_head_hooks(&mut state, &dir).unwrap();

        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        // `overworld.md §9.2`: the overlay is "the same shrine/urn kneel
        // overlay used by E-Enter at a shrine" - approach, kneel and the
        // virtue question, not a bare mantra prompt (playtest F22). The
        // sidecar row names the virtue, as it does for E-Enter.
        let shrine = state.active_shrine.as_ref().expect("the overlay opened");
        assert_eq!(shrine.phase, ShrinePhase::Virtue);
        assert_eq!(shrine.virtue, ShrineVirtue::Honesty);
        assert!(state.staged_narration_active());
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    /// Playtest E61: `overworld.md §9.2` runs the live-gate transit "before
    /// the party is relocated and before any key is read", and the transit
    /// reads no key. The key that reached the hook is still the turn's
    /// command, so it is dispatched after the transit instead of dropped.
    #[test]
    fn natural_moongate_transit_dispatches_the_key_that_reached_the_hook() {
        let dir = debug_game_dir();
        let origin_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(2, 0).unwrap();
        state.set_cached_moon_glyph_bytes(0xAA, 0xBB);
        state.natural_moongate_live_cells.push(origin_idx);

        assert_eq!(
            handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert_eq!((state.player.x, state.player.y), (5, 4));
        assert_eq!(state.turn, 1);

        // At midnight the hook dispatches the shrine kneel overlay, whose
        // staged approach owns the loop; the key is kept for the virtue
        // prompt rather than dropped (playtest F22).
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 5 5 HONESTY 5\n").unwrap();
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(0, 9).unwrap();

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_shrine.is_some(), "the overlay opened");
        assert!(state.staged_narration_active());
        assert_eq!(
            state.keys_buffered_during_hold.front().map(|(key, _)| *key),
            Some('A')
        );
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    /// `karma.md §8` (`cleak/u5-spec#250`, `RETRACTIONS.md` R451): Codex
    /// entry is E-Enter on live tile `0x11`. The midnight natural moongate
    /// dispatches the shrine meditation handler and nothing else - it used to
    /// try the Codex reader first, back when `M` was believed to own Codex
    /// entry - and the cell it stands on has just been rewritten to the
    /// restored moongate terrain, so no Codex tile is under the party.
    #[test]
    fn natural_moongate_midnight_window_opens_the_shrine_prompt_and_not_the_codex() {
        let dir = debug_game_dir();
        let origin_idx = world_cell_index(5, 5);
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 5 5 HONESTY 5\n").unwrap();
        let mut grid = open_world_grid();
        grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(0, 0).unwrap();
        state.shrine_ordained_mask = ShrineVirtue::Justice.bit();

        // The hook runs at the loop head, before any key is read
        // (`overworld.md §9.2`; playtest E25).
        run_overworld_loop_head_hooks(&mut state, &dir).unwrap();

        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert_eq!(state.shrine_codex_mask, 0);
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

        // The hook runs at the loop head, before any key is read
        // (`overworld.md §9.2`; playtest E25).
        run_overworld_loop_head_hooks(&mut state, Path::new("")).unwrap();

        assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        // `karma.md §7`: no coordinate row matches, so the meditation
        // resolves to Spirituality instead of being dropped (playtest F22).
        let shrine = state.active_shrine.as_ref().expect("the overlay opened");
        assert_eq!(shrine.virtue, ShrineVirtue::Spirituality);
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
        assert!(state.diagnostics.iter().any(|n| n.contains("buried Moonstone phase 8 at CASTLE:0 (1, 1)")), "{:?}", state.diagnostics);
        assert_eq!(state.message, MOONSTONE_BURY_SUCCESS);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn use_moonstone_rejections_charge_the_normal_use_turn() {
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
        assert_eq!(state.turn, 1);
        assert_eq!(state.moonstone_slots[0], MoonstoneGateSlot::invalid());
        assert_eq!(state.message, MOONSTONE_BURY_REFUSAL);

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        assert_eq!(
            handle_play_key_input(&mut dungeon, 'U', "1", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(dungeon.turn, 1);
        // `inventory.md §7.1`: the Moonstone branch has exactly one refusal
        // literal, and the dungeon band is a scene outside `0x00..0x20`.
        assert_eq!(dungeon.message, MOONSTONE_BURY_REFUSAL);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn use_picker_lists_carried_moonstones_in_the_dungeon_and_in_combat() {
        // Playtest H33: `inventory.md §4.5` gives a carried stone its row
        // "exactly when its stock value is nonzero ... no usability test
        // anywhere in the path". The dungeon and combat used to answer
        // `No usable items!` instead of opening the picker.
        let dir = debug_game_dir();
        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        assert!(dungeon.moonstone_slots.iter().all(|slot| !slot.is_valid()));
        assert!(dungeon.handle_dungeon_key('U', &dir).unwrap());
        assert_ne!(dungeon.message, USE_NO_USABLE_ITEMS_REFUSAL);
        assert!(dungeon.active_use.is_some());
        let moonstones = dungeon
            .use_item_picker_rows()
            .iter()
            .filter(|row| matches!(row.request, UseItemRequest::Moonstone(_)))
            .count();
        assert_eq!(moonstones, MOONSTONE_SLOT_COUNT);

        // A combat arena is outside the bury band whatever map it was
        // entered from, so the row is listed and the bury refuses it.
        let mut grid = open_world_grid();
        grid[world_cell_index(4, 5)] = 5;
        let mut arena = britannia_state(grid, 4, 5);
        arena.combat_active = true;
        assert!(arena.current_moonstone_bury_context().is_none());
        assert!(
            arena
                .use_item_picker_rows()
                .iter()
                .any(|row| matches!(row.request, UseItemRequest::Moonstone(0)))
        );
        assert_eq!(arena.use_moonstone_phase(Some(0)), MoveOutcome::Blocked);
        assert_eq!(arena.moonstone_slots[0], MoonstoneGateSlot::invalid());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn use_moonstone_bury_refusal_continues_the_item_row_and_wraps_at_the_retained_column() {
        // `cleak/u5-engine#44`: the refusal is written at the cursor the
        // `Item: ` prompt left, "subject to normal wrapping"
        // (`cleak/u5-spec#259`, published `ed0f743`), not as a fresh line
        // wrapped from column zero. Measured (`qa/paired/use-moonstone-bury.tsv`,
        // beat `buried`): the original breaks `Item: Moonstone cannot be
        // buried here!` as `Item: Moonstone` / `cannot be buried` / `here!` -
        // this engine used to wrap the continuation alone from column zero
        // and broke after `cannot be` instead.
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(4, 5)] = 16;
        let mut state = britannia_state(grid, 4, 5);

        // Drive the real picker, the same shape `qa/paired/use-moonstone-bury.tsv`
        // does: `u` opens it, five `Down` presses reach the first Moonstone
        // slot, `Return` accepts - the direct `U1` shortcut the test above
        // uses does not go through the picker's `Item: ` echo at all.
        handle_play_key_input(&mut state, 'u', "", &dir).unwrap();
        for _ in 0..5 {
            handle_play_key_input(&mut state, '\u{4}', "", &dir).unwrap();
        }
        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, MOONSTONE_BURY_REFUSAL);

        let log = message_log_from_entries(state.message_entries(), |text| {
            Some(text.to_string())
        });
        let rows: Vec<&str> = log.lines().iter().map(|line| line.text.as_str()).collect();
        let item_row = rows
            .iter()
            .position(|text| *text == "Item: Moonstone")
            .unwrap_or_else(|| panic!("no `Item: Moonstone` row in {rows:?}"));
        assert_eq!(
            rows.get(item_row + 1..item_row + 3),
            Some(["cannot be buried", "here!"].as_slice()),
            "rows: {rows:?}"
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// The fixture's eight Moonstone rows, then `Shard/Falsehd` - a label
    /// that takes two display lines. With the highlight on the sixth row the
    /// page starts at the third Moonstone, so six one-line rows sit above the
    /// Shard and it begins on the panel's last list row. The underfoot tile
    /// refuses a burial, so accepting a Moonstone prints the §7.1 refusal.
    fn use_picker_state_with_overflowing_last_row() -> PlayState {
        let mut grid = open_world_grid();
        grid[world_cell_index(4, 5)] = 16;
        let mut state = britannia_state(grid, 4, 5);
        state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
        state
    }

    fn message_rows(state: &PlayState) -> Vec<String> {
        message_log_from_entries(state.message_entries(), |text| {
            (!text.trim().is_empty()).then(|| text.to_string())
        })
        .lines()
        .iter()
        .map(|line| line.text.clone())
        .collect()
    }

    #[test]
    fn a_picker_overflow_lifts_item_and_leaves_the_cursor_at_its_column() {
        // `cleak/u5-engine#25` (`cleak/u5-spec#259`, `ed0f743`): a repaint
        // whose last list row starts a two-line entry scrolls the message
        // strip up one row while the message cursor stays put. So `Item:`
        // rises and the cursor waits at column six of the row beneath it -
        // `amuletrow` in `qa/paired/use-specials.tsv`.
        let dir = debug_game_dir();
        let mut state = use_picker_state_with_overflowing_last_row();
        handle_play_key_input(&mut state, 'u', "", &dir).unwrap();
        for _ in 0..4 {
            handle_play_key_input(&mut state, '\u{4}', "", &dir).unwrap();
        }
        assert_eq!(state.active_use.as_ref().unwrap().strip_scrolls, 0);
        handle_play_key_input(&mut state, '\u{4}', "", &dir).unwrap();
        assert_eq!(state.active_use.as_ref().unwrap().strip_scrolls, 1);
        let rows = message_rows(&state);
        assert_eq!(rows[rows.len() - 2..], ["Item:".to_string(), String::new()], "{rows:?}");
        assert_eq!(state.open_prompt_line().as_deref(), Some("      "));

        let log = message_log_from_entries(state.message_entries(), |text| {
            (!text.trim().is_empty()).then(|| text.to_string())
        });
        let layout = layout_message_window_with_open_prompt(
            &log,
            Some(""),
            state.open_prompt_line().as_deref(),
        );
        let item = layout.rows.iter().find(|row| row.text == "Item:").unwrap();
        assert_eq!(
            layout.inline_cursor,
            Some((MESSAGE_WINDOW_LEFT + 6, item.row + 1)),
            "the cursor keeps the prompt's column one row under it"
        );

        // A key the picker does not act on still repaints the same page, and
        // that repaint overflows again: a blank opens between the prompt and
        // the cursor (`crownrow`).
        handle_play_key_input(&mut state, 'u', "", &dir).unwrap();
        assert_eq!(state.active_use.as_ref().unwrap().strip_scrolls, 2);
        let rows = message_rows(&state);
        assert_eq!(
            rows[rows.len() - 3..],
            ["Item:".to_string(), String::new(), String::new()],
            "{rows:?}"
        );

        // The completion is written at that cursor, and the Moonstone
        // refusal continues it from there (`inventory.md §7.1`).
        handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
        let rows = message_rows(&state);
        let item = rows.iter().position(|row| row == "Item:").expect("prompt row");
        assert_eq!(
            rows[item + 1..item + 5],
            [
                String::new(),
                "      Moonstone".to_string(),
                "cannot be buried".to_string(),
                "here!".to_string()
            ],
            "{rows:?}"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn escape_after_a_picker_overflow_writes_none_at_the_cursor_column() {
        let dir = debug_game_dir();
        let mut state = use_picker_state_with_overflowing_last_row();
        handle_play_key_input(&mut state, 'u', "", &dir).unwrap();
        for _ in 0..5 {
            handle_play_key_input(&mut state, '\u{4}', "", &dir).unwrap();
        }
        handle_play_key_input(&mut state, '\u{1b}', "", &dir).unwrap();
        let rows = message_rows(&state);
        let item = rows.iter().position(|row| row == "Item:").expect("prompt row");
        assert_eq!(rows[item + 1], "      None!", "{rows:?}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_picker_with_no_overflow_keeps_the_completion_on_the_item_row() {
        // N = 0: the `skullkeysrow` shape, `Item: Skull Key` on one row.
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.scroll_stock[0] = 1;
        handle_play_key_input(&mut state, 'u', "", &dir).unwrap();
        assert_eq!(state.active_use.as_ref().unwrap().strip_scrolls, 0);
        handle_play_key_input(&mut state, '\u{1b}', "", &dir).unwrap();
        assert!(message_rows(&state).iter().any(|row| row == "Item: None!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_continuation_after_a_strip_scroll_wraps_from_the_cursor_column() {
        // The Moonstone refusal of `cleak/u5-engine#44` continues whatever
        // row the completion was written on; after a scroll that row starts
        // at column six, so `      Moonstone` fills it and the refusal wraps
        // from the next row's column zero (`inventory.md §7.1`).
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.emit_message_line("Item: ");
        let prompt = state
            .break_open_prompt_row_for_strip_scroll("Item: ")
            .expect("`Item: ` is the open row");
        state.commit_prompt_reply(&prompt, USE_ITEM_ECHO_MOONSTONE);
        state.open_message_row();
        state.emit_message_line_continuing_row(MOONSTONE_BURY_REFUSAL);
        let rows = message_rows(&state);
        let item = rows.iter().position(|row| row == "Item:").expect("prompt row");
        assert_eq!(
            rows[item + 1..],
            [
                "      Moonstone".to_string(),
                "cannot be buried".to_string(),
                "here!".to_string()
            ],
            "{rows:?}"
        );
    }


    /// Places one outdoor walker slot on the default (Underworld) plane of a
    /// `world_state`, with a movable animation phase.
    fn world_state_with_walker(
        px: usize,
        py: usize,
        type_byte: u8,
        ox: usize,
        oy: usize,
    ) -> PlayState {
        let mut state = world_state(open_world_grid(), px, py);
        state.active_objects.push(ActiveObject {
            type_byte,
            tile: type_byte,
            x: ox,
            y: oy,
            z: WorldPlane::Underworld.save_floor(),
            // Low nibble zero is the decision point, so this slot is
            // eligible for cleanup movement unless the first phase claims it.
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        state
    }

    /// Mountain tile `0x0c`, which `surface_tile_blocks_projectile` treats as
    /// an obstruction. Tests that only need the shot *not* to connect put one
    /// on the line, so the `overworld.md §6.2.4` payload never runs and the
    /// party's hit points stay out of the assertion.
    fn block_projectile_at(state: &mut PlayState, x: usize, y: usize) {
        // Projectile obstruction samples the post-compositor primary grid.
        // Keep the authored blocker inside the visible radius so this helper
        // tests its tile class rather than the darkness sentinel.
        state.ambient_light = FULL_DAYLIGHT;
        state.grid[world_cell_index(x, y)] = 0x0c;
        let Area::World { plane } = state.area else {
            panic!("projectile fixture must be in an overworld scene");
        };
        state.rebuild_world_live_chunks_from_grid(plane).unwrap();
    }

    /// A six-member party of identical, healthy members, so a whole-party
    /// pass has something to walk.
    fn six_member_party(hp: u16) -> Vec<PartyMember> {
        (0..6)
            .map(|slot| PartyMember {
                slot: slot as u8,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 8,
                hp,
                max_hp: hp,
                level: 8,
            })
            .collect()
    }

    /// `vehicles.md §2` frigate marker, sails furled, facing north.
    fn aboard_frigate(state: &mut PlayState, hull: u8, skiffs: u8) {
        state.player.transport = TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull,
            skiffs,
        };
    }

    #[test]
    fn broadside_announcement_and_command_result_both_reach_the_transcript() {
        // `text-output.md §11`: the original "has no message slot to
        // overwrite" -- "[b]oth lines are emitted, in the order they
        // occur", so "[a] turn that produces an epilogue announcement
        // *and* a command result shows the announcement first, then the
        // result beneath it."
        //
        // The broadside is announced by the per-turn epilogue, which runs
        // inside `advance_turn` before `Pass` writes its own result. When
        // the message slot was the record, the result replaced the
        // announcement and the player never saw it. This is the case no
        // test of an individual message could show: each line is correct
        // on its own.
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        block_projectile_at(&mut state, 7, 5);
        state.turn = 1;

        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();

        let texts: Vec<&str> = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        let boom = texts
            .iter()
            .position(|text| text.contains("BOOOM"))
            .unwrap_or_else(|| panic!("no broadside announcement in {texts:?}"));
        let echo = texts
            .iter()
            .position(|text| *text == "Pass")
            .unwrap_or_else(|| panic!("no verb echo in {texts:?}"));
        // `commands.md §5`: the verb echo opens the turn, and the epilogue's
        // announcement is written beneath it. `Pass` itself contributes no
        // result line (`commands.md §8.1` row B), so the announcement is the
        // last line of the turn rather than the middle one - but it is still
        // there, which is what the slot-as-record model lost.
        assert!(echo < boom, "the echo must open the turn: {texts:?}");
        assert!(state.message_entries()[0].is_command_echo);
    }

    #[test]
    fn broadside_announcement_and_command_result_both_reach_the_message_window() {
        // `text-output.md §11`: "model the message area as an append-and-
        // scroll region, not as a value". The window is drawn from the
        // transcript, so both of the turn's lines get their own row --
        // the announcement above, the result beneath.
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        block_projectile_at(&mut state, 7, 5);
        state.turn = 1;
        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();

        let log = message_log_from_entries(state.message_entries(), |text| {
            Some(text.to_string())
        });
        let layout = layout_message_window(&log, Some(""));
        let announcement = layout
            .rows
            .iter()
            .position(|row| row.text.contains("BOOOM"))
            .unwrap_or_else(|| panic!("no announcement row in {:?}", layout.rows));
        let echo = layout
            .rows
            .iter()
            .position(|row| row.text == "Pass")
            .unwrap_or_else(|| panic!("no echo row in {:?}", layout.rows));
        assert!(
            layout.rows[echo].row < layout.rows[announcement].row,
            "the announcement must sit beneath the echo: {:?}",
            layout.rows
        );
        // `text-output.md §10.2`: the verb echo is the only prefixed row
        // of the turn; the announcement and the result are pure output
        // and start unprefixed at column 24.
        assert!(!layout.rows[announcement].prefixed);
        assert!(layout.rows[echo].prefixed);
    }

    #[test]
    fn hostile_broadside_prints_the_published_boom_literal_and_a_blank_row() {
        // `overworld.md §6.2.1` (u5-spec#297): the hostile-ship announcement
        // is "`* BOOOM! *` followed by a blank line, before the shot", and it
        // "is not the F-Fire `BOOOM!` literal".
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        block_projectile_at(&mut state, 7, 5);
        state.turn = 1;
        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();

        let texts: Vec<&str> = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        let boom = texts
            .iter()
            .position(|text| text.contains("BOOOM"))
            .unwrap_or_else(|| panic!("no broadside announcement in {texts:?}"));
        assert_eq!(texts[boom], "* BOOOM! *");
        assert_eq!(
            texts.get(boom + 1).copied(),
            Some(""),
            "a blank row follows the literal: {texts:?}"
        );
        assert_ne!(texts[boom], crate::commands::CANNON_BOOOM_LINE);
    }

    #[test]
    fn outdoor_walker_broadside_fires_and_suppresses_cleanup_movement() {
        // active-objects.md §8: "Ship-like water-creature and pirate frames
        // aligned with the player on the same row or column within three
        // cells fire a broadside: they print the boom message and then
        // resolve the same traced-line ranged attack". §8 also makes the
        // first phase exclusive with movement -- "[i]f none of those
        // immediate reactions fires, the cleanup phase decides ordinary
        // movement" -- so a firing slot does not step this turn.
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        block_projectile_at(&mut state, 7, 5);

        assert!(state.outdoor_first_phase_ranged_attack(1));

        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        block_projectile_at(&mut state, 7, 5);
        state.advance_outdoor_active_objects();
        state
            .apply_pending_outdoor_reactions(Path::new(""), WorldPlane::Underworld)
            .unwrap();

        assert!(state.message.contains("BOOOM"), "message: {}", state.message);
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (8, 5));
    }

    #[test]
    fn outdoor_projectile_samples_the_post_compositor_primary_grid() {
        let mut terrain_only = world_state_with_walker(5, 5, 0x2C, 8, 5);
        terrain_only.ambient_light = FULL_DAYLIGHT;
        block_projectile_at(&mut terrain_only, 7, 5);
        terrain_only
            .rebuild_world_live_chunks_from_grid(WorldPlane::Underworld)
            .unwrap();
        let blocked = terrain_only
            .outdoor_first_phase_ranged_attack_detail(1)
            .expect("aligned broadside fires");
        assert!(matches!(
            blocked.outcome,
            OutdoorRangedAttackOutcome::Obstructed { .. }
        ));

        let mut stamped = world_state_with_walker(5, 5, 0x2C, 8, 5);
        stamped.ambient_light = FULL_DAYLIGHT;
        block_projectile_at(&mut stamped, 7, 5);
        stamped
            .rebuild_world_live_chunks_from_grid(WorldPlane::Underworld)
            .unwrap();
        stamped.active_objects.push(ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 7,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let clear = stamped
            .outdoor_first_phase_ranged_attack_detail(1)
            .expect("aligned broadside fires");
        assert_eq!(clear.outcome, OutdoorRangedAttackOutcome::Connects);
        assert!(clear.absorption.is_some());
    }

    #[test]
    fn outdoor_walker_adjacency_preempts_the_ranged_class_test() {
        // A `0x2C` broadside family one cell away takes the adjacency arm.
        // It neither fires nor moves before the post-turn engagement handler.
        let mut state = world_state_with_walker(5, 5, 0x2c, 6, 5);
        assert!(!state.outdoor_first_phase_ranged_attack(1));

        state.advance_outdoor_active_objects();

        assert!(!state.message.contains("BOOOM"), "message: {}", state.message);
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (6, 5));
    }

    #[test]
    fn generic_adjacent_hostile_uses_shared_impact_on_exact_carpet_water_gate() {
        let mut grid = open_world_grid();
        grid[world_cell_index(5, 5)] = 0x03;
        let mut state = world_state(grid, 5, 5);
        state.party = six_member_party(40);
        state.player.transport = TransportState::Carpet {
            type_byte: 0x14,
            tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        };
        state.active_objects.push(ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        state.advance_outdoor_active_objects();
        assert_eq!(state.pending_outdoor_reaction_slots, vec![1]);
        assert_eq!(
            state
                .apply_pending_outdoor_reactions(Path::new(""), WorldPlane::Underworld)
                .unwrap(),
            Some(MoveOutcome::Used)
        );

        assert!(!state.combat_active);
        assert!(!state.active_objects[1].is_empty());
        assert!(state.party.iter().all(|member| member.hp < 40));
        assert!(state
            .message_entries()
            .iter()
            .any(|entry| entry.text == "Attacked!"));
    }

    #[test]
    fn generic_adjacent_ordinary_type_uses_full_aquatic_arena_selector() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        state.advance_outdoor_active_objects();
        state
            .apply_pending_outdoor_reactions(&dir, WorldPlane::Underworld)
            .unwrap();

        assert!(state.combat_active);
        assert_eq!(state.message, combat_banner_line());
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].owner_target_class,
            16
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lower_adjacent_reaction_resumes_after_generic_combat_returns() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(40);
        // Lower slot: silent returning reaction.
        state.active_objects.push(ActiveObject {
            type_byte: 0xe0,
            tile: 0xe0,
            x: 4,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        // Higher slot: generic combat pauses the walk before slot 1.
        state.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        state.advance_outdoor_active_objects();
        assert_eq!(state.pending_outdoor_reaction_slots, vec![2, 1]);
        state
            .apply_pending_outdoor_reactions(&dir, WorldPlane::Underworld)
            .unwrap();
        assert!(state.combat_active);
        assert_eq!(state.pending_outdoor_reaction_slots, vec![1]);
        assert!(state.party.iter().all(|member| member.hp == 40));

        state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
        state
            .apply_pending_outdoor_reactions(&dir, WorldPlane::Underworld)
            .unwrap();
        assert!(!state.combat_active);
        assert!(state.pending_outdoor_reaction_slots.is_empty());
        assert!(state.party.iter().all(|member| member.hp < 40));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn outdoor_walker_predicate_reads_byte_zero_and_excludes_environment_runs() {
        // `formats/tiles.md`: the outdoor classifier is false for `0xB4..0xB7`
        // and `0xE8..0xEB`; `active-objects.md §8` classifies by byte 0.
        for type_byte in [0xb4, 0xb7, 0xe8, 0xeb] {
            let state = world_state_with_walker(5, 5, type_byte, 6, 5);
            assert!(!state.outdoor_active_object_is_adjacent(1), "{type_byte:#x}");
        }
        for type_byte in [0x2c, 0x80, 0xb3, 0xb8, 0xe7, 0xec, 0xff] {
            let state = world_state_with_walker(5, 5, type_byte, 6, 5);
            assert!(state.outdoor_active_object_is_adjacent(1), "{type_byte:#x}");
        }
        let mut state = world_state_with_walker(5, 5, 0x10, 6, 5);
        state.active_objects[1].tile = 0x80;
        assert!(!state.outdoor_active_object_is_adjacent(1));
    }

    #[test]
    fn outdoor_walker_reaction_suppresses_every_lower_slots_movement() {
        // The walk is high-to-low. Slot 2 fires first; the running reaction
        // total then suppresses slot 1's otherwise legal directed step.
        let mut state = world_state_with_walker(5, 5, 0x80, 10, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 0x2c,
            tile: 0x2c,
            x: 8,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        block_projectile_at(&mut state, 7, 5);

        state.advance_outdoor_active_objects();
        state
            .apply_pending_outdoor_reactions(Path::new(""), WorldPlane::Underworld)
            .unwrap();

        assert!(state.message.contains("BOOOM"), "message: {}", state.message);
        assert_eq!((state.active_objects[2].x, state.active_objects[2].y), (8, 5));
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (10, 5));
    }

    #[test]
    fn outdoor_walker_tries_the_other_directed_axis_before_random_fallback() {
        // active-objects.md §8: the fair coin chooses only which directed
        // axis is attempted first. A blocked first axis must not skip the
        // other directed candidate and jump straight to random movement.
        let mut state = world_state_with_walker(12, 12, 0x98, 10, 10);
        let object = state.active_objects[1];
        let [Some(first), Some(second)] = state.outdoor_directed_step_directions(1, object) else {
            panic!("a diagonal offset must produce two directed candidates");
        };
        state.active_objects[1].phase = active_object_phase_from_direction(first, 0);
        let (first_dx, first_dy) = first.delta();
        let first_target = (
            (object.x as isize + first_dx) as usize,
            (object.y as isize + first_dy) as usize,
        );
        let (second_dx, second_dy) = second.delta();
        let second_target = (
            (object.x as isize + second_dx) as usize,
            (object.y as isize + second_dy) as usize,
        );
        state.active_objects.push(ActiveObject {
            type_byte: 0x05,
            tile: 0x05,
            x: first_target.0,
            y: first_target.1,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert!(state.try_wander_active_object(1));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            second_target
        );
    }

    #[test]
    fn carpet_like_outdoor_object_families_use_the_carpet_composite_predicate() {
        // `movement.md §4`: Wanderer/Lord British, Bat, Gazer, Daemon/Dragon
        // and Mongbat use the carpet-like composite predicate, which takes
        // the `0x60..0x6F` glyph/river family and force-rejects chairs.
        for class in [0x74, 0x7c, 0x94, 0xb0, 0xd8, 0xdc, 0xf0] {
            for tile in 0u8..=0xff {
                assert_eq!(
                    outdoor_active_object_step_accepts_tile(class, tile, None),
                    carpet_terrain_accepts(tile),
                    "class {class:#04x} tile {tile:#04x}"
                );
            }
            assert!(outdoor_active_object_step_accepts_tile(class, 0x64, None));
            assert!(!outdoor_active_object_step_accepts_tile(class, 0x90, None));
        }
    }

    #[test]
    fn outdoor_walker_random_fallback_rolls_a_fresh_cardinal_and_writes_no_facing() {
        // `active-objects.md §8`: with no acceptable directed axis the slot
        // "rolls one cardinal direction and makes a single attempt at it";
        // §3: byte 6 "carries **no facing**", so it is neither read for the
        // fallback nor written by the step.
        let directions = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ];
        for seed in 0..32u16 {
            let mut state = world_state_with_walker(12, 10, 0x98, 10, 10);
            // The only directed candidate (East) is occupied.
            state.active_objects.push(ActiveObject {
                type_byte: 0x05,
                tile: 0x05,
                x: 11,
                y: 10,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            });
            // A stored West "facing" must not be replayed.
            state.active_objects[1].phase = 0x60;
            state.prng_state = 0x1357 ^ seed.wrapping_mul(0x9e37);
            // `active-objects.md §8`: the planner first "rolls a fair coin to
            // decide which axis to attempt first" (audit A139), then the
            // fallback's cardinal roll.
            let mut expected_prng = state.prng_state;
            let _axis_coin = u5_prng_range_u16(&mut expected_prng, 0, 1);
            let roll = u5_prng_range_u16(&mut expected_prng, 0, 3) as usize;
            let expected = directions[roll];

            let moved = state.try_wander_active_object(1);

            assert_eq!(state.prng_state, expected_prng, "the axis coin and one cardinal draw");
            assert_eq!(state.active_objects[1].phase, 0x60);
            if expected == Direction::East {
                assert!(!moved);
                assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (10, 10));
            } else {
                let (dx, dy) = expected.delta();
                assert!(moved);
                assert_eq!(
                    (state.active_objects[1].x, state.active_objects[1].y),
                    ((10 + dx) as usize, (10 + dy) as usize)
                );
            }
        }
    }

    #[test]
    fn outdoor_walker_shadowlord_proximity_mask_ages_its_first_aux_byte() {
        // `active-objects.md §8`: a `0xFC` slot at a listed wrapped
        // `(dy, dx)` cell increments its first auxiliary byte; unlisted
        // cells fall through to ordinary directed movement untouched.
        // Listed: (dy 0, dx 3), across the torus seam.
        let mut state = world_state_with_walker(1, 40, 0xfc, 254, 40);
        state.active_objects[1].aux1 = 4;
        state.try_wander_active_object(1);
        assert_eq!(state.active_objects[1].aux1, 5);
        // Unlisted: (dy 0, dx 1) is adjacent, (dy 5, dx 0) is outside.
        for (ox, oy) in [(2, 40), (1, 45)] {
            let mut state = world_state_with_walker(1, 40, 0xfc, ox, oy);
            state.active_objects[1].aux1 = 4;
            state.try_wander_active_object(1);
            assert_eq!(state.active_objects[1].aux1, 4, "({ox}, {oy})");
        }
        // Other classes never touch the byte.
        let mut state = world_state_with_walker(1, 40, 0xf8, 254, 40);
        state.active_objects[1].aux1 = 4;
        state.try_wander_active_object(1);
        assert_eq!(state.active_objects[1].aux1, 4);
        // Issue #298 Q21 / R534: the branch is "taken only for type byte
        // exactly `0xFC` ... `0xFD..0xFF` take the default arm".
        for type_byte in [0xfd, 0xfe, 0xff] {
            let mut state = world_state_with_walker(1, 40, type_byte, 254, 40);
            state.active_objects[1].aux1 = 4;
            state.try_wander_active_object(1);
            assert_eq!(state.active_objects[1].aux1, 4, "{type_byte:#x}");
        }
    }

    #[test]
    fn outdoor_whirlpool_moves_when_its_parity_bit_is_set_and_skips_without_a_draw() {
        // Issue #298 Q2 / `RETRACTIONS.md` R533: bit 0 of record byte 5 is
        // toggled on every dispatch; the slot moves "only on the turns where
        // the byte is non-zero after the toggle; on the turn it becomes zero
        // they take no draw and do not move. A spawned whirlpool starts with
        // byte 5 at zero, so it moves on its first dispatched turn and then
        // on every second one."
        let mut state = world_state(vec![0x01; WORLD_CELLS], 4, 5);
        state.active_objects.truncate(1);
        state.active_objects.push(ActiveObject {
            type_byte: 0xec,
            tile: 0xec,
            x: 100,
            y: 100,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        state.prng_state = 0x2468;
        let mut moves = Vec::new();
        for turn in 0..6 {
            let before = (state.active_objects[1].x, state.active_objects[1].y);
            let stream_before = state.prng_state;
            state.advance_outdoor_active_objects();
            let after = (state.active_objects[1].x, state.active_objects[1].y);
            let expected_parity = if turn % 2 == 0 { 1 } else { 0 };
            assert_eq!(state.active_objects[1].aux1, expected_parity, "turn {turn}");
            if expected_parity == 0 {
                assert_eq!(after, before, "turn {turn}: no move on the clear turn");
                assert_eq!(state.prng_state, stream_before, "turn {turn}: no draw");
            } else {
                assert_ne!(state.prng_state, stream_before, "turn {turn}: the 0..1 draw");
            }
            moves.push(after != before);
        }
        assert!(moves[0] && moves[2] && moves[4], "{moves:?}");
    }

    #[test]
    fn outdoor_walker_shadowlord_counter_has_no_twenty_gate_and_wraps() {
        // Issue #298 Q21 / R534: the slot "increments record byte 5
        // (unsigned, wrapping at 256) and then takes the same directed step
        // it would take anyway ... whatever the counter's value".
        for start in [19u8, 20, 21, 200, 255] {
            let mut state = world_state_with_walker(1, 40, 0xfc, 254, 40);
            let mut baseline = world_state_with_walker(1, 40, 0xf8, 254, 40);
            state.active_objects[1].aux1 = start;
            baseline.prng_state = state.prng_state;
            let moved = state.try_wander_active_object(1);
            let baseline_moved = baseline.try_wander_active_object(1);
            assert_eq!(state.active_objects[1].aux1, start.wrapping_add(1));
            assert_eq!(moved, baseline_moved, "start {start}");
            assert_eq!(
                (state.active_objects[1].x, state.active_objects[1].y),
                (baseline.active_objects[1].x, baseline.active_objects[1].y),
                "start {start}"
            );
        }
    }

    #[test]
    fn outdoor_walker_chance_refusal_does_not_try_the_other_axis() {
        // The terrain chance gate is after validation. Once that gate refuses
        // the first valid directed candidate, the slot's attempt ends; only a
        // genuinely blocked candidate falls through to the second axis.
        let mut state = world_state_with_walker(12, 12, 0x98, 10, 10);
        let object = state.active_objects[1];
        let gate_tile = [0x04, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f]
            .into_iter()
            .find(|tile| {
                outdoor_active_object_step_accepts_tile(
                    object.type_byte,
                    *tile,
                    state.passability.as_ref(),
                ) && terrain_chance_gate_denominator(*tile).is_some()
            })
            .expect("at least one gated tile is accepted by this walker");
        let denominator = u16::from(terrain_chance_gate_denominator(gate_tile).unwrap());
        // The axis coin and then the gate are shared-generator draws, in that
        // order; pick a stream position whose gate draw refuses.
        state.prng_state = (0..=u16::MAX)
            .find(|seed| {
                let mut stream = *seed;
                let _coin = u5_prng_range_u16(&mut stream, 0, 1);
                u5_prng_range_u16(&mut stream, 0, denominator - 1) != 0
            })
            .unwrap();
        let [Some(first), Some(_second)] = state.outdoor_directed_step_directions(1, object) else {
            panic!("a diagonal offset must produce two directed candidates");
        };
        let (dx, dy) = first.delta();
        let first_target = (
            (object.x as isize + dx) as usize,
            (object.y as isize + dy) as usize,
        );
        state.grid[world_cell_index(first_target.0, first_target.1)] = gate_tile;

        assert!(!state.try_wander_active_object(1));
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (10, 10));
    }

    #[test]
    fn outdoor_walker_broadside_survives_the_pass_handlers_own_result_line() {
        // The test above calls `advance_outdoor_active_objects()` directly and
        // so never meets the handler that used to erase the line. In
        // production the epilogue runs inside `advance_turn()`, and every
        // command handler assigns `message` afterwards -- `Pass` assigns
        // "Passed." -- so the boom announcement was written and then thrown
        // away before the player could see it.
        //
        // `text-output.md SECTION 11` settles the composition question and
        // rejects the premise behind it: "**The original has no such slot.**
        // ... **Both lines are emitted, in the order they occur** ... A turn
        // that produces an epilogue announcement *and* a command result shows
        // the announcement first, then the result beneath it."
        let dir = debug_game_dir();
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        block_projectile_at(&mut state, 7, 5);

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        // `Pass` prints no result of its own (`commands.md §8.1` row B), so
        // the slot the handler leaves behind is empty and the announcement
        // survives on the transcript rather than in it.
        assert!(state.message.is_empty());

        let lines: Vec<&str> = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        let boom = lines
            .iter()
            .position(|line| line.contains("BOOOM"))
            .unwrap_or_else(|| panic!("broadside line missing from transcript: {lines:?}"));
        let echo = lines
            .iter()
            .position(|line| *line == "Pass")
            .unwrap_or_else(|| panic!("verb echo missing from transcript: {lines:?}"));
        // The echo opens the turn; the epilogue announcement follows it.
        assert!(echo < boom, "wrong order: {lines:?}");
        // Emitted once, not once per transcribe path.
        assert_eq!(
            lines.iter().filter(|line| line.contains("BOOOM")).count(),
            1,
            "{lines:?}"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn outdoor_walker_broadside_fires_across_the_map_seam() {
        // The window is measured on wrapped deltas, so a pirate three cells
        // away across the 256-cell seam is in range. Raw subtraction would
        // read this slot as 253 cells away and never fire.
        let mut state = world_state_with_walker(1, 7, 0x2D, 254, 7);
        block_projectile_at(&mut state, 0, 7);
        state.advance_outdoor_active_objects();
        state
            .apply_pending_outdoor_reactions(Path::new(""), WorldPlane::Underworld)
            .unwrap();

        assert!(state.message.contains("BOOOM"), "message: {}", state.message);
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (254, 7)
        );
    }

    #[test]
    fn outdoor_walker_leaves_unaligned_water_creature_to_cleanup_movement() {
        // §8's broadside needs the same row or column. A diagonal slot one
        // cell away is closer than the three-cell window but not aligned, so
        // the first phase does not claim it.
        let mut state = world_state_with_walker(5, 5, 0x2C, 6, 6);
        assert!(!state.outdoor_first_phase_ranged_attack(1));

        let mut state = world_state_with_walker(5, 5, 0x2C, 6, 6);
        state.advance_outdoor_active_objects();
        assert!(!state.message.contains("BOOOM"), "message: {}", state.message);
    }

    #[test]
    fn outdoor_walker_broadside_is_out_of_range_beyond_three_cells() {
        // Aligned but four cells away: outside §8's "within three cells".
        let mut state = world_state_with_walker(5, 5, 0x2C, 9, 5);
        assert!(!state.outdoor_first_phase_ranged_attack(1));
    }

    #[test]
    fn outdoor_walker_broadside_connecting_shot_damages_the_whole_party() {
        // `overworld.md §6.2.4`: "On a clear line the attack connects, and
        // the payload below runs." On foot that is the whole-party pass --
        // "Every qualifying member is damaged."
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        state.party = six_member_party(40);
        let report = state
            .outdoor_first_phase_ranged_attack_detail(1)
            .expect("the broadside fires whenever the geometry holds");
        assert_eq!(report.figure, OutdoorRangedAttackFigure::SolidBurst);
        assert_eq!(report.outcome, OutdoorRangedAttackOutcome::Connects);
        let OutdoorImpactAbsorption::PartyDamaged(damage) =
            report.absorption.expect("a clear line runs the payload")
        else {
            panic!("on foot the payload is the whole-party pass");
        };
        assert_eq!(damage.len(), 6);
        for entry in &damage {
            assert!(
                (OUTDOOR_IMPACT_MEMBER_DAMAGE_LOW..=OUTDOOR_IMPACT_MEMBER_DAMAGE_HIGH)
                    .contains(&entry.roll),
                "roll {} outside the closed interval [1, 8]",
                entry.roll
            );
            assert_eq!(state.party[entry.slot].hp, 40 - entry.applied);
        }
    }

    #[test]
    fn spec300_q27_a_connecting_broadside_plays_the_impact_rumble_before_the_damage() {
        // `audio.md §5.3` / `§10.2` and `RETRACTIONS.md` R611 (u5-spec#300
        // Q27, #301 Q3): stage one "plays the impact rumble, a random rumble
        // of step 10, target 3000, pitch 100..2000 Hz, 300 updates", and it
        // is "identical on every stage-one caller (both outdoor ranged
        // attacks, rough seas, each drowning pass)". The broadside played
        // only the damage rumbles.
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        state.party = six_member_party(40);
        let serial = state.sound_effect_serial;
        let report = state.outdoor_first_phase_ranged_attack_detail(1).unwrap();
        assert_eq!(report.outcome, OutdoorRangedAttackOutcome::Connects);
        let mut expected = vec![SoundEffect::RoughSeasImpactRumble];
        expected.extend(std::iter::repeat_n(SoundEffect::DamageRumble, 6));
        assert_eq!(state.sound_effects_after(serial), expected);
    }

    #[test]
    fn outdoor_walker_obstructed_shot_leaves_the_party_untouched() {
        // `overworld.md §6.2.2`: "*Blocked* means the shot stops where it
        // stopped and nothing further happens -- no payload, no message,
        // no state change."
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        state.party = six_member_party(40);
        block_projectile_at(&mut state, 7, 5);
        let report = state.outdoor_first_phase_ranged_attack_detail(1).unwrap();
        assert!(matches!(
            report.outcome,
            OutdoorRangedAttackOutcome::Obstructed { .. }
        ));
        assert!(report.absorption.is_none());
        assert!(state.party.iter().all(|member| member.hp == 40));
    }

    #[test]
    fn outdoor_walker_sea_serpent_breath_fires_on_the_one_in_eight_gate() {
        // `overworld.md §6.2.1`: the breath row is "[t]he slot's type byte
        // **equals** the first frame of the Sea Serpent run (`0x88`) or the
        // first frame of the Dragon run (`0xDC`)". The serpent half is now
        // wired alongside the dragon.
        let mut state = world_state_with_walker(5, 5, 0x88, 8, 5);
        state.party = six_member_party(40);
        block_projectile_at(&mut state, 7, 5);
        // `overworld.md §6.2.1`: the gate is the shared generator's `[0, 7]`
        // draw, so the stream position, not the turn, decides it.
        let seed = (0u16..256)
            .find(|seed| {
                state.prng_state = *seed;
                outdoor_serpent_dragon_triggers(state.outdoor_serpent_dragon_breath_roll(1))
            })
            .expect("a hitting gate roll exists within 256 stream positions");
        state.prng_state = seed;

        let report = state.outdoor_first_phase_ranged_attack_detail(1).unwrap();
        assert_eq!(report.figure, OutdoorRangedAttackFigure::SparkCloud);
        // §6.2.1's announcement column for the breath row is "None".
        assert!(!state.message.contains("BOOOM"), "message: {}", state.message);
    }

    #[test]
    fn outdoor_walker_sea_serpent_sibling_frames_never_breathe() {
        // §6.2.1: "Sibling animation frames `0x89..0x8B` and `0xDD..0xDF`
        // never enter the breath branch."
        for sibling in [0x89u8, 0x8A, 0x8B, 0xDD, 0xDE, 0xDF] {
            let mut state = world_state_with_walker(5, 5, sibling, 8, 5);
            state.party = six_member_party(40);
            for turn in 0u64..64 {
                state.turn = turn;
                assert!(
                    !state.outdoor_first_phase_ranged_attack(1),
                    "frame {sibling:#x} fired on turn {turn}"
                );
            }
        }
    }

    #[test]
    fn outdoor_impact_payload_takes_no_attacker_and_damages_every_living_member() {
        // `overworld.md §6.2.4`: "No attacker identity, sprite byte, class
        // or sentinel participates anywhere on this path". The entry point
        // below takes no arguments at all, which is that fact in the type
        // system rather than in a comment.
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(40);
        // "no active-player selection, no first-living selection, no single
        // randomly chosen target, and no fixed slot."
        state.party[2].status = PARTY_STATUS_DEAD;
        state.party[2].hp = 0;

        let OutdoorImpactAbsorption::PartyDamaged(damage) = state.apply_outdoor_impact() else {
            panic!("on foot the payload is the whole-party pass");
        };
        let slots: Vec<usize> = damage.iter().map(|entry| entry.slot).collect();
        assert_eq!(slots, vec![0, 1, 3, 4, 5]);
        assert_eq!(state.party[2].hp, 0);
    }

    #[test]
    fn outdoor_impact_draws_one_fresh_roll_per_member() {
        // §6.2.4: "One roll per damaged member, not one roll shared between
        // them." A shared roll would make every entry in a pass identical
        // for every seed.
        let mut saw_differing_rolls = false;
        for seed in 0u16..64 {
            let mut state = world_state(open_world_grid(), 5, 5);
            state.party = six_member_party(200);
            state.prng_state = seed;
            let OutdoorImpactAbsorption::PartyDamaged(damage) = state.apply_outdoor_impact() else {
                panic!("on foot the payload is the whole-party pass");
            };
            assert_eq!(damage.len(), 6);
            if damage.iter().any(|entry| entry.roll != damage[0].roll) {
                saw_differing_rolls = true;
            }
        }
        assert!(
            saw_differing_rolls,
            "every seed produced one shared roll for the whole pass"
        );
    }

    #[test]
    fn outdoor_impact_skips_the_dead_marker_and_not_a_living_whitelist() {
        // §6.2.5: "Implement the inequality, not a living-letter
        // whitelist." Sleeping, poisoned and even an unattested letter are
        // all damaged; only the dead marker is skipped.
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(40);
        state.party[0].status = b'G';
        state.party[1].status = b'P';
        state.party[2].status = b'S';
        state.party[3].status = b'C';
        state.party[4].status = b'A';
        state.party[5].status = PARTY_STATUS_DEAD;

        let OutdoorImpactAbsorption::PartyDamaged(damage) = state.apply_outdoor_impact() else {
            panic!("on foot the payload is the whole-party pass");
        };
        let slots: Vec<usize> = damage.iter().map(|entry| entry.slot).collect();
        assert_eq!(slots, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn outdoor_impact_clamps_at_zero_marks_dead_and_clears_the_active_player() {
        // §6.2.4: "if the signed result is zero or below, clamps that word
        // to **zero** and writes the **dead** status letter ... if the
        // member that just died is the currently selected character, writes
        // the published 'none selected' value into the active-player index
        // byte".
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(1);
        state.active_player = Some(3);

        let OutdoorImpactAbsorption::PartyDamaged(damage) = state.apply_outdoor_impact() else {
            panic!("on foot the payload is the whole-party pass");
        };
        assert_eq!(damage.len(), 6);
        for entry in &damage {
            assert!(entry.died);
            assert_eq!(entry.hp_after, 0);
            // The clamp: at most the hit points the member actually had.
            assert_eq!(entry.applied, 1);
            assert_eq!(state.party[entry.slot].status, PARTY_STATUS_DEAD);
        }
        assert_eq!(state.active_player, None);
        // Fields the helper does not write.
        assert!(state.party.iter().all(|member| member.max_hp == 1));
        assert!(state.party.iter().all(|member| member.level == 8));
        assert!(state.party.iter().all(|member| member.mana == 8));
    }

    #[test]
    fn outdoor_impact_leaves_an_unselected_active_player_alone() {
        // Only "the member that just died" clears the byte.
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(200);
        state.active_player = Some(2);
        let _ = state.apply_outdoor_impact();
        assert_eq!(state.active_player, Some(2));
    }

    #[test]
    fn outdoor_impact_hull_bounds_are_closed_and_the_hull_never_reaches_zero() {
        // `vehicles.md §6`: "The ship survives **only** when the roll is
        // strictly less than the hull, and the hull is then reduced by
        // exactly the roll. A roll equal to or greater than the hull
        // destroys the ship." And: "the least it can hold after a survived
        // impact is one."
        assert_eq!(OUTDOOR_IMPACT_HULL_ROLL_LOW, 1);
        assert_eq!(OUTDOOR_IMPACT_HULL_ROLL_HIGH, 30);
        for hull in 0u8..=99 {
            for roll in OUTDOOR_IMPACT_HULL_ROLL_LOW..=OUTDOOR_IMPACT_HULL_ROLL_HIGH {
                match outdoor_impact_hull_outcome(roll, hull) {
                    OutdoorImpactHullOutcome::Absorbed { hull_after } => {
                        assert!(roll < hull, "hull {hull} roll {roll} absorbed");
                        assert_eq!(hull_after, hull - roll);
                        assert!(hull_after >= 1, "hull {hull} roll {roll} fell to zero");
                    }
                    OutdoorImpactHullOutcome::ShipDestroyed => {
                        assert!(roll >= hull, "hull {hull} roll {roll} destroyed");
                    }
                }
            }
        }
    }

    #[test]
    fn outdoor_impact_aboard_a_frigate_spends_hull_and_no_hit_points() {
        // `vehicles.md §6`: "**The hull absorbs the impact entirely: no
        // party member loses hit points while the ship survives.**"
        // A hull above the roll's maximum can never be destroyed.
        for seed in 0u16..32 {
            let mut state = world_state(open_world_grid(), 5, 5);
            state.party = six_member_party(40);
            state.prng_state = seed;
            aboard_frigate(&mut state, OUTDOOR_IMPACT_HULL_ROLL_HIGH + 1, 2);

            let OutdoorImpactAbsorption::HullAbsorbed { roll, hull_after } =
                state.apply_outdoor_impact()
            else {
                panic!("a hull above the roll ceiling always survives");
            };
            assert_eq!(hull_after, OUTDOOR_IMPACT_HULL_ROLL_HIGH + 1 - roll);
            let TransportState::Ship { hull, .. } = state.player.transport else {
                panic!("the party is still aboard");
            };
            assert_eq!(hull, hull_after);
            assert!(state.party.iter().all(|member| member.hp == 40));
        }
    }

    #[test]
    fn outdoor_impact_ship_loss_ladder_takes_the_skiff_rung_first() {
        // `vehicles.md §6`: "**A skiff is aboard.** The party abandons into
        // a skiff, keeping the ship's current facing".
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(40);
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 3;
        // Hull one: every roll in [1, 30] is at or above it.
        aboard_frigate(&mut state, 1, 2);
        state.player.transport = TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST + 2,
            tile: FIRST_PLAYABLE_FRIGATE_TILE + 2,
            sails_hoisted: false,
            hull: 1,
            skiffs: 2,
        };

        let OutdoorImpactAbsorption::ShipDestroyed {
            fallback, drowning, ..
        } = state.apply_outdoor_impact()
        else {
            panic!("hull one is always destroyed");
        };
        assert_eq!(fallback, ShipLossFallback::Skiff);
        assert!(drowning.is_empty());
        let TransportState::Skiff { type_byte, .. } = state.player.transport else {
            panic!("the ladder abandons into a skiff");
        };
        assert_eq!(type_byte, TRANSPORT_MARKER_SKIFF_FIRST + 2);
        // The carpet is untouched while a skiff is aboard.
        assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 3);
        // "no party member loses hit points while the ship survives" does
        // not apply here, but the ladder's first two rungs do no damage of
        // their own either.
        assert!(state.party.iter().all(|member| member.hp == 40));
        let lines = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>();
        assert!(lines.ends_with(&[SHIP_SUNK_MESSAGE, ABANDON_SHIP_MESSAGE]));
    }

    #[test]
    fn outdoor_impact_ship_loss_ladder_falls_back_to_a_carried_carpet() {
        // "**Otherwise, a carpet is in stock.** The party deploys a carried
        // carpet, the carried-carpet count is decremented, and the marker
        // becomes one of the two carpet frames".
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(40);
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 2;
        aboard_frigate(&mut state, 1, 0);

        let OutdoorImpactAbsorption::ShipDestroyed {
            fallback, drowning, ..
        } = state.apply_outdoor_impact()
        else {
            panic!("hull one is always destroyed");
        };
        assert_eq!(fallback, ShipLossFallback::Carpet);
        assert!(drowning.is_empty());
        let TransportState::Carpet { type_byte, .. } = state.player.transport else {
            panic!("the ladder deploys a carpet");
        };
        assert!(CARPET_MARKER_FRAMES.contains(&type_byte), "{type_byte:#x}");
        assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 1);
        assert!(state.party.iter().all(|member| member.hp == 40));
        let lines = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>();
        assert!(lines.ends_with(&[SHIP_SUNK_MESSAGE, ABANDON_SHIP_MESSAGE]));
    }

    #[test]
    fn outdoor_impact_ship_loss_ladder_drowns_the_party_last() {
        // "**Otherwise, the party drowns.** The marker is set to the
        // sprite-suppressed value and the drowning outcome runs." The loop
        // repeats the whole-party pass until the living-member scan reports
        // none remaining.
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(20);
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 0;
        aboard_frigate(&mut state, 1, 0);

        let OutdoorImpactAbsorption::ShipDestroyed {
            fallback, drowning, ..
        } = state.apply_outdoor_impact()
        else {
            panic!("hull one is always destroyed");
        };
        assert_eq!(fallback, ShipLossFallback::Drown);
        assert_eq!(state.player.transport, TransportState::SpriteSuppressed);
        assert!(!drowning.is_empty());
        // Every iteration is one whole-party pass, so it never damages a
        // member the previous pass already killed.
        for pass in &drowning {
            assert!(!pass.is_empty());
        }
        assert!(
            state
                .party
                .iter()
                .all(|member| member.status == PARTY_STATUS_DEAD && member.hp == 0)
        );
        assert!(!state.party_has_drowning_loop_survivor());
        state.flush_staged_narration();
        let lines = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>();
        assert!(lines.ends_with(&[SHIP_SUNK_MESSAGE, DROWNING_MESSAGE]));
    }

    #[test]
    fn ship_loss_drowning_tests_before_it_damages() {
        // "A party that is already entirely dead when the ladder reaches
        // this rung takes no damage at all, because the test comes first."
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(20);
        for member in state.party.iter_mut() {
            member.status = PARTY_STATUS_DEAD;
            member.hp = 0;
        }
        aboard_frigate(&mut state, 1, 0);

        let OutdoorImpactAbsorption::ShipDestroyed { drowning, .. } = state.apply_outdoor_impact()
        else {
            panic!("hull one is always destroyed");
        };
        assert!(drowning.is_empty());
    }

    #[test]
    fn ship_loss_fallback_ladder_order_is_skiff_then_carpet_then_drown() {
        // `vehicles.md §6`: "takes the first option that is available."
        assert_eq!(ship_loss_fallback(1, 1), ShipLossFallback::Skiff);
        assert_eq!(ship_loss_fallback(1, 0), ShipLossFallback::Skiff);
        assert_eq!(ship_loss_fallback(0, 1), ShipLossFallback::Carpet);
        assert_eq!(ship_loss_fallback(0, 0), ShipLossFallback::Drown);
    }

    #[test]
    fn drowning_exit_scan_and_damage_filter_are_deliberately_different() {
        // `vehicles.md §6` "Status domain and exit predicate": imported
        // outside-domain bytes are damaged while a G/P/S member holds the
        // loop open, but do not themselves keep it alive.
        for status in [b'G', b'P', b'S'] {
            assert!(party_member_counts_as_living(status));
            assert!(outdoor_impact_damages_member(status));
        }
        // Ashes and charmed are damaged but do not hold the loop open.
        for status in [b'A', b'C'] {
            assert!(!party_member_counts_as_living(status));
            assert!(outdoor_impact_damages_member(status));
        }
        assert!(!party_member_counts_as_living(PARTY_STATUS_DEAD));
        assert!(!outdoor_impact_damages_member(PARTY_STATUS_DEAD));
    }

    #[test]
    fn imported_status_can_survive_the_drowning_helpers_direct_return() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = vec![
            PartyMember {
                status: b'C',
                hp: 40,
                max_hp: 40,
                ..default_party()[0]
            },
            PartyMember {
                slot: 1,
                status: b'G',
                hp: 1,
                max_hp: 1,
                ..default_party()[0]
            },
        ];

        let passes = state.apply_ship_loss_drowning();

        assert_eq!(passes.len(), 1, "the Good member holds open one pass");
        assert_eq!(state.party[1].status, PARTY_STATUS_DEAD);
        assert_eq!(state.party[1].hp, 0);
        assert_eq!(state.party[0].status, b'C');
        assert!((32..40).contains(&state.party[0].hp));
        assert!(!state.party_has_drowning_loop_survivor());
        assert_eq!(state.party_capability(), PartyCapability::Defeated);
    }

    #[test]
    fn outdoor_damage_closing_repaint_clears_a_selected_sleeping_member() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(40);
        state.party[0].status = b'S';
        state.active_player = Some(0);

        let damage = state.apply_shared_party_damage(0, 1);

        assert_eq!(damage.hp_after, 39);
        assert_eq!(state.party[0].status, b'S');
        assert_eq!(state.active_player, None);
    }

    #[test]
    fn outdoor_impact_party_pass_is_bounded_at_six_slots() {
        // §6.2.4: "The pass's own hard bound is six slots, indices `0..5`."
        assert_eq!(OUTDOOR_IMPACT_PARTY_PASS_SLOT_BOUND, 6);
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party = six_member_party(40);
        let mut extra = state.party[0];
        extra.slot = 6;
        state.party.push(extra);

        let OutdoorImpactAbsorption::PartyDamaged(damage) = state.apply_outdoor_impact() else {
            panic!("on foot the payload is the whole-party pass");
        };
        assert_eq!(damage.len(), 6);
        assert_eq!(state.party[6].hp, 40);
    }

    #[test]
    fn outdoor_impact_broadside_aboard_a_frigate_spends_hull_not_hit_points() {
        // The two attacks share one payload path, so the broadside reaches
        // the same frigate branch the sand trap and the whirlpool do.
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        state.party = six_member_party(40);
        aboard_frigate(&mut state, OUTDOOR_IMPACT_HULL_ROLL_HIGH + 1, 2);

        let report = state.outdoor_first_phase_ranged_attack_detail(1).unwrap();
        assert!(matches!(
            report.absorption,
            Some(OutdoorImpactAbsorption::HullAbsorbed { .. })
        ));
        assert!(state.party.iter().all(|member| member.hp == 40));
    }

    #[test]
    fn outdoor_walker_dragon_breath_fires_on_the_one_in_eight_gate() {
        // §8: "Sea Serpent and Dragon first-frame hostile classes within
        // three cells of the player on **both** axes roll a one-in-eight
        // trigger, and on success loose a breath attack". Only the Dragon
        // first frame 0xDC is wired; the Sea Serpent half is withheld
        // pending cleak/u5-spec#90's byte-identity question.
        let mut state = world_state_with_walker(5, 5, 0xDC, 8, 5);
        block_projectile_at(&mut state, 7, 5);
        // `overworld.md §6.2.1`: the gate is the shared generator's `[0, 7]`
        // draw, so the stream position, not the turn, decides it.
        let seed = (0u16..256)
            .find(|seed| {
                state.prng_state = *seed;
                outdoor_serpent_dragon_triggers(state.outdoor_serpent_dragon_breath_roll(1))
            })
            .expect("a hitting gate roll exists within 256 stream positions");
        state.prng_state = seed;

        assert!(state.outdoor_first_phase_ranged_attack(1));
        // §6.2's breath row announces nothing -- the boom message belongs to
        // the broadside, and the generic "attacked" message to the
        // adjacent-engagement path.
        assert!(!state.message.contains("BOOOM"), "message: {}", state.message);
    }

    #[test]
    fn outdoor_breath_gate_and_axis_coin_are_shared_generator_draws() {
        // `overworld.md §6.1`/§6.2.1 and `active-objects.md §8`: the axis
        // coin and the one-in-eight breath gate come from the shared
        // generator, the breath as an inclusive `[0, 7]` range draw.
        let mut state = world_state_with_walker(5, 5, 0xDC, 8, 5);
        block_projectile_at(&mut state, 7, 5);
        let mut expected = 0x1234u16;
        state.prng_state = expected;
        let roll = u5_prng_range_u16(&mut expected, 0, 7) as u8;
        let fired = state.outdoor_first_phase_ranged_attack_detail(1).is_some();
        assert_eq!(fired, outdoor_serpent_dragon_triggers(roll));
        if !fired {
            assert_eq!(state.prng_state, expected, "a missed gate costs one draw");
        }

        // The same geometry and turn answer differently at different
        // stream positions: no clock/turn hash decides the coin.
        let walker = world_state_with_walker(12, 12, 0x98, 10, 10);
        let object = walker.active_objects[1];
        let firsts: Vec<_> = (0u16..64)
            .map(|seed| {
                let mut probe = walker.clone();
                probe.prng_state = seed;
                probe.outdoor_directed_step_directions(1, object)[0]
            })
            .collect();
        assert!(firsts.contains(&Some(Direction::East)), "the coin must pick X first");
        assert!(firsts.contains(&Some(Direction::South)), "the coin must pick Y first");
        let mut probe = walker.clone();
        let before = probe.prng_state;
        assert!(probe.try_wander_active_object(1));
        assert_ne!(probe.prng_state, before, "the walker consumes the coin draw");
    }

    #[test]
    fn outdoor_walker_blocked_on_both_axes_rolls_a_random_cardinal() {
        // `overworld.md §6.1`: "If neither directed move is legal, the
        // creature instead attempts one randomly chosen cardinal move" - not
        // the slot's stored facing.
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0u16..64 {
            let mut state = world_state_with_walker(12, 12, 0x98, 10, 10);
            // Directed candidates are east (11,10) and south (10,11).
            for (x, y) in [(11usize, 10usize), (10, 11)] {
                state.active_objects.push(ActiveObject {
                    type_byte: 0x05,
                    tile: 0x05,
                    x,
                    y,
                    z: WorldPlane::Underworld.save_floor(),
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                });
            }
            state.active_objects[1].phase = active_object_phase_from_direction(Direction::North, 0);
            state.prng_state = seed;
            if state.try_wander_active_object(1) {
                seen.insert((state.active_objects[1].x, state.active_objects[1].y));
            }
        }
        assert!(seen.contains(&(10, 9)), "north reached: {seen:?}");
        assert!(seen.contains(&(9, 10)), "west reached: {seen:?}");
    }

    #[test]
    fn outdoor_walker_dragon_breath_does_not_fire_when_the_gate_misses() {
        let mut state = world_state_with_walker(5, 5, 0xDC, 8, 5);
        block_projectile_at(&mut state, 7, 5);
        // `overworld.md §6.2.1`: the gate is the shared generator's `[0, 7]`
        // draw, so the stream position, not the turn, decides it.
        let seed = (0u16..256)
            .find(|seed| {
                state.prng_state = *seed;
                !outdoor_serpent_dragon_triggers(state.outdoor_serpent_dragon_breath_roll(1))
            })
            .expect("a missing gate roll exists within 256 stream positions");
        state.prng_state = seed;

        assert!(!state.outdoor_first_phase_ranged_attack(1));
    }

    #[test]
    fn outdoor_walker_dragon_breath_needs_both_axes_within_three() {
        // Four cells away on one axis is outside the window regardless of
        // the gate roll, so the slot is never claimed.
        let mut state = world_state_with_walker(5, 5, 0xDC, 9, 5);
        for turn in 0u64..256 {
            state.turn = turn;
            assert!(!state.outdoor_first_phase_ranged_attack(1), "turn {turn}");
        }
    }

    #[test]
    fn outdoor_walker_first_phase_ignores_off_plane_and_non_walker_slots() {
        // The first phase reuses the walker's own eligibility: current
        // plane, and the outdoor animated/monster predicate.
        let mut state = world_state_with_walker(5, 5, 0x2C, 8, 5);
        state.active_objects[1].z = WorldPlane::Britannia.save_floor();
        assert!(!state.outdoor_first_phase_ranged_attack(1));

        // A byte outside the outdoor walker predicate never fires, even
        // aligned and in range.
        let mut state = world_state_with_walker(5, 5, 0x05, 8, 5);
        assert!(!state.outdoor_first_phase_ranged_attack(1));
    }

    /// `inventory.md §7.1`: the worn regalia complete the `Item: ` row with
    /// `Crown\n\n`, `Amulet\n\n` or `Badge\n\n`, so their result sits under
    /// a blank row. The handler's turn used to flush the line before the
    /// picker's join could open that row (`use-specials/crownrow`, `amulet`).
    #[test]
    fn u_use_worn_regalia_result_sits_under_a_blank_row() {
        for (index, family) in [
            (SPECIAL_ITEM_CROWN_LB_INDEX, "Crown"),
            (SPECIAL_ITEM_AMULET_LB_INDEX, "Amulet"),
            (SPECIAL_ITEM_BLACK_BADGE_INDEX, "Badge"),
        ] {
            let dir = debug_game_dir();
            let mut state = britannia_state(open_world_grid(), 4, 5);
            state.special_items[index] = 1;
            // Every stone buried, so no Moonstone row precedes the Badge.
            for slot in &mut state.moonstone_slots {
                slot.scene = 0;
            }
            handle_play_key_input(&mut state, 'u', "", &dir).unwrap();
            handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
            let rows = message_rows(&state);
            let item = rows
                .iter()
                .position(|row| row == &format!("Item: {family}"))
                .unwrap_or_else(|| panic!("{family}: {rows:?}"));
            assert_eq!(rows[item + 1], "", "{family}: {rows:?}");
            assert!(!rows[item + 2].is_empty(), "{family}: {rows:?}");
            let _ = fs::remove_dir_all(dir);
        }
    }

    /// The Sceptre already placed its line after the turn; the fix above must
    /// not give it a second blank row.
    #[test]
    fn u_use_sceptre_result_keeps_a_single_blank_row() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 1;
        handle_play_key_input(&mut state, 'u', "", &dir).unwrap();
        handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
        let rows = message_rows(&state);
        let item = rows.iter().position(|row| row == "Item: Sceptre").expect("sceptre row");
        assert_eq!(rows[item + 1], "", "{rows:?}");
        assert_eq!(rows[item + 2], "Wielding the", "{rows:?}");
        let _ = fs::remove_dir_all(dir);
    }
    
    /// `hidden-treasures.md §3` (added 2026-10-25, u5-spec#300 Q1): the "Get line
    /// after the leading feed" column for every fixed record, staged and taken
    /// with Get. The body records (`-`) grant nothing: Get skips them
    /// (`containers.md §3`, u5-spec#300 Q2).
    #[test]
    fn spec300_q1_every_fixed_record_gets_its_published_line() {
        let expected: [(usize, Option<&str>); 113] = [
            (0, Some("Mystic Armour!")),
            (1, Some("Mystic Sword!")),
            (2, Some("Mystic Armour!")),
            (3, Some("Mystic Sword!")),
            (4, Some("Mystic Armour!")),
            (5, Some("Mystic Sword!")),
            (6, Some("Mystic Armour!")),
            (7, Some("Mystic Sword!")),
            (8, Some("Mystic Armour!")),
            (9, Some("Mystic Sword!")),
            (10, Some("Mystic Armour!")),
            (11, Some("Mystic Sword!")),
            (12, Some("The plans for the HMS Cape!")),
            (13, Some("9 keys!")),
            (14, Some("5 odd keys!")),
            (15, Some("Glass Sword!")),
            (16, Some("Sword of Chaos!")),
            (17, Some("Jeweled Sword!")),
            (18, Some("1 gem!")),
            (19, Some("Leather Armour!")),
            (20, Some("Leather Armour!")),
            (21, Some("A purple potion!")),
            (22, Some("A black potion!")),
            (23, Some("A white potion!")),
            (24, Some("A scroll: KXC!")),
            (25, Some("A scroll: AT!")),
            (26, Some("10 food!")),
            (27, Some("3 torches!")),
            (28, Some("Ring of Invisibility!")),
            (29, Some("5 food!")),
            (30, Some("Main Gauche!")),
            (31, Some("Ring of Protection!")),
            (32, Some("A scroll: RH!")),
            (33, Some("5 food!")),
            (34, Some("10 food!")),
            (35, Some("Magic Axe!")),
            (36, Some("4 torches!")),
            (37, Some("Club!")),
            (38, Some("A green potion!")),
            (39, Some("A scroll: IQW!")),
            (40, None),
            (41, None),
            (42, None),
            (43, None),
            (44, None),
            (45, Some("Spear!")),
            (46, Some("Silver Sword!")),
            (47, Some("5 keys!")),
            (48, Some("99 gold!")),
            (49, Some("A scroll: RH!")),
            (50, Some("A scroll: RH!")),
            (51, Some("A yellow potion!")),
            (52, Some("A red potion!")),
            (53, Some("A scroll: IMC!")),
            (54, Some("4 gems!")),
            (55, None),
            (56, Some("A yellow potion!")),
            (57, Some("A green potion!")),
            (58, Some("7 keys!")),
            (59, Some("9 torches!")),
            (60, Some("A blue potion!")),
            (61, Some("5 gems!")),
            (62, Some("50 gold!")),
            (63, Some("A white potion!")),
            (64, Some("A scroll: KXC!")),
            (65, Some("Amulet/Turning!")),
            (66, Some("A scroll: AT!")),
            (67, Some("A orange potion!")),
            (68, Some("Leather Armour!")),
            (69, Some("Ring Mail!")),
            (70, Some("Cloth Armour!")),
            (71, Some("Scale Mail!")),
            (72, Some("Long Sword!")),
            (73, Some("7 keys!")),
            (74, Some("Ring of Regeneration!")),
            (75, Some("A green potion!")),
            (76, Some("A scroll: KXC!")),
            (77, Some("A scroll: KXC!")),
            (78, Some("A scroll: IS!")),
            (79, Some("A scroll: AT!")),
            (80, Some("Ring of Protection!")),
            (81, None),
            (82, None),
            (83, None),
            (84, None),
            (85, Some("A scroll: IMC!")),
            (86, Some("Ring of Regeneration!")),
            (87, Some("3 gems!")),
            (88, Some("A scroll: RH!")),
            (89, Some("A scroll: RH!")),
            (90, Some("A scroll: RH!")),
            (91, Some("Ring of Invisibility!")),
            (92, Some("5 keys!")),
            (93, Some("Ankh!")),
            (94, Some("1 key!")),
            (95, Some("A green potion!")),
            (96, Some("A scroll: RH!")),
            (97, Some("Spiked Collar!")),
            (98, Some("A purple potion!")),
            (99, Some("A orange potion!")),
            (100, Some("A scroll: KXC!")),
            (101, Some("Ring of Invisibility!")),
            (102, Some("Spiked Collar!")),
            (103, Some("A white potion!")),
            (104, Some("Club!")),
            (105, Some("Spiked Collar!")),
            (106, Some("A yellow potion!")),
            (107, Some("20 food!")),
            (108, Some("A white potion!")),
            (109, Some("16 food!")),
            (110, Some("13 food!")),
            (111, Some("A black potion!")),
            (112, Some("A scroll: AT!")),
        ];
        for (record, line) in expected {
            let mut state = test_state(open_grid(), 4, 8);
            state
                .active_objects
                .push(ActiveObject::fixed_hidden_treasure_pickup(record, 5, 8, 0));
            let outcome = state.get_fixed_hidden_treasure_pickup_at(5, 8);
            match line {
                Some(line) => {
                    assert_eq!(outcome, Some(MoveOutcome::Got), "record {record}");
                    assert_eq!(state.message, format!("\n{line}"), "record {record}");
                }
                None => assert_eq!(outcome, None, "record {record}"),
            }
        }
    }

    /// `containers.md §3` (added 2026-10-25, u5-spec#300 Q2): Get skips a
    /// staged rotting body or moldy corpse without printing and falls through
    /// to the live-tile cascade, so a body on a crop tile still gives
    /// `Crops picked!`, and the body stays on the cell.
    #[test]
    fn spec300_q2_get_skips_a_staged_body_and_reaches_the_tile_cascade() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_GET_TILE_TABLE_FILE),
            &format!(
                "CASTLE:0 0 2 1 {} {} FOOD 1\n",
                GET_TILE_CROPS_REPLACEMENT, GET_TILE_CROPS_SOURCE,
            ),
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = GET_TILE_CROPS_SOURCE;
        // Record 42 is a rotting body, record 40 a moldy corpse.
        for record in [42usize, 40] {
            let mut state = test_state(grid.clone(), 1, 1);
            state.player.facing = Direction::East;
            state
                .active_objects
                .push(ActiveObject::fixed_hidden_treasure_pickup(record, 2, 1, 0));
            let body = state.active_objects[1];

            assert_eq!(
                state.get_facing_with_game_dir(&dir).unwrap(),
                MoveOutcome::Got,
                "record {record}"
            );
            assert_eq!(state.message, GET_TILE_CROPS_PICKED_LINE, "record {record}");
            assert_eq!(state.active_objects[1], body, "record {record}");
        }
        let _ = fs::remove_dir_all(dir);
    }
