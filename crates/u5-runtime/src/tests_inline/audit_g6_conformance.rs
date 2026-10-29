// Regression tests for the 2026-10-16 conformance audit, cluster g6
// (`systems/npc-schedules.md`, `systems/town-mode.md`, `formats/npc.md`,
// `systems/active-objects.md`). Each test names its audit item.

fn g6_open_grid() -> Vec<u8> {
    vec![0x04; 1024]
}

fn g6_npc(slot: usize, x: usize, y: usize, ai: u8, waypoint: (u8, u8, u8)) -> RuntimeNpc {
    let mut schedule = [0u8; 16];
    for wp in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
        schedule[NPC_SCHEDULE_AI_OFFSET + wp] = ai;
        schedule[NPC_SCHEDULE_X_OFFSET + wp] = waypoint.0;
        schedule[NPC_SCHEDULE_Y_OFFSET + wp] = waypoint.1;
        schedule[NPC_SCHEDULE_Z_OFFSET + wp] = waypoint.2;
    }
    // Distinct boundaries that never equal the default test hour, so the
    // walker reaches the AI dispatch rather than the boundary trigger.
    schedule[NPC_SCHEDULE_TIME_OFFSET..NPC_SCHEDULE_TIME_OFFSET + 4].copy_from_slice(&[1, 2, 3, 4]);
    let wp = waypoint_for_hour(&schedule, GameClock::default().hour);
    RuntimeNpc {
        slot,
        type_byte: 0x50,
        dialog_id: 2,
        schedule,
        state: NPC_STATE_IDLE,
        x,
        y,
        z: waypoint.2,
        cached_wp: wp,
        move_queue: Vec::new(),
        move_queue_pos: 0,
        stuck_counter: 0,
        active_object: None,
    }
}

#[test]
fn a011_waypoint_selection_uses_most_recent_unsorted_boundary() {
    // formats/npc.md §5.1: boundaries "are not required to be sorted";
    // the active one has the smallest unsigned `hour - time[i]`.
    let mut schedule = [0u8; 16];
    schedule[NPC_SCHEDULE_TIME_OFFSET..NPC_SCHEDULE_TIME_OFFSET + 4]
        .copy_from_slice(&[8, 20, 12, 22]);
    // hour 13: most recent boundary is time[2] = 12 -> waypoint 2.
    assert_eq!(waypoint_for_hour(&schedule, 13), 2);
    // hour 21: time[1] = 20 -> waypoint 1.
    assert_eq!(waypoint_for_hour(&schedule, 21), 1);
    // hour 9: time[0] = 8 -> waypoint 0.
    assert_eq!(waypoint_for_hour(&schedule, 9), 0);
    // hour 3: wraps back to time[3] = 22 -> waypoint 1.
    assert_eq!(waypoint_for_hour(&schedule, 3), 1);
    assert_eq!(npc_schedule_waypoint_for_hour([8, 20, 12, 22], 13), 2);
    // Sorted schedules keep the published segment table.
    schedule[NPC_SCHEDULE_TIME_OFFSET..NPC_SCHEDULE_TIME_OFFSET + 4]
        .copy_from_slice(&[6, 12, 18, 22]);
    for (hour, wp) in [(6, 0), (11, 0), (12, 1), (18, 2), (21, 2), (22, 1), (5, 1)] {
        assert_eq!(waypoint_for_hour(&schedule, hour), wp, "hour {hour}");
    }
}

fn g6_town_with(npcs: Vec<RuntimeNpc>, px: usize, py: usize) -> PlayState {
    let mut state = test_state(g6_open_grid(), px, py);
    state.npcs = npcs;
    state.relink_npc_objects();
    state
}

fn g6_pos(state: &PlayState, index: usize) -> (usize, usize) {
    (state.npcs[index].x, state.npcs[index].y)
}

/// p10 R14. `npc-schedules.md §9.1`: "`200` .. `204` | No draw and no
/// replan. The counter is incremented" - "five consecutive passes on which
/// the NPC does nothing at all" - and `§7`: state 2 runs "search, route,
/// step" with "no per-direction sweep". A greedy direct step used to run
/// ahead of the band, moving the NPC and zeroing its counter.
#[test]
fn an_npc_in_the_stuck_aging_band_stands_still_and_ages() {
    let mut state = g6_town_with(vec![g6_npc(1, 5, 5, 0, (8, 5, 0))], 20, 20);
    state.npcs[0].stuck_counter = 200;
    let mut searched = false;
    for expected in 201..=204 {
        let outcome = state.advance_npc_in_plane_schedule_step(0, 0, 8, 5, 0, 0, &mut searched);
        assert_eq!(outcome, NpcScheduleStepOutcome::Stalled);
        assert_eq!(g6_pos(&state, 0), (5, 5), "no move inside the band");
        assert_eq!(state.npcs[0].stuck_counter, expected);
    }
    assert!(!searched, "the band spends no search");
}

