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
