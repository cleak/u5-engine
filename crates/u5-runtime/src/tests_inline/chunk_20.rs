    #[test]
    fn idle_tick_advances_visuals_without_turn_time_doors_or_schedules() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_CLEARED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(17, 59).unwrap();
        state.door_tracker = Some(DoorTracker {
            previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
            x: 2,
            y: 1,
            turns_remaining: 1,
        });
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
                schedule: [0, 0, 0, 4, 8, 12, 1, 2, 3, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);
        let npcs_before = state.npcs.clone();
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 3,
            y: 1,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(state.clock, GameClock::new(17, 59).unwrap());
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                x: 2,
                y: 1,
                turns_remaining: 1,
            })
        );
        assert_eq!(state.npcs, npcs_before);
        assert_eq!(state.animation.frame, 1);
        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 168)
            .unwrap();
        assert_eq!(object.phase, 0x21);
        assert_eq!(object.tile, 169);
    }

    #[test]
    fn idle_tick_can_apply_public_random_wind_drift_without_turn() {
        let mut state = britannia_state(open_world_grid(), 1, 10);
        state.clock = GameClock::new(12, 0).unwrap();
        state.wind = WindState::Calm;
        state.wind_save_byte = 0x7a;
        state.sail_cadence = 1;
        state.sail_stall_pending = true;

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
        assert_eq!(state.wind, WindState::North);
        assert_eq!(state.wind_save_byte, 1);
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.message, "Idle animation tick. North Winds");
    }

    #[test]
    fn idle_tick_underworld_drift_uses_non_surface_presentation_branch() {
        // weather.md §2: on the underworld plane the wind state still updates,
        // but the helper uses the non-surface presentation branch instead of
        // printing the cardinal wind label.
        let mut state = britannia_state(open_world_grid(), 1, 10);
        state.area = Area::World {
            plane: WorldPlane::Underworld,
        };
        state.active_objects[0].z = WorldPlane::Underworld.save_floor();
        state.clock = GameClock::new(12, 0).unwrap();
        state.wind = WindState::Calm;
        state.wind_save_byte = 0x7a;

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        // Wind state did update.
        assert_eq!(state.wind, WindState::North);
        // Message must NOT contain the cardinal wind label.
        assert!(!state.message.contains("North Winds"));
        assert!(state.message.contains("Idle animation tick"));
    }

    #[test]
    fn idle_tick_keeps_active_objects_frozen_during_negate_time_without_aging_counter() {
        let mut state = britannia_state(open_world_grid(), 4, 5);
        state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 3;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        assert_eq!(state.turn, 0);
        assert_eq!(state.active_effect_tag, Some(NEGATE_TIME_ACTIVE_EFFECT_TAG));
        assert_eq!(state.active_effect_counter, 3);
        assert_eq!(state.animation.frame, 1);
        assert_eq!(state.active_objects[1].phase, 0x22);
        assert_eq!(state.active_objects[1].tile, 168);
    }

    #[test]
    fn open_facing_rewrites_door_and_auto_closes_after_four_turns() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        assert_eq!(state.turn, 1);
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.message, "Opened!");
        assert!(state.visibility_dirty);
        assert!(state.pending_map_viewport_dissolves.is_empty());
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                x: 2,
                y: 1,
                turns_remaining: 4,
            })
        );

        state.visibility_dirty = false;
        state.advance_turn();
        state.advance_turn();
        state.advance_turn();
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert!(state.door_tracker.is_some());
        assert!(!state.visibility_dirty);

        state.advance_turn();
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
    }

    #[test]
    fn open_facing_non_door_is_not_a_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.open_facing(), MoveOutcome::Blocked);
        assert_eq!(state.turn, 0);
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.message, "Nothing to open!");
    }

    #[test]
    fn jimmy_town_locked_door_roll_success_unlocks_visit_local_tile() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.prng_state = 0x1234;
        let expected_prng_state = u5_prng_advance_state(state.prng_state);

        assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert_eq!(state.message, "Unlocked!");
        assert!(state.visibility_dirty);
    }

    #[test]
    fn jimmy_town_locked_door_roll_failure_breaks_key_without_unlocking() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 0;
        state.prng_state = 0x1234;
        let expected_prng_state = u5_prng_advance_state(state.prng_state);

        assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_LOCKED_TILE);
        assert_eq!(state.message, "Key broke!");
    }

    #[test]
    fn jimmy_wrong_tile_reports_no_lock_and_commits_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.prng_state = 0x1234;

        assert_eq!(state.jimmy_facing(), MoveOutcome::Blocked);

        assert_eq!(state.prng_state, 0x1234);
        assert_eq!(state.message, "No lock!");
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn open_facing_tracked_open_door_consumes_turn_without_resetting_timer() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.ambient_light = FULL_DAYLIGHT;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        state.visibility_dirty = false;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);

        assert_eq!(state.turn, 2);
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                x: 2,
                y: 1,
                turns_remaining: 3,
            })
        );
        assert_eq!(state.message, "It's open!");
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn open_facing_runs_auto_close_before_reopening_expiring_door() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        state.advance_turn();
        state.advance_turn();
        state.advance_turn();
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                x: 2,
                y: 1,
                turns_remaining: 1,
            })
        );

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);

        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.turn, 5);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                x: 2,
                y: 1,
                turns_remaining: 4,
            })
        );
        assert_eq!(state.message, "Opened!");
    }

    #[test]
    fn open_facing_acknowledges_first_open_door_after_second_door_overwrites_timer() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
        grid[2 * 32 + 1] = TOWN_DOOR_WINDOWED_UNLOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);

        state.player.facing = Direction::South;
        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
        assert_eq!(state.grid[2 * 32 + 1], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: TOWN_DOOR_WINDOWED_UNLOCKED_TILE,
                x: 1,
                y: 2,
                turns_remaining: 4,
            })
        );

        state.player.facing = Direction::East;
        assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);

        assert_eq!(state.turn, 3);
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.grid[2 * 32 + 1], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: TOWN_DOOR_WINDOWED_UNLOCKED_TILE,
                x: 1,
                y: 2,
                turns_remaining: 3,
            })
        );
        assert_eq!(state.message, "It's open!");
    }

    #[test]
    fn town_open_locked_sidecar_refuses_without_turn() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Locked!");
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_LOCKED_TILE);
        assert_eq!(state.turn, 0);
        assert_eq!(state.door_tracker, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_jimmy_locked_sidecar_rewrites_to_unlocked_door() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;
        state.prng_state = 0x1234;
        let expected_prng_state = u5_prng_advance_state(state.prng_state);

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.message, "Unlocked!");
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::DoorOpened
        );
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.message, "Opened!");
        assert_eq!(
            state.door_tracker,
            Some(DoorTracker {
                previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                x: 2,
                y: 1,
                turns_remaining: 4,
            })
        );
        assert_eq!(state.turn, 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_jimmy_locked_sidecar_roll_failure_breaks_key_without_rewrite() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 0;
        state.visibility_dirty = false;
        state.prng_state = 0x1234;
        let expected_prng_state = u5_prng_advance_state(state.prng_state);

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_LOCKED_TILE);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_jimmy_inline_party_member_uses_selected_dexterity() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 8,
                hp: DEFAULT_PARTY_HP,
                max_hp: DEFAULT_PARTY_MAX_HP,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: DEFAULT_PARTY_HP,
                max_hp: DEFAULT_PARTY_MAX_HP,
                level: 8,
            },
        ];

        assert_eq!(
            handle_play_key_input(&mut state, 'J', "2", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Unlocked!");
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_jimmy_ordinary_npc_is_not_a_pickpocket_target() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.moral_standing = 98;
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
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                name: None,
            },
        ]);

        assert_eq!(state.jimmy_facing(), MoveOutcome::Blocked);

        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.moral_standing, 98);
        assert_eq!(state.message, "No lock!");
    }

    #[test]
    fn town_jimmy_ordinary_npc_does_not_roll_or_break_a_key() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 0;
        state.moral_standing = 10;
        state.prng_state = 0x1234;
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
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                name: None,
            },
        ]);

        assert_eq!(state.jimmy_facing(), MoveOutcome::Blocked);

        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.moral_standing, 10);
        assert_eq!(state.prng_state, 0x1234);
        assert_eq!(state.message, "No lock!");
    }

    #[test]
    fn town_jimmy_object_without_active_npc_refuses_without_key_loss() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
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

        assert_eq!(state.jimmy_facing(), MoveOutcome::Blocked);

        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.moral_standing, 0);
        assert_eq!(state.message, "No lock!");
    }

    #[test]
    fn town_jimmy_empty_restraint_skips_picker_and_commits_turn() {
        let mut grid = open_grid();
        grid[32 + 2] = JIMMY_STOCKS_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.prng_state = 0x1234;

        assert_eq!(
            state
                .jimmy_facing_with_game_dir_and_member(None, None)
                .unwrap(),
            MoveOutcome::LockTried
        );

        assert!(state.active_jimmy.is_none());
        assert_eq!(state.prng_state, 0x1234);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "No one is there!");
    }

    #[test]
    fn town_jimmy_native_magic_lock_skips_picker_roll_and_breaks_key() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_MAGIC_PLAIN_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.prng_state = 0x1234;

        assert_eq!(
            state
                .jimmy_facing_with_game_dir_and_member(None, None)
                .unwrap(),
            MoveOutcome::LockTried
        );

        assert!(state.active_jimmy.is_none());
        assert_eq!(state.prng_state, 0x1234);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_MAGIC_PLAIN_TILE);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Key broke!");
    }

    #[test]
    fn town_jimmy_restraint_release_updates_live_npc_and_native_removal_mask() {
        let mut grid = open_grid();
        grid[32 + 2] = JIMMY_MANACLES_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 30;
        state.moral_standing = 98;
        let slots = [
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
                dialog_id: 2,
                schedule: [1, 2, 3, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!(
            state
                .jimmy_facing_with_game_dir_and_member(None, None)
                .unwrap(),
            MoveOutcome::Observed
        );
        assert!(state.active_jimmy.is_some());
        assert_eq!(state.turn, 0);

        assert_eq!(
            state.step_active_jimmy('1', "", Path::new("")).unwrap(),
            Some(MoveOutcome::LockTried)
        );
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].dialog_id, NPC_DIALOG_ID_NONE);
        assert_eq!(&state.npcs[0].schedule[..3], &[JIMMY_RELEASE_AI_MODE; 3]);
        assert_eq!(state.removed_town_npc_flags.get(&17), Some(&0b10));
        assert_eq!(state.moral_standing, MORAL_STANDING_MAX);
        assert_eq!(state.grid[32 + 2], JIMMY_MANACLES_TILE);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "I thank thee!");

        state.load_scheduled_npcs(&slots);
        assert!(state.npcs.is_empty());
    }

    #[test]
    fn town_jimmy_restraint_failure_preserves_actor_and_tile() {
        let mut grid = open_grid();
        grid[32 + 2] = JIMMY_STOCKS_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 0;
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
                dialog_id: 2,
                schedule: [1, 2, 3, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                name: None,
            },
        ]);

        assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].dialog_id, 2);
        assert_eq!(&state.npcs[0].schedule[..3], &[1, 2, 3]);
        assert!(state.removed_town_npc_flags.is_empty());
        assert_eq!(state.grid[32 + 2], JIMMY_STOCKS_TILE);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Key broke!");
    }

    #[test]
    fn town_open_object_chest_consumes_slot_trap_and_public_reward_pools() {
        let mut grid = open_grid();
        grid[32 + 2] = 0x4f;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.party_names = default_party_names(2);
        state.party_names[1][..4].copy_from_slice(b"Iolo");
        state.moral_standing = 8;
        state.visibility_dirty = false;
        state.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0xff,
            aux3: 0,
        });

        assert_eq!(state.open_facing(), MoveOutcome::Observed);
        assert!(state.active_surface_chest.is_some());
        assert_eq!(
            state.step_active_surface_chest('2', "").unwrap(),
            Some(MoveOutcome::ContainerOpened)
        );

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.moral_standing, 6);
        assert!(state.visibility_dirty);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Opened object chest at (2, 1)"));
        assert!(
            state
                .message
                .contains("Acid trap hit party member 2 for 12 HP.")
        );
        assert_eq!(state.party[1].hp, DEFAULT_PARTY_HP - 12);
        let selected_name = party_name_to_string(&state.party_names[1]).unwrap();
        assert!(
            state
                .message_entries()
                .iter()
                .any(|entry| entry.text == selected_name),
            "a prompted pick echoes the selected member's name"
        );
        assert!(state.message.contains("chest grants"));
        assert!(state.food > DEFAULT_FOOD_STOCK || state.gold > DEFAULT_GOLD_STOCK);
    }

    #[test]
    fn town_get_object_chest_uses_chest_helper_before_blocking_object_refusal() {
        let mut grid = open_grid();
        grid[32 + 2] = 0x4f;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x7f,
            aux3: 0,
        });

        // `traps.md §2.1` branch 3: this party has exactly one
        // Good-or-Poisoned member, so the acting member is auto-selected
        // **silently** - no prompt session opens, and the chosen member's
        // name is not echoed. An earlier revision of this test asserted a
        // prompt here, on the invented rule that the site always prompts;
        // that is withdrawn.
        assert_eq!(
            state.get_town_facing(Path::new(""), Scene::new(0x11).unwrap(), 0).unwrap(),
            MoveOutcome::ContainerOpened
        );
        assert!(state.active_surface_chest.is_none());

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.grid[32 + 2], 16);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got object chest at (2, 1)"));
        assert!(state.message.contains("chest grants"));
    }

    #[test]
    fn town_object_chest_member_prompt_can_cancel_without_consuming() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        // `traps.md §2.1`: the prompt is branch 3's two-or-more case, so
        // it needs a second Good-or-Poisoned member to appear at all.
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x7f,
            aux3: 0,
        });

        assert_eq!(state.open_facing(), MoveOutcome::Observed);
        assert_eq!(
            state.step_active_surface_chest(' ', "").unwrap(),
            Some(MoveOutcome::PromptDeclined)
        );

        assert!(state.active_surface_chest.is_none());
        assert!(!state.active_objects[1].is_empty());
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "None!");
    }

    /// `traps.md §2.1` branch 3: exactly one Good-or-Poisoned member is
    /// auto-selected **silently**, and it is that member the trap hits -
    /// not slot 0, and not the first roster position. The invented tail
    /// this replaces ("active player, else slot 0") got this case wrong.
    #[test]
    fn container_trap_auto_selects_the_single_able_member_silently() {
        let mut grid = open_grid();
        grid[32 + 2] = 0x4f;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].status = b'S';
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0xff,
            aux3: 0,
        });

        assert_eq!(
            state.surface_container_acting_member(),
            ActingMemberSelection::Selected(1)
        );
        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);
        assert!(state.active_surface_chest.is_none());
        // The name of the auto-selected member is not echoed; §2.1 is
        // explicit that only a prompted pick echoes it.
        assert!(!state.message.contains("Who opens?"));
    }

    /// `traps.md §2.1` branch 2: a set active character is returned
    /// directly and silently, with **no status re-check**. The hint screens
    /// for Dead and Asleep only at the moment it is set, so a member who has
    /// since become disabled can still be the trap victim. This is one of
    /// the two override branches that skip the status test, and it is why
    /// the stronger reading - "a party with no able-bodied member can never
    /// spring a container trap" - is false without its scope.
    #[test]
    fn container_trap_active_hint_wins_without_a_status_recheck() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'S',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.party.push(PartyMember {
            slot: 2,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.active_player = Some(1);

        // Slot 1 is Asleep and would fail the branch-3 scan outright, yet
        // the hint delivers it anyway.
        assert!(!acting_member_status_eligible(state.party[1].status));
        assert_eq!(
            state.surface_container_acting_member(),
            ActingMemberSelection::Selected(1)
        );
        assert_eq!(
            state.dungeon_container_acting_member(),
            ActingMemberSelection::Selected(1)
        );
    }

    /// `traps.md §2.1` branch 3, zero-match case: the command reports that
    /// nobody is able and aborts **before** the trap can fire. The
    /// container is left untouched and no turn is spent.
    #[test]
    fn container_trap_aborts_when_nobody_is_able() {
        let mut grid = open_grid();
        grid[32 + 2] = 0x4f;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].status = b'D';
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'A',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0xff,
            aux3: 0,
        });

        assert_eq!(
            state.surface_container_acting_member(),
            ActingMemberSelection::NoneAble
        );
        assert_eq!(state.open_facing(), MoveOutcome::Blocked);
        assert!(state.active_surface_chest.is_none());
        assert_eq!(state.turn, 0);
        // The trap never ran: the container record is still there.
        assert!(!state.active_objects[1].is_empty());
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP);
    }

    /// `traps.md §2.1`/§4: the combat override is branch 1, and the `O`
    /// dispatcher routes combat-class scenes to the **surface/town**
    /// handler, so the override can fire there and can never fire at the
    /// dungeon chest site. It is also silent: no prompt and no status test,
    /// so it fires even for a party with nobody able.
    #[test]
    fn container_trap_combat_override_is_surface_only() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party[0].status = b'D';
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'S',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.combat_active = true;
        state.pending_combat_actor_slot = Some(1);

        assert_eq!(
            state.surface_container_acting_member(),
            ActingMemberSelection::Selected(1)
        );
        // Same state, dungeon site: the override is unreachable, so the
        // selection falls through to the scan, which finds nobody able.
        assert_eq!(
            state.dungeon_container_acting_member(),
            ActingMemberSelection::NoneAble
        );
    }

    /// `traps.md §4` / `containers.md`: Open clears the matched container
    /// record outright - kind, position, and the byte carrying the trap
    /// flag - after copying that byte and before testing it. So the trap
    /// fires on this open and **a trapped surface or town container cannot
    /// spring a second time**: a later Open of the same square matches no
    /// container at all. `traps.md` §4 published this as an UNVERIFIED gap
    /// and has since withdrawn that wording.
    #[test]
    fn trapped_town_container_cannot_spring_twice() {
        let mut grid = open_grid();
        grid[32 + 2] = 0x4f;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0xff,
            aux3: 0,
        });

        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);
        assert!(state.message.contains("trap"));

        // Every field the clear covers is zeroed, the trap flag included.
        let record = state.active_objects[1];
        assert!(record.is_empty());
        assert_eq!(record.type_byte, 0);
        assert_eq!(record.tile, 0);
        assert_eq!((record.x, record.y, record.z), (0, 0, 0));
        assert_eq!(record.aux1, 0);
        assert!(state.surface_object_chest_slot_at(2, 1).is_none());

        // A second Open matches no container and springs nothing.
        let hp_after_first = state.party[0].hp;
        let turn_after_first = state.turn;
        assert_eq!(state.open_facing(), MoveOutcome::Blocked);
        assert_eq!(state.message, "Nothing to open!");
        assert_eq!(state.party[0].hp, hp_after_first);
        assert_eq!(state.turn, turn_after_first);
    }

    #[test]
    fn town_jimmy_object_chest_uses_dexterity_strict_compare_and_lock_bit() {
        let mut success = test_state(open_grid(), 1, 1);
        success.player.facing = Direction::East;
        success.party[0].climb_stat = 30;
        success.prng_state = 0x1234;
        let expected_success_prng_state = u5_prng_advance_state(success.prng_state);
        success.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x80,
            aux3: 0,
        });

        assert_eq!(success.jimmy_facing(), MoveOutcome::LockTried);
        assert_eq!(success.prng_state, expected_success_prng_state);
        assert_eq!(success.keys, DEFAULT_KEY_STOCK);
        assert_eq!(success.active_objects[1].aux1, 0x00);
        assert_eq!(success.turn, 1);
        assert_eq!(success.message, "Unlocked!");

        let mut failure = test_state(open_grid(), 1, 1);
        failure.player.facing = Direction::East;
        failure.party[0].climb_stat = 0;
        failure.prng_state = 0x1234;
        let expected_failure_prng_state = u5_prng_advance_state(failure.prng_state);
        failure.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0xff,
            aux3: 0,
        });

        assert_eq!(failure.jimmy_facing(), MoveOutcome::LockTried);
        assert_eq!(failure.prng_state, expected_failure_prng_state);
        assert_eq!(failure.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(failure.active_objects[1].aux1, 0xff);
        assert_eq!(failure.turn, 1);
        assert_eq!(failure.message, "Key broke!");
    }

    #[test]
    fn town_jimmy_already_unlocked_object_wastes_one_key_without_a_roll() {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.keys = 2;
        state.prng_state = 0x1234;
        state.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x01,
            aux3: 0,
        });

        assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);
        assert_eq!(state.prng_state, 0x1234);
        assert_eq!(state.keys, 1);
        assert_eq!(state.active_objects[1].aux1, 0x01);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Key broke!");
    }

    #[test]
    fn town_jimmy_without_inline_party_prompts_without_turn() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            handle_play_key_input(&mut state, 'J', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.message.contains("Who picks?"));
        assert!(state.active_jimmy.is_some());
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_LOCKED_TILE);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_town_jimmy_picker_unlocks_with_selected_member() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        handle_play_key_input(&mut state, 'J', "", &dir).unwrap();
        assert!(state.active_jimmy.is_some());
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_jimmy.is_none());
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Unlocked!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_dungeon_jimmy_picker_preserves_prompt_before_key_check() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.keys = 0;

        assert_eq!(
            handle_play_key_input(&mut state, 'J', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_jimmy.is_some());
        assert!(state.message.contains("Who picks?"));
        assert_eq!(state.turn, 0);
        assert_eq!(state.keys, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_jimmy.is_none());
        assert_eq!(state.message, "No keys!");
        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, 0);
    }

    #[test]
    fn active_dungeon_jimmy_cancel_commits_one_action() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.keys = 2;
        state.prng_state = 0x1234;

        assert_eq!(
            handle_play_key_input(&mut state, 'J', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_jimmy.is_some());
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_jimmy.is_none());
        assert_eq!(state.turn, 1);
        assert_eq!(state.keys, 2);
        assert_eq!(state.prng_state, 0x1234);
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x4b);
        assert_eq!(state.message, "None!");
    }

    #[test]
    fn town_jimmy_magic_lock_sidecar_breaks_one_key_without_a_roll() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_LOCK_TABLE_FILE),
            "CASTLE:0 0 2 1 151 184 MAGIC\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_MAGIC_PLAIN_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.prng_state = 0x1234;

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.prng_state, 0x1234);
        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_MAGIC_PLAIN_TILE);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_secret_door_entries_accepts_town_and_dungeon_rows() {
        let entries = parse_secret_door_entries(
            "TOWN CASTLE:0 0 2 1 184 24\nDUNGEON DUNGEON:0 0 2 1 0xF0 0x30\n",
        )
        .unwrap();

        assert_eq!(
            entries,
            vec![
                SecretDoorEntry::Town {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    reveal_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                    expected_tile: Some(24),
                },
                SecretDoorEntry::Dungeon {
                    scene: DungeonScene::new(33).unwrap(),
                    level: 0,
                    x: 2,
                    y: 1,
                    reveal_cell: 0xF0,
                    expected_cell: Some(0x30),
                },
            ]
        );
    }

    #[test]
    fn town_search_uses_clean_sidecar_to_reveal_secret_door() {
        let dir = debug_game_dir();
        fs::write(dir.join(SECRET_DOOR_TABLE_FILE), "TOWN CASTLE:0 0 2 1 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Revealed secret door at (2, 1).");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_search_secret_door_tile_guard_mismatch_is_not_a_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "TOWN CASTLE:0 0 2 1 184 25\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[32 + 2], 24);
        assert_eq!(state.turn, 0);
        assert!(!state.visibility_dirty);
        assert_eq!(state.message, "No secret door found.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_open_revealed_secret_door_stays_open_without_auto_close_tracker() {
        let dir = debug_game_dir();
        fs::write(dir.join(SECRET_DOOR_TABLE_FILE), "TOWN CASTLE:0 0 2 1 184\n").unwrap();
        let scene = Scene::new(17).unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert!(state.is_revealed_town_secret_door(scene, 0, 2, 1));

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::DoorOpened
        );

        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.message, "Opened!");
        assert_eq!(state.turn, 2);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(scene, 0, 2, 1));

        for _ in 0..4 {
            state.advance_turn();
        }

        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(state.door_tracker, None);
        assert!(state.is_recorded_open_town_door(scene, 0, 2, 1));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_floor_reload_preserves_opened_secret_door_for_visit() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut pages = vec![16; 16 * 1024];
        let floor_zero = 5 * 1024;
        let floor_one = 6 * 1024;
        pages[floor_zero] = TOWN_KLIMB_ASCEND_TILE;
        pages[floor_zero + 32 + 2] = 24;
        pages[floor_one] = TOWN_KLIMB_DESCEND_TILE;
        fs::write(dir.join("CASTLE.DAT"), pages).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(dir.join(SECRET_DOOR_TABLE_FILE), "TOWN CASTLE:0 0 2 1 184\n").unwrap();
        let mut grid = open_grid();
        grid[0] = TOWN_KLIMB_ASCEND_TILE;
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::DoorOpened
        );
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);

        state.player.x = 0;
        state.player.y = 0;
        state.sync_player_object();

        assert_eq!(
            state.climb(&dir, ClimbIntent::Up).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );
        assert_eq!(
            state.climb(&dir, ClimbIntent::Down).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 0 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert!(state.is_revealed_town_secret_door(scene, 0, 2, 1));
        assert!(state.is_recorded_open_town_door(scene, 0, 2, 1));
        assert_eq!(state.door_tracker, None);
        assert_eq!(state.turn, 4);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_jimmy_revealed_secret_door_reports_no_lock() {
        let dir = debug_game_dir();
        fs::write(dir.join(SECRET_DOOR_TABLE_FILE), "TOWN CASTLE:0 0 2 1 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert_eq!(state.message, "No lock!");
        assert_eq!(state.turn, 2);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_search_without_matching_sidecar_entry_is_not_a_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[32 + 2], 24);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "No secret door found.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_uppercase_s_routes_to_sidecar_secret_search() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "DUNGEON DUNGEON:0 0 2 1 0xF0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x30;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;

        assert!(state.handle_dungeon_key('S', &dir).unwrap());

        assert_eq!(
            state.active_direction_prompt.as_ref().map(|session| session.kind),
            Some(DirectionPromptKind::DungeonSearch)
        );
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x30);
        assert_eq!(state.turn, 0);
        assert_eq!(
            handle_play_key_input(&mut state, 'A', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xF0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Revealed dungeon secret door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_prompt_can_target_relative_right() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "DUNGEON DUNGEON:0 0 1 2 0xF0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 2)] = 0x30;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'S', "R", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 2)], 0xF0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Revealed dungeon secret door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_secret_door_cell_guard_mismatch_uses_normal_cell_search() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "DUNGEON DUNGEON:0 0 2 1 0xF0 0x30\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x4c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x4c);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Searched dungeon chest at (2, 1)"));
        assert!(!state.message.contains("secret door"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_chest_reports_trap_detail_without_consuming_chest() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x4c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x4c);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Searched dungeon chest at (2, 1)"));
        assert!(state.message.contains("trap"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_bomb_trap_marks_fired_without_level_change() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x62;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.party[0].class_byte = 30;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x00);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.pending_map_viewport_dissolves.is_empty());
        assert_eq!(
            state.message,
            "Searched dungeon bomb trap at (2, 1) on DUNGEON:0 level 0; sprung the bomb."
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_bomb_trap_can_report_nothing_without_rewrite() {
        let dir = debug_game_dir();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x62;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.party[0].class_byte = b'A';

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x62);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("nothing found"));
        assert!(state.pending_map_viewport_dissolves.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_requires_light_before_revealing_or_mutating() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "DUNGEON DUNGEON:0 0 2 1 0xF0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x30;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;

        assert_eq!(
            state.search_dungeon_focus_with_game_dir(DungeonLookFocus::Ahead, &dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x30);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "You see: darkness.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_secret_pit_rewrites_and_marks_level_below() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x61;
        grid[dungeon_cell_index(1, 2, 1)] = 0x00;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x60);
        assert_eq!(
            state.grid[dungeon_cell_index(1, 2, 1)] & DUNGEON_RUNTIME_VARIANT_BIT,
            DUNGEON_RUNTIME_VARIANT_BIT
        );
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("found a secret door"));
        assert_eq!(
            state.take_pending_map_viewport_dissolves(),
            vec![run_map_viewport_dissolve(
                MapViewportDissolveSource::DungeonSearchReveal {
                    scene,
                    level: 0,
                    x: 2,
                    y: 1,
                    original_cell: 0x61,
                    revealed_cell: 0x60,
                }
            )]
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_wall_rewrite_updates_visit_local_cell() {
        let dir = debug_game_dir();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xD8;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.visibility_dirty = false;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xE8);
        assert_eq!(state.turn, 1);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("revealed a hidden wall"));
        assert_eq!(
            state.take_pending_map_viewport_dissolves(),
            vec![run_map_viewport_dissolve(
                MapViewportDissolveSource::DungeonSearchReveal {
                    scene: DungeonScene::new(33).unwrap(),
                    level: 0,
                    x: 2,
                    y: 1,
                    original_cell: 0xD8,
                    revealed_cell: 0xE8,
                }
            )]
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_flavour_rewrite_dissolves_but_narration_only_does_not() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();

        let mut rewrite_grid = open_dungeon_record();
        rewrite_grid[dungeon_cell_index(0, 2, 1)] = 0xC0;
        let mut rewrite = dungeon_state(rewrite_grid, 0, 1, 1);
        rewrite.player.facing = Direction::East;
        rewrite.torch_counter = 5;
        assert_eq!(
            rewrite.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(rewrite.grid[dungeon_cell_index(0, 2, 1)], 0xB0);
        assert_eq!(rewrite.pending_map_viewport_dissolves.len(), 1);
        assert!(matches!(
            rewrite.pending_map_viewport_dissolves[0].source,
            MapViewportDissolveSource::DungeonSearchReveal {
                scene: recorded_scene,
                original_cell: 0xC0,
                revealed_cell: 0xB0,
                ..
            } if recorded_scene == scene
        ));

        let mut narration_grid = open_dungeon_record();
        narration_grid[dungeon_cell_index(0, 2, 1)] = 0xC1;
        let mut narration = dungeon_state(narration_grid, 0, 1, 1);
        narration.player.facing = Direction::East;
        narration.torch_counter = 5;
        assert_eq!(
            narration.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(narration.grid[dungeon_cell_index(0, 2, 1)], 0xC1);
        assert!(narration.pending_map_viewport_dissolves.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_fall_trap_reports_feature_without_triggering_drop() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x69;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x69);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("found a pit or trap"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_field_reports_feature_without_applying_effect() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x89;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: 30,
            mana: 8,
            hp: 10,
            max_hp: 20,
            level: 8,
        }];

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x89);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("found poison gas field"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_open_chest_consumes_turn_and_marks_visit_local_open_chest() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.ambient_light = FULL_DARKNESS;
        state.visibility_dirty = false;

        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x78);
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP - 12);
        assert_eq!(state.turn, 1);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Opened dungeon chest"));
        assert!(
            state
                .message
                .contains("Acid trap hit party member 1 for 12 HP.")
        );
        assert!(state.message.contains("marked visit-local open chest"));
        assert!(state.pending_map_viewport_dissolves.is_empty());
    }

    /// `traps.md §2.1`: the dungeon chest site uses the same interactive
    /// acting-member picker as the surface site when two or more members
    /// qualify. The command remains suspended until a valid pick, echoes the
    /// prompted member's name, and applies a single-slot trap to that member.
    #[test]
    fn dungeon_open_chest_prompts_and_uses_the_confirmed_member() {
        let mut grid = open_dungeon_record();
        let index = dungeon_cell_index(0, 1, 1);
        grid[index] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.party_names = default_party_names(2);
        state.party_names[1][..4].copy_from_slice(b"Iolo");

        assert_eq!(state.open_facing(), MoveOutcome::Observed);
        assert!(state.active_surface_chest.is_some());
        assert_eq!(state.grid[index], 0x4b);
        assert_eq!(state.turn, 0);

        let slot_zero_hp = state.party[0].hp;
        assert_eq!(
            state.step_active_surface_chest('2', "").unwrap(),
            Some(MoveOutcome::ContainerOpened)
        );

        assert!(state.active_surface_chest.is_none());
        assert_eq!(state.grid[index], 0x78);
        assert_eq!(state.turn, 1);
        assert_eq!(state.party[0].hp, slot_zero_hp);
        assert!(state.party[1].hp < DEFAULT_PARTY_HP);
        let selected_name = party_name_to_string(&state.party_names[1]).unwrap();
        assert!(
            state
                .message_entries()
                .iter()
                .any(|entry| entry.text == selected_name)
        );
        assert!(state.message.contains("Acid trap hit party member 2"));
    }

    #[test]
    fn dungeon_chest_picker_reprompts_disabled_and_cancel_leaves_chest_closed() {
        let mut grid = open_dungeon_record();
        let index = dungeon_cell_index(0, 1, 1);
        grid[index] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });
        state.party.push(PartyMember {
            slot: 2,
            class_byte: b'C',
            status: b'S',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });

        assert_eq!(state.open_facing(), MoveOutcome::Observed);
        assert_eq!(state.step_active_surface_chest('3', "").unwrap(), None);
        assert!(state.active_surface_chest.is_some());
        assert!(state.message.contains("unavailable"));
        assert_eq!(state.grid[index], 0x4b);
        assert_eq!(state.turn, 0);

        assert_eq!(
            state.step_active_surface_chest(' ', "").unwrap(),
            Some(MoveOutcome::PromptDeclined)
        );
        assert!(state.active_surface_chest.is_none());
        assert_eq!(state.grid[index], 0x4b);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_get_chest_consumes_turn_and_marks_visit_local_passage() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x7c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.ambient_light = FULL_DARKNESS;
        state.visibility_dirty = false;

        assert_eq!(
            state.get_dungeon_underfoot(scene, 0),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x08);
        assert_eq!(state.turn, 1);
        assert_eq!(state.door_tracker, None);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Got dungeon chest"));
        assert!(state.message.contains("generated chest grants"));
    }

    #[test]
    fn dungeon_get_chest_generated_rewards_follow_public_rows() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(7, 0, 2)] = 0x7c;
        let mut state = dungeon_state(grid, 7, 0, 2);
        state.food = 0;
        state.gold = 0;
        state.keys = 0;
        state.gems = 0;
        state.torches = 0;

        assert_eq!(
            state.get_dungeon_underfoot(scene, 7),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.grid[dungeon_cell_index(7, 0, 2)], 0x08);
        assert_eq!(state.food, 11);
        assert_eq!(state.gold, 52);
        assert_eq!(state.keys, 2);
        assert_eq!(state.gems, 1);
        assert_eq!(state.torches, 2);
        assert_eq!(state.potion_stock[7], 1);
        assert_eq!(state.scroll_stock[6], 1);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("generated chest grants 11 food"));
        assert!(state.message.contains("52 gold"));
        assert!(state.message.contains("1 white potion"));
        assert!(state.message.contains("1 CIM scroll"));
    }

    #[test]
    fn dungeon_get_chest_applies_clean_sidecar_grants() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_CHEST_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x4c GOLD 7 GEMS 2 TORCHES 1\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x7c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.gold = 10;
        state.gems = 1;
        state.torches = 0;

        assert_eq!(
            state
                .get_dungeon_underfoot_with_game_dir(Some(&dir), scene, 0)
                .unwrap(),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x08);
        assert_eq!(state.gold, 17);
        assert_eq!(state.gems, 3);
        assert_eq!(state.torches, 1);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Got dungeon chest"));
        assert!(
            state
                .message
                .contains("authored chest grants 7 gold, 2 gems, 1 torches")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_open_chest_does_not_apply_clean_sidecar_grants() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_CHEST_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x4b KEYS 2 FOOD 5\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.keys = 1;
        state.food = 12;

        assert_eq!(
            state.open_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::ContainerOpened
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x78);
        assert_eq!(state.keys, 1);
        assert_eq!(state.food, 12);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Opened dungeon chest"));
        assert!(!state.message.contains("authored chest grants"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_search_chest_ignores_clean_sidecar_grants_and_guard() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_CHEST_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x4c KEYS 2\n",
        )
        .unwrap();
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x4c;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.keys = 1;

        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x4c);
        assert_eq!(state.keys, 1);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Searched dungeon chest"));
        assert!(!state.message.contains("authored chest grants"));

        let mut mismatch_grid = open_dungeon_record();
        mismatch_grid[dungeon_cell_index(0, 2, 1)] = 0x4b;
        let mut mismatch = dungeon_state(mismatch_grid, 0, 1, 1);
        mismatch.player.facing = Direction::East;
        mismatch.torch_counter = 5;
        mismatch.keys = 1;

        assert_eq!(
            mismatch.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );

        assert_eq!(mismatch.keys, 1);
        assert_eq!(mismatch.grid[dungeon_cell_index(0, 2, 1)], 0x4b);
        assert!(!mismatch.message.contains("generated chest grants"));
        let _ = fs::remove_dir_all(dir);
    }