/// p10 R15. `npc-schedules.md §9`: values 3 and 6 share one step chooser,
/// which tests "legal orthogonal neighbours in **east, north, west, south**
/// order", and "mode 3 retreats". With east, north and west all tied at the
/// farthest distance, the retreat takes east; it used to take west.
#[test]
fn a_fleeing_npc_breaks_a_distance_tie_east_first() {
    // Party at (5,8), NPC at (5,5): east (6,5), north (5,4) and west (4,5)
    // all reach distance 4.
    let state = g6_town_with(vec![g6_npc(1, 5, 5, 3, (5, 5, 0))], 5, 8);
    assert_eq!(state.town_npc_flee_step(0, 0), Some((6, 5)));
}

#[test]
fn a133_modes_three_and_six_act_only_strictly_inside_four() {
    // npc-schedules.md §9 "The engagement step": "Values 3/6 use a
    // Manhattan distance strictly below four from the NPC's current
    // position to the party."
    for ai in [3u8, 6] {
        let mut state = g6_town_with(vec![g6_npc(1, 9, 5, ai, (9, 5, 0))], 5, 5);
        state.advance_npc_schedules();
        assert_eq!(
            g6_pos(&state, 0),
            (9, 5),
            "AI {ai} at distance 4 must not act"
        );

        let mut state = g6_town_with(vec![g6_npc(1, 8, 5, ai, (8, 5, 0))], 5, 5);
        state.advance_npc_schedules();
        assert_ne!(g6_pos(&state, 0), (8, 5), "AI {ai} at distance 3 acts");
    }
}

#[test]
fn a133_mode_four_engages_from_the_waypoint_distance_alone() {
    // npc-schedules.md §9: "Value 4 instead measures from its selected
    // waypoint to the party". The NPC itself is ten cells away, but its
    // waypoint is two from the party, so it engages and approaches.
    let mut state = g6_town_with(vec![g6_npc(1, 15, 5, 4, (6, 6, 0))], 5, 5);
    state.advance_npc_schedules();
    assert_eq!(g6_pos(&state, 0), (14, 5));
}

#[test]
fn a134_approach_tests_east_north_west_south_first_fit() {
    // npc-schedules.md §9: "the chooser tests legal orthogonal neighbours
    // in east, north, west, south order and takes the first that strictly
    // reduces Manhattan distance to the party". dx = +1, dy = -5: east
    // reduces the distance and is tested first.
    let mut state = g6_town_with(vec![g6_npc(1, 4, 10, 7, (4, 10, 0))], 5, 5);
    state.advance_npc_schedules();
    assert_eq!(g6_pos(&state, 0), (5, 10));

    // East blocked: north is next.
    let mut grid = g6_open_grid();
    grid[10 * 32 + 5] = 0x4E;
    let mut state = test_state(grid, 5, 5);
    state.npcs = vec![g6_npc(1, 4, 10, 7, (4, 10, 0))];
    state.relink_npc_objects();
    state.advance_npc_schedules();
    assert_eq!(g6_pos(&state, 0), (4, 9));
}

#[test]
fn a135_self_created_adjacency_waits_for_a_later_invocation() {
    // npc-schedules.md §9: "When the NPC's own move first creates
    // adjacency, contact waits until a later engagement invocation."
    let mut state = g6_town_with(vec![g6_npc(1, 7, 5, 7, (7, 5, 0))], 5, 5);
    state.advance_npc_schedules();
    assert_eq!(g6_pos(&state, 0), (6, 5));
    assert_eq!(state.pending_town_npc_contact, None);
    state.advance_npc_schedules();
    assert_eq!(g6_pos(&state, 0), (6, 5), "contact produces no movement");
    assert_eq!(
        state.pending_town_npc_contact,
        Some(TownNpcContactEvent {
            npc_slot: 1,
            behavior: NpcAiBehavior::RandomChase,
        })
    );
}

