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
    // Issue #298 Q29: a countdown pass decrements the delay and leaves the
    // displayed frame alone ("the frame is unchanged").
    assert_eq!(object.tile, 168);
}

/// Drive idle ticks until the `weather.md §2` 1-in-64 selector fires,
/// returning the tick count. The selector now draws off the shared
/// gameplay PRNG (`prng.md §4`), so the firing tick is seed-dependent
/// rather than fixed; a few hundred ticks is far beyond the published
/// rate.
fn idle_until_wind_change(state: &mut PlayState) -> usize {
    for tick in 1..=4096 {
        let before = state.wind;
        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);
        if state.wind != before {
            return tick;
        }
    }
    panic!("the 1-in-64 wind selector never fired in 4096 idle ticks");
}

#[test]
fn idle_tick_can_apply_public_random_wind_drift_without_turn() {
    let mut state = britannia_state(open_world_grid(), 1, 10);
    state.clock = GameClock::new(12, 0).unwrap();
    state.wind = WindState::Calm;
    state.wind_save_byte = 0x7a;
    state.sail_cadence = 1;

    idle_until_wind_change(&mut state);

    // `weather.md §2`: the drift "is not a spell cast, does not consume a
    // turn by itself, and does not run from the time cleanup".
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
    assert_ne!(state.wind, WindState::Calm);
    assert_eq!(state.wind_save_byte, state.wind.save_byte());
    assert!((1..=4).contains(&state.wind_save_byte));
    // "Setting a new state also clears the cached wind-cadence byte used by
    // sailing and by wind-driven actors."
    assert_eq!(state.sail_cadence, 0);
    // The tick itself narrates nothing (`u5-engine#19`); the wind result
    // above is the whole of what it produces.
    assert!(state.message.is_empty());
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

    idle_until_wind_change(&mut state);

    // Wind state did update.
    assert_ne!(state.wind, WindState::Calm);
    // The banner is not drawn on this plane, and the tick prints no line
    // of its own either, so nothing reaches the window.
    assert!(state.message.is_empty());
}

/// `animation.md §13.1`: "**Negate Time freezes all of it.** ... For the
/// effect's full duration nothing advances: no water rotation, no fire
/// flicker, no fountain, no banner, no clock or bellows, no object animation,
/// no AI roll ... An engine that keeps animating during Negate Time is
/// visibly wrong."
///
/// The shared static-tile phase counter and the driver-side water rotation
/// are the water/fountain/banner/clock animation in this engine, so an idle
/// tick under Negate Time must leave both exactly where they were. An earlier
/// revision of this test asserted `animation.frame == 1` here, pinning the
/// withdrawn behaviour.
#[test]
fn idle_tick_wind_selector_runs_inside_a_town_scene() {
    // `weather.md §2`: "The store happens before any scene test, so the
    // state is always updated; only the banner repaint is conditional."
    // The paired Britain captures show the original cycling the wind while
    // the party is inside a town, which the old world-plane-only gate made
    // impossible.
    let mut state = test_state(open_grid(), 5, 5);
    state.wind = WindState::Calm;
    state.wind_save_byte = 0;

    idle_until_wind_change(&mut state);

    assert_ne!(state.wind, WindState::Calm);
    assert_eq!(state.turn, 0);
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
    assert_eq!(state.animation.frame, 0);
    assert_eq!(state.water_scroll.phase, 0);
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
    // Was `assert_eq!(state.door_tracker, None)`. The DOS build does not
    // clear the four-byte block when the door re-closes: a session that
    // opened one door and then took fourteen turns saved
    // `0x03A9..0x03AC` as `B8 0F 13 F6`, previous tile / X / Y intact.
    assert_eq!(
        state.door_tracker,
        Some(DoorTracker {
            previous_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
            x: 2,
            y: 1,
            turns_remaining: 0,
        })
    );
    assert!(state.door_tracker_closed);
    assert!(state.visibility_dirty);
}

#[test]
fn open_facing_non_door_commits_one_turn() {
    // `doors-and-z-transitions.md §4`: "Every resolved non-dungeon Open
    // attempt returns the dispatcher's ordinary committed-action status,
    // including the already-open, too-heavy, locked, and generic refusal
    // paths."
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(state.open_facing(), MoveOutcome::Blocked);
    assert_eq!(state.turn, 1);
    assert_eq!(state.grid[32 + 2], 16);
    assert_eq!(state.message, "Nothing to open!");
}

#[test]
fn world_open_with_no_container_commits_one_turn() {
    // `doors-and-z-transitions.md §4`: the generic refusal of the
    // non-dungeon Open still "returns the dispatcher's ordinary
    // committed-action status" on the overworld.
    let mut state = world_state(vec![0x05; WORLD_SIDE * WORLD_SIDE], 10, 10);

    assert_eq!(
        state.open_direction_with_game_dir(Direction::East, None).unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, "Nothing to open!");
    assert_eq!(state.turn, 1);
}

