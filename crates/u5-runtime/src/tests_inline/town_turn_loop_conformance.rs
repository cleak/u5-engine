// Conformance tests for the town turn loop's per-turn order and the two
// walkers it drives: `town-mode.md` §7 (as corrected by `RETRACTIONS.md`
// R328), `npc-schedules.md` §5, §6, §9 (R317) and §9.1, `encounters.md` §2.1,
// `active-objects.md` §8 (R316), `shops.md` §8.4 and `timing.md` §8.2.

/// A mounted-horse marker, which is inside `npc-schedules.md §5`'s published
/// four-value transport window `0x12..0x15`.
fn mounted_horse_transport() -> TransportState {
    TransportState::Horse {
        type_byte: HORSE_TRANSPORT_FIRST,
        tile: FIRST_PLAYABLE_HORSE_TILE,
    }
}

/// The first PRNG state whose `npc-schedules.md §9.1` stage-one coin comes up
/// on the losing half, so a wander-eligible NPC spends its turn without
/// moving.
fn losing_wander_coin_seed() -> u16 {
    let mut probe = test_state(open_grid(), 5, 5);
    for seed in 0..=u16::MAX {
        probe.prng_state = seed;
        if !probe.town_npc_wander_gate_passes() {
            return seed;
        }
    }
    panic!("no PRNG state loses the wander coin");
}

/// The first PRNG state whose stage-one coin passes and whose stage-two
/// direction draw folds to `direction`.
fn wander_seed_for(direction: Direction) -> u16 {
    let mut probe = test_state(open_grid(), 5, 5);
    for seed in 0..=u16::MAX {
        probe.prng_state = seed;
        if probe.town_npc_wander_gate_passes() && probe.town_npc_wander_direction() == direction {
            return seed;
        }
    }
    panic!("no PRNG state passes the coin and draws {direction:?}");
}

fn wandering_npc(ai: u8, x: usize, y: usize, waypoint: (u8, u8)) -> RuntimeNpc {
    let mut schedule = [0u8; 16];
    for wp in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
        schedule[NPC_SCHEDULE_AI_OFFSET + wp] = ai;
        schedule[NPC_SCHEDULE_X_OFFSET + wp] = waypoint.0;
        schedule[NPC_SCHEDULE_Y_OFFSET + wp] = waypoint.1;
        schedule[NPC_SCHEDULE_Z_OFFSET + wp] = 0;
    }
    // Boundary hours the ordinary test clock (12:00) never sits on, so the
    // §6 diversion is out of the way unless a test asks for it.
    schedule[NPC_SCHEDULE_TIME_OFFSET] = 1;
    schedule[NPC_SCHEDULE_TIME_OFFSET + 1] = 2;
    schedule[NPC_SCHEDULE_TIME_OFFSET + 2] = 3;
    schedule[NPC_SCHEDULE_TIME_OFFSET + 3] = 4;
    RuntimeNpc {
        slot: 1,
        type_byte: 0x50,
        dialog_id: 0,
        schedule,
        state: NPC_STATE_IDLE,
        x,
        y,
        z: 0,
        cached_wp: 0,
        move_queue: Vec::new(),
        move_queue_pos: 0,
        stuck_counter: 0,
        active_object: None,
    }
}

#[test]
fn town_and_overworld_test_the_same_three_gates_in_opposite_orders() {
    // `npc-schedules.md §5`: "In the order the town loop tests them:
    // 1. **Transport marker.** ... 2. **Negate Time.** ... 3. **Quickness.**"
    // `encounters.md §2.1`: "1. **Negate Time.** ... 2. **Quickness.** ...
    // 3. **The transport marker.**"
    let mut town = test_state(open_grid(), 5, 5);
    town.player.transport = mounted_horse_transport();
    assert!(transport_marker_gates_per_turn_walkers(
        town.player.transport.save_marker()
    ));
    town.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    town.active_effect_counter = 10;

    // Town reaches the transport gate first, so its parity bit advances and,
    // on the set turn, it is the transport gate - not Negate Time - that
    // reports the skip.
    assert_eq!(
        town.town_walker_effect_gates(),
        WalkerEffectGate::SkippedByTransportMarker
    );
    assert!(town.transport_walker_gate_parity);
    assert_eq!(
        town.town_walker_effect_gates(),
        WalkerEffectGate::SkippedByNegateTime
    );
    assert!(!town.transport_walker_gate_parity);

    // Outdoors Negate Time returns first, and "an early return leaves the
    // later gates' parity bits un-flipped".
    let mut world = world_state(open_world_grid(), 5, 5);
    world.player.transport = mounted_horse_transport();
    world.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    world.active_effect_counter = 10;
    for _ in 0..4 {
        assert_eq!(
            world.overworld_walker_effect_gates(),
            WalkerEffectGate::SkippedByNegateTime
        );
        assert!(!world.transport_walker_gate_parity);
        assert!(!world.quickness_walker_gate_parity);
    }
}