#[test]
fn a135_last_producer_wins_and_no_pass_means_no_contact() {
    // npc-schedules.md §9.2: "if more than one NPC produces one during the
    // pass, the last producer visited supplies the event".
    let mut state = g6_town_with(
        vec![g6_npc(1, 6, 5, 7, (6, 5, 0)), g6_npc(2, 4, 5, 6, (4, 5, 0))],
        5,
        5,
    );
    let scene = Scene::new(0x11).unwrap();
    // No schedule pass has run: town mode has no event to handle.
    assert_eq!(
        state
            .apply_town_npc_contact_event(scene, 0, Path::new(""))
            .unwrap(),
        None
    );
    state.advance_npc_schedules();
    assert_eq!(
        state.pending_town_npc_contact.map(|event| event.npc_slot),
        Some(2)
    );
}

#[test]
fn a135_mode_four_contact_needs_the_engagement_path() {
    // npc-schedules.md §9: value 4 engages only while the party is closer
    // than four to its waypoint; adjacency alone "is not a global trigger".
    let mut state = g6_town_with(vec![g6_npc(1, 6, 5, 4, (20, 20, 0))], 5, 5);
    state.advance_npc_schedules();
    assert_eq!(state.pending_town_npc_contact, None);
}

#[test]
fn a136_guard_sprite_arrests_and_low_sprite_clears_the_slot() {
    // npc-schedules.md §9.2 / town-mode.md §14: the exact linked live
    // sprite 0x70 enters arrest; another sprite below 0x40 clears the slot.
    let scene = Scene::new(0x11).unwrap();
    let mut guard = g6_npc(1, 6, 5, 6, (6, 5, 0));
    guard.type_byte = 0x70;
    let mut state = g6_town_with(vec![guard], 5, 5);
    state.advance_npc_schedules();
    let _ = state
        .apply_town_npc_contact_event(scene, 0, Path::new(""))
        .unwrap();
    assert!(state.pending_town_arrest.is_some());

    let mut critter = g6_npc(1, 6, 5, 7, (6, 5, 0));
    critter.type_byte = 0x30;
    let mut state = g6_town_with(vec![critter], 5, 5);
    state.advance_npc_schedules();
    assert_eq!(
        state
            .apply_town_npc_contact_event(scene, 0, Path::new(""))
            .unwrap(),
        Some(MoveOutcome::Used)
    );
    assert!(state.npcs.is_empty());
    assert!(state.pending_town_arrest.is_none());
    assert!(!state.diagnostics_contain("alarm raised"));
}

#[test]
fn a127_per_step_gate_refuses_any_occupant_but_the_workspace_keeps_the_cutoff() {
    // npc-schedules.md §10: "the per-step gate applies no cutoff", while
    // the workspace builder ignores occupants four or more from the NPC's
    // current (x, y) (§8.3 phase 4 as corrected by issue #298 Q22, R536 -
    // the radius used to be measured from the destination).
    let mut state = g6_town_with(
        vec![
            g6_npc(1, 1, 1, 0, (20, 20, 0)),
            g6_npc(2, 8, 1, 0, (8, 1, 0)),
        ],
        30,
        30,
    );
    // The occupant at (8, 1) is seven from the NPC at (1, 1), even though
    // it sits within four of a destination at (9, 1).
    assert!(!state.npc_can_step_toward(0, 8, 1, 0, 9, 1));
    assert!(state.npc_workspace_cell_open(0, 8, 1, 0, 9, 1));
    // Within four of the NPC the workspace refuses whatever the destination.
    state.npcs[1].x = 3;
    state.relink_npc_objects();
    assert!(!state.npc_workspace_cell_open(0, 3, 1, 0, 20, 20));
    // Remove the occupant the way the engine's own removals do - its live
    // slot with it. The bare relink below frees only roster NPC slots, so a
    // record left behind would stay put.
    if let Some(slot) = state.npcs[1].active_object {
        state.free_active_object_slot(slot);
    }
    state.npcs.truncate(1);
    state.relink_npc_objects();
    assert!(state.npc_can_step_toward(0, 3, 1, 0, 20, 20));
}

