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
    // the workspace builder ignores occupants four or more from the
    // destination (§8.3 phase 4).
    let mut state = g6_town_with(
        vec![
            g6_npc(1, 1, 1, 0, (20, 20, 0)),
            g6_npc(2, 2, 1, 0, (2, 1, 0)),
        ],
        30,
        30,
    );
    // The occupant at (2, 1) is far from the destination (20, 20).
    assert!(!state.npc_can_step_toward(0, 2, 1, 0, 20, 20));
    assert!(state.npc_workspace_cell_open(0, 2, 1, 0, 20, 20));
    // Within the cutoff both refuse.
    assert!(!state.npc_workspace_cell_open(0, 2, 1, 0, 3, 1));
    state.npcs.truncate(1);
    state.relink_npc_objects();
    assert!(state.npc_can_step_toward(0, 2, 1, 0, 20, 20));
}
