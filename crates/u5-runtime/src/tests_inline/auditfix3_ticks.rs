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

fn auditfix3_ticks_held(kind: crate::text_hold::TextHoldKind) -> PlayState {
    let mut state = britannia_state(open_world_grid(), 4, 4);
    state.prng_state = 0x2468;
    state.emit_tlk_message(crate::tlk_runner::TlkRenderedText::plain(format!(
        "\"Squeek...\"\n\n{}more\n",
        kind.marker_str()
    )));
    assert!(state.text_hold_active());
    state
}

fn auditfix3_ticks_after_world_ticks(state: &PlayState, ticks: usize) -> u16 {
    let mut reference = state.clone();
    reference.pending_text_hold = None;
    for _ in 0..ticks {
        let _ = reference.advance_visual_tick();
    }
    assert!(ticks == 0 || reference.prng_state != state.prng_state, "world ticks draw");
    reference.prng_state
}

/// `conversation.md §7.3` / `§10` (R626): "PAUSE is the bounded loop of
/// Section 7.3 and runs one world tick per pass", up to 28 passes, and
/// "outside dungeon scenes the world view keeps animating and drawing random
/// numbers while the player reads". With no key the pause runs all 28.
#[test]
fn auditfix3_ticks_conversation_pause_runs_a_world_tick_per_pass() {
    use crate::text_hold::TextHoldKind;
    let mut state = auditfix3_ticks_held(TextHoldKind::TimedPause);
    let expected = auditfix3_ticks_after_world_ticks(&state, 28);
    state.flush_text_hold();
    assert!(!state.text_hold_active());
    assert_eq!(state.prng_state, expected);

    // Paced by the clock: passes 0..=3 are due 3.5 BIOS ticks in, and the
    // pass that finds a key runs its redraw before the poll - five ticks.
    let mut state = auditfix3_ticks_held(TextHoldKind::TimedPause);
    let expected = auditfix3_ticks_after_world_ticks(&state, 5);
    assert!(state.advance_text_hold_to(100.0));
    assert!(state.advance_text_hold_to(100.0 + crate::narration::bios_ticks_secs(1) * 3.5));
    assert!(state.consume_text_hold_key());
    assert!(!state.text_hold_active());
    assert_eq!(state.prng_state, expected);

    // Run out on the clock: 28 ticks and the pause ends by itself.
    let mut state = auditfix3_ticks_held(TextHoldKind::TimedPause);
    let expected = auditfix3_ticks_after_world_ticks(&state, 28);
    state.advance_text_hold_to(100.0);
    assert!(!state.advance_text_hold_to(100.0 + crate::narration::bios_ticks_secs(40)));
    assert_eq!(state.prng_state, expected);
}

/// `conversation.md §10` (R626): WAIT-KEY "uses the shared single-keystroke
/// wait ... so the world tick runs on each no-key pass exactly as at the
/// command prompt"; the pass that finds the key ends the wait with no tick
/// (`input.md §2`). `§7.6`: the `0x84` party-full refusal waits "through the
/// same single-keystroke wait ... world ticks as for WAIT-KEY".
#[test]
fn auditfix3_ticks_conversation_wait_key_ticks_each_no_key_pass() {
    use crate::text_hold::TextHoldKind;
    let mut state = auditfix3_ticks_held(TextHoldKind::WaitKey);
    let expected = auditfix3_ticks_after_world_ticks(&state, 6);
    assert!(state.advance_text_hold_to(50.0));
    // Never times out, however long it waits.
    assert!(state.advance_text_hold_to(50.0 + crate::narration::bios_ticks_secs(1) * 5.5));
    assert!(state.consume_text_hold_key());
    assert!(!state.text_hold_active());
    assert_eq!(state.prng_state, expected);

    // A key already waiting satisfies it at once: no tick.
    let mut state = auditfix3_ticks_held(TextHoldKind::WaitKey);
    let expected = state.prng_state;
    state.flush_text_hold();
    assert_eq!(state.prng_state, expected);
}

/// A sign's pause byte waits inside Look's print-and-wait loop, which
/// `input.md §2` lists among the readers that do not use the shared wait.
#[test]
fn auditfix3_ticks_a_world_freezing_hold_takes_no_draws() {
    use crate::text_hold::TextHoldKind;
    let mut state = auditfix3_ticks_held(TextHoldKind::WaitKey);
    state.pending_text_hold.as_mut().unwrap().freezes_world = true;
    let expected = state.prng_state;
    state.advance_text_hold_to(10.0);
    state.advance_text_hold_to(20.0);
    assert!(state.consume_text_hold_key());
    assert_eq!(state.prng_state, expected);
}
