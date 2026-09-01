// Regression tests for `systems/npc-schedules.md` conformance defects.
// Every assertion here is anchored to a published sentence; several of
// them replace behaviour the engine inherited from spec revisions that
// were later retracted (the tile-set polarity, the zero-based hidden-NPC
// scene table, and the floor-ordering convention).

/// Fully open NPC pathfinding ground. Tile `0x04` is in §10's open list
/// (`0x04..0x0B`); the shared `open_grid()` fixture uses `0x10`, which
/// §10 lists as an obstacle for NPC routing.
fn npc_open_grid() -> Vec<u8> {
    vec![0x04; 1024]
}

fn scheduled_npc(slot: usize, x: usize, y: usize, z: u8, waypoint: (u8, u8, u8)) -> RuntimeNpc {
    let mut schedule = [0u8; 16];
    for wp in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
        schedule[NPC_SCHEDULE_X_OFFSET + wp] = waypoint.0;
        schedule[NPC_SCHEDULE_Y_OFFSET + wp] = waypoint.1;
        schedule[NPC_SCHEDULE_Z_OFFSET + wp] = waypoint.2;
    }
    RuntimeNpc {
        slot,
        type_byte: 0x50,
        dialog_id: 2,
        schedule,
        state: NPC_STATE_IDLE,
        x,
        y,
        z,
        cached_wp: NPC_SCHEDULE_WAYPOINT_COUNT - 1,
        move_queue: Vec::new(),
        move_queue_pos: 0,
        stuck_counter: 0,
        active_object: None,
    }
}

#[test]
fn npc_routing_walks_unlocked_doors_and_chairs_and_stops_at_locked_doors() {
    // npc-schedules.md §10: "the plain wooden door 0xB8 and the wooden
    // door with a window 0xBA are open for NPC routing, while the locked
    // door 0xB9 and the locked door with a window 0xBB are obstacles",
    // and "chairs are walkable for NPC routing and beds are not".
    let mut grid = npc_open_grid();
    grid[1 * 32 + 2] = 0xB8;
    grid[2 * 32 + 2] = 0xBA;
    grid[3 * 32 + 2] = 0x91;
    grid[4 * 32 + 2] = 0xB9;
    grid[5 * 32 + 2] = 0xBB;
    grid[6 * 32 + 2] = 0xAB;
    let mut state = test_state(grid, 20, 20);
    state.npcs.push(scheduled_npc(1, 1, 1, 0, (9, 9, 0)));

    for y in [1usize, 2, 3] {
        assert!(
            state.npc_can_step_toward(0, 2, y, 0, 9, 9),
            "row {y} should be routable"
        );
    }
    for y in [4usize, 5, 6] {
        assert!(
            !state.npc_can_step_toward(0, 2, y, 0, 9, 9),
            "row {y} should be an obstacle"
        );
    }

    // §10 rule 1, the waypoint-match escape hatch: an authored waypoint
    // that sits on an obstacle tile is still reachable.
    assert!(state.npc_can_step_toward(0, 2, 6, 0, 2, 6));
}

#[test]
fn npc_floor_link_marker_follows_the_published_state_table() {
    // npc-schedules.md §8.5: "the walker hunts the link that points toward
    // whichever floor is not the displayed one."
    // State 6 - NPC on the displayed floor, waypoint above.
    assert_eq!(npc_floor_link_marker_toward(1, 2), NPC_FLOOR_LINK_TILE_C8);
    // State 7 - waypoint below.
    assert_eq!(npc_floor_link_marker_toward(1, 0), NPC_FLOOR_LINK_TILE_C9);
    // State 4 - NPC above the displayed floor, waypoint on it: the
    // "other" floor is the NPC's own, so it hunts the ascend link.
    assert_eq!(npc_floor_link_marker_toward(1, 2), NPC_FLOOR_LINK_TILE_C8);
    // State 5 - NPC below the displayed floor.
    assert_eq!(npc_floor_link_marker_toward(1, 0), NPC_FLOOR_LINK_TILE_C9);
    // §6: the basement byte 0xFF orders below 0x00, so a basement NPC
    // surfaces at a descend link, not an ascend link.
    assert_eq!(
        npc_floor_link_marker_toward(0x00, 0xFF),
        NPC_FLOOR_LINK_TILE_C9
    );
    assert_eq!(
        npc_floor_link_marker_toward(0xFF, 0x00),
        NPC_FLOOR_LINK_TILE_C8
    );
}