#[test]
fn r536_workspace_overlay_stamps_only_person_and_creature_art() {
    // Issue #298 Q22 / R536: "Only person and creature art is stamped;
    // items, vehicles, horses and furniture below that type value are not."
    let mut state = g6_town_with(vec![g6_npc(1, 1, 1, 0, (20, 20, 0))], 30, 30);
    let mut item = ActiveObject::empty();
    item.type_byte = 0x3f;
    item.tile = 0x3f;
    item.x = 2;
    item.y = 1;
    let slot = state
        .active_objects
        .iter()
        .position(|object| object.is_empty() && !std::ptr::eq(object, &state.active_objects[0]))
        .unwrap_or(state.active_objects.len());
    if slot == state.active_objects.len() {
        state.active_objects.push(item);
    } else {
        state.active_objects[slot] = item;
    }
    assert!(state.npc_workspace_cell_open(0, 2, 1, 0, 20, 20));
    assert!(!state.npc_can_step_toward(0, 2, 1, 0, 20, 20));
    state.active_objects[slot].type_byte = 0x40;
    assert!(!state.npc_workspace_cell_open(0, 2, 1, 0, 20, 20));
    assert!(npc_workspace_overlay_stamps(31, 0x40));
    assert!(!npc_workspace_overlay_stamps(0, 0x40));
    assert!(!npc_workspace_overlay_stamps(5, NPC_WORKSPACE_OVERLAY_MIN_TYPE_EXCLUSIVE));
}

#[test]
fn r536_player_is_stamped_only_within_four_of_the_npc() {
    // Issue #298 Q22 / R536: "The player is therefore stamped only within
    // that radius; the overlay alone does not keep an NPC out of the
    // player's cell." The per-step gate still refuses it.
    let state = g6_town_with(vec![g6_npc(1, 1, 1, 0, (20, 20, 0))], 9, 1);
    assert!(state.npc_workspace_cell_open(0, 9, 1, 0, 20, 20));
    assert!(!state.npc_can_step_toward(0, 9, 1, 0, 20, 20));
    let state = g6_town_with(vec![g6_npc(1, 1, 1, 0, (20, 20, 0))], 3, 1);
    assert!(!state.npc_workspace_cell_open(0, 3, 1, 0, 20, 20));
}

#[test]
fn a131_bfs_expands_neighbours_in_the_cyclic_code_order() {
    // npc-schedules.md §8.4: four directions "in cyclic order" over codes
    // 1=W, 2=S, 3=E, 4=N, starting from the dequeued cell's current
    // direction (the start cell is seeded north). From (5,5) to (6,6) the
    // south neighbour is dequeued before the east one, and from south the
    // cycle tries east next, so the route is south-then-east.
    let state = g6_town_with(vec![g6_npc(1, 5, 5, 0, (6, 6, 0))], 20, 20);
    assert_eq!(
        state.npc_path_route(0, (5, 5), (6, 6), 0),
        Some(vec![NPC_PATH_DIR_SOUTH, NPC_PATH_DIR_EAST])
    );
}

#[test]
fn a130_floor_link_search_goals_are_exactly_the_marker_and_reachable() {
    // npc-schedules.md §8.3 phase 3 / §8.5: goals are the cells carrying
    // the selected marker; the stairway family is not a goal.
    let mut grid = g6_open_grid();
    grid[5 * 32 + 6] = TOWN_STAIR_TILE_FIRST;
    grid[5 * 32 + 9] = NPC_FLOOR_LINK_TILE_C9;
    let state = {
        let mut state = test_state(grid, 20, 20);
        state.npcs = vec![g6_npc(1, 5, 5, 0, (5, 5, 0))];
        state.relink_npc_objects();
        state
    };
    let route = state
        .npc_path_route_to_floor_link_marker(0, (5, 5), NPC_FLOOR_LINK_TILE_C9, (5, 5), 0)
        .unwrap();
    assert_eq!(route.len(), 4, "the stairway at distance 1 is not a goal");
    assert_eq!(
        state.floor_link_marker_coordinates(NPC_FLOOR_LINK_TILE_C9),
        vec![(9, 5)]
    );

    // States 4/5 take the nearest *reachable* marker by BFS from the active
    // waypoint (issue #298 Q22): the marker nearer the waypoint is walled
    // off, so the search reaches the far one.
    let mut grid = g6_open_grid();
    grid[10 * 32 + 10] = NPC_FLOOR_LINK_TILE_C8;
    for (x, y) in [(9, 10), (11, 10), (10, 9), (10, 11)] {
        grid[y * 32 + x] = 0x4E;
    }
    grid[2 * 32 + 2] = NPC_FLOOR_LINK_TILE_C8;
    let mut state = test_state(grid, 30, 30);
    let mut npc = g6_npc(1, 4, 2, 0, (11, 11, 0));
    npc.z = 1;
    npc.state = NPC_STATE_DESCEND_TOWARD_TARGET;
    state.npcs = vec![npc];
    let mut searched = false;
    assert!(state.advance_npc_floor_transition_step(0, 0, 11, 11, 0, 0, &mut searched));
    assert_eq!((state.npcs[0].x, state.npcs[0].y, state.npcs[0].z), (2, 2, 0));
    assert_eq!(state.npcs[0].state, NPC_STATE_INPLANE_MOVE);
}

