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
    fn town_render_active_objects_do_not_block_visibility_carve() {
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
        assert_ne!(row[4], ' ');
    }

    fn scheduled_npc_test_slot(slot: usize, type_byte: u8, x: u8, y: u8) -> NpcSlot {
        let mut schedule = [0u8; NPC_SCHEDULE_RECORD_LEN];
        schedule[NPC_SCHEDULE_X_OFFSET] = x;
        schedule[NPC_SCHEDULE_Y_OFFSET] = y;
        schedule[NPC_SCHEDULE_Z_OFFSET] = 0;
        schedule[NPC_SCHEDULE_TIME_OFFSET] = 0;
        schedule[NPC_SCHEDULE_TIME_OFFSET + 1] = 6;
        schedule[NPC_SCHEDULE_TIME_OFFSET + 2] = 12;
        schedule[NPC_SCHEDULE_TIME_OFFSET + 3] = 18;
        NpcSlot {
            slot,
            type_byte,
            dialog_id: slot as u8,
            schedule,
            name: Some(format!("slot {slot}")),
        }
    }

    #[test]
    fn scheduled_npc_runtime_skips_slot_zero_even_when_bytes_are_nonzero() {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(SCENE_MOONGLOW).unwrap(),
            floor: 0,
        };
        state.clock = GameClock::new(0, 0).unwrap();
        state.sync_player_object();
        let slots = vec![
            scheduled_npc_test_slot(NPC_SENTINEL_SLOT, 0x10, 4, 4),
            scheduled_npc_test_slot(1, 0x11, 5, 5),
        ];

        let effective = effective_npc_slots(&slots)
            .map(|slot| slot.slot)
            .collect::<Vec<_>>();
        assert_eq!(effective, vec![1]);

        state.load_scheduled_npcs(&slots);

        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].slot, 1);
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (5, 5));
        assert!(state.npc_at_current_floor(4, 4).is_none());
        assert!(state.npc_live_tile_at(4, 4).is_none());
        assert_eq!(state.town_attack_target_at(0, 4, 4), None);
        assert!(state.object_at_current_floor(4, 4).is_none());
        assert!(state
            .active_objects
            .iter()
            .all(|object| object.type_byte != 0x10 || (object.x, object.y) != (4, 4)));
    }

    #[test]
    fn scheduled_npc_existing_active_object_relink_skips_slot_zero() {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(SCENE_MOONGLOW).unwrap(),
            floor: 0,
        };
        state.clock = GameClock::new(0, 0).unwrap();
        state.sync_player_object();
        state
            .active_objects
            .push(npc_active_object(0x10, 4, 4, 0));
        state
            .active_objects
            .push(npc_active_object(0x11, 5, 5, 0));
        let slots = vec![
            scheduled_npc_test_slot(NPC_SENTINEL_SLOT, 0x10, 4, 4),
            scheduled_npc_test_slot(1, 0x11, 5, 5),
        ];

        state.load_scheduled_npcs_from_existing_active_objects(&slots);

        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].slot, 1);
        assert_eq!(state.npcs[0].active_object, Some(2));
        assert!(state.npc_at_current_floor(4, 4).is_none());
        assert!(state.npc_live_tile_at(4, 4).is_none());
        assert_eq!(state.town_attack_target_at(0, 4, 4), None);
    }

    #[test]
    fn sync_player_object_preserves_linked_0xfc_scheduled_npc() {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(SCENE_MOONGLOW).unwrap(),
            floor: 0,
        };
        state.clock = GameClock::new(0, 0).unwrap();
        state.sync_player_object();
        state.load_scheduled_npcs(&[
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            scheduled_npc_test_slot(1, 0xfc, 2, 1),
        ]);
        let active_slot = state.npcs[0].active_object.unwrap();
        assert_eq!(state.active_objects[active_slot].type_byte, 0xfc);

        state.sync_player_object();

        assert_eq!(state.npcs[0].active_object, Some(active_slot));
        assert_eq!(state.active_objects[active_slot].type_byte, 0xfc);
        assert_eq!((state.active_objects[active_slot].x, state.active_objects[active_slot].y), (2, 1));
    }

    #[test]
    fn world_render_line_of_sight_wraps_with_viewport() {
        // A propagation-blocking barrier at the wrapped viewport edge should
        // be visible itself while preventing the carve from reaching cells
        // behind it.
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 254)] = 10;
        grid[world_cell_index(0, 255)] = 10;
        grid[world_cell_index(0, 0)] = 10;
        grid[world_cell_index(0, 1)] = 10;
        grid[world_cell_index(0, 2)] = 10;
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
        let row: Vec<_> = view.lines().nth(3).unwrap().chars().collect();

        // The cell two steps east of the player must be visible (not space).
        assert_ne!(row[4], ' ', "landmark tile must not occlude cell behind it");
    }

    #[test]
    fn world_render_active_objects_do_not_block_visibility_carve() {
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
        assert_ne!(row[4], ' ');
    }

    #[test]
    fn world_render_visibility_carve_stops_at_propagation_blockers() {
        let mut grid = open_world_grid();
        for y in 3..=7 {
            grid[world_cell_index(6, y)] = 0x0a;
        }
        grid[world_cell_index(7, 5)] = 5;
        let mut state = britannia_state(grid, 5, 5);
        state.ambient_light = FULL_DAYLIGHT;

        let view = state.render_text_view(2);
        let row: Vec<_> = view.lines().nth(3).unwrap().chars().collect();

        assert_eq!(row[2], '@');
        assert_ne!(row[3], ' ');
        assert_eq!(row[4], ' ');
    }

    #[test]
    fn world_render_visibility_carve_limits_orthogonal_only_tiles() {
        let mut grid = open_world_grid();
        for y in 2..=8 {
            grid[world_cell_index(7, y)] = 0x0a;
        }
        grid[world_cell_index(8, 5)] = 5;
        grid[world_cell_index(7, 5)] = 0x98;
        let mut state = britannia_state(grid, 5, 5);
        state.ambient_light = FULL_DAYLIGHT;

        let view = state.render_text_view(3);
        let row: Vec<_> = view.lines().nth(4).unwrap().chars().collect();

        assert_eq!(row[3], '@');
        assert_ne!(row[5], ' ');
        assert_eq!(row[6], ' ');
    }

    /// `lighting.md §3` + `visibility.md §5`: the ambient byte is the
    /// squared-distance threshold. Night with no personal light is
    /// `FULL_DARKNESS` (2), which lights exactly the eight cells around
    /// the party — not nothing. A torch raises ambient to its floor of
    /// 18, which fills this 5x5 window (max squared distance 8).
    #[test]
    fn world_render_applies_first_playable_visibility_radius() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.mode_zero_cleanup();
        assert_eq!(state.ambient_light, FULL_DARKNESS);

        let view = state.render_text_view(2);
        let rows: Vec<_> = view.lines().skip(1).take(5).collect();

        assert_eq!(rows.len(), 5);
        assert_eq!(rows[2].chars().nth(2), Some('@'));
        assert_eq!(
            rows.iter()
                .flat_map(|row| row.chars())
                .filter(|ch| !matches!(ch, ' ' | '@'))
                .count(),
            8
        );

        state.torch_counter = 1;
        state.recompute_daylight();
        let lit_view = state.render_text_view(2);
        let lit_rows: Vec<_> = lit_view.lines().skip(1).take(5).collect();

        assert!(lit_rows.iter().any(|row| row.contains(',')));
    }

    #[test]
    fn world_render_ordinary_water_underfoot_keeps_visibility_radius() {
        let mut state = britannia_state(vec![1; WORLD_CELLS], 5, 5);
        state.ambient_light = FULL_DAYLIGHT;
        state.torch_counter = 1;
        state.light_spell_counter = 1;

        let view = state.render_text_view(2);
        let rows: Vec<_> = view.lines().skip(1).take(5).collect();

        assert_eq!(rows[2].chars().nth(2), Some('@'));
        assert!(
            rows.iter()
                .flat_map(|row| row.chars())
                .any(|ch| !matches!(ch, ' ' | '@'))
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
    fn dungeon_ladder_never_inspects_the_cell_it_lands_on() {
        // dungeon-mode.md §13.1: "A climb **never inspects the cell it lands
        // on.** The ladder or pit under the party is treated as proof enough
        // that the destination is reachable, so a climb cannot be blocked by
        // what is on the level above or below." This test previously pinned
        // the opposite - a plain-passage landing refusing the climb - which
        // the spec withdraws; the destination test belongs to the Up/Down
        // level-change spells instead.
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(1, 1, 1)] = 0x10;
        grid[dungeon_cell_index(0, 1, 1)] = 0x00;
        let mut state = dungeon_state(grid, 1, 1, 1);

        assert_eq!(
            state.climb(Path::new(""), ClimbIntent::Up).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel {
                scene: DungeonScene::new(33).unwrap(),
                level: 0,
            })
        );

        assert_eq!(
            state.area,
            Area::Dungeon {
                scene: DungeonScene::new(33).unwrap(),
                level: 0,
            }
        );
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn dungeon_k_non_ladder_reports_public_refusal_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert!(state.handle_dungeon_key('k', Path::new("")).unwrap());

        // `dungeon-mode.md §8.1` Klimb prompts: `Klimb-what?` when the cell
        // has no climbable feature at all.
        assert_eq!(state.message, DUNGEON_KLIMB_WHAT_REFUSAL);
        assert_eq!(state.turn, 0);
    }

    /// `dungeon-mode.md §8`: "Under K-Klimb the pit family behaves as an
    /// ordinary climb-*down* feature: the dispatcher masks the underfoot cell
    /// to its high nibble before any comparison, so the whole `0x6?` family -
    /// not just the exact byte `0x60`, and including the marked/fired variants
    /// - enables the down arm. That arm calls the same level-step helper a
    /// down ladder uses, so the party simply descends one level".
    #[test]
    fn dungeon_k_on_any_pit_byte_descends_one_level() {
        let scene = DungeonScene::new(33).unwrap();
        // Shipped data puts `0x60` at Destard level zero (7, 3) and (1, 7)
        // and Deceit level zero (1, 3), so this route is reachable in
        // ordinary play; the withdrawn reading printed "Not climbable!".
        for pit in [0x60u8, 0x62, 0x68, 0x69, 0x6F] {
            let mut grid = open_dungeon_record();
            grid[dungeon_cell_index(0, 1, 1)] = pit;
            let mut state = dungeon_state(grid, 0, 1, 1);

            assert!(state.handle_dungeon_key('k', Path::new("")).unwrap());
            assert_eq!(
                state.area,
                Area::Dungeon { scene, level: 1 },
                "pit byte 0x{pit:02X} should descend one level"
            );
            assert_eq!(state.turn, 1, "pit byte 0x{pit:02X} should cost a turn");
        }
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
        assert_eq!(up.message, "Klimb-");
        assert_eq!(
            up.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::Klimb)
        );

        assert_eq!(
            handle_play_key_input(&mut up, '<', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(up.area, Area::Dungeon { scene, level: 1 });
        assert_eq!(up.active_objects[0].z, 1);
        assert_eq!(up.turn, 1);
        assert!(up.active_direction_prompt.is_none());

        let mut down_grid = open_dungeon_record();
        down_grid[dungeon_cell_index(2, 1, 1)] = 0x30;
        down_grid[dungeon_cell_index(3, 1, 1)] = 0x10;
        let mut down = dungeon_state(down_grid, 2, 1, 1);

        assert!(down.handle_dungeon_key('k', Path::new("")).unwrap());
        assert_eq!(
            down.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::Klimb)
        );
        assert_eq!(
            handle_play_key_input(&mut down, '>', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(down.area, Area::Dungeon { scene, level: 3 });
        assert_eq!(down.active_objects[0].z, 3);
        assert_eq!(down.turn, 1);
    }

    #[test]
    fn dungeon_ladder_changes_level_and_boundary_up_leaves_the_dungeon() {
        // dungeon-mode.md §13: an up ladder moves Z to Z-1, "or leaves the
        // dungeon when the current level is already zero", through the one
        // shared exit contract of §13.2. This test previously pinned the
        // level-zero up climb as a refusal that stayed in the dungeon.
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 DUNGEON:0
",
        )
        .unwrap();
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

        assert_eq!(
            state.climb(&dir, ClimbIntent::Up).unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeonToWorldPlane {
                scene,
                plane: WorldPlane::Britannia,
            })
        );
        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(
            state.message,
            DUNGEON_EXIT_TO_BRITANNIA_NARRATION
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_bottom_ladder_uses_uniform_underworld_exit_without_sidecar() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 DUNGEON:0\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(7, 1, 1)] = 0x20;
        let mut state = dungeon_state(grid, 7, 1, 1);

        assert_eq!(
            state.climb(&dir, ClimbIntent::Down).unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeonToWorldPlane {
                scene,
                plane: WorldPlane::Underworld,
            })
        );
        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld,
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 1);
        assert_eq!(
            state.message,
            DUNGEON_EXIT_TO_UNDERWORLD_NARRATION
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_bottom_ladder_ignores_retired_deeper_transition_sidecar() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 DUNGEON:0\n",
        )
        .unwrap();
        fs::write(
            dir.join(DUNGEON_DEEPER_TRANSITION_TABLE_FILE),
            "DUNGEON:0 7 1 1 UNDERWORLD 30 40\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(7, 1, 1)] = 0x20;
        let mut state = dungeon_state(grid, 7, 1, 1);
        state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = QUICKNESS_ACTIVE_EFFECT_DURATION;
        state.sail_cadence = 1;

        assert_eq!(
            state.climb(&dir, ClimbIntent::Down).unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeonToWorldPlane {
                scene,
                plane: WorldPlane::Underworld,
            })
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld,
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.active_effect_timing_status(), TimingStatusTag::HalfTime);
        assert_eq!(state.active_effect_tag, Some(QUICKNESS_ACTIVE_EFFECT_TAG));
        assert_eq!(state.sail_cadence, 0);
        assert_eq!(state.active_objects[0].z, WorldPlane::Underworld.save_floor());
        assert_eq!(state.turn, 1);
        assert_eq!(
            state.message,
            DUNGEON_EXIT_TO_UNDERWORLD_NARRATION
        );
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
        // `formats/npc.md` section 6 row `1` and `catalogs/npc-roster.md`
        // section 4: tag `01` is the default-person sentinel, so the linked
        // slot keeps the roster type byte and draws the forced person tile.
        // The old expectation of `192` for both fields pinned the withdrawn
        // "clamp every tag outside `192..=255` to `192`" behaviour.
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: NPC_TYPE_DEFAULT_HUMAN_SPRITE,
                tile: NPC_DEFAULT_PERSON_SPRITE_TILE,
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
                // `formats/npc.md` section 6 row `1`: the default-person
                // sentinel keeps its roster type byte and draws the forced
                // person tile. The withdrawn clamp made both fields `192`.
                type_byte: NPC_TYPE_DEFAULT_HUMAN_SPRITE,
                tile: NPC_DEFAULT_PERSON_SPRITE_TILE,
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

    /// `npc-schedules.md §6` ("Floor classification"): the above/below test
    /// "is a **signed eight-bit** comparison: `0xFF` orders below `0x00`, not
    /// above it", so a waypoint on floor byte `0xFF` is *below* the displayed
    /// floor `0x00` and classifies as state 7. `§8.5` then fixes the marker:
    /// state 7 hunts "`0xC9` (descend link)". The earlier fixture paired a
    /// descend link with a waypoint on floor `1`, which the corrected signed
    /// ordering places *above* the displayed floor — the retracted unsigned
    /// reading.
    ///
    /// `§8.5` also owns what happens once the NPC stands on the link: "When
    /// the gate accepts, the walker writes the NPC's position directly to the
    /// active waypoint's own `(x, y, z)`, caches the waypoint, deactivates the
    /// move queue and returns the state to idle; the NPC leaves the displayed
    /// floor and its sprite is released." There is "no 'paired marker cell on
    /// the destination floor'" and no one-floor-at-a-time climb: the NPC lands
    /// on `(6, 6, 0xFF)`, not on the link cell it departed from.
    #[test]
    fn scheduled_npc_leaving_current_floor_uses_floor_link_before_detaching() {
        let mut grid = npc_open_grid();
        grid[32 + 3] = NPC_FLOOR_LINK_TILE_C9;
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
                schedule: [0, 0, 0, 0, 2, 6, 0, 1, 6, 0, 0, 0xFF, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!(state.npcs[0].active_object, Some(1));
        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        // §8.5, states 6/7: the gate refuses on ordinary ground, so the walker
        // "falls back to the tile-ID search for the matching marker, routes
        // toward it and enters the queue-replay state, so the NPC walks onto
        // the link and passes the gate on a later tick."
        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(
            (state.npcs[0].x, state.npcs[0].y, state.npcs[0].z),
            (3, 1, 0)
        );
        assert_eq!(state.npcs[0].active_object, Some(1));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (3, 1)
        );
        assert!(state.visibility_dirty);

        state.visibility_dirty = false;
        state.advance_npc_schedules();

        assert_eq!(
            (state.npcs[0].x, state.npcs[0].y, state.npcs[0].z),
            (6, 6, 0xFF)
        );
        assert_eq!(state.npcs[0].state, NPC_STATE_IDLE);
        assert_eq!(state.npcs[0].cached_wp, 2);
        assert!(state.npcs[0].move_queue.is_empty());
        assert_eq!(state.npcs[0].active_object, None);
        assert!(state.active_objects[1].is_empty());
        assert!(state.visibility_dirty);
    }

    #[test]
    fn scheduled_npc_arriving_on_current_floor_uses_floor_link_before_waypoint() {
        let mut grid = open_grid();
        grid[6 * 32 + 5] = NPC_FLOOR_LINK_TILE_C8;
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
                schedule: [0, 0, 0, 0, 2, 6, 0, 1, 6, 0, 1, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);
        state.active_objects.push(ActiveObject::empty());
        state.active_objects.push(ActiveObject::empty());

        assert_eq!(state.npcs[0].active_object, None);
        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(state.npcs[0].active_object, Some(1));
        assert_eq!(
            (state.npcs[0].x, state.npcs[0].y, state.npcs[0].z),
            (5, 6, 0)
        );
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                // `formats/npc.md` section 6 row `1`: the default-person
                // sentinel keeps its roster type byte and draws the forced
                // person tile. The withdrawn clamp made both fields `192`.
                type_byte: NPC_TYPE_DEFAULT_HUMAN_SPRITE,
                tile: NPC_DEFAULT_PERSON_SPRITE_TILE,
                x: 5,
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
    fn scheduled_npc_off_floor_to_off_floor_is_placed_at_its_waypoint() {
        let mut state = test_state(npc_open_grid(), 10, 10);
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
                schedule: [0, 0, 0, 0, 2, 6, 0, 1, 6, 1, 1, 2, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!(
            (state.npcs[0].x, state.npcs[0].y, state.npcs[0].z),
            (2, 1, 1)
        );
        assert_eq!(state.npcs[0].active_object, None);
        state.clock = GameClock::new(18, 0).unwrap();
        state.advance_npc_schedules();

        // npc-schedules.md §7: state 8 "is *not* a parked state: the walker
        // resolves it immediately by writing the active waypoint's (x, y, z)
        // straight into the NPC's runtime position, caching the waypoint,
        // deactivating the move queue and returning the state to idle."
        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(
            (state.npcs[0].x, state.npcs[0].y, state.npcs[0].z),
            (6, 6, 2)
        );
        assert_eq!(state.npcs[0].state, NPC_STATE_IDLE);
        assert_eq!(state.npcs[0].cached_wp, 2);
        assert!(state.npcs[0].move_queue.is_empty());
        // "Because neither the old nor the new position is on the displayed
        // floor, no sprite is allocated and nothing is visible."
        assert_eq!(state.npcs[0].active_object, None);
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn scheduled_npc_moves_one_step_after_hour_boundary() {
        let mut state = test_state(npc_open_grid(), 1, 1);
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
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (3, 1)
        );
    }

    #[test]
    fn ordinary_town_turn_defers_scheduler_until_after_underfoot_tail() {
        let dir = debug_game_dir();
        let mut state = test_state(npc_open_grid(), 1, 1);
        state.clock = GameClock::new(17, 59).unwrap();
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ]);

        let turn_before = state.turn;
        state.advance_turn();

        // `town-mode.md §7`: clock/dawn-dusk happen first, while the NPC
        // remains unmoved until waking, tile effects, and the trailing shared
        // status/provision pass have all completed.
        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        assert!(state.pending_town_status_provision_pass);
        assert!(state.pending_town_npc_schedule_pass);

        assert_eq!(
            state
                .apply_town_post_turn_effects_after_turn(turn_before, &dir)
                .unwrap(),
            None
        );
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert!(!state.pending_town_status_provision_pass);
        assert!(!state.pending_town_npc_schedule_pass);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scheduled_npc_continues_waypoint_move_after_boundary_hour() {
        let mut state = test_state(npc_open_grid(), 1, 1);
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
                schedule: [0, 0, 0, 0, 2, 6, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert_eq!(state.npcs[0].state, NPC_STATE_INPLANE_MOVE);
        assert_eq!(state.npcs[0].cached_wp, 1);

        state.clock = GameClock::new(19, 0).unwrap();
        state.advance_npc_schedules();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (4, 1));
        assert_eq!(state.npcs[0].state, NPC_STATE_INPLANE_MOVE);
        assert_eq!(state.npcs[0].cached_wp, 1);
    }

    /// `npc-schedules.md §8.5` ("Which marker a state selects"): "Route
    /// selection is entirely 'nearest reachable cell carrying the selected
    /// marker', measured by the same breadth-first search used for ordinary
    /// movement, so an authored map with several links routes the NPC through
    /// whichever one the search reaches first."
    ///
    /// The two cells flanking the NPC are walled off with mountain `0x0C`,
    /// which `§10` ("Tile passability") lists as an obstacle range: "A set bit
    /// marks the tile id as an obstacle for NPC pathfinding; a clear bit marks
    /// it open." The earlier fixture used tile `0x00` as the wall, which the
    /// published open list (`0x00`, `0x04..0x0B`, ...) says is open ground;
    /// that reading came from the retracted inverted polarity.
    #[test]
    fn scheduled_npc_floor_link_route_uses_single_bfs_over_matching_markers() {
        let mut grid = npc_open_grid();
        grid[5 * 32 + 4] = 0x0C;
        grid[5 * 32 + 6] = 0x0C;
        grid[5 * 32 + 7] = NPC_FLOOR_LINK_TILE_C9;
        grid[8 * 32 + 5] = NPC_FLOOR_LINK_TILE_C9;
        let mut state = test_state(grid, 10, 10);
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
                dialog_id: 0,
                schedule: [0, 0, 0, 5, 5, 5, 5, 5, 5, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ]);

        assert_eq!(
            state.npc_path_step_to_floor_link(0, NPC_FLOOR_LINK_TILE_C9, 5, 8, 0),
            Some((5, 6))
        );
    }

    /// `npc-schedules.md §8.5`: one call "hunts `0xC8` cells or `0xC9`
    /// cells, never both", so the opposite marker is not a goal.
    ///
    /// `§10` ("Tile passability", rule 3) settles the per-step gate: the two
    /// floor-link ids "short-circuit to open without consulting the tile set.
    /// Both ids are already clear in the tile set, so this path changes
    /// nothing about the result; it is a shortcut, not a special case, and it
    /// must not be read as 'floor links are blocked as intermediate cells'."
    /// The earlier assertion here pinned exactly that retracted reading.
    #[test]
    fn scheduled_npc_floor_link_route_ignores_opposite_marker_id() {
        let mut grid = npc_open_grid();
        grid[6 * 32 + 5] = NPC_FLOOR_LINK_TILE_C8;
        grid[4 * 32 + 5] = NPC_FLOOR_LINK_TILE_C9;
        let mut state = test_state(grid, 10, 10);
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
                dialog_id: 0,
                schedule: [0, 0, 0, 5, 5, 5, 5, 5, 5, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ]);

        assert_eq!(
            state.npc_path_step_to_floor_link(0, NPC_FLOOR_LINK_TILE_C9, 5, 4, 0),
            Some((5, 4))
        );
        // §10: "Both floor links are ordinary open ground. They are never
        // obstacles at any point in the pass."
        assert!(state.npc_can_step_toward(0, 5, 4, 0, 5, 5));
        assert!(state.npc_can_step_toward(0, 5, 6, 0, 5, 5));
        assert!(state.npc_can_step_toward_floor_link_marker(
            0,
            5,
            4,
            NPC_FLOOR_LINK_TILE_C9,
            (5, 4),
            0
        ));
    }

    fn approach_and_attack_town_state(player_x: usize, player_y: usize) -> PlayState {
        let mut state = test_state(npc_open_grid(), player_x, player_y);
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
                type_byte: 0x50,
                dialog_id: 0,
                schedule: [4, 4, 4, 9, 9, 9, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        state
    }

    #[test]
    fn hostile_town_npc_chases_player_from_active_waypoint() {
        // `npc-schedules.md §9` value `4`: "when the player is closer than
        // that [four tiles from the waypoint], it enters the engagement path
        // and can raise the town-mode attack event". The waypoint is `(9, 5)`
        // and the party stands two cells from it, so this is the engagement
        // arm and it is behind no coin.
        let mut state = approach_and_attack_town_state(7, 5);

        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (8, 5));
        assert!(state.visibility_dirty);
    }

    #[test]
    fn approach_and_attack_npc_wanders_while_the_player_is_far_from_the_waypoint() {
        // `npc-schedules.md §9` value `4`, **corrected (R317)**: "While the
        // player is four or more tiles from the *waypoint*, the NPC takes the
        // ordinary bounded wander step with the **same constant cap of
        // three** that value `1` uses". The withdrawn reading - "uses the
        // wander step with a shrinking range around the waypoint" - and the
        // engine's own "chase whenever the player is within eight cells of
        // the NPC" both had this NPC stepping toward the party here.
        //
        // The wander arm is behind §9.1's one-in-two coin, so a seed on the
        // losing half makes the distinction observable: an engaging NPC would
        // step, a wandering one spends the turn.
        let far = approach_and_attack_town_state(5, 5);
        assert_eq!(far.town_player_distance_to(9, 5), 4);
        let mut state = approach_and_attack_town_state(5, 5);
        state.prng_state = losing_wander_coin_seed();

        state.advance_npc_schedules();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (9, 5));
    }

    /// `npc-schedules.md §10` ("Tile passability"): NPC pathfinding uses its
    /// own one-bit-per-tile-id resource, where "A set bit marks the tile id as
    /// an obstacle for NPC pathfinding; a clear bit marks it open." The chair
    /// family `0x90..0x93` is on the published open list, and the spec draws
    /// the consequence out: "Chairs are walkable for NPC routing and beds are
    /// not." The NPC therefore takes the direct step east onto the chair.
    ///
    /// The earlier fixture put mountain `0x0C` — a published obstacle range —
    /// in the direct step's path and still expected the step to be taken; that
    /// only holds under the retracted inverted polarity.
    #[test]
    fn scheduled_npc_uses_npc_path_bitmap_for_direct_step() {
        let mut grid = npc_open_grid();
        grid[32 + 3] = 0x90;
        let mut state = test_state(grid, 10, 10);
        state.clock = GameClock::new(17, 59).unwrap();
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ]);

        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (3, 1)
        );
    }

    #[test]
    fn scheduled_npc_dynamic_obstacle_radius_ignores_far_occupant() {
        let mut state = test_state(npc_open_grid(), 10, 10);
        state.clock = GameClock::new(17, 59).unwrap();
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 20, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
            NpcSlot {
                slot: 2,
                type_byte: 1,
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 3, 3, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ]);

        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert_eq!((state.npcs[1].x, state.npcs[1].y), (3, 1));
    }

    #[test]
    fn adjacent_hostile_town_npc_raises_alarm_without_combat() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
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
                type_byte: 0x50,
                dialog_id: 0,
                schedule: [4, 4, 4, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Used
        );

        assert!(!state.combat_active);
        assert!(state.message.contains("Hostile NPC slot 1"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn mode_five_adjacent_attack_requires_live_dialogue_awareness() {
        let mut state = test_state(open_grid(), 5, 5);
        let scene = match state.area {
            Area::Town { scene, .. } => scene,
            _ => unreachable!(),
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
                type_byte: 0x0E,
                dialog_id: NPC_DIALOG_ID_NONE,
                schedule: [5, 5, 5, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(state.town_adjacent_event_npc(scene, 0), None);

        state.npcs[0].dialog_id = 2;
        assert_eq!(
            state.town_adjacent_event_npc(scene, 0),
            Some((1, 0x0E, NpcAiBehavior::ReservedEngage))
        );
    }

    #[test]
    fn adjacent_guard_town_npc_prompts_and_refusal_raises_alarm() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
        let scene = match state.area {
            Area::Town { scene, .. } => scene,
            _ => unreachable!(),
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
                slot: 2,
                type_byte: 0x70,
                dialog_id: 0,
                schedule: [6, 6, 6, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Used
        );
        assert_eq!(
            state.pending_town_arrest,
            Some(TownArrestPrompt {
                scene_byte: scene.byte,
                floor: 0,
                npc_slot: 2
            })
        );

        assert_eq!(
            handle_play_key_input(&mut state, 'n', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.pending_town_arrest.is_none());
        assert!(state.message.contains("Refused surrender"));
        let guard = state.npcs.iter().find(|npc| npc.slot == 2).unwrap();
        assert_eq!(&guard.schedule[..3], &[7, 7, 7]);
        assert_eq!(&guard.schedule[12..16], &[0, 0, 0, 0]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn forced_town_rewrites_preserve_their_published_fields_and_gates() {
        let schedule = [
            1, 2, 4, 9, 10, 11, 12, 13, 14, 0, 1, 2, 6, 12, 18, 22,
        ];
        let mut near = RuntimeNpc::from_slot(
            &NpcSlot {
                slot: 1,
                type_byte: 0x2E,
                dialog_id: 0x22,
                schedule,
                name: None,
            },
            8,
        );
        near.force_town_pursuit();
        assert_eq!(&near.schedule[..3], &[6, 6, 6]);
        assert_eq!(&near.schedule[3..12], &schedule[3..12]);
        assert_eq!(&near.schedule[12..16], &[0, 0, 0, 0]);
        assert_eq!(near.dialog_id, 0x22);

        let mut random = RuntimeNpc::from_slot(
            &NpcSlot {
                slot: 2,
                type_byte: 0x40,
                dialog_id: 0x23,
                schedule,
                name: None,
            },
            8,
        );
        random.force_town_pursuit();
        assert_eq!(&random.schedule[..3], &[7, 7, 7]);

        let mut fleeing = RuntimeNpc::from_slot(
            &NpcSlot {
                slot: 3,
                type_byte: 0x73,
                dialog_id: 0x24,
                schedule,
                name: None,
            },
            8,
        );
        assert!(fleeing.force_town_flight());
        assert_eq!(&fleeing.schedule[..3], &[3, 3, 3]);
        assert_eq!(&fleeing.schedule[3..], &schedule[3..]);
        assert_eq!(fleeing.dialog_id, TOWN_NPC_COWERING_DIALOG_ID);

        let mut rejected = RuntimeNpc::from_slot(
            &NpcSlot {
                slot: 4,
                type_byte: 0x3F,
                dialog_id: TOWN_NPC_BRUSHOFF_DIALOG_ID,
                schedule,
                name: None,
            },
            8,
        );
        assert!(!rejected.force_town_flight());
        assert_eq!(rejected.schedule, schedule);
        assert_eq!(rejected.dialog_id, TOWN_NPC_BRUSHOFF_DIALOG_ID);

        let mut zero_time_schedule = schedule;
        zero_time_schedule[12..16].fill(0);
        let mut warned = RuntimeNpc::from_slot(
            &NpcSlot {
                slot: 5,
                type_byte: 0x40,
                dialog_id: TOWN_NPC_BRUSHOFF_DIALOG_ID,
                schedule: zero_time_schedule,
                name: None,
            },
            8,
        );
        assert!(warned.force_town_flight());
        assert_eq!(&warned.schedule[..3], &[3, 3, 3]);
    }

    #[test]
    fn town_alarm_sweeps_all_floors_with_exact_special_and_draw_routing() {
        let mut state = test_state(open_grid(), 1, 1);
        let scheduled = |slot, type_byte, z| NpcSlot {
            slot,
            type_byte,
            dialog_id: 0x20 + slot as u8,
            schedule: [1, 2, 4, 5, 6, 7, 5, 6, 7, z, z, z, 6, 12, 18, 22],
            name: None,
        };
        let slots = [
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            scheduled(1, SHADOWLORD_ACTOR_TILE, 0),
            scheduled(2, TOWN_NPC_ALARM_LICH_TYPE, 2),
            scheduled(3, TOWN_NPC_ALARM_GUARD_TYPE, 3),
            scheduled(4, 0x50, 0),
            scheduled(5, 0x50, 2),
            scheduled(6, 0x30, 3),
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!(state.town_alarm_sweep_with_draws(&[127, 128, 0]), (3, 1));
        for slot in 1..=3 {
            let npc = state.npcs.iter().find(|npc| npc.slot == slot).unwrap();
            assert_eq!(&npc.schedule[..3], &[7, 7, 7]);
            assert_eq!(&npc.schedule[12..16], &[0, 0, 0, 0]);
        }
        let fled = state.npcs.iter().find(|npc| npc.slot == 4).unwrap();
        assert_eq!(&fled.schedule[..3], &[3, 3, 3]);
        assert_eq!(fled.dialog_id, TOWN_NPC_COWERING_DIALOG_ID);
        let unchanged = state.npcs.iter().find(|npc| npc.slot == 5).unwrap();
        assert_eq!(&unchanged.schedule[..3], &[1, 2, 4]);
        let rejected = state.npcs.iter().find(|npc| npc.slot == 6).unwrap();
        assert_eq!(&rejected.schedule[..3], &[1, 2, 4]);

        assert_eq!(state.town_npc_mutations.len(), 4);
        state.load_scheduled_npcs(&slots);
        let restored = state.npcs.iter().find(|npc| npc.slot == 4).unwrap();
        assert_eq!(&restored.schedule[..3], &[3, 3, 3]);
        assert_eq!(restored.dialog_id, TOWN_NPC_COWERING_DIALOG_ID);
    }

    #[test]
    fn brush_off_contact_rewrites_to_flight_without_raising_an_alarm() {
        let mut state = test_state(open_grid(), 5, 5);
        let scene = match state.area {
            Area::Town { scene, .. } => scene,
            _ => unreachable!(),
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
                type_byte: 0x50,
                dialog_id: TOWN_NPC_BRUSHOFF_DIALOG_ID,
                schedule: [7, 7, 7, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 0, 0, 0],
                name: None,
            },
        ]);

        assert_eq!(
            state.apply_town_npc_contact_event(scene, 0).unwrap(),
            Some(MoveOutcome::Used)
        );
        assert_eq!(state.message, TOWN_NPC_BRUSHOFF_RESPONSE);
        assert_eq!(&state.npcs[0].schedule[..3], &[3, 3, 3]);
        assert_eq!(state.npcs[0].dialog_id, TOWN_NPC_COWERING_DIALOG_ID);
    }

    #[test]
    fn shipped_npc_scheduler_corpus_initializes_and_ticks_when_present() {
        let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
            return;
        };
        let game_dir = game_dir.as_path();
        if !game_dir.join(TOWNE_NPC_FILENAME).exists() {
            return;
        }

        let sample_hours = [0u8, 5, 8, 12, 16, 20, 23];
        let mut loaded_scenes = 0usize;
        let mut runtime_npcs = 0usize;
        let mut linked_npcs = 0usize;
        let mut moved_or_relinked = 0usize;

        for scene_byte in SCENE_TOWN_FAMILY_FIRST..=SCENE_TOWN_FAMILY_LAST {
            let scene = Scene::new(scene_byte).unwrap();
            let tlk = parse_tlk(&game_dir.join(npc_tlk_filename(scene_byte).unwrap())).unwrap();
            let npc_slots = parse_npc_block(game_dir, scene, &tlk).unwrap();
            loaded_scenes += 1;

            for hour in sample_hours {
                let grid = load_town_runtime_floor(game_dir, scene, 0, hour).unwrap();
                let mut state = test_state(
                    grid,
                    usize::from(VIEWPORT_CENTER),
                    usize::from(VIEWPORT_CENTER),
                );
                state.area = Area::Town { scene, floor: 0 };
                state.clock = GameClock::new(hour, 0).unwrap();
                state.sync_player_object();
                state.load_scheduled_npcs(&npc_slots);

                let before = state
                    .npcs
                    .iter()
                    .map(|npc| (npc.slot, npc.x, npc.y, npc.z, npc.active_object))
                    .collect::<Vec<_>>();
                runtime_npcs += before.len();
                linked_npcs += before
                    .iter()
                    .filter(|(_, _, _, _, active_object)| active_object.is_some())
                    .count();

                for npc in &state.npcs {
                    assert_ne!(npc.slot, NPC_SENTINEL_SLOT);
                    assert!(npc.x < TOWN_GRID_SIDE);
                    assert!(npc.y < TOWN_GRID_SIDE);
                    assert!(
                        npc.z <= 7 || npc.z == u8::MAX,
                        "scene {scene_byte} hour {hour} slot {} has unexpected floor byte {}",
                        npc.slot,
                        npc.z
                    );
                    if let Some(active_slot) = npc.active_object {
                        let object = &state.active_objects[active_slot];
                        assert_eq!((object.x, object.y, object.z), (npc.x, npc.y, 0));
                        assert_ne!(object.type_byte, 0);
                    } else {
                        assert!(
                            npc.z != 0 || (npc.x, npc.y) == (state.player.x, state.player.y),
                            "scene {scene_byte} hour {hour} slot {} on current floor was not linked",
                            npc.slot
                        );
                    }
                }

                state.advance_npc_schedules();

                for (slot, old_x, old_y, old_z, old_active_object) in before {
                    let npc = state
                        .npcs
                        .iter()
                        .find(|npc| npc.slot == slot)
                        .unwrap_or_else(|| panic!("scene {scene_byte} lost NPC slot {slot}"));
                    assert!(npc.x < TOWN_GRID_SIDE);
                    assert!(npc.y < TOWN_GRID_SIDE);
                    assert!(
                        npc.z <= 7 || npc.z == u8::MAX,
                        "scene {scene_byte} hour {hour} slot {slot} ticked to unexpected floor byte {}",
                        npc.z
                    );
                    let distance = npc.x.abs_diff(old_x) + npc.y.abs_diff(old_y);
                    if npc.z == old_z {
                        assert!(
                            distance <= 1,
                            "scene {scene_byte} hour {hour} slot {slot} moved {distance} cells"
                        );
                    }
                    if npc.active_object != old_active_object || distance > 0 || npc.z != old_z {
                        moved_or_relinked += 1;
                    }
                    if let Some(active_slot) = npc.active_object {
                        let object = &state.active_objects[active_slot];
                        assert_eq!((object.x, object.y), (npc.x, npc.y));
                        assert_eq!(object.z, npc.z as i8);
                        assert_ne!(object.type_byte, 0);
                    }
                }
            }
        }

        assert_eq!(loaded_scenes, SCENE_TOWN_FAMILY_LAST as usize);
        assert!(runtime_npcs >= 325 * sample_hours.len());
        assert!(linked_npcs > 0);
        assert!(moved_or_relinked > 0);
    }

    fn shipped_npc_boundary_hours(slots: &[NpcSlot]) -> Vec<u8> {
        let mut hours = std::collections::BTreeSet::new();
        hours.extend([0u8, 5, 8, 12, 16, 20, 23]);
        for slot in effective_npc_slots(slots).filter(|slot| npc_type_byte_occupied(slot.type_byte))
        {
            for boundary in slot.schedule[NPC_SCHEDULE_TIME_OFFSET
                ..NPC_SCHEDULE_TIME_OFFSET + NPC_SCHEDULE_TIME_BOUNDARY_COUNT]
                .iter()
                .copied()
            {
                let hour = boundary % HOURS_PER_DAY;
                hours.insert(hour);
                hours.insert((hour + HOURS_PER_DAY - 1) % HOURS_PER_DAY);
                hours.insert((hour + 1) % HOURS_PER_DAY);
            }
        }
        hours.into_iter().collect()
    }

    fn shipped_npc_referenced_floors(slots: &[NpcSlot]) -> Vec<i8> {
        let mut floors = std::collections::BTreeSet::new();
        floors.insert(0i8);
        for slot in effective_npc_slots(slots).filter(|slot| npc_type_byte_occupied(slot.type_byte))
        {
            for z in slot.schedule
                [NPC_SCHEDULE_Z_OFFSET..NPC_SCHEDULE_Z_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT]
                .iter()
                .copied()
            {
                if z <= 7 {
                    floors.insert(z as i8);
                }
            }
        }
        floors.into_iter().collect()
    }

    fn shipped_npc_corpus_player_cell(
        grid: &[u8],
        slots: &[NpcSlot],
        floor: i8,
        hour: u8,
    ) -> (usize, usize) {
        let occupied = effective_npc_slots(slots)
            .filter(|slot| npc_type_byte_occupied(slot.type_byte))
            .filter_map(|slot| {
                let wp = waypoint_for_hour(&slot.schedule, hour);
                (slot.schedule[NPC_SCHEDULE_Z_OFFSET + wp] as i8 == floor).then_some((
                    slot.schedule[NPC_SCHEDULE_X_OFFSET + wp] as usize,
                    slot.schedule[NPC_SCHEDULE_Y_OFFSET + wp] as usize,
                ))
            })
            .collect::<std::collections::BTreeSet<_>>();
        for y in 0..TOWN_GRID_SIDE {
            for x in 0..TOWN_GRID_SIDE {
                if occupied.contains(&(x, y)) {
                    continue;
                }
                if grid
                    .get(y * TOWN_GRID_SIDE + x)
                    .copied()
                    .is_some_and(|tile| !npc_path_tile_obstacle(tile))
                {
                    return (x, y);
                }
            }
        }
        (usize::from(VIEWPORT_CENTER), usize::from(VIEWPORT_CENTER))
    }

    fn assert_shipped_npc_runtime_invariants(
        state: &PlayState,
        scene_byte: u8,
        floor: i8,
        hour: u8,
        tick: usize,
    ) {
        let mut linked_slots = std::collections::BTreeSet::new();
        for npc in &state.npcs {
            assert_ne!(
                npc.slot, NPC_SENTINEL_SLOT,
                "scene {scene_byte} floor {floor} hour {hour} tick {tick} kept slot zero"
            );
            assert!(
                npc_schedule_state_classify(npc.state).is_some(),
                "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {} has invalid scheduler state {}",
                npc.slot,
                npc.state
            );
            assert!(
                npc.x < TOWN_GRID_SIDE,
                "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {} x {} outside town grid",
                npc.slot,
                npc.x
            );
            assert!(
                npc.y < TOWN_GRID_SIDE,
                "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {} y {} outside town grid",
                npc.slot,
                npc.y
            );
            assert!(
                npc.z <= 7 || npc.z == u8::MAX,
                "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {} has unexpected floor byte {}",
                npc.slot,
                npc.z
            );
            assert_ne!(
                (npc.x, npc.y, npc.z as i8),
                (state.player.x, state.player.y, floor),
                "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {} stepped onto the player",
                npc.slot
            );

            if let Some(active_slot) = npc.active_object {
                assert!(
                    linked_slots.insert(active_slot),
                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} active-object slot {active_slot} was linked twice"
                );
                let object = &state.active_objects[active_slot];
                assert_eq!(
                    (object.x, object.y, object.z),
                    (npc.x, npc.y, floor),
                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {} active-object coordinate mismatch",
                    npc.slot
                );
                assert_eq!(
                    npc.z as i8, floor,
                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} off-floor slot {} stayed linked",
                    npc.slot
                );
                assert_ne!(
                    object.type_byte, 0,
                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {} type {:#04x} state {} linked to empty active-object slot {active_slot}",
                    npc.slot,
                    npc.type_byte,
                    npc.state
                );
                if npc_hidden_sprite_slot(scene_byte, npc.slot) {
                    assert_eq!(
                        object.tile, NPC_HIDDEN_SPRITE_TILE,
                        "scene {scene_byte} floor {floor} hour {hour} tick {tick} hidden slot {} did not use transparent tile",
                        npc.slot
                    );
                }
            } else {
                assert!(
                    npc.z as i8 != floor || (npc.x, npc.y) == (state.player.x, state.player.y),
                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} visible slot {} was not linked",
                    npc.slot
                );
            }
        }
    }

    /// `npc-schedules.md §14` ("Rest / time-elapsing command path"): "the
    /// scheduler's contract is only that each call advances each NPC by at
    /// most one cell." That one-cell bound governs *walked* steps, so the
    /// corpus checks it for NPCs that begin and end a tick on the displayed
    /// floor.
    ///
    /// The published placements are not walked steps and are not bounded by
    /// one cell. `§7` state 8: "The NPC is placed directly at the active
    /// waypoint's `(x, y, z)` with no gate; the cached waypoint is updated, the
    /// move queue is deactivated, and the state returns to idle... Because
    /// neither the old nor the new position is on the displayed floor, no
    /// sprite is allocated and nothing is visible; the NPC simply teleports
    /// off-screen to where its schedule says it should be." With the corrected
    /// signed floor ordering of `§6`, shipped rosters reach that state with the
    /// NPC's floor and its waypoint's floor equal and both away from the
    /// displayed floor, so the floor byte does not change across the teleport.
    /// The earlier form of this loop treated any same-floor jump as a
    /// violation, which only held while the classifier's polarity was inverted.
    #[test]
    fn shipped_npc_scheduler_corpus_runs_boundary_routes_when_present() {
        let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
            return;
        };
        let game_dir = game_dir.as_path();
        if !game_dir.join(TOWNE_NPC_FILENAME).exists() {
            return;
        }

        let mut loaded_scenes = 0usize;
        let mut loaded_floor_hours = 0usize;
        let mut runtime_npcs = 0usize;
        let mut linked_hidden_npcs = 0usize;
        let mut state_transitions = 0usize;
        let mut floor_handoffs = 0usize;
        let mut off_floor_placements = 0usize;
        let mut visible_steps = 0usize;

        for scene_byte in SCENE_TOWN_FAMILY_FIRST..=SCENE_TOWN_FAMILY_LAST {
            let scene = Scene::new(scene_byte).unwrap();
            let tlk = parse_tlk(&game_dir.join(npc_tlk_filename(scene_byte).unwrap())).unwrap();
            let npc_slots = parse_npc_block(game_dir, scene, &tlk).unwrap();
            loaded_scenes += 1;

            let hours = shipped_npc_boundary_hours(&npc_slots);
            let floors = shipped_npc_referenced_floors(&npc_slots);
            assert!(
                hours.len() >= 7,
                "scene {scene_byte} did not produce the baseline NPC scheduler hours"
            );

            for floor in floors {
                for hour in hours.iter().copied() {
                    let Ok(grid) = load_town_runtime_floor(game_dir, scene, floor, hour) else {
                        continue;
                    };
                    loaded_floor_hours += 1;
                    let (player_x, player_y) =
                        shipped_npc_corpus_player_cell(&grid, &npc_slots, floor, hour);

                    let mut state = test_state(grid, player_x, player_y);
                    state.area = Area::Town { scene, floor };
                    state.clock = GameClock::new(hour, 59).unwrap();
                    state.sync_player_object();
                    state.load_scheduled_npcs(&npc_slots);

                    runtime_npcs += state.npcs.len();
                    assert_shipped_npc_runtime_invariants(&state, scene_byte, floor, hour, 0);
                    linked_hidden_npcs += state
                        .npcs
                        .iter()
                        .filter(|npc| npc_hidden_sprite_slot(scene_byte, npc.slot))
                        .filter_map(|npc| npc.active_object)
                        .filter(|slot| state.active_objects[*slot].tile == NPC_HIDDEN_SPRITE_TILE)
                        .count();

                    for tick in 1..=48 {
                        let before = state
                            .npcs
                            .iter()
                            .map(|npc| {
                                (
                                    npc.slot,
                                    npc.x,
                                    npc.y,
                                    npc.z,
                                    npc.state,
                                    npc.active_object,
                                )
                            })
                            .collect::<Vec<_>>();

                        state.advance_turn_with_minutes_and_door_tick_and_active_objects(
                            1, false, false,
                        );
                        assert_shipped_npc_runtime_invariants(
                            &state, scene_byte, floor, hour, tick,
                        );

                        for (slot, old_x, old_y, old_z, old_state, old_active_object) in before {
                            let npc = state
                                .npcs
                                .iter()
                                .find(|npc| npc.slot == slot)
                                .unwrap_or_else(|| {
                                    panic!("scene {scene_byte} floor {floor} hour {hour} lost NPC slot {slot}")
                                });
                            let distance = npc.x.abs_diff(old_x) + npc.y.abs_diff(old_y);
                            if npc.z != old_z {
                                // §8.5: the two floor-transition placements
                                // (link arrival, gate hand-off) both change the
                                // floor byte.
                                floor_handoffs += 1;
                            } else if npc.z as i8 == floor && old_z as i8 == floor {
                                // §11/§14: an NPC that starts and ends the tick
                                // on the displayed floor took an ordinary
                                // walked step, and "each call advances each NPC
                                // by at most one cell".
                                assert!(
                                    distance <= 1,
                                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {slot} walked {distance} cells on the displayed floor"
                                );
                            } else if distance > 1 {
                                // §7 state 8: neither end is on the displayed
                                // floor, so "the walker resolves it immediately
                                // by writing the active waypoint's (x, y, z)
                                // straight into the NPC's runtime position...
                                // the NPC simply teleports off-screen to where
                                // its schedule says it should be."
                                let wp = waypoint_for_hour(&npc.schedule, state.clock.hour);
                                assert_eq!(
                                    npc.waypoint_position(wp),
                                    (npc.x, npc.y, npc.z),
                                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {slot} jumped {distance} cells off-floor without landing on its active waypoint"
                                );
                                assert_eq!(
                                    npc.state, NPC_STATE_IDLE,
                                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {slot} ungated placement did not return to idle"
                                );
                                assert!(
                                    npc.active_object.is_none(),
                                    "scene {scene_byte} floor {floor} hour {hour} tick {tick} slot {slot} kept a sprite while off the displayed floor"
                                );
                                off_floor_placements += 1;
                            }
                            if npc.state != old_state {
                                state_transitions += 1;
                            }
                            if distance > 0 || npc.active_object != old_active_object {
                                visible_steps += 1;
                            }
                        }
                    }
                }
            }
        }

        assert_eq!(loaded_scenes, SCENE_TOWN_FAMILY_LAST as usize);
        assert!(loaded_floor_hours >= 100);
        assert!(runtime_npcs >= 325 * 7);
        assert!(state_transitions > 0);
        assert!(floor_handoffs > 0);
        // The shipped rosters do reach §7's ungated placement, so the branch
        // above is exercised rather than vacuously true.
        assert!(off_floor_placements > 0);
        assert!(visible_steps > 0);
        assert!(linked_hidden_npcs > 0);
    }

    #[test]
    fn blackthorn_captive_arrest_enters_audience_and_handoffs_after_answer() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
        // blackthorn.md §4: "If more than one companion is still alive,
        // Blackthorn thanks the player for their honesty and **kills**
        // one companion as 'a merciful death'."
        for slot in 1..=2u8 {
            state.party.push(PartyMember {
                slot,
                class_byte: b'F',
                status: b'G',
                climb_stat: 10,
                mana: 0,
                hp: 30,
                max_hp: 42,
                level: 3,
            });
        }
        let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap();
        state.area = Area::Town { scene, floor: 0 };
        state.pending_town_arrest = Some(TownArrestPrompt {
            scene_byte: BLACKTHORN_CAPTIVE_CELL_SCENE,
            floor: 0,
            npc_slot: 1,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 0x70,
            tile: 0x70,
            x: 6,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.pending_town_arrest.is_none());
        assert!(state.active_blackthorn.is_some());
        // blackthorn.md §3 step 3: "Clear the active-object table so the
        // audience scene can reuse those records as temporary cinematic
        // actors."
        assert_eq!(
            state.active_objects[1].aux3,
            BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER
        );
        assert!(state.message.contains("Blackthorn audience"));
        assert!(state.message.contains("Honesty"));

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "hm", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_blackthorn.is_none());
        // blackthorn.md §8: "Roster removal of an executed companion |
        // Durable and irreversible: record lifted from the party, party
        // count decremented". §5: "The victim is the second living party
        // member (the first living companion behind the Avatar)". The
        // withdrawn reading set a durable per-member jail flag instead
        // and left the roster intact.
        assert_eq!(state.party.len(), 2);
        assert_eq!(state.party[0].slot, 0);
        // blackthorn.md §5: the routine "lifts their roster record out
        // of the party, compacts the remaining records up, and
        // decrements the party count". Compaction is the spec clause;
        // the resulting slot *byte* is the engine's own contract, since
        // `execute_blackthorn_companion` renumbers every survivor
        // (`member.slot = slot as u8`) and `synced_party_roster` would
        // reproduce that numbering on its next call anyway. So the
        // surviving companion is reachable only at slot 1; the earlier
        // pin of 2 encoded a pre-compaction bug, not a published value.
        //
        // A prior pass deleted this citation, having grepped the local
        // `u5-spec` checkout, which is ~93 commits stale and carries a
        // 338-line `blackthorn.md` against HEAD's 560. The sentence is
        // verbatim at HEAD. Verify spec claims against the remote, not
        // that checkout.
        assert_eq!(state.party[1].slot, 1);
        assert!(state.message.contains("merciful death"));
        // blackthorn.md §4: "A correct answer ruins that shrine and
        // costs five points of moral standing."
        assert_ne!(state.shrine_ruin_flags[0], 0);
        assert_eq!(
            state.area,
            Area::Town {
                scene,
                floor: 0
            }
        );
        assert_eq!(
            (state.player.x, state.player.y),
            (
                BLACKTHORN_CAPTIVE_CELL_X as usize,
                BLACKTHORN_CAPTIVE_CELL_Y as usize
            )
        );
        assert!(state.message.contains("Returned to Blackthorn's captive cell"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn blackthorn_audience_uses_miscmsg_opening_when_available() {
        let dir = debug_game_dir();
        let mut miscmsg = Vec::new();
        for index in 0..MISCMSG_DAT_RECORDS {
            if index == 0 {
                miscmsg.extend_from_slice(b"authored capture line");
            } else {
                miscmsg.extend_from_slice(format!("rec{index}").as_bytes());
            }
            miscmsg.push(0);
        }
        fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();

        let mut state = test_state(open_grid(), 5, 5);

        assert_eq!(
            state.begin_blackthorn_audience_capture(&dir).unwrap(),
            Some(MoveOutcome::Used)
        );

        assert!(state.active_blackthorn.is_some());
        assert!(state.message.contains("authored capture line"));
        assert!(state.message.contains("Honesty"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn blackthorn_audience_loads_miscmaps_cutscene_record_zero() {
        let dir = debug_game_dir();
        let mut miscmaps = vec![0; MISCMAPS_CUTSCENE_SECTION_BYTES];
        for row in 0..MISCMAPS_CUTSCENE_ROWS {
            let row_start = row * MISCMAPS_CUTSCENE_ROW_STRIDE;
            for col in 0..MISCMAPS_CUTSCENE_VISIBLE_COLUMNS {
                miscmaps[row_start + col] = (row * 16 + col) as u8;
            }
        }
        fs::write(dir.join(MISCMAPS_DAT_FILE), miscmaps).unwrap();

        let mut state = test_state(open_grid(), 5, 5);

        assert_eq!(
            state.begin_blackthorn_audience_capture(&dir).unwrap(),
            Some(MoveOutcome::Used)
        );

        let map = state
            .blackthorn_audience_map
            .as_ref()
            .expect("audience should retain cutscene map");
        assert_eq!(map.record_index, BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD);
        assert_eq!(map.tile(0, 0), Some(0));
        assert_eq!(map.tile(10, 10), Some(170));

        state
            .apply_blackthorn_captive_cell_handoff(&dir, "done")
            .unwrap();
        assert!(state.blackthorn_audience_map.is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn blackthorn_audience_installs_temporary_actor_slots() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: 10,
            mana: 0,
            hp: 30,
            max_hp: 30,
            level: 2,
        });

        assert_eq!(
            state.begin_blackthorn_audience_capture(&dir).unwrap(),
            Some(MoveOutcome::Used)
        );

        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            let slot = placement.actor.slot_index() as usize;
            let object = state.active_objects[slot];
            let expected_position = match placement.actor {
                BlackthornCutsceneActor::LeftGuard => (1, 5),
                BlackthornCutsceneActor::RightGuard => (9, 9),
                _ => (placement.x, placement.y),
            };
            let expected_actor_byte = match placement.actor {
                BlackthornCutsceneActor::SecondPartyMember => 0x48,
                BlackthornCutsceneActor::SeatedBlackthorn => 0x78,
                _ => placement.tile,
            };
            assert_eq!(object.type_byte, expected_actor_byte);
            assert_eq!(object.tile, expected_actor_byte);
            assert_eq!((object.x, object.y), expected_position);
            assert_eq!(object.aux1, placement.actor.slot_index());
            assert_eq!(object.aux3, BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER);
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn blackthorn_cutscene_beats_sync_actor_slots_and_map_tiles() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: 10,
            mana: 0,
            hp: 30,
            max_hp: 30,
            level: 2,
        });

        state.begin_blackthorn_audience_capture(&dir).unwrap();
        let sound_before = state.sound_effect_serial;
        let vm = state.run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::PerQuestionIntermission);

        assert_eq!(
            state
                .blackthorn_audience_map
                .as_ref()
                .and_then(|map| map.tile(0, 4)),
            Some(BLACKTHORN_LOCKED_DOOR_TILE)
        );
        assert_eq!(
            state.active_objects[BlackthornCutsceneActor::Avatar.slot_index() as usize].y,
            7
        );
        for actor in [
            BlackthornCutsceneActor::SeatedBlackthorn,
            BlackthornCutsceneActor::LeftGuard,
            BlackthornCutsceneActor::RightGuard,
        ] {
            assert!(state.active_objects[actor.slot_index() as usize].is_empty());
        }
        assert_eq!(vm.world_ticks, 29);
        assert_eq!(vm.bios_ticks, 28);
        assert_eq!(vm.stinger_count, 14);
        let effects = state.sound_effects_after(sound_before);
        assert_eq!(effects.len(), usize::from(vm.stinger_count));
        assert!(effects
            .iter()
            .all(|effect| *effect == SoundEffect::BlackthornMovementStinger));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn blackthorn_failed_challenge_beat_drags_victim_and_writes_scene_tiles() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: 10,
            mana: 0,
            hp: 30,
            max_hp: 30,
            level: 2,
        });

        state.begin_blackthorn_audience_capture(&dir).unwrap();
        let vm = state.run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::FailedChallengeReaction);

        assert_eq!(vm.world_ticks, 82);
        assert_eq!(vm.bios_ticks, 81);
        assert_eq!(vm.stinger_count, 22);
        assert!(state.active_objects[BlackthornCutsceneActor::SecondPartyMember.slot_index() as usize]
            .is_empty());
        assert_eq!(
            state
                .blackthorn_audience_map
                .as_ref()
                .and_then(|map| map.tile(5, 7)),
            Some(BLACKTHORN_PENDULUM_TILE)
        );
        assert_eq!(
            state
                .blackthorn_audience_map
                .as_ref()
                .and_then(|map| map.tile(5, 9)),
            Some(BLACKTHORN_HOURGLASS_TILE)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn blackthorn_rescue_restores_party_and_clamps_standing() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(KARMA_DAT_FILE),
            karma_bytes(&[
                "strayed",
                "corrective",
                "potential",
                "praise",
                "destiny",
                "camp-only",
            ]),
        )
        .unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 3, 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'D',
            climb_stat: 10,
            mana: 0,
            hp: 0,
            max_hp: 42,
            level: 3,
        });
        state.moral_standing = 12;
        state.active_effect_tag = Some(AMULET_LB_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = PERMANENT_ACTIVE_EFFECT_DURATION;
        state.torch_counter = 20;
        state.light_spell_counter = 30;
        state.food = 0;
        let prng_before = state.prng_state;
        let sound_serial_before = state.sound_effect_serial;

        assert!(matches!(
            state.apply_blackthorn_rescue_refuge(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
                if scene.byte == BLACKTHORN_RESCUE_HANDOFF_SCENE
        ));

        let dissolves = state.take_pending_map_viewport_dissolves();
        assert_eq!(dissolves.len(), 2);
        assert_eq!(
            dissolves[0].source,
            MapViewportDissolveSource::BlackthornRescueBlack
        );
        assert_eq!(
            dissolves[1].source,
            MapViewportDissolveSource::BlackthornRescuePartyOnBlack {
                cell: BLACKTHORN_RESCUE_PARTY_CELL,
            }
        );
        assert!(dissolves
            .iter()
            .all(|playback| playback.rect == MAP_VIEWPORT_DISSOLVE_RECT
                && playback.copied_pixels == 176 * 176
                && playback.world_ticks_advanced == 0
                && playback.caller_redraws_during_dissolve == 0));

        let rescue = state.take_pending_blackthorn_rescue_playbacks();
        assert_eq!(rescue.len(), 1);
        let rescue = &rescue[0];
        assert_eq!(rescue.party_cell, (5, 5));
        assert_eq!(rescue.party_atlas_index, 0x11c);
        assert_eq!(rescue.software_envelope_count, 6);
        assert_eq!(rescue.guardian_reveals.len(), 2);
        assert_eq!(rescue.guardian_reveals[0].cell, (2, 7));
        assert_eq!(rescue.guardian_reveals[0].atlas_index, 0x5e);
        assert_eq!(rescue.guardian_reveals[1].cell, (8, 7));
        assert_eq!(rescue.guardian_reveals[1].atlas_index, 0x5f);
        assert!(rescue
            .guardian_reveals
            .iter()
            .all(|reveal| reveal.pixel_order.len() == 256
                && reveal.world_tick_after_operations.len() == 31));
        assert_eq!(rescue.spectral_reveal.cell, (5, 2));
        assert_eq!(rescue.spectral_reveal.atlas_index, 0x174);
        assert_eq!(rescue.redraw_count, 4);
        assert_eq!(rescue.bios_waits, vec![4, 4]);
        assert_eq!(rescue.flash_count, 2);
        assert_eq!(rescue.flash_prng_draws, 3_712);
        assert_eq!(
            rescue.persistent_terrain,
            vec![((2, 7), 0x5e), ((8, 7), 0x5f)]
        );
        assert_eq!(
            rescue.persistent_actors,
            vec![((5, 5), PLAYER_TILE), ((5, 2), 0x74)]
        );
        let mut expected_prng = U5Prng::new(prng_before);
        let _ = audio::draw_major_flash_bands(&mut expected_prng);
        let _ = audio::draw_major_flash_bands(&mut expected_prng);
        assert_eq!(state.prng_state, expected_prng.state());
        let effects = state.sound_effects_after(sound_serial_before);
        assert_eq!(effects.len(), 3);
        assert_eq!(effects[0], SoundEffect::BlackthornRescueEnvelopes);
        assert!(effects[1..]
            .iter()
            .all(|effect| matches!(effect, SoundEffect::MajorFlash { .. })));

        assert_eq!(state.moral_standing, BLACKTHORN_RESCUE_STANDING_FLOOR);
        assert_eq!(state.food, 63);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[1].hp, 42);
        assert_eq!(state.active_effect_tag, None);
        assert_eq!(state.active_effect_counter, 0);
        assert_eq!(state.torch_counter, 0);
        assert_eq!(state.light_spell_counter, 0);
        assert_eq!(
            (state.player.x, state.player.y),
            (
                BLACKTHORN_RESCUE_HANDOFF_X as usize,
                BLACKTHORN_RESCUE_HANDOFF_Y as usize
            )
        );
        assert_eq!(state.message, "strayed");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn blackthorn_rescue_uses_top_band_record_four_not_camp_variant() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(KARMA_DAT_FILE),
            karma_bytes(&[
                "strayed",
                "corrective",
                "potential",
                "praise",
                "destiny",
                "camp-only",
            ]),
        )
        .unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 3, 1, 1);
        state.moral_standing = 99;
        state.food = 0x0100;

        assert!(matches!(
            state.apply_blackthorn_rescue_refuge(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
                if scene.byte == BLACKTHORN_RESCUE_HANDOFF_SCENE
        ));

        assert_eq!(state.moral_standing, 99);
        assert_eq!(state.food, 0x0100, "every nonzero Food word is preserved");
        assert_eq!(state.message, "destiny");
        let _ = fs::remove_dir_all(dir);
    }

    /// `npc-schedules.md §10` ("Tile passability"): "A set bit marks the tile
    /// id as an obstacle for NPC pathfinding; a clear bit marks it open." The
    /// mountain range `0x0C..0x0D` is on the published obstacle list, so the
    /// direct step east is refused and the flood fill (§8.4) detours north.
    ///
    /// The earlier fixture walled the NPC in with `0x2C`, which the published
    /// open list (`... 0x2C..0x2D ...`, "wooden planks and cobble") calls open
    /// ground — a fixture that only blocks under the retracted polarity.
    #[test]
    fn scheduled_npc_pathfinds_around_blocked_direct_step() {
        let mut grid = npc_open_grid();
        grid[32 + 1] = 0x0C;
        grid[32 + 3] = 0x0C;
        grid[2 * 32 + 2] = 0x0C;
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
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 0));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (2, 0)
        );
    }

    /// `npc-schedules.md §7` state 3: "once the pathfinder produces a route,
    /// subsequent ticks dequeue and apply, and the pathfinder is not
    /// re-invoked until the queue drains or resets."
    ///
    /// The walls use mountain `0x0C`, a published obstacle range under `§10`
    /// ("A set bit marks the tile id as an obstacle for NPC pathfinding; a
    /// clear bit marks it open"); the earlier fixture used `0x2C`, which that
    /// same section lists as open ground.
    #[test]
    fn scheduled_npc_replays_cached_path_queue_after_pathfind() {
        let mut grid = npc_open_grid();
        grid[32 + 1] = 0x0C;
        grid[32 + 3] = 0x0C;
        grid[2 * 32 + 2] = 0x0C;
        let mut state = test_state(grid, 10, 10);
        state.clock = GameClock::new(17, 59).unwrap();
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ]);

        state.advance_turn();
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 0));
        assert_eq!(state.npcs[0].state, NPC_STATE_REPLAY_QUEUE);
        assert!(!state.npcs[0].move_queue.is_empty());

        state.advance_npc_schedules();

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 0));
        assert_eq!(state.npcs[0].state, NPC_STATE_REPLAY_QUEUE);
    }

    #[test]
    fn scheduled_npc_stuck_counter_resets_blocked_cached_queue() {
        let mut state = test_state(open_grid(), 3, 1);
        state.clock = GameClock::new(17, 59).unwrap();
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
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ]);
        state.clock = GameClock::new(18, 0).unwrap();
        state.npcs[0].state = NPC_STATE_REPLAY_QUEUE;
        state.npcs[0].move_queue = vec![NPC_PATH_DIR_EAST];

        for _ in 0..=NPC_STUCK_REPLAN_THRESHOLD {
            state.advance_npc_schedules();
        }

        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
        assert_eq!(state.npcs[0].state, NPC_STATE_IDLE);
        assert!(state.npcs[0].move_queue.is_empty());
        assert_eq!(state.npcs[0].stuck_counter, 0);
    }

    #[test]
    fn scheduled_npc_routes_around_player_instead_of_stepping_into_player() {
        let mut state = test_state(npc_open_grid(), 3, 1);
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
        state.apply_pending_town_status_provision_pass();
        state.apply_pending_town_object_epilogue();

        assert_ne!((state.npcs[0].x, state.npcs[0].y), (3, 1));
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 0));
    }

    #[test]
    fn hidden_npc_mask_matches_published_scene_slots() {
        // npc-schedules.md §11: the mask is indexed by the ONE-BASED public
        // scene byte, and the shipped data sets bits in exactly four scenes.
        // Yew (scene 4): two of the three rodent-class actors.
        assert!(npc_hidden_sprite_slot(SCENE_YEW, 15));
        assert!(npc_hidden_sprite_slot(SCENE_YEW, 17));
        assert!(!npc_hidden_sprite_slot(SCENE_YEW, 16));

        // Minoc (scene 5): Tactus alone.
        assert!(npc_hidden_sprite_slot(SCENE_MINOC, 1));
        assert!(!npc_hidden_sprite_slot(SCENE_MINOC, 0));
        assert!(!npc_hidden_sprite_slot(SCENE_MINOC, 2));

        // Windemere (scene 28): the keep's entire rodent-class group.
        for slot in 3..=9 {
            assert!(npc_hidden_sprite_slot(SCENE_WINDEMERE, slot), "slot {slot}");
        }
        assert!(!npc_hidden_sprite_slot(SCENE_WINDEMERE, 2));
        assert!(!npc_hidden_sprite_slot(SCENE_WINDEMERE, 10));

        // Stonegate (scene 29): the four bat-class actors only - the three
        // Shadow Lord slots, the daemon, and the Sceptre stay visible.
        for slot in 5..=8 {
            assert!(npc_hidden_sprite_slot(SCENE_STONEGATE, slot), "slot {slot}");
        }
        for slot in [1usize, 2, 3, 4, 9] {
            assert!(!npc_hidden_sprite_slot(SCENE_STONEGATE, slot), "slot {slot}");
        }

        // The retracted zero-based reading claimed Moonglow, Trinsic and the
        // Lycaeum hid slots. §11: "No shipped scene hides a talkable named
        // NPC except Minoc's single row."
        for slot in 0..32 {
            assert!(!npc_hidden_sprite_slot(SCENE_MOONGLOW, slot), "slot {slot}");
            assert!(!npc_hidden_sprite_slot(SCENE_TRINSIC, slot), "slot {slot}");
            assert!(
                !npc_hidden_sprite_slot(SCENE_THE_LYCAEUM, slot),
                "slot {slot}"
            );
        }
    }

    #[test]
    fn hidden_npc_allocates_logical_object_with_transparent_tile() {
        // npc-schedules.md §11: Windemere (public scene 28) hides roster
        // slots 3..=9.
        let mut state = test_state(open_grid(), 3, 5);
        state.area = Area::Town {
            scene: Scene::new(SCENE_WINDEMERE).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::North;
        let slots = vec![
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            NpcSlot {
                slot: 4,
                type_byte: 0xc4,
                dialog_id: 2,
                schedule: [0, 0, 0, 3, 3, 3, 4, 4, 4, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];

        state.load_scheduled_npcs(&slots);

        assert_eq!(state.npcs[0].slot, 4);
        assert_eq!(state.npcs[0].active_object, Some(1));
        assert_eq!(state.active_objects[1].type_byte, 0xc4);
        assert_eq!(state.active_objects[1].tile, NPC_HIDDEN_SPRITE_TILE);
        assert_eq!(
            state.facing_talk_target(),
            Some((2, state.player.x, state.player.y - 1))
        );
        assert!(state.blocking_object_at(3, 4).is_some());
        assert!(state.sight_blocking_object_at_current_floor(3, 4).is_none());
    }

    #[test]
    fn resident_shadowlord_npc_has_fixed_stationary_schedule() {
        let npc = RuntimeNpc::from_resident_shadowlord(31, 15, 9, 17);

        assert_eq!(npc.slot, 31);
        assert_eq!(npc.type_byte, SHADOWLORD_ACTOR_TILE);
        assert_eq!(npc.dialog_id, NPC_DIALOG_ID_NONE);
        assert_eq!((npc.x, npc.y, npc.z), (15, 9, 0));
        assert_eq!(npc.state, NPC_STATE_IDLE);
        assert_eq!(npc.active_object, None);
        for wp in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
            assert_eq!(npc.schedule[NPC_SCHEDULE_AI_OFFSET + wp], 0);
            assert_eq!(npc.waypoint_position(wp), (15, 9, 0));
        }
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
        assert_eq!(
            (
                world.active_objects[0].type_byte,
                world.active_objects[0].tile
            ),
            (PLAYER_TILE, PLAYER_TILE)
        );

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon.clock = GameClock::new(12, 0).unwrap();
        dungeon.load_scheduled_npcs(&slots);

        assert_eq!(dungeon.npcs.len(), 1);
        assert_eq!(dungeon.npcs[0].active_object, None);
        assert_eq!(dungeon.active_objects.len(), 1);
        assert_eq!(
            (
                dungeon.active_objects[0].type_byte,
                dungeon.active_objects[0].tile
            ),
            (PLAYER_TILE, PLAYER_TILE)
        );
    }

    #[test]
    fn pass_turn_advances_clock_and_consumes_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::new(17, 59).unwrap();

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);

        assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
        assert_eq!(state.turn, 1);
    }