#[test]
fn npc_floor_link_acceptance_is_asymmetric_between_gate_and_arrival() {
    // npc-schedules.md §8.5 "Stairway acceptance": both halves accept the
    // visible stairway family, but "the on-floor gate accepts a slightly
    // wider band of tile ids than the off-floor arrival test does,
    // additionally treating 0xCC..0xCF as stairway-like".
    let marker = NPC_FLOOR_LINK_TILE_C8;
    assert!(npc_floor_link_gate_accepts(marker, marker));
    assert!(npc_floor_link_arrival_accepts(marker, marker));
    for stair in TOWN_STAIR_TILE_FIRST..=TOWN_STAIR_TILE_LAST {
        assert!(npc_floor_link_gate_accepts(stair, marker), "{stair:#04x}");
        assert!(npc_floor_link_arrival_accepts(stair, marker), "{stair:#04x}");
    }
    for wide in 0xCCu8..=0xCF {
        assert!(npc_floor_link_gate_accepts(wide, marker), "{wide:#04x}");
        assert!(
            !npc_floor_link_arrival_accepts(wide, marker),
            "{wide:#04x} is gate-only"
        );
    }
    // The opposite link marker is not a substitute for the state's own.
    assert!(!npc_floor_link_gate_accepts(NPC_FLOOR_LINK_TILE_C9, marker));
    assert!(!npc_floor_link_arrival_accepts(
        NPC_FLOOR_LINK_TILE_C9,
        marker
    ));
}

#[test]
fn npc_floor_transition_gate_accepts_a_stairway_tile() {
    // npc-schedules.md §8.5: the states 6/7 gate accepts "the
    // direction-matching link ... or a stairway-family tile", and on
    // acceptance the NPC leaves the displayed floor.
    let mut grid = npc_open_grid();
    grid[1 * 32 + 1] = TOWN_STAIR_TILE_FIRST;
    let mut state = test_state(grid, 20, 20);
    let mut npc = scheduled_npc(1, 1, 1, 0, (4, 4, 1));
    npc.state = NPC_STATE_CLIMB_UP_OFF_FLOOR;
    state.npcs.push(npc);

    let mut searched = false;
    state.advance_npc_floor_transition_step(0, 0, 4, 4, 1, 0, &mut searched);

    assert_ne!(state.npcs[0].z, 0, "the NPC should have left floor 0");
    assert!(state.npcs[0].active_object.is_none());
    // The gate performs no search, so the per-tick latch stays clear.
    assert!(!searched);
}

#[test]
fn npc_state_eight_places_the_npc_at_its_waypoint_ungated() {
    // npc-schedules.md §7: state 8 "is *not* a parked state: the walker
    // resolves it immediately by writing the active waypoint's (x, y, z)
    // straight into the NPC's runtime position, caching the waypoint,
    // deactivating the move queue and returning the state to idle."
    let mut state = test_state(npc_open_grid(), 20, 20);
    state.area = Area::Town {
        scene: Scene::new(0x11).unwrap(),
        floor: 1,
    };
    let mut npc = scheduled_npc(1, 1, 1, 3, (5, 6, 2));
    npc.state = NPC_STATE_PARKED_OFF_FLOOR;
    npc.move_queue = vec![NPC_PATH_DIR_EAST, NPC_PATH_DIR_EAST];
    npc.stuck_counter = 2;
    state.npcs.push(npc);

    state.advance_npc_schedules();

    let wp = waypoint_for_hour(&state.npcs[0].schedule, state.clock.hour);
    assert_eq!((state.npcs[0].x, state.npcs[0].y, state.npcs[0].z), (5, 6, 2));
    assert_eq!(state.npcs[0].state, NPC_STATE_IDLE);
    assert_eq!(state.npcs[0].cached_wp, wp);
    assert!(state.npcs[0].move_queue.is_empty());
    assert_eq!(state.npcs[0].stuck_counter, 0);
    // Neither end is on the displayed floor, so no sprite is allocated.
    assert!(state.npcs[0].active_object.is_none());
    assert!(!state.visibility_dirty);
}

