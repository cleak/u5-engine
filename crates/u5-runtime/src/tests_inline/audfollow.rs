// Closed-audit follow-ups (audfollow): the per-iteration half of the
// `overworld.md §5` step 2 underfoot probe, and the spawn record write of
// `encounters.md §4` / `active-objects.md §3`.

fn audfollow_world_on(tile: u8) -> PlayState {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = tile;
    let mut state = britannia_state(grid, 5, 5);
    state.clock = GameClock::new(12, 0).unwrap();
    state.active_effect_tag = None;
    state.prng_state = 0x1234;
    state
}

/// `overworld.md §5` step 2 (R641): "On every frame on which the probe does
/// not take the `0xFF` arm, it runs a zero-minute cleanup call so daylight is
/// recomputed without advancing time." The probe sits at the loop head, ahead
/// of the party-capability scan, and runs once per iteration - not only when
/// a latch clears and not only on a move.
#[test]
fn audfollow_world_probe_runs_zero_minute_cleanup_on_every_ordinary_frame() {
    let dir = debug_game_dir();
    let mut state = audfollow_world_on(0x05);
    let clock = state.clock;
    let turn = state.turn;
    // Never latched: the old code ran the cleanup only on a latch clear.
    assert!(!state.world_underfoot_blackout_latched);
    state.ambient_light = 0;

    state.apply_exploration_turn_gate(&dir).unwrap();

    assert_eq!(state.ambient_light, FULL_DAYLIGHT, "daylight recomputed");
    assert_eq!(state.clock, clock, "a zero-minute call advances no time");
    assert_eq!(state.turn, turn);
    assert!(!state.world_underfoot_probe_owed);

    // A frontend that polls the gate again inside the same iteration does
    // not probe again.
    state.ambient_light = 0;
    state.apply_exploration_turn_gate(&dir).unwrap();
    assert_eq!(state.ambient_light, 0, "one probe per iteration");

    // A returned command ends the iteration; the next one probes again.
    state.world_underfoot_probe_owed = true;
    state.apply_exploration_turn_gate(&dir).unwrap();
    assert_eq!(state.ambient_light, FULL_DAYLIGHT);
    let _ = fs::remove_dir_all(dir);
}

/// A returned command - free or acted - is the end of a loop iteration
/// (`overworld.md §5` step 7: "Back to step 2"), so it owes the next probe.
#[test]
fn audfollow_world_returned_command_owes_the_next_iterations_probe() {
    let dir = debug_game_dir();
    let mut state = audfollow_world_on(0x05);
    state.world_underfoot_probe_owed = false;

    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();

    assert!(state.world_underfoot_probe_owed);
    let _ = fs::remove_dir_all(dir);
}

/// `overworld.md §5` step 2: on the `0xFF` tile the first frame runs one world
/// tick and later latched frames run "neither a world tick nor a clock call"
/// - so no zero-minute cleanup either, and the forced zero light holds.
#[test]
fn audfollow_world_probe_latched_frames_run_no_tick_and_no_cleanup() {
    let dir = debug_game_dir();
    let mut state = audfollow_world_on(OVERWORLD_UNDERFOOT_BLACKOUT_TILE);
    state.ambient_light = FULL_DAYLIGHT;
    let mut reference = audfollow_world_on(OVERWORLD_UNDERFOOT_BLACKOUT_TILE);
    let _ = reference.advance_visual_tick();

    state.apply_exploration_turn_gate(&dir).unwrap();
    assert!(state.world_underfoot_blackout_latched);
    assert_eq!(state.prng_state, reference.prng_state, "first-frame tick");
    assert_eq!(state.ambient_light, 0);

    let latched = state.prng_state;
    let clock = state.clock;
    state.world_underfoot_probe_owed = true;
    state.apply_exploration_turn_gate(&dir).unwrap();
    assert_eq!(state.prng_state, latched, "a latched frame takes no tick");
    assert_eq!(state.ambient_light, 0, "no cleanup recomputes daylight");
    assert_eq!(state.clock, clock);

    // Leaving the tile: that frame takes the cleanup arm and clears the latch.
    state.grid[world_cell_index(5, 5)] = 0x05;
    state.world_underfoot_probe_owed = true;
    state.apply_exploration_turn_gate(&dir).unwrap();
    assert!(!state.world_underfoot_blackout_latched);
    assert_eq!(state.ambient_light, FULL_DAYLIGHT);
    let _ = fs::remove_dir_all(dir);
}

/// `overworld.md §5`: "The pre-loop underfoot latch is also passed into the
/// outdoor movement dispatcher." A move in an iteration whose probe already
/// ran reads that probe's latch rather than probing again.
#[test]
fn audfollow_world_move_reads_the_loop_head_latch() {
    let mut state = audfollow_world_on(0x05);
    // This iteration's probe has run and left the latch set.
    state.world_underfoot_probe_owed = false;
    state.world_underfoot_blackout_latched = true;

    assert_eq!(
        state
            .step_world(Direction::East, 6, 5, WorldPlane::Britannia, None)
            .unwrap(),
        MoveOutcome::Used
    );
    assert_eq!((state.player.x, state.player.y), (5, 5), "commit held");
}

/// A move reached without passing the loop head (a direct driver) runs the
/// iteration's owed probe itself: on `0xFF` that is the first-frame world
/// tick, and the latch then holds the commit.
#[test]
fn audfollow_world_move_without_a_loop_head_runs_the_owed_probe() {
    let mut state = audfollow_world_on(OVERWORLD_UNDERFOOT_BLACKOUT_TILE);
    assert!(state.world_underfoot_probe_owed);

    assert_eq!(
        state
            .step_world(Direction::East, 6, 5, WorldPlane::Britannia, None)
            .unwrap(),
        MoveOutcome::Used
    );
    assert!(!state.world_underfoot_probe_owed);
    assert!(state.world_underfoot_blackout_latched);
    assert_eq!((state.player.x, state.player.y), (5, 5));
}

