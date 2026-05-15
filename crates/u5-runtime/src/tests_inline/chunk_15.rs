    #[test]
    fn town_render_prefers_lower_active_object_slot_at_same_cell() {
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
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let view = state.render_text_view(1);

        assert!(view.contains("@n"));
        assert!(!view.contains("@v"));
    }

    #[test]
    fn town_render_checks_all_overlapping_objects_for_sight_blocking() {
        let mut state = test_state(open_grid(), 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 16,
            tile: 16,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
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

        let view = state.render_text_view(2);
        let row: Vec<_> = view.lines().nth(3).unwrap().chars().collect();

        assert_eq!(row[2], '@');
        assert_eq!(row[3], '.');
        assert_eq!(row[4], ' ');
    }

    #[test]
    fn world_render_line_of_sight_wraps_with_viewport() {
        // On the overworld only mountain tiles (10..=15) block sight. Wall
        // tile ids 24..=79 are landmark icons (towns, shrines, dwellings)
        // visible from distance, not sight-blocking obstructions.
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 10;
        grid[world_cell_index(1, 0)] = 5;
        let mut state = britannia_state(grid, 255, 0);
        state.ambient_light = FULL_DAYLIGHT;

        let view = state.render_text_view(2);
        let row: Vec<_> = view.lines().nth(3).unwrap().chars().collect();

        assert_eq!(row[2], '@');
        // (0, 0) is the mountain; rendered glyph differs from the open grass.
        assert_ne!(row[3], '.');
        // Cell behind the mountain (with wrap) is occluded.
        assert_eq!(row[4], ' ');
    }

    #[test]
    fn world_render_landmark_tiles_do_not_block_sight() {
        // Regression: tile id 0x35 (53) is a coastal/dwelling icon on the
        // overworld, not a wall. Using it adjacent to the player must not
        // black out cells behind it.
        let mut grid = open_world_grid();
        grid[world_cell_index(6, 5)] = 0x35;
        grid[world_cell_index(7, 5)] = 5;
        let mut state = britannia_state(grid, 5, 5);
        state.ambient_light = FULL_DAYLIGHT;

        let view = state.render_text_view(2);
        let row: Vec<_> = view.lines().nth(2).unwrap().chars().collect();

        // The cell two steps east of the player must be visible (not space).
        assert_ne!(row[4], ' ', "landmark tile must not occlude cell behind it");
    }

    #[test]
    fn world_render_checks_all_overlapping_objects_for_sight_blocking() {
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.ambient_light = FULL_DAYLIGHT;
        state.active_objects.push(ActiveObject {
            type_byte: 16,
            tile: 16,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let view = state.render_text_view(2);
        let row: Vec<_> = view.lines().nth(3).unwrap().chars().collect();

        assert_eq!(row[2], '@');
        assert_eq!(row[3], '.');
        assert_eq!(row[4], ' ');
    }

    #[test]
    fn world_render_applies_first_playable_visibility_radius() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.mode_zero_cleanup();

        let view = state.render_text_view(2);
        let rows: Vec<_> = view.lines().skip(1).take(5).collect();

        assert_eq!(rows.len(), 5);
        assert_eq!(rows[2].chars().nth(2), Some('@'));
        assert_eq!(
            rows.iter()
                .flat_map(|row| row.chars())
                .filter(|ch| !matches!(ch, ' ' | '@'))
                .count(),
            0
        );

        state.torch_counter = 1;
        state.recompute_daylight();
        let lit_view = state.render_text_view(2);
        let lit_rows: Vec<_> = lit_view.lines().skip(1).take(5).collect();

        assert!(lit_rows.iter().any(|row| row.contains(',')));
    }

    #[test]
    fn world_render_water_underfoot_clears_effective_visibility_radius() {
        let mut state = britannia_state(vec![1; WORLD_CELLS], 5, 5);
        state.ambient_light = FULL_DAYLIGHT;
        state.torch_counter = 1;
        state.light_spell_counter = 1;

        let view = state.render_text_view(2);
        let rows: Vec<_> = view.lines().skip(1).take(5).collect();

        assert_eq!(rows[2].chars().nth(2), Some('@'));
        assert_eq!(
            rows.iter()
                .flat_map(|row| row.chars())
                .filter(|ch| !matches!(ch, ' ' | '@'))
                .count(),
            0
        );
        assert_eq!(state.ambient_light, FULL_DAYLIGHT);
        assert_eq!(state.torch_counter, 1);
        assert_eq!(state.light_spell_counter, 1);
    }

    #[test]
    fn light_counters_decay_by_turn_increment() {
        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon.torch_counter = 2;
        dungeon.light_spell_counter = 1;

        assert_eq!(dungeon.step(Direction::East), MoveOutcome::Moved);

        assert_eq!(dungeon.torch_counter, 1);
        assert_eq!(dungeon.light_spell_counter, 0);

        let mut world = world_state(open_world_grid(), 1, 1);
        world.torch_counter = 3;
        world.light_spell_counter = 2;

        assert_eq!(world.step(Direction::East), MoveOutcome::Moved);

        assert_eq!(world.torch_counter, 1);
        assert_eq!(world.light_spell_counter, 0);
    }

    #[test]
    fn dungeon_k_key_climbs_one_way_ladders() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(1, 1, 1)] = 0x10;
        grid[dungeon_cell_index(0, 1, 1)] = 0x20;
        let mut state = dungeon_state(grid, 1, 1, 1);

        assert!(state.handle_dungeon_key('k', Path::new("")).unwrap());

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn dungeon_ladder_rejects_plain_passage_landing_without_turn() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(1, 1, 1)] = 0x10;
        let mut state = dungeon_state(grid, 1, 1, 1);

        assert_eq!(
            state.climb(Path::new(""), ClimbIntent::Up).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(
            state.area,
            Area::Dungeon {
                scene: DungeonScene::new(33).unwrap(),
                level: 1,
            }
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Blocked!");
    }

    #[test]
    fn dungeon_k_non_ladder_reports_public_refusal_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert!(state.handle_dungeon_key('k', Path::new("")).unwrap());

        assert_eq!(state.message, "Not climbable!");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_two_way_ladder_keys_choose_up_or_down() {
        let scene = DungeonScene::new(33).unwrap();
        let mut up_grid = open_dungeon_record();
        up_grid[dungeon_cell_index(2, 1, 1)] = 0x30;
        up_grid[dungeon_cell_index(1, 1, 1)] = 0x20;
        let mut up = dungeon_state(up_grid, 2, 1, 1);

        assert!(up.handle_dungeon_key('k', Path::new("")).unwrap());
        assert_eq!(up.area, Area::Dungeon { scene, level: 2 });
        assert_eq!(up.turn, 0);
        assert!(up.message.contains("Two-way ladder"));

        assert!(up.handle_dungeon_key('<', Path::new("")).unwrap());

        assert_eq!(up.area, Area::Dungeon { scene, level: 1 });
        assert_eq!(up.active_objects[0].z, 1);
        assert_eq!(up.turn, 1);

        let mut down_grid = open_dungeon_record();
        down_grid[dungeon_cell_index(2, 1, 1)] = 0x30;
        down_grid[dungeon_cell_index(3, 1, 1)] = 0x10;
        let mut down = dungeon_state(down_grid, 2, 1, 1);

        assert!(down.handle_dungeon_key('>', Path::new("")).unwrap());

        assert_eq!(down.area, Area::Dungeon { scene, level: 3 });
        assert_eq!(down.active_objects[0].z, 3);
        assert_eq!(down.turn, 1);
    }

    #[test]
    fn dungeon_ladder_changes_level_and_boundary_up_stays_in_dungeon() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(3, 1, 1)] = 0x10;
        grid[dungeon_cell_index(2, 1, 1)] = 0x20;
        grid[dungeon_cell_index(0, 1, 1)] = 0x10;
        let mut state = dungeon_state(grid, 3, 1, 1);

        assert_eq!(
            state.climb(Path::new(""), ClimbIntent::Up).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 2 })
        );
        assert_eq!(state.area, Area::Dungeon { scene, level: 2 });
        assert_eq!(state.active_objects[0].z, 2);

        assert_eq!(
            state.climb(Path::new(""), ClimbIntent::Down).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 3 })
        );
        state.area = Area::Dungeon { scene, level: 0 };
        state.sync_player_object();
        let turn_before_missing_return = state.turn;

        assert_eq!(
            state.climb(Path::new(""), ClimbIntent::Up).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, turn_before_missing_return);
        assert_eq!(state.message, "Blocked!");
    }

    #[test]
    fn dungeon_bottom_ladder_without_clean_deeper_transition_refuses_without_turn() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(7, 1, 1)] = 0x20;
        let mut state = dungeon_state(grid, 7, 1, 1);

        assert_eq!(
            state.climb(Path::new(""), ClimbIntent::Down).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(
            state.area,
            Area::Dungeon {
                scene: DungeonScene::new(33).unwrap(),
                level: 7,
            }
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Blocked!");
    }

    #[test]
    fn dungeon_bottom_ladder_ignores_clean_deeper_transition_table() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_DEEPER_TRANSITION_TABLE_FILE),
            "DUNGEON:0 7 1 1 UNDERWORLD 30 40\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(7, 1, 1)] = 0x20;
        let mut state = dungeon_state(grid, 7, 1, 1);
        state.timing_status = TimingStatusTag::HalfTime;
        state.sail_cadence = 1;
        state.sail_stall_pending = true;

        assert_eq!(
            state.climb(&dir, ClimbIntent::Down).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(
            state.area,
            Area::Dungeon { scene, level: 7 }
        );
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.timing_status, TimingStatusTag::HalfTime);
        assert_eq!(state.sail_cadence, 1);
        assert!(state.sail_stall_pending);
        assert_eq!(state.active_objects[0].z, 7);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Blocked!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scheduled_npc_linking_uses_current_hour_and_floor() {
        let mut state = test_state(open_grid(), 1, 1);
        let slots = vec![
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
                dialog_id: 0,
                schedule: [0, 0, 0, 3, 4, 5, 6, 7, 8, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
            NpcSlot {
                slot: 2,
                type_byte: 1,
                dialog_id: 0,
                schedule: [0, 0, 0, 9, 10, 11, 12, 13, 14, 1, 1, 1, 8, 12, 18, 22],
                name: None,
            },
        ];

        state.clock = GameClock::new(18, 0).unwrap();
        state.load_scheduled_npcs(&slots);

        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 192,
                tile: 192,
                x: 5,
                y: 8,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
    }

    #[test]
    fn saved_town_active_object_slots_relink_matching_scheduled_npcs() {
        let mut state = test_state(open_grid(), 1, 1);
        state.active_objects.push(npc_active_object(1, 2, 1, 0));
        let slots = vec![
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
                dialog_id: 42,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];

        state.load_scheduled_npcs_from_existing_active_objects(&slots);

        assert_eq!(state.npcs[0].active_object, Some(1));
        assert_eq!(state.npcs[0].dialog_id, 42);
        assert_eq!(state.active_objects[1], npc_active_object(1, 2, 1, 0));
    }

    #[test]
    fn scheduled_npc_relink_preserves_active_object_slots_without_compacting() {
        let mut state = test_state(open_grid(), 1, 1);
        state.active_objects.push(ActiveObject::empty());
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 8,
            y: 8,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject::empty());
        let mut schedule = [0; 16];
        for wp in 0..3 {
            schedule[3 + wp] = 4;
            schedule[6 + wp] = 5;
            schedule[9 + wp] = 0;
        }
        let slots = vec![
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
                dialog_id: 0,
                schedule,
                name: None,
            },
        ];

        state.load_scheduled_npcs(&slots);

        assert_eq!(state.active_objects.len(), 4);
        assert_eq!(state.npcs[0].active_object, Some(1));
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 192,
                tile: 192,
                x: 4,
                y: 5,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        assert!(state.active_objects[2].is_empty());
        assert!(state.active_objects[3].is_empty());
    }

    #[test]
    fn scheduled_npc_leaving_current_floor_zeroes_active_object_slot() {
        let mut state = test_state(open_grid(), 10, 10);
        state.clock = GameClock::new(17, 59).unwrap();
        let slots = vec![
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 6, 0, 1, 6, 0, 0, 1, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!(state.npcs[0].active_object, Some(1));
        state.advance_turn();

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(
            (state.npcs[0].x, state.npcs[0].y, state.npcs[0].z),
            (6, 6, 1)
        );
        assert_eq!(state.npcs[0].active_object, None);
        assert!(state.active_objects[1].is_empty());
        assert!(state.visibility_dirty);
    }

    #[test]
    fn scheduled_npc_arriving_on_current_floor_allocates_first_empty_slot() {
        let mut state = test_state(open_grid(), 10, 10);
        state.clock = GameClock::new(17, 59).unwrap();
        let slots = vec![
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 6, 0, 1, 6, 0, 1, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);
        state.active_objects.push(ActiveObject::empty());
        state.active_objects.push(ActiveObject::empty());

        assert_eq!(state.npcs[0].active_object, None);
        state.advance_turn();

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(state.npcs[0].active_object, Some(1));
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 192,
                tile: 192,
                x: 6,
                y: 6,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        assert!(state.active_objects[2].is_empty());
        assert!(state.visibility_dirty);
    }

    #[test]
    fn scheduled_npc_moves_one_step_after_hour_boundary() {
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::new(17, 59).unwrap();
        let slots = vec![
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        state.advance_turn();

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (3, 1)
        );
    }

    #[test]
    fn scheduled_npc_pathfinds_around_blocked_direct_step() {
        let mut grid = open_grid();
        grid[1 * 32 + 1] = 24;
        grid[1 * 32 + 3] = 24;
        grid[2 * 32 + 2] = 24;
        let mut state = test_state(grid, 10, 10);
        state.clock = GameClock::new(17, 59).unwrap();
        let slots = vec![
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        state.advance_turn();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 0));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (2, 0)
        );
    }

    #[test]
    fn scheduled_npc_routes_around_player_instead_of_stepping_into_player() {
        let mut state = test_state(open_grid(), 3, 1);
        state.clock = GameClock::new(17, 59).unwrap();
        let slots = vec![
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        state.advance_turn();

        assert_ne!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 0));
    }

    #[test]
    fn player_phantom_npc_is_stationary_and_idempotent() {
        let mut state = test_state(open_grid(), 3, 4);

        state.attach_player_phantom_npc();
        state.attach_player_phantom_npc();

        assert_eq!(state.npcs.len(), 1);
        let phantom = state
            .npcs
            .iter()
            .find(|npc| npc.is_player_phantom())
            .unwrap();
        assert_eq!(phantom.slot, PLAYER_NPC_SLOT);
        assert_eq!(phantom.type_byte, PLAYER_NPC_SENTINEL_TYPE);
        assert_eq!(phantom.dialog_id, PLAYER_NPC_DIALOG_ID);
        assert_eq!((phantom.x, phantom.y, phantom.z), (3, 4, 0));
        let active_slot = phantom.active_object.unwrap();
        assert_ne!(active_slot, 0);
        assert_eq!(
            state.active_objects[active_slot],
            player_phantom_active_object(3, 4, 0)
        );
        for wp in 0..3 {
            assert_eq!(phantom.waypoint_position(wp), (3, 4, 0));
        }

        state.clock = GameClock::new(17, 59).unwrap();
        state.advance_turn();

        let phantom = state
            .npcs
            .iter()
            .find(|npc| npc.is_player_phantom())
            .unwrap();
        assert_eq!((phantom.x, phantom.y, phantom.z), (3, 4, 0));
        assert_eq!(phantom.active_object, Some(active_slot));
        assert_eq!(
            state.active_objects[active_slot],
            player_phantom_active_object(3, 4, 0)
        );
        assert_eq!(state.active_objects.len(), 2);
    }

    #[test]
    fn player_phantom_active_object_is_logical_only_for_surface_interaction() {
        let mut state = test_state(open_grid(), 3, 1);
        state.ambient_light = FULL_DAYLIGHT;
        state.attach_player_phantom_npc();
        let active_slot = state
            .npcs
            .iter()
            .find(|npc| npc.is_player_phantom())
            .and_then(|npc| npc.active_object)
            .unwrap();

        state.player.x = 4;
        state.player.y = 1;
        state.sync_player_object();

        assert_eq!(
            state.active_objects[active_slot],
            player_phantom_active_object(3, 1, 0)
        );
        assert!(state.object_at_current_floor(3, 1).is_none());
        assert!(state.blocking_object_at(3, 1).is_none());
        assert!(state.sight_blocking_object_at_current_floor(3, 1).is_none());
        assert!(
            state
                .npc_at_current_floor(3, 1)
                .is_some_and(|npc| npc.is_player_phantom())
        );

        let view = state.render_text_view(1);
        assert!(view.lines().nth(2).unwrap().contains(".@."));
    }

    #[test]
    fn player_phantom_npc_blocks_npc_pathing_after_player_leaves_spawn() {
        let mut state = test_state(open_grid(), 3, 1);
        state.clock = GameClock::new(17, 59).unwrap();
        let slots = vec![
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);
        state.attach_player_phantom_npc();
        state.player.x = 10;
        state.player.y = 10;
        state.sync_player_object();

        state.advance_turn();

        let real_npc = state
            .npcs
            .iter()
            .find(|npc| !npc.is_player_phantom())
            .unwrap();
        assert_eq!((real_npc.x, real_npc.y), (2, 0));
        assert!(
            state
                .npc_at_current_floor(3, 1)
                .is_some_and(|npc| npc.is_player_phantom())
        );
    }

    #[test]
    fn npc_schedules_do_not_advance_outside_town_modes() {
        fn moving_slots() -> Vec<NpcSlot> {
            vec![
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
                    dialog_id: 0,
                    schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                    name: None,
                },
            ]
        }

        let mut world = world_state(open_world_grid(), 10, 10);
        world.clock = GameClock::new(17, 59).unwrap();
        world.load_scheduled_npcs(&moving_slots());
        world.active_objects.truncate(1);
        world.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        world.npcs[0].active_object = Some(1);
        assert_eq!((world.npcs[0].x, world.npcs[0].y), (2, 1));

        assert_eq!(world.pass_turn(), MoveOutcome::Passed);

        assert_eq!(world.clock, GameClock::new(18, 1).unwrap());
        assert_eq!((world.npcs[0].x, world.npcs[0].y), (2, 1));
        assert_eq!(
            (world.active_objects[1].x, world.active_objects[1].y),
            (2, 1)
        );

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon.clock = GameClock::new(17, 59).unwrap();
        dungeon.load_scheduled_npcs(&moving_slots());
        dungeon.active_objects.truncate(1);
        dungeon.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        dungeon.npcs[0].active_object = Some(1);
        assert_eq!((dungeon.npcs[0].x, dungeon.npcs[0].y), (2, 1));

        assert_eq!(dungeon.pass_turn(), MoveOutcome::Passed);

        assert_eq!(dungeon.clock, GameClock::new(18, 0).unwrap());
        assert_eq!((dungeon.npcs[0].x, dungeon.npcs[0].y), (2, 1));
        assert_eq!(
            (dungeon.active_objects[1].x, dungeon.active_objects[1].y),
            (2, 1)
        );
    }

    #[test]
    fn npc_schedule_loading_does_not_link_active_objects_outside_town_modes() {
        let slots = vec![
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];

        let mut world = world_state(open_world_grid(), 10, 10);
        world.clock = GameClock::new(12, 0).unwrap();
        world.load_scheduled_npcs(&slots);

        assert_eq!(world.npcs.len(), 1);
        assert_eq!(world.npcs[0].active_object, None);
        assert_eq!(world.active_objects.len(), 1);
        assert!(world.active_objects[0].is_player());

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon.clock = GameClock::new(12, 0).unwrap();
        dungeon.load_scheduled_npcs(&slots);

        assert_eq!(dungeon.npcs.len(), 1);
        assert_eq!(dungeon.npcs[0].active_object, None);
        assert_eq!(dungeon.active_objects.len(), 1);
        assert!(dungeon.active_objects[0].is_player());
    }

    #[test]
    fn pass_turn_advances_clock_and_consumes_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::new(17, 59).unwrap();

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(state.turn, 1);
    }