#[test]
fn q22_states_four_five_search_from_the_waypoint_not_the_off_floor_npc() {
    // Issue #298 Q22, `npc-schedules.md §8.5`: "The search runs first,
    // starting from the active waypoint's (x, y) on the displayed floor.
    // The NPC's own coordinates belong to another floor and are not used,
    // so the link chosen is the one the search reaches first from the
    // waypoint." Here the link at (2, 2) is nearest the NPC's stored cell
    // and the link at (12, 12) is nearest the waypoint.
    let mut grid = g6_open_grid();
    grid[2 * 32 + 2] = NPC_FLOOR_LINK_TILE_C8;
    grid[12 * 32 + 12] = NPC_FLOOR_LINK_TILE_C8;
    let mut state = test_state(grid, 30, 30);
    let mut npc = g6_npc(1, 3, 2, 0, (11, 11, 0));
    npc.z = 1;
    npc.state = NPC_STATE_DESCEND_TOWARD_TARGET;
    state.npcs = vec![npc];
    let mut searched = false;
    assert!(state.advance_npc_floor_transition_step(0, 0, 11, 11, 0, 0, &mut searched));
    assert_eq!((state.npcs[0].x, state.npcs[0].y, state.npcs[0].z), (12, 12, 0));
}

#[test]
fn a132_refused_on_floor_gate_installs_the_route_and_replays_it() {
    // npc-schedules.md §8.5: on a refused 6/7 gate the walker "routes
    // toward it and enters the queue-replay state, so the NPC walks onto
    // the link and passes the gate on a later tick".
    let mut grid = g6_open_grid();
    grid[5 * 32 + 8] = NPC_FLOOR_LINK_TILE_C8;
    let mut state = test_state(grid, 20, 20);
    let mut npc = g6_npc(1, 5, 5, 0, (12, 12, 1));
    npc.z = 0;
    npc.state = NPC_STATE_CLIMB_UP_OFF_FLOOR;
    state.npcs = vec![npc];
    state.relink_npc_objects();

    state.advance_npc_schedules();
    assert_eq!((state.npcs[0].x, state.npcs[0].y, state.npcs[0].z), (6, 5, 0));
    assert_eq!(state.npcs[0].state, NPC_STATE_REPLAY_QUEUE);
    assert_eq!(
        state.npcs[0].move_queue[state.npcs[0].move_queue_pos..],
        [NPC_PATH_DIR_EAST, NPC_PATH_DIR_EAST]
    );
    // Later ticks replay the queue on the displayed floor ...
    state.advance_npc_schedules();
    state.advance_npc_schedules();
    assert_eq!((state.npcs[0].x, state.npcs[0].y, state.npcs[0].z), (8, 5, 0));
    assert_eq!(state.npcs[0].state, NPC_STATE_CLIMB_UP_OFF_FLOOR);
    // ... and the gate accepts on the next tick.
    state.advance_npc_schedules();
    assert_eq!((state.npcs[0].x, state.npcs[0].y, state.npcs[0].z), (12, 12, 1));
}