#[test]
fn open_facing_exact_too_heavy_tile_commits_one_turn() {
    let mut grid = open_grid();
    grid[32 + 2] = TOWN_OPEN_TOO_HEAVY_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(state.open_facing(), MoveOutcome::Blocked);

    assert_eq!(TOWN_OPEN_TOO_HEAVY_TILE, 0x99);
    assert_eq!(state.grid[32 + 2], 0x99);
    assert_eq!(state.message, "Too heavy!");
    assert_eq!(state.turn, 1);
    assert_eq!(state.door_tracker, None);
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
fn open_facing_cleared_door_cell_is_not_already_open() {
    // `doors-and-z-transitions.md §2`: "Open alone treats exact tile `0xAF`
    // as already open"; a door Open cleared is the ordinary `0x44` cell and
    // falls through to the generic refusal, still committing the turn.
    let mut grid = open_grid();
    grid[32 + 2] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.ambient_light = FULL_DAYLIGHT;

    assert_eq!(state.open_facing(), MoveOutcome::DoorOpened);
    state.visibility_dirty = false;

    assert_eq!(state.open_facing(), MoveOutcome::Blocked);

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
    assert_eq!(state.message, "Nothing to open!");
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
fn open_facing_first_open_door_after_second_door_overwrites_timer_is_plain_floor() {
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
    assert_eq!(state.open_facing(), MoveOutcome::Blocked);

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
    assert_eq!(state.message, "Nothing to open!");
}

#[test]
fn town_open_locked_sidecar_refuses_and_commits_turn() {
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
    assert_eq!(state.turn, 1);
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
            // Kind 1 is the chest object class (`magic.md`, An Sanct).
            type_byte: 0x50,
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
            // Kind 1 is the chest object class (`magic.md`, An Sanct).
            type_byte: 0x50,
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

/// A second Good member, so the shared acting-member selection is a real
/// choice and Jimmy opens its picker (`commands.md §5.8`).
fn push_second_jimmy_member(state: &mut PlayState) {
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
}

#[test]
fn town_jimmy_restraint_release_updates_live_npc_and_native_removal_mask() {
    let mut grid = open_grid();
    grid[32 + 2] = JIMMY_MANACLES_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.party[0].climb_stat = 30;
    push_second_jimmy_member(&mut state);
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
        state.step_active_jimmy('1', "\r", Path::new("")).unwrap(),
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
    assert_eq!(state.message, JIMMY_PRISONER_THANKS_LINE);

    state.load_scheduled_npcs(&slots);
    assert!(state.npcs.is_empty());
}

/// Playtest F02. `doors-and-z-transitions.md`, Jimmy restraints: the
/// release "attempts to set the persistent removal bit through the
/// ordinary removable-NPC class filter", and "If an already-marked NPC is
/// successfully targeted again before leaving, the command ... does not
/// repeat the schedule rewrite, thanks line, or moral-standing reward."
/// A townsperson (type 0x44) passes `town-mode.md §4`'s filter; a guard
/// does not, so its release stays repeatable.
#[test]
fn town_jimmy_restraint_release_marks_a_townsperson_once() {
    let release = |type_byte: u8| {
        let mut grid = open_grid();
        grid[32 + 2] = JIMMY_STOCKS_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 30;
        state.moral_standing = 0;
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
                type_byte,
                dialog_id: 0x17,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                name: None,
            },
        ]);
        let mut messages = Vec::new();
        for _ in 0..2 {
            assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);
            messages.push(state.message.clone());
        }
        (state, messages)
    };

    let (state, messages) = release(0x44);
    assert_eq!(state.removed_town_npc_flags.get(&17), Some(&0b10));
    assert_eq!(messages, vec![JIMMY_PRISONER_THANKS_LINE.to_string(), String::new()]);
    assert_eq!(state.moral_standing, 2);

    let (state, messages) = release(0x70);
    assert!(state.removed_town_npc_flags.get(&17).is_none_or(|mask| *mask == 0));
    assert_eq!(messages, vec![JIMMY_PRISONER_THANKS_LINE.to_string(); 2]);
    assert_eq!(state.moral_standing, 4);
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

/// `traps.md` §3, effect id 0 (Acid): the resolver "rolls a random
/// damage amount bounded by `1..30`, applies it to the triggering party
/// slot ... The roll is an inclusive `0..60` roll halved with truncation
/// and floored to one - the same shape `systems/magic.md` publishes for
/// Mani - so it is **not** uniform over `1..30`: low values are markedly
/// more likely."
///
/// The family and the raw roll are shared-PRNG draws (`traps.md §2`), so
/// the test steers the PRNG to a state whose trap lands on Acid and then
/// checks the reported damage against the hit points actually lost.
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

    let template = state.clone();
    let mut damage = 0u16;
    for seed in 0..=u16::MAX {
        state = template.clone();
        state.prng_state = seed;
        assert_eq!(state.open_facing(), MoveOutcome::Observed);
        assert!(state.active_surface_chest.is_some());
        assert_eq!(state.message, PARTY_SELECTION_PROMPT);
        assert_eq!(
            state.step_active_surface_chest('2', "\r").unwrap(),
            Some(MoveOutcome::ContainerOpened)
        );
        let notes = state.diagnostics.join(" ");
        if let Some(rest) = notes.split("Acid trap hit party member 2 for ").nth(1) {
            damage = rest.split(" HP.").next().unwrap().parse().unwrap();
            break;
        }
    }
    assert!((1..=30).contains(&damage), "acid damage {damage}");

    assert!(state.active_objects[1].is_empty());
    assert_eq!(state.grid[32 + 2], 16);
    assert_eq!(state.moral_standing, 6);
    assert!(state.visibility_dirty);
    assert_eq!(state.turn, 1);
    assert!(state.diagnostics.join(" ").contains("Opened object chest at (2, 1)"));
    // The resolver's own `ACID!` went to the transcript; this engine note
    // naming the slot and the damage is a diagnostic.
    assert_eq!(state.party[1].hp, DEFAULT_PARTY_HP - damage);
    let selected_name = party_name_to_string(&state.party_names[1]).unwrap();
    assert!(
        state
            .message_entries()
            .iter()
            // `commands.md §5.8`: the pick "completes that row with the
            // member's name" - the bare `Player: ` row this fixture opens.
            .any(|entry| entry.text.ends_with(&selected_name)),
        "a prompted pick echoes the selected member's name"
    );
    // `containers.md §4.1`: the surface/town transcript is a `Found:`
    // heading followed by one fixed family line per successful grant.
    assert!(state.message.starts_with("Found:\n"), "{:?}", state.message);
    // `containers.md §4`: each successful row is *staged* as a floor object
    // at the chest's cell for a later Get; nothing is credited here.
    assert_eq!(state.food, DEFAULT_FOOD_STOCK);
    assert_eq!(state.gold, DEFAULT_GOLD_STOCK);
    assert!(state
        .active_objects
        .iter()
        .skip(1)
        .any(|object| !object.is_empty() && (object.x, object.y) == (2, 1)));
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
        state
            .get_town_facing(Path::new(""), Scene::new(0x11).unwrap(), 0)
            .unwrap(),
        MoveOutcome::ContainerOpened
    );
    assert!(state.active_surface_chest.is_none());

    assert!(state.active_objects[1].is_empty());
    assert_eq!(state.grid[32 + 2], 16);
    assert_eq!(state.turn, 1);
    assert!(state.diagnostics.join(" ").contains("Got object chest at (2, 1)"));
    assert!(state.message.starts_with("Found:\n"), "{:?}", state.message);
}