#[test]
fn town_transport_gate_is_the_published_value_window_not_a_vehicle_test() {
    // `npc-schedules.md §5`: "Implement this as the four-value window, not as
    // an 'is the party mounted or on a carpet' test. ... A value test is
    // correct under both readings, a family test only under one."
    for marker in 0x12u8..=0x15 {
        assert!(transport_marker_gates_per_turn_walkers(marker));
    }
    assert!(!transport_marker_gates_per_turn_walkers(0x11));
    assert!(!transport_marker_gates_per_turn_walkers(0x16));
    // A furled or hoisted ship, a skiff and foot all sit outside the window.
    for marker in [0x00u8, 0x20, 0x24, 0x28] {
        assert!(!transport_marker_gates_per_turn_walkers(marker));
    }
}

#[test]
fn town_transport_gate_skips_both_walkers_on_alternate_turns() {
    // `town-mode.md §7`: "While the party's marker is one of the four values
    // `0x12..0x15`, a stored parity bit flips each turn and skips *both*
    // walkers on the turns it comes up set."
    let mut state = test_state(open_grid(), 5, 5);
    state.player.transport = mounted_horse_transport();
    let mut skipped = 0;
    for _ in 0..6 {
        match state.town_walker_effect_gates() {
            WalkerEffectGate::SkippedByTransportMarker => skipped += 1,
            WalkerEffectGate::Run => {}
            other => panic!("unexpected gate {other:?}"),
        }
    }
    assert_eq!(skipped, 3, "alternate turns");

    // On foot the gate is not reached at all and its bit does not move.
    let mut on_foot = test_state(open_grid(), 5, 5);
    for _ in 0..4 {
        assert_eq!(on_foot.town_walker_effect_gates(), WalkerEffectGate::Run);
        assert!(!on_foot.transport_walker_gate_parity);
    }
}

#[test]
fn negate_time_stops_all_town_movement() {
    // `town-mode.md §7`: "**Negate Time.** While that effect is active,
    // *both* walkers are skipped outright - nothing in town moves."
    let mut state = test_state(open_grid(), 5, 5);
    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
    state.npcs.push(wandering_npc(1, 9, 9, (9, 9)));
    state.active_objects.push(ActiveObject {
        type_byte: 0x10,
        tile: 0x10,
        x: 3,
        y: 3,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.town_walker_effect_gates(),
        WalkerEffectGate::SkippedByNegateTime
    );

    let prng_before = state.prng_state;
    state.advance_turn();
    state.apply_pending_town_status_provision_pass();
    state.apply_pending_town_object_epilogue();

    assert_eq!((state.npcs[0].x, state.npcs[0].y), (9, 9));
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (3, 3)
    );
    assert_eq!(
        state.prng_state, prng_before,
        "neither walker drew from the shared stream"
    );
}

