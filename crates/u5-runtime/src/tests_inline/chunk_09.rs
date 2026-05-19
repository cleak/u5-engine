    #[test]
    fn moongate_prompt_yes_teleports_without_extra_turn() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 0, 0);
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;
        state.moongates.push(MoongateEntry {
            x: 1,
            y: 0,
            destination_plane: WorldPlane::Britannia,
            destination_x: 30,
            destination_y: 40,
            active_hours: None,
            expected_tile: None,
        });

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);
        assert_eq!(
            state.resolve_moongate_prompt('y', &dir).unwrap(),
            Some(MoveOutcome::Transition(
                AreaTransition::MoongateTeleported {
                    from: WorldPlane::Britannia,
                    to: WorldPlane::Britannia,
                }
            ))
        );

        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(state.active_objects[0].x, 30);
        assert_eq!(state.turn, 1);
        assert_eq!(state.pending_moongate, None);
        assert!(state.visibility_dirty);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_render_overlays_active_moongate_origin_and_destination_without_mutating_grid() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.ambient_light = FULL_DAYLIGHT;
        state.moongates.push(MoongateEntry {
            x: 0,
            y: 1,
            destination_plane: WorldPlane::Britannia,
            destination_x: 2,
            destination_y: 1,
            active_hours: None,
            expected_tile: None,
        });

        let view = state.render_text_view(1);

        assert!(view.lines().nth(2).unwrap().contains("^@^"));
        assert_eq!(state.grid[world_cell_index(0, 1)], 5);
        assert_eq!(state.grid[world_cell_index(2, 1)], 5);
    }

    #[test]
    fn moongate_destination_overlay_does_not_trigger_entry_prompt() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 2, 1);
        state.ambient_light = FULL_DAYLIGHT;
        state.moongates.push(MoongateEntry {
            x: 0,
            y: 1,
            destination_plane: WorldPlane::Britannia,
            destination_x: 2,
            destination_y: 1,
            active_hours: None,
            expected_tile: None,
        });

        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 0);
        assert!(!state.message.contains("moongate"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn enter_moongate_teleports_within_britannia() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;
        state.moongates.push(MoongateEntry {
            x: 10,
            y: 20,
            destination_plane: WorldPlane::Britannia,
            destination_x: 30,
            destination_y: 40,
            active_hours: None,
            expected_tile: None,
        });

        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::MoongateTeleported {
                from: WorldPlane::Britannia,
                to: WorldPlane::Britannia,
            })
        );

        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(state.active_objects[0].x, 30);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn moongate_requires_daylight_for_render_movement_and_entry() {
        let dir = debug_game_dir();
        let mut grid = open_world_grid();
        grid[world_cell_index(1, 0)] = 24;
        let mut state = britannia_state(grid, 0, 0);
        state.clock = GameClock::new(20, 0).unwrap();
        state.ambient_light = FULL_DARKNESS;
        state.player.facing = Direction::East;
        state.moongates.push(MoongateEntry {
            x: 1,
            y: 0,
            destination_plane: WorldPlane::Britannia,
            destination_x: 30,
            destination_y: 40,
            active_hours: None,
            expected_tile: None,
        });

        let view = state.render_text_view(1);
        assert!(!view.contains('^'));

        assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.turn, 0);

        state.player.x = 1;
        state.player.y = 0;
        state.sync_player_object();
        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.turn, 0);
        assert!(!state.message.contains("moongate"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn moongate_animation_frame_advances_only_for_visible_active_gates() {
        let mut dark = britannia_state(open_world_grid(), 0, 0);
        dark.clock = GameClock::new(12, 0).unwrap();
        dark.ambient_light = FULL_DARKNESS;
        dark.animation = AnimationClock {
            frame: 0,
            moongate_frame: 7,
        };
        dark.moongates.push(MoongateEntry {
            x: 1,
            y: 0,
            destination_plane: WorldPlane::Britannia,
            destination_x: 30,
            destination_y: 40,
            active_hours: None,
            expected_tile: None,
        });

        assert_eq!(dark.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(dark.animation.frame, 1);
        assert_eq!(dark.animation.moongate_frame, 7);

        let mut inactive = britannia_state(open_world_grid(), 0, 0);
        inactive.clock = GameClock::new(12, 0).unwrap();
        inactive.ambient_light = FULL_DAYLIGHT;
        inactive.animation = AnimationClock {
            frame: 0,
            moongate_frame: 7,
        };
        inactive.moongates.push(MoongateEntry {
            x: 1,
            y: 0,
            destination_plane: WorldPlane::Britannia,
            destination_x: 30,
            destination_y: 40,
            active_hours: Some((13, 13)),
            expected_tile: None,
        });

        assert_eq!(inactive.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(inactive.animation.frame, 1);
        assert_eq!(inactive.animation.moongate_frame, 7);

        let mut visible = britannia_state(open_world_grid(), 0, 0);
        visible.clock = GameClock::new(12, 0).unwrap();
        visible.ambient_light = FULL_DAYLIGHT;
        visible.animation = AnimationClock {
            frame: 0,
            moongate_frame: 0,
        };
        visible.moongates.push(MoongateEntry {
            x: 1,
            y: 0,
            destination_plane: WorldPlane::Britannia,
            destination_x: 30,
            destination_y: 40,
            active_hours: None,
            expected_tile: None,
        });

        assert_eq!(visible.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(visible.animation.frame, 1);
        // The moongate sprite at 0xDC is a single static tile per
        // LOOK2.DAT ("a moon gate!"); the frame ring is one wide so
        // ticking is a no-op.
        assert_eq!(visible.animation.moongate_frame, 0);
    }

    #[test]
    fn moongate_active_hour_changes_dirty_visibility_without_daylight_change() {
        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.clock = GameClock::new(12, 58).unwrap();
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;
        state.moongates.push(MoongateEntry {
            x: 11,
            y: 20,
            destination_plane: WorldPlane::Britannia,
            destination_x: 30,
            destination_y: 40,
            active_hours: Some((13, 13)),
            expected_tile: None,
        });

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);
        assert_eq!(state.clock, GameClock::new(13, 0).unwrap());
        assert!(state.visibility_dirty);

        state.visibility_dirty = false;
        assert_eq!(state.pass_turn(), MoveOutcome::Passed);
        assert_eq!(state.clock, GameClock::new(13, 2).unwrap());
        assert!(!state.visibility_dirty);

        state.clock = GameClock::new(13, 58).unwrap();
        state.visibility_dirty = false;
        assert_eq!(state.pass_turn(), MoveOutcome::Passed);
        assert_eq!(state.clock, GameClock::new(14, 0).unwrap());
        assert!(state.visibility_dirty);
    }

    #[test]
    fn enter_moongate_can_transition_to_underworld() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;
        state.wind = WindState::North;
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 0,
            skiffs: 0,
        };
        state.sail_cadence = 1;
        state.sail_stall_pending = true;
        state.sync_player_object();
        state.moongates.push(MoongateEntry {
            x: 10,
            y: 20,
            destination_plane: WorldPlane::Underworld,
            destination_x: 30,
            destination_y: 40,
            active_hours: None,
            expected_tile: None,
        });

        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::MoongateTeleported {
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
        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(
            state.active_objects[0].z,
            WorldPlane::Underworld.save_floor()
        );
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.timing_status, TimingStatusTag::Normal);
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.grid[world_cell_index(30, 40)], 5);
        assert_eq!(state.ambient_light, FULL_DARKNESS);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("North Winds"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_step_uses_clean_plane_transition_table_for_chasm_fall() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 11 20 UNDERWORLD 30 40\n",
        )
        .unwrap();
        let mut under_ool = vec![0; OOL_PLANE_LEN];
        let slot = OOL_RECORD_LEN;
        under_ool[slot] = 168;
        under_ool[slot + 1] = 169;
        under_ool[slot + 2] = 31;
        under_ool[slot + 3] = 40;
        under_ool[slot + 4] = 0xff;
        under_ool[slot + 6] = 0x22;
        fs::write(dir.join("UNDER.OOL"), under_ool).unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.wind = WindState::East;
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 0,
            skiffs: 0,
        };
        state.sail_cadence = 1;
        state.sail_stall_pending = true;
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();
        state.sync_player_object();
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
                status: b'P',
                climb_stat: 30,
                mana: 8,
                hp: 6,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'D',
                climb_stat: 0,
                mana: 8,
                hp: 9,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 3,
                class_byte: b'A',
                status: b'S',
                climb_stat: 30,
                mana: 8,
                hp: 8,
                max_hp: 20,
                level: 8,
            },
        ];

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld
            })
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(state.active_objects[0].z, -1);
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.timing_status, TimingStatusTag::Normal);
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.grid[world_cell_index(30, 40)], 5);
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 168,
                tile: 169,
                x: 31,
                y: 40,
                z: -1,
                phase: 0x22,
                aux1: 0,
                aux3: 0,
            }
        );
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("F-A-L-L-S"));
        assert!(state.message.contains("fall damage"));
        assert!(state.message.contains("East Winds"));
        assert!(state.message.contains("party slot 0"));
        assert!(state.message.contains("party slot 1"));
        assert!(!state.message.contains("party slot 2"));
        assert!(!state.message.contains("party slot 3"));
        assert!(state.party[0].hp < 10);
        assert!(state.party[1].hp < 6);
        assert_eq!(state.party[2].hp, 9);
        assert_eq!(state.party[3].hp, 8);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_surface_chasm_fires_without_sidecar_table() {
        let dir = debug_game_dir();
        let mut under_ool = vec![0; OOL_PLANE_LEN];
        let slot = OOL_RECORD_LEN;
        under_ool[slot] = 168;
        under_ool[slot + 1] = 169;
        under_ool[slot + 2] = SURFACE_CHASM_X.wrapping_add(1);
        under_ool[slot + 3] = SURFACE_CHASM_Y;
        under_ool[slot + 4] = 0xff;
        under_ool[slot + 6] = 0x22;
        fs::write(dir.join("UNDER.OOL"), under_ool).unwrap();

        let mut state = world_state(
            open_world_grid(),
            usize::from(SURFACE_CHASM_X - 1),
            usize::from(SURFACE_CHASM_Y),
        );
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.party[0].hp = 10;
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();
        state.sync_player_object();

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld
            })
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!(
            (state.player.x, state.player.y),
            (usize::from(SURFACE_CHASM_X), usize::from(SURFACE_CHASM_Y))
        );
        assert_eq!(state.active_objects[0].z, WorldPlane::Underworld.save_floor());
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 168,
                tile: 169,
                x: usize::from(SURFACE_CHASM_X.wrapping_add(1)),
                y: usize::from(SURFACE_CHASM_Y),
                z: -1,
                phase: 0x22,
                aux1: 0,
                aux3: 0,
            }
        );
        assert!(state.party[0].hp < 10);
        assert!(state.message.contains("F-A-L-L-S"));
        assert!(state.message.contains("fall damage"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fixed_surface_chasm_underfoot_pass_triggers_without_sidecar_table() {
        let dir = debug_game_dir();
        let mut state = britannia_state(
            open_world_grid(),
            usize::from(SURFACE_CHASM_X),
            usize::from(SURFACE_CHASM_Y),
        );
        state.party[0].hp = 10;

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld
            })
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!(
            (state.player.x, state.player.y),
            (usize::from(SURFACE_CHASM_X), usize::from(SURFACE_CHASM_Y))
        );
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("F-A-L-L-S"));
        assert!(state.party[0].hp < 10);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_plane_transition_table_overrides_base_tile_blocking() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 11 20 UNDERWORLD 30 40 24\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(11, 20)] = 24;
        let mut state = britannia_state(grid, 10, 20);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld
            })
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("F-A-L-L-S"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_clean_plane_transition_applies_underfoot_transition() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 11 20 UNDERWORLD 30 40 5\n",
        )
        .unwrap();
        let state_grid = open_world_grid();
        let mut state = britannia_state(state_grid, 11, 20);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld
            })
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (30, 40));
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("F-A-L-L-S"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_plane_transition_tile_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
            "BRITANNIA 11 20 UNDERWORLD 30 40 24\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 10, 20);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (11, 20));
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("F-A-L-L-S"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_plane_transition_preserves_runtime_overlay_cache_between_planes() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 11,
            y: 20,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        let mut cached_underworld = vec![ActiveObject::empty(); OOL_SLOTS - 1];
        cached_underworld[4] = ActiveObject {
            type_byte: 194,
            tile: 194,
            x: 31,
            y: 40,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        };
        state
            .world_overlays
            .set(WorldPlane::Underworld, cached_underworld);

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
        assert!(state.active_objects[1].is_empty());

        state
            .apply_world_plane_transition(
                &dir,
                WorldPlaneTransitionEntry {
                    from_plane: WorldPlane::Britannia,
                    x: 11,
                    y: 20,
                    to_plane: WorldPlane::Underworld,
                    to_x: 30,
                    to_y: 40,
                    expected_tile: None,
                },
            )
            .unwrap();

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!(state.player.transport, TransportState::Foot);
        assert!(state.world_overlays.get(WorldPlane::Britannia).unwrap()[0].is_empty());
        assert_eq!(
            state.world_object_at(31, 40),
            Some(&ActiveObject {
                type_byte: 194,
                tile: 194,
                x: 31,
                y: 40,
                z: WorldPlane::Underworld.save_floor(),
                phase: 0x22,
                aux1: 0,
                aux3: 0,
            })
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_plane_transition_save_load_round_trips_both_plane_overlays() {
        let dir = debug_game_dir();
        write_save_template_and_empty_overlays(&dir, 0, 0, 10, 20);
        let britannia_object = ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 11,
            y: 20,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 7,
            aux3: 1,
        };
        let underworld_object = ActiveObject {
            type_byte: 194,
            tile: 194,
            x: 31,
            y: 40,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        };
        let updated_underworld_object = ActiveObject {
            type_byte: 194,
            tile: 195,
            x: 32,
            y: 41,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x33,
            aux1: 4,
            aux3: 5,
        };
        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.active_objects.push(britannia_object);
        let mut cached_underworld = vec![ActiveObject::empty(); OOL_SLOTS - 1];
        cached_underworld[0] = underworld_object;
        state
            .world_overlays
            .set(WorldPlane::Underworld, cached_underworld);

        state
            .apply_world_plane_transition(
                &dir,
                WorldPlaneTransitionEntry {
                    from_plane: WorldPlane::Britannia,
                    x: 11,
                    y: 20,
                    to_plane: WorldPlane::Underworld,
                    to_x: 30,
                    to_y: 40,
                    expected_tile: None,
                },
            )
            .unwrap();
        assert_eq!(state.active_objects[1], underworld_object);
        state.active_objects[1] = updated_underworld_object;

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved_gam = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(saved_gam[SAVE_SCENE_OFFSET], 0);
        assert_eq!(saved_gam[SAVE_Z_OFFSET], 0xff);
        assert_eq!(saved_gam[SAVE_X_OFFSET], 30);
        assert_eq!(saved_gam[SAVE_Y_OFFSET], 40);
        let active_table = decode_saved_active_objects(&saved_gam).unwrap();
        assert_eq!(active_table[0], updated_underworld_object);

        let saved_ool = fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap();
        let britannia_overlay = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
        let underworld_overlay = decode_ool_plane_objects(&saved_ool[OOL_PLANE_LEN..]).unwrap();
        assert_eq!(britannia_overlay[0], britannia_object);
        assert_eq!(underworld_overlay[0], updated_underworld_object);
        assert_eq!(
            fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(),
            saved_ool[..OOL_PLANE_LEN].to_vec()
        );
        assert_eq!(
            fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap(),
            saved_ool[OOL_PLANE_LEN..].to_vec()
        );

        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.target, PlayTarget::World(WorldPlane::Underworld));
        assert_eq!(options.start, Some((30, 40)));
        assert_eq!(
            options.saved_active_objects.as_ref().unwrap()[0],
            updated_underworld_object
        );
        let reloaded = PlayState::load_scene(&dir, options).unwrap();
        assert_eq!(
            reloaded.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((reloaded.player.x, reloaded.player.y), (30, 40));
        assert_eq!(reloaded.active_objects[1], updated_underworld_object);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ool_decoder_keeps_non_player_slot_shape_and_skips_slot_zero() {
        let mut bytes = vec![0; OOL_PLANE_LEN];
        bytes[0] = 0xaa;
        bytes[1] = 0xab;
        let empty_payload_slot = OOL_RECORD_LEN * 3;
        bytes[empty_payload_slot + 1] = 0x44;
        bytes[empty_payload_slot + 2] = 250;
        bytes[empty_payload_slot + 3] = 251;
        bytes[empty_payload_slot + 4] = 0xfe;
        bytes[empty_payload_slot + 5] = 0x55;
        bytes[empty_payload_slot + 6] = 0x66;
        bytes[empty_payload_slot + 7] = 0x77;
        let slot = OOL_RECORD_LEN * 7;
        bytes[slot] = 168;
        bytes[slot + 1] = 169;
        bytes[slot + 2] = 12;
        bytes[slot + 3] = 34;
        bytes[slot + 4] = 0xff;
        bytes[slot + 5] = 88;
        bytes[slot + 6] = 0x22;
        bytes[slot + 7] = 3;

        let objects = decode_ool_plane_objects(&bytes).unwrap();

        assert_eq!(objects.len(), OOL_SLOTS - 1);
        assert!(objects[..6].iter().all(|object| object.is_empty()));
        assert_eq!(
            objects[2],
            ActiveObject {
                type_byte: 0,
                tile: 0x44,
                x: 250,
                y: 251,
                z: -2,
                phase: 0x66,
                aux1: 0x55,
                aux3: 0x77,
            }
        );
        assert_eq!(
            objects[6],
            ActiveObject {
                type_byte: 168,
                tile: 169,
                x: 12,
                y: 34,
                z: -1,
                phase: 0x22,
                aux1: 88,
                aux3: 3,
            }
        );
        assert!(objects[7..].iter().all(|object| object.is_empty()));
        assert!(decode_ool_plane_objects(&bytes[..OOL_PLANE_LEN - 1]).is_err());
    }

    #[test]
    fn ool_encoder_round_trips_empty_slot_payload_bytes() {
        let payload = ActiveObject {
            type_byte: 0,
            tile: 0x44,
            x: 250,
            y: 251,
            z: -2,
            phase: 0x66,
            aux1: 0x55,
            aux3: 0x77,
        };

        let bytes = encode_ool_plane_objects(&[payload]).unwrap();
        let slot = OOL_RECORD_LEN;

        assert_eq!(
            &bytes[slot..slot + OOL_RECORD_LEN],
            &[0, 0x44, 250, 251, 0xfe, 0x55, 0x66, 0x77]
        );
        assert_eq!(decode_ool_plane_objects(&bytes).unwrap()[0], payload);
    }

    #[test]
    fn active_object_encoder_writes_new_empty_records_as_all_zero() {
        let bytes = encode_active_object_table(&[ActiveObject::empty()]).unwrap();

        assert_eq!(&bytes[..OOL_RECORD_LEN], &[0; OOL_RECORD_LEN]);
    }

    #[test]
    fn world_overlay_loader_uses_saved_ool_plane_half() {
        let dir = debug_game_dir();
        let mut saved = vec![0; SAVED_OOL_LEN];
        let slot = OOL_PLANE_LEN + OOL_RECORD_LEN;
        saved[slot] = 170;
        saved[slot + 1] = 170;
        saved[slot + 2] = 4;
        saved[slot + 3] = 5;
        saved[slot + 4] = 0xff;
        saved[slot + 5] = 99;
        saved[slot + 6] = STEADY_PHASE;
        saved[slot + 7] = 4;
        fs::write(dir.join("SAVED.OOL"), saved).unwrap();
        let mut under = vec![0; OOL_PLANE_LEN];
        under[OOL_RECORD_LEN] = 171;
        under[OOL_RECORD_LEN + 1] = 171;
        fs::write(dir.join("UNDER.OOL"), under).unwrap();

        let objects = load_world_overlay_objects(&dir, WorldPlane::Underworld).unwrap();

        assert_eq!(objects.len(), OOL_SLOTS - 1);
        assert_eq!(
            objects[0],
            ActiveObject {
                type_byte: 170,
                tile: 170,
                x: 4,
                y: 5,
                z: -1,
                phase: STEADY_PHASE,
                aux1: 99,
                aux3: 4,
            }
        );
        assert!(objects.iter().skip(1).all(|object| object.is_empty()));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_from_save_uses_live_active_object_table() {
        let dir = debug_game_dir();
        let mut under = vec![0; OOL_PLANE_LEN];
        let slot = OOL_RECORD_LEN;
        under[slot] = 171;
        under[slot + 1] = 171;
        under[slot + 2] = 4;
        under[slot + 3] = 5;
        under[slot + 4] = 0xff;
        fs::write(dir.join("UNDER.OOL"), under).unwrap();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            clock: GameClock::default(),
            food: DEFAULT_FOOD_STOCK,
            gold: DEFAULT_GOLD_STOCK,
            keys: DEFAULT_KEY_STOCK,
            gems: DEFAULT_GEM_STOCK,
            climbing_gear: DEFAULT_CLIMBING_GEAR,
            special_items: [0; SPECIAL_ITEM_COUNT],
            party: default_party(),
            party_names: default_party_names(1),
            party_experience: default_party_experience(1),
            party_stay_counters: default_party_stay_counters(1),
            party_strengths: default_party_strengths(1),
            party_intelligence: default_party_intelligence(1),
            party_equipment: default_party_equipment(1),
            party_roster: default_party_roster(1),
            equipment_stock: [0; EQUIPMENT_COUNT],
            spell_charges: [0; SPELL_COUNT],
            scroll_stock: [0; SCROLL_COUNT],
            potion_stock: [0; POTION_COUNT],
            reagents: DEFAULT_REAGENTS,
            rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
                RARE_REAGENT_HARVEST_POINT_COUNT],
            fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
            fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
            dungeon_room_clear_bitmap: [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
            saved_dungeon_working_buffer: None,
            moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
            shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
            shrine_ordained_mask: 0,
            shrine_codex_mask: 0,
            moral_standing: 0,
            avatar_stats: AvatarStats::default(),
            torches: DEFAULT_TORCH_STOCK,
            torch_counter: 0,
            light_spell_counter: 0,
            wind: WindState::default(),
            wind_save_byte: 0,
            timing_status: TimingStatusTag::default(),
            time_stop_counter: 0,
            active_effect_tag: None,
            active_effect_counter: 0,
            fortunes_of_war: 0,
            active_player: None,
            combat_round_counter: 0,
            transport: TransportState::Foot,
            facing: None,
            pending_vehicle: None,
            inn_registry: Vec::new(),
            blackthorn_story: BlackthornStoryState::default(),
            initial_britannia_overlay: None,
            debug_enter: None,
            saved_active_objects: Some(vec![ActiveObject {
                type_byte: 170,
                tile: 170,
                x: 11,
                y: 20,
                z: -1,
                phase: 0x22,
                aux1: 0,
                aux3: 0,
            }]),
            save_template_source: SaveTemplateSource::PreferSavedGame,
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!(
            state.active_objects,
            vec![
                ActiveObject {
                    type_byte: PLAYER_TILE,
                    tile: PLAYER_TILE,
                    x: 10,
                    y: 20,
                    z: -1,
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                },
                ActiveObject {
                    type_byte: 170,
                    tile: 170,
                    x: 11,
                    y: 20,
                    z: -1,
                    phase: 0x22,
                    aux1: 0,
                    aux3: 0,
                },
            ]
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_message_reports_current_wind_status() {
        let dir = debug_game_dir();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            clock: GameClock::default(),
            food: DEFAULT_FOOD_STOCK,
            gold: DEFAULT_GOLD_STOCK,
            keys: DEFAULT_KEY_STOCK,
            gems: DEFAULT_GEM_STOCK,
            climbing_gear: DEFAULT_CLIMBING_GEAR,
            special_items: [0; SPECIAL_ITEM_COUNT],
            party: default_party(),
            party_names: default_party_names(1),
            party_experience: default_party_experience(1),
            party_stay_counters: default_party_stay_counters(1),
            party_strengths: default_party_strengths(1),
            party_intelligence: default_party_intelligence(1),
            party_equipment: default_party_equipment(1),
            party_roster: default_party_roster(1),
            equipment_stock: [0; EQUIPMENT_COUNT],
            spell_charges: [0; SPELL_COUNT],
            scroll_stock: [0; SCROLL_COUNT],
            potion_stock: [0; POTION_COUNT],
            reagents: DEFAULT_REAGENTS,
            rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
                RARE_REAGENT_HARVEST_POINT_COUNT],
            fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
            fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
            dungeon_room_clear_bitmap: [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
            saved_dungeon_working_buffer: None,
            moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
            shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
            shrine_ordained_mask: 0,
            shrine_codex_mask: 0,
            moral_standing: 0,
            avatar_stats: AvatarStats::default(),
            torches: DEFAULT_TORCH_STOCK,
            torch_counter: 0,
            light_spell_counter: 0,
            wind: WindState::West,
            wind_save_byte: 0,
            timing_status: TimingStatusTag::default(),
            time_stop_counter: 0,
            active_effect_tag: None,
            active_effect_counter: 0,
            fortunes_of_war: 0,
            active_player: None,
            combat_round_counter: 0,
            transport: TransportState::Foot,
            facing: None,
            pending_vehicle: None,
            inn_registry: Vec::new(),
            blackthorn_story: BlackthornStoryState::default(),
            initial_britannia_overlay: None,
            debug_enter: None,
            saved_active_objects: Some(Vec::new()),
            save_template_source: SaveTemplateSource::PreferSavedGame,
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert!(state.message.contains("Entered UNDERWORLD at (10, 20)."));
        assert!(state.message.contains("West Winds"));
        let _ = fs::remove_dir_all(dir);
    }