/// `containers.md §4.1`: "`Chest empty!` if nothing granted" - a chest class
/// too low for any primary- or secondary-pool row to clear its threshold.
#[test]
fn town_open_object_chest_prints_chest_empty_when_nothing_grants() {
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
        aux1: 1,
        aux3: 0,
    });

    assert_eq!(
        state.consume_surface_object_chest_at(2, 1, 0, "Opened"),
        Some(MoveOutcome::ContainerOpened)
    );
    assert_eq!(state.message, CHEST_TRANSCRIPT_EMPTY_LINE);
}

/// `containers.md §4.1`/`traps.md §4`: "print `Trapped!` plus one line feed
/// - no leading feed, no member name, no trap-kind word" before the shared
/// resolver's own line, independent of whether the chest itself grants
/// anything.
#[test]
fn town_open_object_chest_prints_trapped_notice_before_resolver_line() {
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
        aux1: 0x81,
        aux3: 0,
    });

    assert_eq!(
        state.consume_surface_object_chest_at(2, 1, 0, "Opened"),
        Some(MoveOutcome::ContainerOpened)
    );
    let entries: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(
        entries.iter().any(|text| *text == "Trapped!"),
        "{:?}",
        entries
    );
    assert_eq!(state.message, CHEST_TRANSCRIPT_EMPTY_LINE);
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
    // `commands.md §5.8`: Escape is the picker's cancel; Space commits
    // the indicated row (`inventory.md §4.3`).
    assert_eq!(
        state.step_active_surface_chest('\u{1b}', "").unwrap(),
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
    // The trap fired: `traps.md §3`'s own word went to the
    // transcript through the resolver, and the engine note naming
    // the slot and the damage went to `diagnostics`.
    assert!(state.diagnostics.join(" ").to_lowercase().contains("trap"));

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
    // `doors-and-z-transitions.md §4`: the generic refusal still commits.
    assert_eq!(state.turn, turn_after_first + 1);
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
fn town_jimmy_prompts_for_direction_then_party_without_turn() {
    let dir = debug_game_dir();
    fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    push_second_jimmy_member(&mut state);

    assert_eq!(
        handle_play_key_input(&mut state, 'J', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "Jimmy-");
    assert!(state.active_direction_prompt.is_some());
    assert!(state.active_jimmy.is_none());
    assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_LOCKED_TILE);
    assert_eq!(state.keys, DEFAULT_KEY_STOCK);
    assert_eq!(state.turn, 0);

    assert_eq!(
        handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert!(state.active_direction_prompt.is_none());
    assert!(state.active_jimmy.is_some());
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
    push_second_jimmy_member(&mut state);

    handle_play_key_input(&mut state, 'J', "", &dir).unwrap();
    assert!(state.active_direction_prompt.is_some());
    assert_eq!(state.turn, 0);

    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();
    assert!(state.active_jimmy.is_some());
    assert_eq!(state.turn, 0);

    assert_eq!(
        handle_play_key_input(&mut state, '1', "\r", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_jimmy.is_none());
    assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "Unlocked!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn active_town_jimmy_picker_rejects_ineligible_pick_and_echoes_eligible_name() {
    // `commands.md §5.8`: "a confirmed eligible pick completes that row
    // with the member's name. An ineligible pick prints `Disabled!\n\n`
    // and repeats the prompt." Eligible is Good or Poisoned only
    // (`traps.md §2.1`): a Charmed member is conscious but not eligible.
    let dir = debug_game_dir();
    fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    push_second_jimmy_member(&mut state);
    let mut charmed = state.party[1];
    charmed.slot = 2;
    charmed.status = b'C';
    state.party.push(charmed);
    state.party_names = default_party_names(3);
    state.party_names[1][..4].copy_from_slice(b"Iolo");
    assert!(state.party[2].conscious());

    handle_play_key_input(&mut state, 'J', "", &dir).unwrap();
    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();
    assert!(state.active_jimmy.is_some());

    handle_play_key_input(&mut state, '3', "\r", &dir).unwrap();
    assert!(state.active_jimmy.is_some());
    assert!(state
        .message_entries()
        .iter()
        .any(|entry| entry.text == DUNGEON_ACTING_MEMBER_DISABLED.trim_end()));
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_LOCKED_TILE);
    assert_eq!(state.turn, 0);

    handle_play_key_input(&mut state, '2', "\r", &dir).unwrap();
    assert!(state.active_jimmy.is_none());
    assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
    assert!(
        state
            .message_entries()
            .iter()
            .any(|entry| entry.text.contains("Iolo")),
        "{:?}",
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>()
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_jimmy_one_member_party_picks_silently() {
    // `commands.md §5.8`: "Exactly one is chosen silently: no prompt and
    // no name echo", and the exchange "applies whenever Get, Jimmy, Open
    // or dungeon Look reaches the shared selector". Playtest E01.
    let dir = debug_game_dir();
    fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.party[0].climb_stat = 30;
    assert_eq!(state.party.len(), 1);

    handle_play_key_input(&mut state, 'J', "", &dir).unwrap();
    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();

    assert!(state.active_jimmy.is_none());
    assert_ne!(state.message, PARTY_SELECTION_PROMPT);
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
        // Two conscious members, so the shared acting-member selection
        // is a real choice and opens the picker. With one it picks
        // silently - see `jimmy_direction_with_game_dir_and_member`.
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });

    state.keys = 0;

    assert_eq!(
        handle_play_key_input(&mut state, 'J', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_jimmy.is_some());
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert_eq!(state.turn, 0);
    assert_eq!(state.keys, 0);

    assert_eq!(
        handle_play_key_input(&mut state, '1', "\r", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_jimmy.is_none());
    assert_eq!(state.message, DUNGEON_CHEST_JIMMY_NO_KEYS);
    assert_eq!(state.turn, 1);
    assert_eq!(state.keys, 0);
}

#[test]
fn active_dungeon_jimmy_cancel_commits_one_action() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
    let mut state = dungeon_state(grid, 0, 1, 1);
        // Two conscious members, so the shared acting-member selection
        // is a real choice and opens the picker. With one it picks
        // silently - see `jimmy_direction_with_game_dir_and_member`.
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 8,
            hp: DEFAULT_PARTY_HP,
            max_hp: DEFAULT_PARTY_MAX_HP,
            level: 8,
        });

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

/// `doors-and-z-transitions.md §8` (`RETRACTIONS.md` R412): the town hidden
/// door is the terrain byte `0x4E` and needs no sidecar row at all.
///
/// **Measured** 2026-09-08 (`qa/paired/town-hidden-door.tsv`), against the
/// `0x4E` cell at Britain `(30, 12)`: Look answers `Thou dost see a wall with
/// a nick`, Search answers `Thou dost find` / `a hidden door!`, and Open on
/// what it leaves answers `Locked!` - so the published replacement id `0xB9`
/// is the *locked* form, whatever §8's adjective says.
#[test]
fn town_search_reveals_the_published_hidden_door_terrain_without_a_sidecar() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 2] = TOWN_HIDDEN_DOOR_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched
    );

    assert_eq!(state.grid[32 + 2], TOWN_HIDDEN_DOOR_REVEAL_ABOVE_GROUND);
    assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_LOCKED_TILE);
    assert_eq!(state.message, TOWN_SEARCH_HIDDEN_DOOR_LINE);
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_search_uses_clean_sidecar_to_reveal_secret_door() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(SECRET_DOOR_TABLE_FILE),
        "TOWN CASTLE:0 0 2 1 184\n",
    )
    .unwrap();
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
    // The coordinates are a harness diagnostic now, not player text
    // (`commands.md §8.1`).
    assert!(state.message.is_empty());
    assert!(state.diagnostics_contain("Revealed secret door at (2, 1)."));
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
    assert_eq!(state.message, SEARCH_NOTHING_FOUND);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_open_revealed_below_ground_secret_door_takes_the_ordinary_door_path() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(SECRET_DOOR_TABLE_FILE),
        "TOWN CASTLE:0 0 2 1 184\n",
    )
    .unwrap();
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

    // `RETRACTIONS.md` R448 / `doors-and-z-transitions.md` §8: the
    // below-ground reveal "accepts O-Open, prints `Opened!` and clears the
    // cell through the ordinary door-opening path". The engine used to give
    // the reveal its own path, with no auto-close tracker, on R412's withdrawn
    // "responds like an ordinary unlocked door" reading.
    assert_eq!(state.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
    assert_eq!(state.message, "Opened!");
    assert_eq!(state.turn, 2);
    assert_eq!(
        state.door_tracker.map(|tracker| (tracker.x, tracker.y)),
        Some((2, 1))
    );
    assert!(state.is_recorded_open_town_door(scene, 0, 2, 1));

    // The tracker restores the reveal tile, not a bare wall.
    assert_eq!(
        state
            .door_tracker
            .map(|tracker| tracker.previous_tile),
        Some(TOWN_HIDDEN_DOOR_REVEAL_BELOW_GROUND)
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn codex_entry_is_the_live_tile_and_needs_no_sidecar() {
    // `karma.md §8`: reading the Codex is the middle state of the shrine quest
    // cycle - "1. Correct shrine mantra ... 2. Reading the corresponding Codex
    // urn/page sets the Codex-read bit. 3. Returning to the shrine with both
    // bits set ...". The lookup used to return early whenever `codex_urns.tsv`
    // was absent, and no shipped profile carries one, so no virtue could ever
    // be completed.
    //
    // The gate is now the published one (`cleak/u5-spec#250`): "Use **E-Enter
    // while standing on live terrain tile `0x11`** to begin the Codex
    // interaction. Its shipped location is the Britannia surface cell
    // `(233, 233)` ... This tile arm does not require a separate
    // coordinate-table match."
    let dir = debug_game_dir();
    let (x, y) = crate::PUBLISHED_CODEX_URN_COORDINATE;
    let mut grid = open_world_grid();
    grid[world_cell_index(x, y)] = CODEX_SHRINE_ENTRY_TILE;
    // `world_state` seats the party in the Underworld; the Codex is a
    // Britannia landmark.
    let mut state = world_state(grid.clone(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();

    assert_eq!(
        handle_play_key_input(&mut state, 'E', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    // `karma.md §8.2`: the reader stamps on its third advance key.
    for _ in 0..3 {
        state.flush_staged_narration();
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    }
    assert_eq!(state.shrine_codex_mask, ShrineVirtue::Honesty.bit());

    // One cell away is not the Codex: the ordinary refusal, and no bit set.
    let mut elsewhere = world_state(grid, x, y + 2);
    elsewhere.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    elsewhere.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
    assert_eq!(
        handle_play_key_input(&mut elsewhere, 'E', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(elsewhere.shrine_codex_mask, 0);
    let _ = fs::remove_dir_all(dir);
}
#[test]
fn town_ground_floor_secret_door_reveal_is_locked() {
    // `RETRACTIONS.md` R448 / `doors-and-z-transitions.md` §8: "direct O-Open
    // on the ground/upper-floor reveal prints `Locked!\n` and leaves the door
    // unchanged". The engine gave every revealed secret door a lock bypass,
    // on R412's now-withdrawn "responds like an ordinary unlocked door".
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = TOWN_HIDDEN_DOOR_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched
    );
    assert_eq!(state.grid[32 + 2], TOWN_HIDDEN_DOOR_REVEAL_ABOVE_GROUND);
    assert!(state.is_revealed_town_secret_door(scene, 0, 2, 1));

    assert_eq!(
        state.open_facing_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, "Locked!");
    assert_eq!(state.grid[32 + 2], TOWN_HIDDEN_DOOR_REVEAL_ABOVE_GROUND);
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
    fs::write(
        dir.join(SECRET_DOOR_TABLE_FILE),
        "TOWN CASTLE:0 0 2 1 184\n",
    )
    .unwrap();
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
    // The floor reload cancels the pending close; the other three bytes
    // stay resident (playtest N23).
    assert_eq!(state.door_tracker.map(|tracker| tracker.previous_tile), Some(0));
    assert_eq!(
        state.door_tracker.map(|tracker| (tracker.x, tracker.y)),
        Some((2, 1))
    );
    assert_eq!(state.turn, 4);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_jimmy_revealed_secret_door_reports_no_lock() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(SECRET_DOOR_TABLE_FILE),
        "TOWN CASTLE:0 0 2 1 184\n",
    )
    .unwrap();
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
    assert_eq!(state.message, SEARCH_NOTHING_FOUND);
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
        state
            .active_direction_prompt
            .as_ref()
            .map(|session| session.kind),
        Some(DirectionPromptKind::DungeonSearch { party_index: Some(0) })
    );
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x30);
    assert_eq!(state.turn, 0);
    assert_eq!(
        handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xF0);
    assert_eq!(state.turn, 1);
    // `RETRACTIONS.md` R322: the found-door line.
    assert_eq!(state.message, DUNGEON_SEARCH_HIDDEN_DOOR);
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
        handle_play_key_input(&mut state, 'S', "6", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 2)], 0xF0);
    assert_eq!(state.turn, 1);
    // `RETRACTIONS.md` R322: the found-door line.
    assert_eq!(state.message, DUNGEON_SEARCH_HIDDEN_DOOR);
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
    // `dungeon-mode.md §8.1` "Search outcomes": the chest arm prints exactly
    // one of the four published trap-tier lines, "**none of which carries a
    // terminal period**", after the unconditional `You find:` preamble - it
    // no longer narrates coordinates.
    assert!([
        DUNGEON_SEARCH_NO_TRAP,
        DUNGEON_SEARCH_SIMPLE_TRAP,
        DUNGEON_SEARCH_GENERIC_TRAP,
        DUNGEON_SEARCH_COMPLEX_TRAP,
    ]
    .contains(&state.message.as_str()));
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
    // `dungeon-mode.md §8.1` "Search outcomes": the chest arm prints exactly
    // one of the four published trap-tier lines, "**none of which carries a
    // terminal period**", after the unconditional `You find:` preamble - it
    // no longer narrates coordinates.
    assert!([
        DUNGEON_SEARCH_NO_TRAP,
        DUNGEON_SEARCH_SIMPLE_TRAP,
        DUNGEON_SEARCH_GENERIC_TRAP,
        DUNGEON_SEARCH_COMPLEX_TRAP,
    ]
    .contains(&state.message.as_str()));
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
        DUNGEON_SEARCH_A_BOMB_TRAP
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
    // `dungeon-mode.md §8`: the threshold reads the acting member's
    // Dexterity. 31 on level 0 makes `2 * Z - Dex + 30` wrap under the
    // unsigned convention, so every `1..30` roll is at or below it.
    state.party[0].climb_stat = 31;

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched
    );

    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x62);
    assert_eq!(state.turn, 1);
    // `RETRACTIONS.md` R324: a `0x62` roll at or below the threshold takes
    // the `Nothing of note.` outcome.
    assert_eq!(state.message, DUNGEON_SEARCH_NOTHING_OF_NOTE);
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
        state
            .search_dungeon_focus_with_game_dir(DungeonLookFocus::Ahead, &dir)
            .unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x30);
    assert_eq!(state.turn, 0);
    // `RETRACTIONS.md` R323: the unlit Search refusal is a *find* line with a
    // leading blank row - there is no "too dark" literal anywhere.
    assert_eq!(state.message, DUNGEON_SEARCH_DARKNESS_REFUSAL);
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
    // `RETRACTIONS.md` R322: Search on exact `0x61` reports `A pit!`.
    assert_eq!(state.message, DUNGEON_SEARCH_A_PIT);
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
    // `RETRACTIONS.md` R322: `A hidden door!` is the `0xD?` wall branch.
    assert_eq!(state.message, DUNGEON_SEARCH_HIDDEN_DOOR);
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

    // `dungeon-mode.md §8` (audit A090): the branch keys on the dungeon
    // flavour. Despise is the normal flavour and Shame the mine flavour,
    // so the same `0xC0` byte only narrates there.
    for (record, line) in [
        (1, DUNGEON_SEARCH_NOTHING_ON_STALACTITE),
        (5, DUNGEON_SEARCH_NOTHING_IN_CAVED_IN_PASSAGE),
    ] {
        let mut narration_grid = open_dungeon_record();
        narration_grid[dungeon_cell_index(0, 2, 1)] = 0xC0;
        let mut narration = dungeon_state(narration_grid, 0, 1, 1);
        narration.area = Area::Dungeon {
            scene: DungeonScene::from_record(record).unwrap(),
            level: 0,
        };
        narration.player.facing = Direction::East;
        narration.torch_counter = 5;
        assert_eq!(
            narration.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched
        );
        assert_eq!(narration.grid[dungeon_cell_index(0, 2, 1)], 0xC0);
        assert_eq!(narration.message, line);
        assert!(narration.pending_map_viewport_dissolves.is_empty());
    }
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
    // `dungeon-mode.md §8`, "Other `0x6?` values": "No extra Search-specific
    // handling beyond the preamble."
    assert!(state.message.is_empty());
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
    // `dungeon-mode.md §8.1`: "The field classes reuse the L-Look wording of
    // this section", and L-Look (§12 step 5) names only the exact bytes
    // `0x80..0x83`: the marked `0x89` reads as the generic energy field.
    assert_eq!(state.message, "an energy field.");
    let _ = fs::remove_dir_all(dir);
}