#[test]
fn result_two_turn_still_runs_the_town_object_walker() {
    // `npc-schedules.md §5`: the result-two gate "sits after the three effect
    // gates and after the town object walker has already made its pass ...
    // That is why a result-two turn can still move a loose horse-family
    // object while no scheduled NPC moves." `town-mode.md §7` (R328) states
    // the same order.
    let mut state = test_state(open_grid(), 5, 5);
    state.npcs.push(wandering_npc(1, 9, 9, (9, 9)));
    state.active_objects.push(ActiveObject {
        type_byte: 0x10,
        tile: 0x10,
        x: 3,
        y: 3,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    state.pending_town_npc_schedule_pass = true;
    state.pending_town_active_object_pass = true;
    state.pending_town_arrest = Some(TownArrestPrompt {
        scene_byte: 0x11,
        floor: 0,
        npc_slot: 1,
    });

    let prng_before = state.prng_state;
    state.apply_pending_town_object_epilogue();

    assert_ne!(
        state.prng_state, prng_before,
        "the object walker made its per-slot draw before the processor was skipped"
    );
    assert_eq!(
        (state.npcs[0].x, state.npcs[0].y),
        (9, 9),
        "the schedule processor is the only thing a result-two turn skips"
    );
}

#[test]
fn a_town_object_the_walker_moved_repaints_on_a_result_two_turn() {
    // `town-mode.md §7` step 5: "The test has to cover **both** walkers, not
    // just the schedule processor: the object walker runs on turns whose
    // result code skips the processor, so a repaint conditioned on the
    // processor's report alone leaves a moved object stale until some later
    // turn repaints for another reason."
    for seed in 0..512u16 {
        let mut state = test_state(open_grid(), 5, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 3,
            y: 3,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        state.prng_state = seed;
        state.visibility_dirty = false;
        state.pending_town_npc_schedule_pass = true;
        state.pending_town_active_object_pass = true;
        state.pending_town_arrest = Some(TownArrestPrompt {
            scene_byte: 0x11,
            floor: 0,
            npc_slot: 1,
        });

        state.apply_pending_town_object_epilogue();

        if (state.active_objects[1].x, state.active_objects[1].y) != (3, 3) {
            assert!(
                state.visibility_dirty,
                "seed {seed} moved the object without asking for a repaint"
            );
            return;
        }
    }
    panic!("no seed in the probe range let the town object walker commit a step");
}

#[test]
fn town_wander_gate_is_a_fair_coin_over_the_whole_generator_state_space() {
    // `npc-schedules.md §9.1` stage 1: "A fair coin - **one in two**, not one
    // in eight. ... Enumerating the generator's whole state space gives an
    // empirical rate of `0.5014` for this bit on the dominant state cycle."
    let mut probe = test_state(open_grid(), 5, 5);
    let mut passes = 0usize;
    for seed in 0..=u16::MAX {
        probe.prng_state = seed;
        if probe.town_npc_wander_gate_passes() {
            passes += 1;
        }
    }
    let rate = passes as f64 / 65536.0;
    assert!(
        (rate - 0.5).abs() < 0.01,
        "gate rate {rate} is not a fair coin"
    );
    // One in eight is the reading §9.1 rules out.
    assert!((rate - 0.125).abs() > 0.3);
}

#[test]
fn town_wander_direction_draw_is_the_slightly_biased_sixty_five_value_fold() {
    // `npc-schedules.md §9.1` stage 2: "a uniform value over the sixty-five
    // integers `0..64`, folded to its low two bits and mapped to east, north,
    // west, south in that order. ... east is drawn about `26.2%` of the time
    // and each of the other three about `24.6%`."
    let mut probe = test_state(open_grid(), 5, 5);
    let mut east = 0usize;
    let mut north = 0usize;
    let mut west = 0usize;
    let mut south = 0usize;
    for seed in 0..=u16::MAX {
        probe.prng_state = seed;
        match probe.town_npc_wander_direction() {
            Direction::East => east += 1,
            Direction::North => north += 1,
            Direction::West => west += 1,
            Direction::South => south += 1,
            other => panic!("wander drew a non-cardinal direction {other:?}"),
        }
    }
    let total = 65536.0;
    assert!(
        (east as f64 / total - 0.262).abs() < 0.01,
        "east share {}",
        east as f64 / total
    );
    for (name, count) in [("north", north), ("west", west), ("south", south)] {
        assert!(
            (count as f64 / total - 0.246).abs() < 0.01,
            "{name} share {}",
            count as f64 / total
        );
    }
    assert!(east > north && east > west && east > south);
}

#[test]
fn town_wander_spends_the_turn_on_a_lost_coin() {
    // `npc-schedules.md §9.1`: "On the losing half the NPC does nothing at
    // all this turn; there is no second look and no accumulated credit."
    let mut state = test_state(npc_open_grid(), 20, 20);
    state.npcs.push(wandering_npc(1, 9, 9, (9, 9)));
    state.prng_state = losing_wander_coin_seed();

    assert_eq!(state.town_npc_wander_step(0, 0, 9, 9, 3), None);
}

#[test]
fn town_wander_makes_one_attempt_and_never_tries_a_second_direction() {
    // `npc-schedules.md §9.1`: "**A rejection at stage 3 or 4 is a spent
    // turn.** The NPC does not try a second direction, does not re-roll, and
    // does not fall back to a different rule. One coin, one direction, one
    // candidate, one turn."
    let mut grid = npc_open_grid();
    // Wall off only the cell the drawn direction would reach; the other three
    // neighbours stay open, so a sweeping implementation would still move.
    grid[9 * 32 + 10] = 0xB9;
    let mut state = test_state(grid, 20, 20);
    state.npcs.push(wandering_npc(1, 9, 9, (9, 9)));
    state.prng_state = wander_seed_for(Direction::East);

    assert_eq!(state.town_npc_wander_step(0, 0, 9, 9, 3), None);
}

#[test]
fn bounded_wander_caps_manhattan_distance_at_three_from_the_waypoint() {
    // `npc-schedules.md §9.1` stage 3: "The candidate cell's **Manhattan
    // distance to the active waypoint's stored `(x, y)`** is compared against
    // the cap, and a distance **strictly greater** than the cap rejects the
    // step. Note that the rule caps where the NPC may *stand*, not how far it
    // may travel."
    //
    // The waypoint is `(9, 9)`. From `(12, 9)` an eastward step lands at
    // Manhattan four and is refused; from `(11, 9)` it lands at three and
    // commits. The old per-axis radius-two test rejected both.
    let mut refused = test_state(npc_open_grid(), 20, 20);
    refused.npcs.push(wandering_npc(1, 12, 9, (9, 9)));
    refused.prng_state = wander_seed_for(Direction::East);
    assert_eq!(refused.town_npc_wander_step(0, 0, 9, 9, 3), None);

    let mut accepted = test_state(npc_open_grid(), 20, 20);
    accepted.npcs.push(wandering_npc(1, 11, 9, (9, 9)));
    accepted.prng_state = wander_seed_for(Direction::East);
    assert_eq!(accepted.town_npc_wander_step(0, 0, 9, 9, 3), Some((12, 9)));
}

#[test]
fn unbounded_wander_switches_the_radius_test_off_entirely() {
    // `npc-schedules.md §9` value `2`: "the same one-attempt-per-turn coin
    // and the same single direction draw, with the waypoint-radius test
    // switched off entirely (cap zero)."
    let mut state = test_state(npc_open_grid(), 20, 20);
    state.npcs.push(wandering_npc(2, 20, 9, (9, 9)));
    state.prng_state = wander_seed_for(Direction::East);
    assert_eq!(state.town_npc_wander_step(0, 0, 9, 9, 0), Some((21, 9)));
}

#[test]
fn a_boundary_hour_diverts_a_settled_wanderer_away_from_the_ai_dispatch() {
    // `npc-schedules.md §6`: "For the whole of an hour that *does* equal one
    // of them, a settled NPC is routed into the route/state machine of
    // Section 7 instead, so a wander-AI NPC normally stands still for that
    // entire hour, whether or not its waypoint actually changed."
    let mut state = test_state(npc_open_grid(), 20, 20);
    let mut npc = wandering_npc(1, 9, 9, (9, 9));
    npc.schedule[NPC_SCHEDULE_TIME_OFFSET] = state.clock.hour;
    state.npcs.push(npc);
    state.prng_state = wander_seed_for(Direction::East);

    state.advance_npc_schedules();

    assert_eq!((state.npcs[0].x, state.npcs[0].y), (9, 9));
    assert_eq!(
        state.prng_state,
        wander_seed_for(Direction::East),
        "the diverted NPC never reached the wander gate, so it drew nothing"
    );
}

#[test]
fn a_boundary_hour_does_not_divert_a_wanderer_on_a_non_boundary_hour() {
    // The complement of the rule above: "The **direct** route to the AI
    // dispatch of Section 9 is taken only when the trigger reports 'no
    // action' - that is, only while the current hour matches none of the
    // NPC's own four `time` bytes."
    let mut state = test_state(npc_open_grid(), 20, 20);
    state.npcs.push(wandering_npc(1, 9, 9, (9, 9)));
    state.prng_state = wander_seed_for(Direction::East);

    state.advance_npc_schedules();

    assert_eq!((state.npcs[0].x, state.npcs[0].y), (10, 9));
}

#[test]
fn spent_heavy_work_budget_re_enters_the_ai_dispatch_on_a_boundary_hour() {
    // `npc-schedules.md §6`: "**The tick's heavy-work budget was already
    // spent.** ... If a lower-numbered NPC already consumed it, a diverted
    // NPC takes the cheap arm instead, which re-checks the floor and then
    // enters the AI dispatch after all ... Such an NPC wanders normally on
    // that turn." "An engine that models the boundary hour as a hard freeze
    // will be right most of the time and will diverge in a crowded location."
    let mut grid = npc_open_grid();
    // Block the router's direct cardinal probe so it has to run the flood
    // fill - "The cardinal probe is not a search; only the flood fill is."
    grid[1 * 32 + 2] = 0xB9;
    grid[2 * 32 + 1] = 0xB9;
    let mut state = test_state(grid, 20, 20);
    // Slot 1 is route-walking to a far waypoint and consumes the pass's one
    // flood fill; slot 2 sits on a boundary hour and would otherwise freeze.
    let mut router = wandering_npc(0, 1, 1, (30, 30));
    router.slot = 1;
    router.state = NPC_STATE_INPLANE_MOVE;
    state.npcs.push(router);
    let mut diverted = wandering_npc(1, 9, 9, (9, 9));
    diverted.slot = 2;
    diverted.schedule[NPC_SCHEDULE_TIME_OFFSET] = state.clock.hour;
    state.npcs.push(diverted);
    state.prng_state = wander_seed_for(Direction::East);

    state.advance_npc_schedules();

    assert_eq!((state.npcs[1].x, state.npcs[1].y), (10, 9));
}

#[test]
fn inn_rest_runs_the_clock_to_six_whatever_hour_it_began_at() {
    // `shops.md §8.4`: "The clock is then run forward in paced steps until
    // the hour byte reads **six** - the rest always ends at 06:00, whatever
    // hour it began at, so a party that rents a room at 21:00 sleeps nine
    // hours and one that rents at 04:00 sleeps two."
    for (start, expected) in [(21u8, 9u8), (4, 2), (6, 0)] {
        let mut state = test_state(open_grid(), 5, 5);
        state.clock.hour = start;
        state.clock.minute = 0;
        let hours = state.advance_inn_rest_clock_to_morning();
        assert_eq!(hours, expected, "starting at {start}");
        assert_eq!(state.clock.hour, INN_REST_WAKE_HOUR);
    }
}

#[test]
fn the_clear_and_re_place_pass_puts_a_mid_route_npc_back_on_its_waypoint() {
    // `npc-schedules.md §12`: "every non-party active-object record is
    // cleared, and every scheduled NPC is re-placed at the position its
    // schedule gives for the *current* hour ... an NPC that was mid-route is
    // standing on its scheduled position rather than where it had walked to,
    // and its queued route is gone."
    let mut state = test_state(npc_open_grid(), 20, 20);
    let mut npc = wandering_npc(1, 3, 3, (9, 9));
    npc.state = NPC_STATE_INPLANE_MOVE;
    npc.move_queue = vec![NPC_PATH_DIR_EAST, NPC_PATH_DIR_EAST];
    state.npcs.push(npc);

    state.clear_and_replace_scheduled_npcs();

    assert_eq!((state.npcs[0].x, state.npcs[0].y), (9, 9));
    assert_eq!(state.npcs[0].state, NPC_STATE_IDLE);
    assert!(state.npcs[0].move_queue.is_empty());
    assert!(state.npcs[0].active_object.is_some());
}

#[test]
fn under_sail_idle_wait_is_two_ticks_one_world_step_and_no_command_wait() {
    // `timing.md §8.2`: "On the overworld the input helper performs one
    // scripted step-and-wait - one world step followed by one one-tick wait -
    // before either entering the command wait or, when sails are set,
    // performing a bare cursor poll instead; so an **under-sail auto-advance
    // pass costs two ticks and one world step and never enters the command
    // wait at all**."
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: true,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 1,
    };
    assert!(state.under_sail_wait_pass_applies());

    let first = state.idle_wait_pass();
    assert_eq!(first, IdleWaitPass::UnderSailWorldStep);
    assert!(first.performed_world_step());
    assert!(!first.enters_command_wait());

    let frame_before = state.animation.frame;
    let second = state.idle_wait_pass();
    assert_eq!(second, IdleWaitPass::UnderSailCursorPoll);
    assert!(!second.performed_world_step());
    assert!(!second.enters_command_wait());
    assert_eq!(
        state.animation.frame, frame_before,
        "the bare cursor poll costs its tick and steps nothing"
    );

    // The route repeats: two passes, one world step.
    assert_eq!(state.idle_wait_pass(), IdleWaitPass::UnderSailWorldStep);
}

#[test]
fn the_under_sail_route_does_not_leak_into_combat_or_a_furled_ship() {
    // The route belongs to the overworld command-wait helper. Combat sets its
    // own scene value and runs the ordinary wait, and a furled ship has no
    // sails set, so neither takes the auto-advance arm - and the `.` pass
    // command, which consumes a turn through the clock rather than the idle
    // wait, never reaches this code at all.
    let mut combat = world_state(open_world_grid(), 5, 5);
    combat.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: true,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 1,
    };
    combat.combat_active = true;
    assert!(!combat.under_sail_wait_pass_applies());
    assert_eq!(combat.idle_wait_pass(), IdleWaitPass::CommandWait);

    let mut furled = world_state(open_world_grid(), 5, 5);
    furled.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 1,
    };
    assert!(!furled.under_sail_wait_pass_applies());
    assert_eq!(furled.idle_wait_pass(), IdleWaitPass::CommandWait);

    let mut town = test_state(open_grid(), 5, 5);
    assert!(!town.under_sail_wait_pass_applies());
    assert_eq!(town.idle_wait_pass(), IdleWaitPass::CommandWait);
}

