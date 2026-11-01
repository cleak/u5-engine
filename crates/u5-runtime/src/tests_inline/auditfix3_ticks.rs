// Closed re-audit tick gaps (u5-spec#302 re-audit, a3-ticks): presentation
// pauses that the spec runs as full world ticks, each with its animator and
// `0..63` wind-check draw.

fn auditfix3_ticks_troll_state(dexterity: u8) -> PlayState {
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 1)] = TROLL_BRIDGE_TILE_FIRST;
    let mut state = britannia_state(grid, 2, 1);
    state.party.truncate(1);
    state.party[0].climb_stat = dexterity;
    let seed = (0u32..65536)
        .find(|&seed| {
            let mut probe = state.clone();
            probe.prng_state = seed as u16;
            probe.troll_bridge_trigger_roll() == 0
        })
        .expect("some seed hits the one-in-eight trigger");
    state.prng_state = seed as u16;
    state
}

/// `overworld.md §7.1` step 2: "pause ten world-tick units", then per walked
/// member "three dots each preceded by a five-unit pause" before the `1..30`
/// roll. The pauses "run through the world-tick pause helper", so each unit is
/// a full world tick with its `0..63` wind check.
#[test]
fn auditfix3_ticks_troll_bridge_pauses_are_full_world_ticks() {
    // Dexterity 30 always saves, so the lone member gets across.
    let mut state = auditfix3_ticks_troll_state(30);
    let mut reference = state.clone();

    state.apply_troll_bridge_ambush(WorldPlane::Britannia);
    assert_eq!(state.message, TROLL_BRIDGE_EVADED_LINE);

    assert_eq!(reference.troll_bridge_trigger_roll(), 0);
    for _ in 0..TROLL_BRIDGE_ANNOUNCE_PAUSE_TICKS + 3 * TROLL_BRIDGE_DOT_PAUSE_TICKS {
        let _ = reference.advance_visual_tick();
    }
    let _ = reference.troll_bridge_sneak_roll();
    assert_eq!(state.prng_state, reference.prng_state);
    assert_eq!(TROLL_BRIDGE_ANNOUNCE_PAUSE_TICKS, 10);
    assert_eq!(TROLL_BRIDGE_DOT_PAUSE_TICKS, 5);
}

/// `overworld.md §9.2` steps 1-2: the transit opens with "One world-tick
/// pause" and stage A is "paced by a world tick every eight steps" (32 over
/// 256). `§5` item 4 counts the `0xDC` arrival presentation's "own paced
/// world ticks" among the gameplay draws, so each is a full world step.
#[test]
fn auditfix3_ticks_moongate_transit_runs_full_world_ticks() {
    let mut state = britannia_state(open_world_grid(), 4, 4);
    state.natural_moongate_counter = MOONGATE_PHASE_FULL;
    state.prng_state = 0x1234;
    let mut reference = state.clone();

    let playback = state.play_natural_moongate_transit().unwrap();
    assert!(playback.ran_to_completion);
    assert_eq!(playback.stage_a_world_ticks, 32);

    for _ in 0..1 + 32 {
        let _ = reference.advance_visual_tick();
    }
    assert_ne!(reference.prng_state, 0x1234, "a world tick draws");
    assert_eq!(state.prng_state, reference.prng_state);
    // Stage B still ends the shared presence counter at zero.
    assert_eq!(state.natural_moongate_counter, 0);
}