#[test]
fn a129_spent_search_latch_skips_a_blocked_state_two_npc() {
    // npc-schedules.md §7: once one NPC has searched this tick, "every later
    // slot in the same tick that would have searched is skipped until the
    // next tick" - no cap-zero recovery step, no draw, no aging.
    let mut grid = g6_open_grid();
    // Both NPCs face a wall on their direct step east.
    grid[2 * 32 + 3] = 0x4E;
    grid[8 * 32 + 3] = 0x4E;
    let mut state = test_state(grid, 30, 30);
    let mut first = g6_npc(1, 2, 2, 0, (6, 2, 0));
    first.state = NPC_STATE_INPLANE_MOVE;
    let mut second = g6_npc(2, 2, 8, 0, (6, 8, 0));
    second.state = NPC_STATE_INPLANE_MOVE;
    state.npcs = vec![first, second];
    state.relink_npc_objects();
    let prng_before_second = {
        let mut probe = state.clone();
        probe.npcs.truncate(1);
        probe.advance_npc_schedules();
        probe.prng_state
    };
    state.advance_npc_schedules();
    assert_ne!(g6_pos(&state, 0), (2, 2), "the first NPC searched and moved");
    assert_eq!(g6_pos(&state, 1), (2, 8), "the latched NPC stands still");
    assert_eq!(state.npcs[1].state, NPC_STATE_INPLANE_MOVE);
    assert_eq!(state.npcs[1].stuck_counter, 0);
    assert_eq!(state.prng_state, prng_before_second, "no draw is spent");
}

fn q30_town_scene(state: &PlayState) -> (Scene, i8) {
    let Area::Town { scene, floor } = state.area else {
        panic!("g6 fixtures are town states");
    };
    (scene, floor)
}

#[test]
fn q30_town_bed_elapsed_loop_runs_no_schedule_pass() {
    // Issue #298 Q30 / `RETRACTIONS.md` R538, `town-mode.md §7`: "the
    // elapsed-time loop that follows runs no walker". An AI 6 NPC three
    // cells away acts on any schedule pass (see a133 above), so it must stay
    // put across the ten-minute step.
    let mut state = g6_town_with(vec![g6_npc(1, 8, 5, 6, (8, 5, 0))], 5, 5);
    let (scene, floor) = q30_town_scene(&state);
    let minute_before = state.clock;
    let _ = state.advance_town_rest_until_target_hour(1, None, scene, floor);
    assert_ne!(state.clock, minute_before, "the loop took its ten-minute step");
    assert_eq!(g6_pos(&state, 0), (8, 5));
}

#[test]
fn q30_town_bed_burst_aborts_only_on_an_arrest_or_conflict_contact() {
    // Issue #298 Q30, `town-mode.md §12`: "If any pass leaves an
    // arrest/conflict contact event (kind `0x61`), the command ends at once
    // ... A conversation-contact event (kind `0x74`) does not interrupt".
    // The burst does not re-test the bed (`town-mode.md §12`: "Otherwise
    // the party sleeps"), so it takes no rest-surface inputs.
    let mut state = g6_town_with(vec![g6_npc(1, 6, 5, 7, (6, 5, 0))], 5, 5);
    let clock = state.clock;
    assert_eq!(
        state.advance_town_rest_initial_schedule_burst(),
        TownRestBurstOutcome::ArrestOrConflictContact
    );
    assert_eq!(state.clock, clock, "no time passed");

    let mut state = g6_town_with(vec![g6_npc(1, 6, 5, 4, (6, 5, 0))], 5, 5);
    assert_eq!(
        state.advance_town_rest_initial_schedule_burst(),
        TownRestBurstOutcome::Completed
    );
}

#[test]
fn q30_town_bed_hole_up_arrest_contact_ends_with_no_sleep_and_the_ordinary_epilogue() {
    // Issue #298 Q30, `town-mode.md §12`: on an arrest/conflict contact the
    // command ends "at once: no sleep status, no time passed, no further
    // message; it returns the ordinary acted result, so the normal town
    // epilogue follows and re-evaluates contact".
    let dir = debug_game_dir();
    std::fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 5 5 4\n").unwrap();
    let mut state = g6_town_with(vec![g6_npc(1, 6, 5, 7, (6, 5, 0))], 5, 5);
    let clock = state.clock;
    let outcome = state.hole_up_command(&dir, Some(2)).unwrap();
    assert_eq!(outcome, MoveOutcome::Passed);
    assert!(state.party.iter().all(|member| member.status != b'S'));
    assert!(state.message.is_empty());
    // The ordinary town epilogue is one indoor-minute turn.
    let mut expected = clock;
    expected.advance_minutes(MINUTES_PER_INDOOR_TURN);
    assert_eq!(state.clock, expected);
    // Its own schedule pass re-raised the still-adjacent NPC's contact.
    assert!(state.pending_town_npc_schedule_pass || state.pending_town_npc_contact.is_some());
    let _ = std::fs::remove_dir_all(dir);
}