#[test]
fn the_town_animator_is_not_one_of_the_two_gated_walkers() {
    // `npc-schedules.md §5`: the three effect gates "sit in the town loop's
    // per-turn epilogue, ahead of both town walkers - the object walker that
    // moves loose horse-family objects and this schedule processor". The
    // animator is neither of those two: `npc-schedules.md §12` has "the
    // active-object animator, run independently each render frame", and
    // `RETRACTIONS.md` R316 gives its whole contract as "the displayed-tile
    // byte and the packed phase/facing byte; it never writes a slot's column
    // or row". So a turn the transport-marker gate skips still animates the
    // town's sprites - otherwise a mounted or carpet-borne party, or one
    // under Quickness, would see town sprites freeze on half of all turns.
    let mut state = test_state(open_grid(), 5, 5);
    state.player.transport = mounted_horse_transport();
    state.npcs.push(wandering_npc(1, 9, 9, (9, 9)));
    // Type `192` is outside the loose-horse family `0x10..0x11`, so it is the
    // animator's slot and not the town object walker's.
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 3,
        y: 3,
        z: 0,
        phase: 0x22,
        aux1: 0,
        aux3: 0,
    });

    state.advance_turn();
    state.apply_pending_town_status_provision_pass();
    state.apply_pending_town_object_epilogue();

    assert_eq!(
        (state.npcs[0].x, state.npcs[0].y),
        (9, 9),
        "the transport gate skipped the schedule processor this turn"
    );
    assert_eq!(
        state.active_objects[1].phase, 0x21,
        "the animator still ticked the countdown on a gated turn"
    );
    assert_eq!(
        state.active_objects[1].tile, 193,
        "the animator still advanced the displayed frame on a gated turn"
    );
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (3, 3),
        "the animator moves nothing (R316)"
    );
}

#[test]
fn negate_time_stops_the_town_animator_too() {
    // `animation.md §13.1`: "**Negate Time freezes all of it.** ... For the
    // effect's full duration nothing advances: no water rotation, no fire
    // flicker, ... no object animation". That freeze is a separate rule from
    // the walker gates and is the one thing that does stop the animator.
    let mut state = test_state(open_grid(), 5, 5);
    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 3,
        y: 3,
        z: 0,
        phase: 0x22,
        aux1: 0,
        aux3: 0,
    });

    state.advance_turn();
    state.apply_pending_town_status_provision_pass();
    state.apply_pending_town_object_epilogue();

    assert_eq!(state.active_objects[1].phase, 0x22);
    assert_eq!(state.active_objects[1].tile, 192);
}