/// `traps.md §3` effect id 0 (acid): the handler "rolls a random damage
/// amount bounded by `1..30`, applies it to the triggering party slot …
/// The roll is an inclusive `0..60` roll halved with truncation and
/// floored to one - the same shape `systems/magic.md` publishes for Mani -
/// so it is **not** uniform over `1..30`: low values are markedly more
/// likely." Both the family and the roll come from the shared PRNG.
#[test]
fn dungeon_open_chest_consumes_turn_and_marks_visit_local_open_chest() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.ambient_light = FULL_DARKNESS;
    state.visibility_dirty = false;

    // The trap family and the acid roll are shared-PRNG draws (`traps.md
    // §2`): steer to a state whose trap lands on Acid.
    let template = state.clone();
    let mut damage = 0u16;
    for seed in 0..=u16::MAX {
        state = template.clone();
        state.prng_state = seed;
        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);
        let notes = state.diagnostics.join(" ");
        if let Some(rest) = notes.split("Acid trap hit party member 1 for ").nth(1) {
            damage = rest.split(" HP.").next().unwrap().parse().unwrap();
            break;
        }
    }
    assert!((1..=30).contains(&damage), "acid damage {damage}");

    assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x78);
    assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP.saturating_sub(damage));
    assert_eq!(state.turn, 1);
    assert_eq!(state.door_tracker, None);
    assert!(state.visibility_dirty);
    assert!(state.diagnostics.iter().any(|note| note.contains("Opened dungeon chest")));
    assert!(
        state
            .diagnostics
            .iter()
            .any(|note| note.contains(&format!("Acid trap hit party member 1 for {damage} HP.")))
    );
    assert!(
        state
            .diagnostics
            .iter()
            .any(|note| note.contains("marked visit-local open chest"))
    );
    // `dungeon-mode.md §8.1`: the trap word, then `\nChest opened\n`.
    assert!(state.message.ends_with(DUNGEON_CHEST_OPENED));
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
        state.step_active_surface_chest('2', "\r").unwrap(),
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
            // `commands.md §5.8`: the pick "completes that row with the
            // member's name" - the bare `Player: ` row this fixture opens.
            .any(|entry| entry.text.ends_with(&selected_name))
    );
    assert!(state.diagnostics.iter().any(|note| note.contains("Acid trap hit party member 2")));
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
    assert_eq!(state.step_active_surface_chest('3', "\r").unwrap(), None);
    assert!(state.active_surface_chest.is_some());
    // `Disabled!` and the prompt again (playtest F13).
    assert!(state
        .message_entries()
        .iter()
        .any(|entry| entry.text == DUNGEON_ACTING_MEMBER_DISABLED.trim_end()));
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert_eq!(state.grid[index], 0x4b);
    assert_eq!(state.turn, 0);

    // `commands.md §5.8`: Escape is the picker's cancel; Space commits
    // the indicated row (`inventory.md §4.3`).
    assert_eq!(
        state.step_active_surface_chest('\u{1b}', "").unwrap(),
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
    assert!(state.diagnostics.iter().any(|note| note.contains("Got dungeon chest") || note.contains("got dungeon chest")));
    assert!(state.diagnostics.iter().any(|note| note.contains("generated chest grants") || note.contains("generated chest grants")));
}