#[test]
fn npc_unexpected_state_byte_takes_the_same_ungated_placement() {
    // npc-schedules.md §7: "The same ungated placement is what happens if
    // any unexpected state value reaches the floor-transition arm."
    let mut state = test_state(npc_open_grid(), 20, 20);
    state.area = Area::Town {
        scene: Scene::new(0x11).unwrap(),
        floor: 1,
    };
    let mut npc = scheduled_npc(1, 1, 1, 3, (5, 6, 2));
    npc.state = 200;
    state.npcs.push(npc);

    state.advance_npc_schedules();

    assert_eq!((state.npcs[0].x, state.npcs[0].y, state.npcs[0].z), (5, 6, 2));
    assert_eq!(state.npcs[0].state, NPC_STATE_IDLE);
}

#[test]
fn npc_schedule_search_latch_allows_one_fresh_search_per_tick() {
    // npc-schedules.md §7: "At most one NPC per tick may start a fresh
    // search... every later slot in the same tick that would have
    // searched is skipped until the next tick."
    let mut grid = npc_open_grid();
    // Block each NPC's direct cardinal probe so both need the flood fill.
    grid[1 * 32 + 2] = 0xB9;
    grid[5 * 32 + 2] = 0xB9;
    let mut state = test_state(grid, 20, 20);
    let mut first = scheduled_npc(1, 1, 1, 0, (6, 1, 0));
    first.state = NPC_STATE_INPLANE_MOVE;
    let mut second = scheduled_npc(2, 1, 5, 0, (6, 5, 0));
    second.state = NPC_STATE_INPLANE_MOVE;
    state.npcs.push(first);
    state.npcs.push(second);

    state.advance_npc_schedules();

    assert!(
        !state.npcs[0].move_queue.is_empty(),
        "the first searching slot routes"
    );
    assert!(
        state.npcs[1].move_queue.is_empty(),
        "the second searching slot is skipped until the next tick"
    );
    assert_eq!(state.npcs[1].stuck_counter, 1);
}

#[test]
fn npc_search_latch_leaves_queue_replay_alone() {
    // npc-schedules.md §7: "Queue replay is not affected by the latch."
    let mut state = test_state(npc_open_grid(), 20, 20);
    let mut npc = scheduled_npc(1, 1, 1, 0, (6, 1, 0));
    npc.state = NPC_STATE_REPLAY_QUEUE;
    npc.move_queue = vec![NPC_PATH_DIR_EAST];
    state.npcs.push(npc);

    let outcome = state.advance_npc_replay_queue_step(0, 0, 6, 1, 0, 0);

    assert_eq!(outcome, NpcScheduleStepOutcome::Moved);
    assert_eq!((state.npcs[0].x, state.npcs[0].y), (2, 1));
}

#[test]
fn npc_queue_drain_re_enters_the_floor_transition_state_and_ends_the_tick() {
    // npc-schedules.md §7: "when a queued route drains while the NPC is
    // still in state 3, the walker re-reads the active waypoint and
    // re-enters state 6 or 7 according to whether that waypoint's floor
    // is above or below the displayed floor. That one transition also
    // ends the tick."
    let mut grid = npc_open_grid();
    grid[1 * 32 + 1] = NPC_FLOOR_LINK_TILE_C8;
    let mut state = test_state(grid, 20, 20);
    let mut first = scheduled_npc(1, 1, 1, 0, (4, 4, 1));
    first.state = NPC_STATE_REPLAY_QUEUE;
    let mut second = scheduled_npc(2, 8, 8, 0, (10, 8, 0));
    second.state = NPC_STATE_INPLANE_MOVE;
    state.npcs.push(first);
    state.npcs.push(second);

    state.advance_npc_schedules();

    assert_eq!(state.npcs[0].state, NPC_STATE_CLIMB_UP_OFF_FLOOR);
    assert_eq!(
        (state.npcs[1].x, state.npcs[1].y),
        (8, 8),
        "every slot after the one that ended the tick is skipped"
    );

    // A drained queue whose waypoint is on the displayed floor just goes
    // idle, and does not end the tick.
    let mut state = test_state(npc_open_grid(), 20, 20);
    let mut npc = scheduled_npc(1, 1, 1, 0, (4, 4, 0));
    npc.state = NPC_STATE_REPLAY_QUEUE;
    state.npcs.push(npc);
    assert_eq!(
        state.advance_npc_replay_queue_step(0, 0, 4, 4, 0, 0),
        NpcScheduleStepOutcome::Stalled
    );
    assert_eq!(state.npcs[0].state, NPC_STATE_IDLE);
}
