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