/// `containers.md §6`: every gate and quantity draw comes from the shared
/// uniform random helper, in row order - the gate `1..(4 * depth + 4)`, then
/// the row's own quantity or subtype draw when the gate passes. The
/// expected transcript is replayed from the same seed here.
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
    state.prng_state = 0x2468;

    let mut prng = U5Prng::new(0x2468);
    let mut rows = Vec::new();
    let (mut food, mut gold, mut keys, mut gems, mut torches) = (0u16, 0u16, 0u8, 0u8, 0u8);
    let mut potion = None;
    let mut scroll = None;
    for row in DUNGEON_CHEST_ROWS {
        let gate = prng.next_range_u8(1, 32);
        if !dungeon_chest_row_awarded(row, gate) {
            continue;
        }
        match row.reward {
            DungeonChestReward::Food => {
                let n = prng.next_range_u8(1, 31);
                food += u16::from(n);
                rows.push(format!("{n} food!"));
            }
            DungeonChestReward::Gold => {
                let n = prng.next_range_u8(1, 56);
                gold += u16::from(n);
                rows.push(format!("{n} gold!"));
            }
            DungeonChestReward::Keys => {
                let n = prng.next_range_u8(1, 3);
                keys += n;
                rows.push(ObjectPickupKind::Keys.pickup_result_line(n));
            }
            DungeonChestReward::Gems => {
                let n = prng.next_range_u8(1, 3);
                gems += n;
                rows.push(ObjectPickupKind::Gems.pickup_result_line(n));
            }
            DungeonChestReward::Torches => {
                let n = prng.next_range_u8(1, 3);
                torches += n;
                rows.push(ObjectPickupKind::Torches.pickup_result_line(n));
            }
            DungeonChestReward::Potion => {
                let k = usize::from(prng.next_range_u8(0, 7));
                potion = Some(k);
                rows.push(ObjectPickupKind::Potion(k).pickup_result_line(1));
            }
            DungeonChestReward::Scroll => {
                let k = usize::from(prng.next_range_u8(0, 7));
                scroll = Some(k);
                rows.push(ObjectPickupKind::Scroll(k).pickup_result_line(1));
            }
        }
    }
    assert!(rows.len() >= 3, "seed should fire several rows: {rows:?}");

    assert_eq!(
        state.get_dungeon_underfoot(scene, 7),
        MoveOutcome::ContainerOpened
    );

    assert_eq!(state.grid[dungeon_cell_index(7, 0, 2)], 0x08);
    assert_eq!(state.prng_state, prng.state());
    assert_eq!(state.food, food);
    assert_eq!(state.gold, gold);
    assert_eq!(state.keys, keys);
    assert_eq!(state.gems, gems);
    assert_eq!(state.torches, torches);
    if let Some(k) = potion {
        assert_eq!(state.potion_stock[k], 1);
    }
    if let Some(k) = scroll {
        assert_eq!(state.scroll_stock[k], 1);
    }
    assert_eq!(state.turn, 1);
    // `containers.md §6` "What each row prints": the handler's own `Get` row,
    // the three-row preamble, then one row per reward row that fired, in the
    // table's order.
    assert_eq!(
        state.message,
        format!("{DUNGEON_CHEST_GET_CONTENTS}{}", rows.join("\n"))
    );
}

/// `containers.md §6`: "Nothing per-chest, per-room, per-level or
/// per-coordinate is consulted" - X and Y only index the cell, and the cell
/// byte's low nibble "cannot reach content generation". Two chests at
/// different coordinates with different low nibbles, opened from the same
/// PRNG state, grant the same contents.
#[test]
fn dungeon_chest_contents_ignore_coordinates_and_cell_low_nibble() {
    let scene = DungeonScene::new(33).unwrap();
    let run = |x: usize, y: usize, cell: u8| {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(7, x, y)] = cell;
        let mut state = dungeon_state(grid, 7, x, y);
        state.prng_state = 0x0bad;
        state.turn = (x * 3 + y) as u64;
        state.get_dungeon_underfoot(scene, 7);
        (state.message.clone(), state.food, state.gold, state.prng_state)
    };
    assert_eq!(run(0, 2, 0x7c), run(5, 6, 0x71));
}

/// `containers.md §6`: "**The generator is the whole contract. There is no
/// authored per-chest override.** ... An implementation carrying an authored
/// per-chest content sidecar has no counterpart for it in the original and
/// should retire it." This test used to assert the sidecar overrode the
/// generator.
#[test]
fn dungeon_get_chest_ignores_an_authored_content_sidecar() {
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
    assert_eq!(state.turn, 1);
    assert!(
        !state
            .diagnostics
            .iter()
            .any(|note| note.contains("authored chest grants"))
    );
    assert!(
        state
            .diagnostics
            .iter()
            .any(|note| note.contains("generated chest grants"))
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
    assert!(state.diagnostics.iter().any(|note| note.contains("Opened dungeon chest")));
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
    // `dungeon-mode.md §8.1` "Search outcomes": the chest arm prints exactly
    // one of the four published trap-tier lines, "**none of which carries a
    // terminal period**", after the unconditional `You find:` preamble - it
    // no longer narrates coordinates.
    assert!([
        DUNGEON_SEARCH_NO_TRAP,
        DUNGEON_SEARCH_SIMPLE_TRAP,
        DUNGEON_SEARCH_GENERIC_TRAP,
        DUNGEON_SEARCH_COMPLEX_TRAP,
    ]
    .contains(&state.message.as_str()));
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

/// `dungeon-mode.md §11`: the dungeon overlays' relative-direction prompt is
/// the open `Dir-` line, and the movement keys complete it with the relative
/// word. Measured 2026-09-07 against the stock game: up is `Ahead`, left and
/// right rotate a quarter turn, down is `Here`, and Space answers `Pass`.
#[test]
fn dungeon_search_relative_prompt_renders_dir_and_completes_with_the_relative_word() {
    let dir = debug_game_dir();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xd0;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;
    state.torch_counter = 5;

    assert!(state.handle_dungeon_key('S', &dir).unwrap());
    assert_eq!(state.message, DUNGEON_DIRECTION_PROMPT);

    assert_eq!(
        handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state
        .message_transcript
        .iter()
        .any(|entry| entry.text == "Dir-Ahead"));
    assert_eq!(state.message, DUNGEON_SEARCH_HIDDEN_DOOR);
    assert!(state
        .message_transcript
        .iter()
        .any(|entry| entry.text == "You find:"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dungeon_search_relative_prompt_ignores_letters_and_passes_on_space() {
    let dir = debug_game_dir();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xd0;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;
    state.torch_counter = 5;

    assert!(state.handle_dungeon_key('S', &dir).unwrap());
    // Measured: `A`/`R`/`L`/`H` and Escape are not accepted here.
    for key in ['A', 'R', 'L', 'H', '\u{1b}'] {
        assert_eq!(
            handle_play_key_input(&mut state, key, "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_some());
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xd0);
    }

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_direction_prompt.is_none());
    assert!(state
        .message_transcript
        .iter()
        .any(|entry| entry.text == "Dir-Pass"));
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xd0);
    let _ = fs::remove_dir_all(dir);
}

/// The "Who acts" contract: silent for one eligible member, `Player: ` for
/// two or more, `Disabled!` for an ineligible pick, `None!` for none at all.
#[test]
fn dungeon_search_runs_the_shared_acting_member_prompt() {
    let dir = debug_game_dir();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xd0;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;
    state.torch_counter = 5;
    state.party.push(state.party[0]);
    state.party[1].status = b'D';
    state.party_names = vec![*b"Iolo     ", *b"Shamino  "];

    assert!(state.handle_dungeon_key('S', &dir).unwrap());
    // One eligible member of two: still silent, straight to `Dir-`.
    assert_eq!(state.message, DUNGEON_DIRECTION_PROMPT);
    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    state.party[1].status = b'G';
    assert!(state.handle_dungeon_key('S', &dir).unwrap());
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    handle_play_key_input(&mut state, '2', "", &dir).unwrap();
    assert_eq!(
        handle_play_key_input(&mut state, '\r', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state
        .message_transcript
        .iter()
        .any(|entry| entry.text == "Player: Shamino"));
    assert_eq!(state.message, DUNGEON_DIRECTION_PROMPT);
    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    state.party[0].status = b'S';
    state.party[1].status = b'S';
    assert!(state.handle_dungeon_key('S', &dir).unwrap());
    assert_eq!(state.message, DUNGEON_ACTING_MEMBER_NONE);
    assert!(state.active_direction_prompt.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dungeon_search_acting_member_prompt_refuses_a_disabled_pick() {
    let dir = debug_game_dir();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xd0;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;
    state.torch_counter = 5;
    state.party.push(state.party[0]);
    state.party.push(state.party[0]);
    state.party[2].status = b'S';
    state.party_names = vec![*b"Iolo     ", *b"Shamino  ", *b"Dupre    "];

    assert!(state.handle_dungeon_key('S', &dir).unwrap());
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    handle_play_key_input(&mut state, '3', "", &dir).unwrap();
    assert_eq!(
        handle_play_key_input(&mut state, '\r', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state
        .message_transcript
        .iter()
        .any(|entry| entry.text == "Disabled!"));
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert_eq!(
        state.active_party_selector.map(|session| session.target),
        Some(PartySelectorTarget::DungeonSearch)
    );
    let _ = fs::remove_dir_all(dir);
}

/// The sighted look line breaks after the colon, like the darkness form.
#[test]
fn dungeon_look_prints_the_two_row_you_see_line() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x00;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;
    state.torch_counter = 5;

    assert_eq!(state.look_dungeon(), MoveOutcome::Observed);

    assert!(state.message.starts_with(DUNGEON_LOOK_PREAMBLE));
    assert!(!state.message.contains("You see: "));
}

/// `dungeon-mode.md §8` chest Search (audit A087): the threshold uses the
/// *acting* member's Dexterity, and the `1..30` roll is a shared-PRNG draw.
/// Member 2 with Dexterity 30 on level 0 gives threshold `(0 - 30 + 30) / 2
/// = 0`, so any roll is above it and a plain closed chest reads `No trap`
/// after exactly one draw. Member 1 (Dexterity 0) would give 15.
#[test]
fn dungeon_search_chest_rolls_acting_member_dexterity_from_shared_prng() {
    let dir = debug_game_dir();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x40;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;
    state.torch_counter = 5;
    let mut second = state.party[0];
    second.slot = 1;
    second.climb_stat = 30;
    state.party[0].climb_stat = 0;
    state.party.push(second);
    state.prng_state = 0x1234;
    let mut expected_prng = state.prng_state;
    let _ = u5_prng_range_u16(&mut expected_prng, 1, 30);

    assert_eq!(
        state
            .search_dungeon_focus_with_game_dir_as(DungeonLookFocus::Ahead, &dir, Some(1))
            .unwrap(),
        MoveOutcome::Searched
    );
    assert_eq!(state.message, DUNGEON_SEARCH_NO_TRAP);
    assert_eq!(state.prng_state, expected_prng);
    let _ = fs::remove_dir_all(dir);
}

/// `dungeon-mode.md §8`, `0x62` Search row (audit A087): Dexterity 30 on
/// level 0 gives threshold 0, so the shared-PRNG `1..30` roll is above it
/// and the bomb springs, clearing the cell to `0x00`.
#[test]
fn dungeon_search_bomb_uses_acting_member_dexterity_and_shared_prng() {
    let dir = debug_game_dir();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x62;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;
    state.torch_counter = 5;
    state.party[0].climb_stat = 30;
    state.prng_state = 0x4321;
    let mut expected_prng = state.prng_state;
    let _ = u5_prng_range_u16(&mut expected_prng, 1, 30);

    assert_eq!(
        state
            .search_dungeon_focus_with_game_dir_as(DungeonLookFocus::Ahead, &dir, Some(0))
            .unwrap(),
        MoveOutcome::Searched
    );
    assert_eq!(state.message, DUNGEON_SEARCH_A_BOMB_TRAP);
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x00);
    assert_eq!(state.prng_state, expected_prng);
    let _ = fs::remove_dir_all(dir);
}

/// `dungeon-mode.md §2` / `§12` step 5 (audit A082): L-Look on class `0xC?`
/// prints the flavour's own description - stalactite for the normal
/// flavour, caved-in passage for the mine flavour, and the corpse line for
/// flavour byte 3, which becomes the software-pirate line on a one-in-255
/// draw.
#[test]
fn dungeon_look_flavour_class_prints_the_flavour_description() {
    let look = |record: u8, prng_state: u16| {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xC0;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.area = Area::Dungeon {
            scene: DungeonScene::from_record(record).unwrap(),
            level: 0,
        };
        state.player.facing = Direction::East;
        state.torch_counter = 5;
        state.prng_state = prng_state;
        assert_eq!(state.look_dungeon(), MoveOutcome::Observed);
        (state.message.clone(), state.prng_state)
    };
    // Despise is Normal, Shame is Mine: no draw on either.
    assert_eq!(
        look(1, 7),
        (format!("{DUNGEON_LOOK_PREAMBLE}a dripping stalactite."), 7)
    );
    assert_eq!(
        look(5, 7),
        (format!("{DUNGEON_LOOK_PREAMBLE}a caved in passage."), 7)
    );
    // Deceit is flavour byte 3: one shared-PRNG draw picks the line.
    let hits = |seed: u16| {
        let mut probe = seed;
        u5_prng_range_u16(&mut probe, 0, 254) == 0
    };
    let miss = (0..=u16::MAX).find(|seed| !hits(*seed)).unwrap();
    let hit = (0..=u16::MAX).find(|seed| hits(*seed)).unwrap();
    let mut after_miss = miss;
    let _ = u5_prng_range_u16(&mut after_miss, 0, 254);
    assert_eq!(
        look(0, miss),
        (
            format!("{DUNGEON_LOOK_PREAMBLE}a less fortunate adventurer."),
            after_miss
        )
    );
    assert_eq!(
        look(0, hit).0,
        format!("{DUNGEON_LOOK_PREAMBLE}an unfortunate software pirate.")
    );
}

/// `dungeon-mode.md §8.1`: the result continues the dispatcher's open
/// `Open-` echo row even when the turn before closed its own row. Measured
/// 2026-10-16: `dungeon-chest`, beat `opened` - stock `>Open-GAS!`, a blank,
/// `Chest opened`; `dungeon-chest-jimmy`, beats `opened` and `again` - stock
/// `>Open-` directly above `Chest opened`.
#[test]
fn dungeon_chest_result_continues_the_open_echo_row_after_a_closed_turn() {
    fn rows_after_open(tile: u8) -> Vec<String> {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = tile;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.emit_message_line("Must open first!\n");
        state.close_message_row();
        state.emit_command_echo_line(DUNGEON_CHEST_OPEN_ECHO);
        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);
        state.flush_message_slot();
        let rows: Vec<String> = state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect();
        let echo = rows
            .iter()
            .position(|row| row.starts_with(DUNGEON_CHEST_OPEN_ECHO))
            .expect("echo row");
        rows[echo..].to_vec()
    }
    let untrapped = rows_after_open(0x40);
    assert_eq!(untrapped[..2], ["Open-", "Chest opened"], "{untrapped:?}");
    let trapped = rows_after_open(0x4b);
    assert!(
        trapped[0].len() > DUNGEON_CHEST_OPEN_ECHO.len() && trapped[0].ends_with('!'),
        "the trap word completes the echo row: {trapped:?}"
    );
    assert_eq!(trapped[1..3], ["", "Chest opened"], "{trapped:?}");
}
