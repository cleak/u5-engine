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

/// A hoisted frigate on open deep water, which is the only terrain the
/// released sail step accepts.
fn under_sail_world(x: usize, y: usize, wind: WindState) -> PlayState {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], x, y);
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: true,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 1,
    };
    state.wind = wind;
    state
}

#[test]
fn a_steered_ship_keeps_sailing_with_no_keypress_at_the_published_cadence() {
    // `timing.md §8.2`: per under-sail wait pass the helper pays "**One world
    // step, unconditionally**", "**One fully consumed game turn**" and
    // "**Zero, one or two one-tick waits - not two flat**" - `RETRACTIONS.md`
    // R371 withdrew the older "costs two ticks" figure in favour of that
    // maximum. The world-step figure and "never enters the command wait at
    // all" both stand, and they are what this test pins.
    //
    // `overworld.md §5` step 3 says what the pass buys: "Under sail on the
    // wind-driven cadence, this step does not enter the command wait: the
    // input helper runs its own auto-advance loop and returns the cached sail
    // direction, which is how a ship keeps moving with no keypress." (R372
    // withdrew the older "does not read the keyboard at all" half; the loop
    // polls one key per pass - see the swallow test below.)
    //
    // A north wind is perpendicular to an easterly heading, which
    // `weather.md §5`'s table releases immediately, so every auto-advance
    // pass here commits a cell.
    let mut state = under_sail_world(5, 5, WindState::North);

    // Before the player steers there is no cached sail direction to
    // synthesize from, so the loop reads a command and the world runs at the
    // ordinary one-tick rate.
    assert!(state.sail_cached_direction.is_none());
    assert!(!state.under_sail_wait_pass_applies());
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::CommandWait
    );
    assert_eq!(state.animation.frame, 1);
    assert_eq!((state.player.x, state.player.y), (5, 5));

    // One keyed movement command establishes the cache (`weather.md §5`: a
    // movement command "first establishes or changes the ship's heading"),
    // and that command turns without moving: "If the requested heading
    // differs from the current cached sail direction, the ship turns and
    // clears the sailing counter; that action does not also move the ship."
    assert_eq!(state.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!(state.sail_cached_direction, Some(Direction::East));
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.under_sail_wait_pass_applies());

    // Control: what one *keyed* sail command costs the world clock on its
    // own. The auto-advance pass below must cost exactly this plus the one
    // scripted world step - the bare cursor poll adds no world step of its
    // own, because it is one of the pumps that "share the one-tick wait but
    // not the world step".
    // `animation.md §6` wraps the shared phase counter at
    // [`STATIC_TILE_ANIMATION_PERIOD_TICKS`], so every comparison below is
    // modular.
    let period = STATIC_TILE_ANIMATION_PERIOD_TICKS;
    let keyed_command_frames = {
        let mut control = under_sail_world(5, 5, WindState::North);
        // Spend the heading-establishing turn first so the measured command
        // is a released one, the same kind the auto-advance pass synthesizes.
        assert_eq!(control.step(Direction::East), MoveOutcome::SailTurned);
        let before = control.animation.frame;
        assert_eq!(control.step(Direction::East), MoveOutcome::Moved);
        (control.animation.frame + period - before) % period
    };

    for expected_x in [6usize, 7, 8] {
        let frame_before = state.animation.frame;
        let turn_before = state.turn;
        let x_before = state.player.x;

        // Half one: the scripted step-and-wait. One world step, no movement.
        let first = state.idle_wait_pass(None).unwrap();
        assert_eq!(first, IdleWaitPass::UnderSailWorldStep);
        assert!(first.performed_world_step());
        assert!(!first.enters_command_wait());
        assert_eq!(state.animation.frame, (frame_before + 1) % period);
        assert_eq!(state.player.x, x_before);
        assert_eq!(state.turn, turn_before);

        // Half two: the bare cursor poll. No world step - it is one of the
        // pumps that "share the one-tick wait but not the world step" - and
        // in place of the command wait the helper returns the cached
        // direction, which the loop dispatches. The ship advances one cell
        // and consumes its turn with no keypress.
        let second = state.idle_wait_pass(None).unwrap();
        assert_eq!(second, IdleWaitPass::UnderSailCursorPoll);
        assert!(!second.performed_world_step());
        assert!(!second.enters_command_wait());
        assert_eq!(
            state.animation.frame,
            (frame_before + 1 + keyed_command_frames) % period,
            "the pass costs one scripted world step plus the synthesized              command's own turn, and nothing for the poll itself"
        );
        assert_eq!((state.player.x, state.player.y), (expected_x, 5));
        assert!(state.turn > turn_before);
    }
}

#[test]
fn an_auto_advance_pass_into_the_wind_pays_the_published_wait_ticks() {
    // `weather.md §5`: an easterly heading in an east wind is sailing into
    // the wind, which the table gives as "move after two wait ticks", and "a
    // wait tick is a real overworld wait pass inside the input helper: the
    // helper ... increments the sailing counter, and then tests the cached
    // sail direction again. After movement is released, the counter is
    // cleared".
    //
    // So the cursor-poll half does published work on a stalled turn too: it
    // is the pass that increments the counter.
    let mut state = under_sail_world(5, 5, WindState::East);

    // The keyed command that establishes the heading turns without moving and
    // leaves the counter at zero - `weather.md §5.1`: the store "is the same
    // step that zeroes the sailing counter".
    assert_eq!(state.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!(state.sail_cadence, 0);
    assert_eq!((state.player.x, state.player.y), (5, 5));

    // Wait tick one.
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailWorldStep
    );
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailCursorPoll
    );
    assert_eq!(state.sail_cadence, 1);
    assert_eq!((state.player.x, state.player.y), (5, 5));

    // Wait tick two.
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailWorldStep
    );
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailCursorPoll
    );
    assert_eq!(state.sail_cadence, 2);
    assert_eq!((state.player.x, state.player.y), (5, 5));

    // The next wait pass releases the movement, and the counter clears.
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailWorldStep
    );
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailCursorPoll
    );
    assert_eq!((state.player.x, state.player.y), (6, 5));
    assert_eq!(state.sail_cadence, 0);
}

#[test]
fn a_sailing_collision_clears_the_cache_and_ends_the_auto_advance_route() {
    // `overworld.md §6.2.5`: "The step remains refused, party coordinates do
    // not change, and the cached sail direction is cleared." With the cache
    // gone the helper has nothing to synthesize, so the route stops and the
    // loop goes back to reading commands at the ordinary one-tick rate
    // instead of driving the ship into the obstruction for ever.
    let mut grid = vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS];
    grid[world_cell_index(7, 5)] = 0x05;
    let mut state = world_state(grid, 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: true,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 1,
    };
    state.wind = WindState::North;

    // The keyed command turns without moving (`weather.md §5`); the crosswind
    // release lands on the auto-advance pass that follows it.
    assert_eq!(state.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.under_sail_wait_pass_applies());

    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailWorldStep
    );
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailCursorPoll
    );
    assert_eq!((state.player.x, state.player.y), (6, 5));

    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailWorldStep
    );
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::UnderSailCursorPoll
    );

    assert_eq!(state.message, "COLLISION!");
    assert_eq!((state.player.x, state.player.y), (6, 5));
    assert!(state.sail_cached_direction.is_none());
    assert!(!state.under_sail_wait_pass_applies());
    assert_eq!(
        state.idle_wait_pass(None).unwrap(),
        IdleWaitPass::CommandWait
    );
}

#[test]
fn the_under_sail_route_does_not_leak_into_combat_or_a_furled_ship() {
    // The route belongs to the overworld command-wait helper. Combat sets its
    // own scene value and runs the ordinary wait, and a furled ship has no
    // sails set, so neither takes the auto-advance arm - and the `.` pass
    // command, which consumes a turn through the clock rather than the idle
    // wait, never reaches this code at all.
    let mut combat = under_sail_world(5, 5, WindState::North);
    assert_eq!(combat.step(Direction::East), MoveOutcome::SailTurned);
    assert!(combat.under_sail_wait_pass_applies());
    combat.combat_active = true;
    assert!(!combat.under_sail_wait_pass_applies());
    assert_eq!(
        combat.idle_wait_pass(None).unwrap(),
        IdleWaitPass::CommandWait
    );

    // Furling is one of the published cache clears (`vehicles.md` "Ship
    // Sails": "wind-driven drift should not advance the ship while furled").
    let mut furled = under_sail_world(5, 5, WindState::North);
    assert_eq!(furled.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!(furled.toggle_sails(), MoveOutcome::SailToggled);
    assert!(!furled.player.transport.is_ship_under_sail());
    assert!(furled.sail_cached_direction.is_none());
    assert!(!furled.under_sail_wait_pass_applies());
    assert_eq!(
        furled.idle_wait_pass(None).unwrap(),
        IdleWaitPass::CommandWait
    );

    let mut town = test_state(open_grid(), 5, 5);
    assert!(!town.under_sail_wait_pass_applies());
    assert_eq!(town.idle_wait_pass(None).unwrap(), IdleWaitPass::CommandWait);
}


#[test]
fn the_town_animator_is_not_one_of_the_two_gated_walkers() {
    // `npc-schedules.md §5`: the three effect gates "sit in the town loop's
    // per-turn epilogue, ahead of both town walkers - the object walker that
    // moves loose horse-family objects and this schedule processor". The
    // animator is neither of those two: `npc-schedules.md §12` has the
    // animator running "independently of the scheduler, once per world tick
    // rather than once per player turn" and selecting frames while the
    // drawing "belongs to the shared visibility post-pass" (`RETRACTIONS.md`
    // R373 withdrew "run independently each render frame" / "draws the
    // per-NPC sprites"), and R316 gives its whole write set as "the
    // displayed-tile byte and the packed phase/facing byte; it never writes a
    // slot's column or row". So a turn the transport-marker gate skips still
    // animates the town's sprites - otherwise a mounted or carpet-borne
    // party, or one under Quickness, would see town sprites freeze on half of
    // all turns.
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

#[test]
fn a_refused_queued_route_step_falls_into_the_cap_zero_wander() {
    // `npc-schedules.md §9.1`, "**The same stepper is reached with the cap
    // switched off.**": "Two route-recovery paths in the state machine call
    // the identical wander step with cap zero: **the queued-route replay when
    // its next queued step is refused**, and the state-2/3 arm whose queue is
    // exhausted and whose replan either failed or was not attempted ... Both
    // are still behind the same one-in-two coin, so an NPC that is recovering
    // from a blocked route also moves on about half its turns, but without
    // the waypoint-radius limit."
    //
    // The NPC stands at (3, 3) with a queued step west into a wall, and its
    // waypoint is nine cells away, so a bounded cap of three would refuse
    // every neighbour. Cap zero skips the radius test entirely, which is what
    // this pins.
    let mut grid = npc_open_grid();
    grid[2 + 3 * 32] = 0xB9;
    let mut state = test_state(grid, 20, 20);
    let mut npc = wandering_npc(0, 3, 3, (12, 12));
    npc.state = NPC_STATE_REPLAY_QUEUE;
    npc.move_queue = vec![NPC_PATH_DIR_WEST, NPC_PATH_DIR_WEST];
    state.npcs.push(npc);
    state.prng_state = wander_seed_for(Direction::East);

    state.advance_npc_schedules();

    // The queued west step was refused; the recovery wander drew east and
    // committed it, even though (4, 3) is Manhattan distance seventeen from
    // the waypoint - far outside the cap of three that the bounded entry
    // would have applied.
    assert_eq!((state.npcs[0].x, state.npcs[0].y), (4, 3));
    assert_eq!(
        state.npcs[0].x.abs_diff(12) + state.npcs[0].y.abs_diff(12),
        17
    );

    // The same refusal on a losing coin is a spent turn: no draw of a
    // direction, no step.
    let mut grid = npc_open_grid();
    grid[2 + 3 * 32] = 0xB9;
    let mut losing = test_state(grid, 20, 20);
    let mut npc = wandering_npc(0, 3, 3, (12, 12));
    npc.state = NPC_STATE_REPLAY_QUEUE;
    npc.move_queue = vec![NPC_PATH_DIR_WEST, NPC_PATH_DIR_WEST];
    losing.npcs.push(npc);
    losing.prng_state = losing_wander_coin_seed();

    losing.advance_npc_schedules();

    assert_eq!((losing.npcs[0].x, losing.npcs[0].y), (3, 3));
}

#[test]
fn a_stuck_npc_only_replans_on_the_one_in_three_draw() {
    // `npc-schedules.md §9.1`, "The replan gate and the stuck counter's three
    // bands": in the `1..=199` band the walker takes "The **one-in-three
    // replan draw** ... On a pass the NPC re-plans; on a failure the NPC's
    // turn simply ends. **The counter is not aged, incremented or otherwise
    // touched on a failure.**"
    //
    // `RETRACTIONS.md` R368 withdraws the aging half this test used to pin:
    // "The rate and the gate stand; **the aging half is withdrawn.** ... in
    // that band a **failed draw ages nothing** - it simply ends that NPC's
    // turn."
    //
    // The draw's form is also published: "a uniform range draw over `0`, `1`,
    // `2` accepted on **exactly the middle value** ... not a byte-and-mask
    // draw like the one-in-two wander coin. ... A mask model consumes the
    // same single advance and selects a different third of the stream."
    //
    // The NPC is in state 2 with its direct step blocked, so the pathfinder
    // is the only way it can move.
    let blocked_state = |seed: u16| {
        let mut grid = npc_open_grid();
        // Wall off the direct step toward the waypoint at (3, 9).
        grid[3 + 4 * 32] = 0xB9;
        let mut state = test_state(grid, 20, 20);
        let mut npc = wandering_npc(0, 3, 3, (3, 9));
        npc.state = NPC_STATE_INPLANE_MOVE;
        npc.stuck_counter = 1;
        state.npcs.push(npc);
        state.prng_state = seed;
        state
    };

    // The draw is accepted on the middle value of `0..=2`, so `0` and `2`
    // both refuse. A mask model accepting on `0` would pick a different third
    // of the stream while spending the same advance.
    let refusing_seed = (0..=u16::MAX)
        .find(|seed| {
            let mut probe = test_state(open_grid(), 5, 5);
            probe.prng_state = *seed;
            probe.random_range_u8(0, 2) != 1
        })
        .expect("some PRNG state fails the one-in-three replan draw");
    let mut refused = blocked_state(refusing_seed);
    refused.advance_npc_schedules();
    assert_eq!((refused.npcs[0].x, refused.npcs[0].y), (3, 3));
    // R368: "a failed draw ages nothing - it simply ends that NPC's turn."
    assert_eq!(refused.npcs[0].stuck_counter, 1);
    assert!(refused.npcs[0].move_queue.is_empty());

    let passing_seed = (0..=u16::MAX)
        .find(|seed| {
            let mut probe = test_state(open_grid(), 5, 5);
            probe.prng_state = *seed;
            probe.random_range_u8(0, 2) == 1
        })
        .expect("some PRNG state passes the one-in-three replan draw");
    let mut replanned = blocked_state(passing_seed);
    replanned.advance_npc_schedules();
    // The replan ran, the route was found and its first step committed.
    assert_ne!((replanned.npcs[0].x, replanned.npcs[0].y), (3, 3));

    // A seed that would pass a `== 0` mask model but fails the published
    // middle-value draw still ends the turn, which is the parity the
    // retraction is about.
    let mask_only_seed = (0..=u16::MAX)
        .find(|seed| {
            let mut probe = test_state(open_grid(), 5, 5);
            probe.prng_state = *seed;
            probe.random_range_u8(0, 2) == 0
        })
        .expect("some PRNG state draws zero");
    let mut mask_model = blocked_state(mask_only_seed);
    mask_model.advance_npc_schedules();
    assert_eq!((mask_model.npcs[0].x, mask_model.npcs[0].y), (3, 3));
    assert_eq!(mask_model.npcs[0].stuck_counter, 1);
}

/// A town NPC in the in-plane route state at `(3, 3)`, walled in on the
/// cardinals named by `open`, with an unreachable waypoint at `(3, 9)`.
fn boxed_route_npc_state(open: &[(usize, usize)]) -> PlayState {
    let mut grid = npc_open_grid();
    for cell in [(2usize, 3usize), (4, 3), (3, 2), (3, 4)] {
        if !open.contains(&cell) {
            grid[cell.0 + cell.1 * 32] = 0xB9;
        }
    }
    // A wall right across the map keeps the waypoint unreachable, so every
    // replan attempt fails.
    for x in 0..32 {
        grid[x + 6 * 32] = 0xB9;
    }
    let mut state = test_state(grid, 20, 20);
    let mut npc = wandering_npc(0, 3, 3, (3, 9));
    npc.state = NPC_STATE_INPLANE_MOVE;
    state.npcs.push(npc);
    state
}

#[test]
fn a_counter_of_zero_replans_unconditionally_and_spends_no_draw() {
    // `npc-schedules.md §9.1`: "`0` | The NPC re-plans **unconditionally**.
    // No draw is taken, and no draw is spent."
    //
    // Every seed must behave the same way, including seeds on which the
    // one-in-three draw would refuse: the draw is simply not reached.
    for seed in [0x0001u16, 0x1234, 0x2468, 0xBEEF] {
        let mut state = boxed_route_npc_state(&[]);
        state.prng_state = seed;
        assert_eq!(state.npcs[0].stuck_counter, 0);

        state.advance_npc_schedules();

        // The replan was attempted and failed, so the high band was entered
        // by assignment rather than by counting.
        assert_eq!(state.npcs[0].stuck_counter, NPC_STUCK_HIGH_BAND_ENTRY);
    }
}

#[test]
fn a_failed_replan_assigns_two_hundred_on_a_turn_that_is_not_still() {
    // `npc-schedules.md §9.1`, "The aging band, `200..204`": "The value `200`
    // is never *counted* into: it is **assigned** outright when a replan is
    // attempted and the pathfinder fails to reach the waypoint." And
    // `RETRACTIONS.md` R369: "**The turn on which the counter is set to
    // `200` is not a still turn.** That turn falls straight through into the
    // cap-zero recovery step ... which can and does commit a move".
    let mut state = boxed_route_npc_state(&[(4, 3)]);
    state.prng_state = wander_seed_for(Direction::East);

    state.advance_npc_schedules();

    assert_eq!(state.npcs[0].stuck_counter, NPC_STUCK_HIGH_BAND_ENTRY);
    assert_eq!(
        (state.npcs[0].x, state.npcs[0].y),
        (4, 3),
        "the assigning turn falls through into the cap-zero recovery step"
    );
}

#[test]
fn the_high_band_is_five_still_turns_and_the_sixth_replans() {
    // `npc-schedules.md §9.1`: "`200` .. `204` | No draw and no replan. The
    // counter is incremented, and once the increment carries it past `204`
    // the same pass zeroes it." R369: "the band that follows is **five**
    // consecutive turns of complete stillness (entry values two hundred
    // through two hundred four, the fifth pass both incrementing past two
    // hundred four and zeroing), after which the **sixth** turn re-plans
    // unconditionally with no draw."
    let mut state = boxed_route_npc_state(&[]);

    // Turn one: counter zero, unconditional replan, pathfinder fails, `200`
    // assigned.
    state.advance_npc_schedules();
    assert_eq!(state.npcs[0].stuck_counter, NPC_STUCK_HIGH_BAND_ENTRY);

    // Five aging passes. The NPC is walled in, so stillness is guaranteed by
    // the geometry; what this pins is the counter's walk and the zeroing
    // pass.
    for expected in [201u16, 202, 203, 204, 0] {
        state.advance_npc_schedules();
        assert_eq!(state.npcs[0].stuck_counter, expected);
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 3));
    }
    assert_eq!(NPC_STUCK_HIGH_BAND_LAST, 204);

    // The sixth turn sees a counter of zero and re-plans unconditionally,
    // which fails again and re-assigns the band entry.
    state.advance_npc_schedules();
    assert_eq!(state.npcs[0].stuck_counter, NPC_STUCK_HIGH_BAND_ENTRY);
}

#[test]
fn only_a_refused_queued_route_step_increments_the_stuck_counter() {
    // `RETRACTIONS.md` R367: "**Exactly one event increments the counter: a
    // queued route step refused by the per-step cell check.** Neither
    // cap-zero route-recovery step of Section 9.1 can increment it ... and
    // the recovery stepper never touches the counter table on any of its
    // three rejection paths."
    let mut grid = npc_open_grid();
    grid[2 + 3 * 32] = 0xB9;
    let mut state = test_state(grid, 20, 20);
    let mut npc = wandering_npc(0, 3, 3, (12, 12));
    npc.state = NPC_STATE_REPLAY_QUEUE;
    npc.move_queue = vec![NPC_PATH_DIR_WEST, NPC_PATH_DIR_WEST];
    state.npcs.push(npc);
    // A losing coin makes the recovery step reject, which R367 names as one
    // of the three rejection paths that must not touch the counter - the one
    // increment already happened ahead of it.
    state.prng_state = losing_wander_coin_seed();

    state.advance_npc_schedules();

    assert_eq!((state.npcs[0].x, state.npcs[0].y), (3, 3));
    assert_eq!(state.npcs[0].stuck_counter, 1);

    // The exhausted-queue arm takes the same recovery step and increments
    // nothing at all: the counter it starts with is the counter it keeps.
    let mut exhausted = boxed_route_npc_state(&[]);
    exhausted.npcs[0].stuck_counter = 2;
    let mut searched = true;
    let outcome = exhausted.advance_npc_in_plane_schedule_step(0, 0, 3, 9, 0, 0, &mut searched);
    assert_eq!(outcome, NpcScheduleStepOutcome::Stalled);
    assert_eq!(exhausted.npcs[0].stuck_counter, 2);
}

#[test]
fn a_successful_queued_route_step_resets_the_stuck_counter() {
    // `npc-schedules.md §5` step 7: "A queued route step that commits resets
    // the counter to zero" - one of the "two facts the old wording hid"
    // (`RETRACTIONS.md` R367).
    let mut state = test_state(npc_open_grid(), 20, 20);
    let mut npc = wandering_npc(0, 3, 3, (6, 3));
    npc.state = NPC_STATE_REPLAY_QUEUE;
    npc.move_queue = vec![NPC_PATH_DIR_EAST, NPC_PATH_DIR_EAST];
    npc.stuck_counter = 2;
    state.npcs.push(npc);

    state.advance_npc_schedules();

    assert_eq!((state.npcs[0].x, state.npcs[0].y), (4, 3));
    assert_eq!(state.npcs[0].stuck_counter, 0);
}

#[test]
fn a_quickness_suppressed_outdoor_turn_does_not_advance_the_transport_parity() {
    // `encounters.md §2.1`'s per-turn-shape table, and `RETRACTIONS.md`
    // R370/R374: "a Quickness-suppressed turn returns before the transport
    // marker is read, so it does **not** advance the transport parity", while
    // "a gated turn does not 'skip' the alternation - advancing the
    // alternation is what the gate does".
    let mut world = world_state(open_world_grid(), 5, 5);
    world.player.transport = mounted_horse_transport();
    world.active_effect_tag = Some(TimingStatusTag::HalfTime.save_byte());
    world.active_effect_counter = 10;

    // Pass one: Quickness is reached, flips its own bit and comes up set, so
    // the turn is suppressed before the transport marker is read.
    assert_eq!(
        world.overworld_walker_effect_gates(),
        WalkerEffectGate::SkippedByQuickness
    );
    assert!(world.quickness_walker_gate_parity);
    assert!(
        !world.transport_walker_gate_parity,
        "the transport gate was never reached, so its parity stands"
    );

    // Pass two: Quickness lets the turn through, so the transport gate is
    // reached and advances its own alternation.
    assert_eq!(
        world.overworld_walker_effect_gates(),
        WalkerEffectGate::SkippedByTransportMarker
    );
    assert!(!world.quickness_walker_gate_parity);
    assert!(world.transport_walker_gate_parity);
}

#[test]
fn a_repeated_heading_neither_rewrites_the_cache_nor_restarts_the_wait() {
    // `weather.md §5.1`, "**The one setter.**": "The store runs only when the
    // requested heading **differs** from the cached one, and it is the same
    // step that zeroes the sailing counter. Repeating the heading already
    // cached changes neither the cache nor the counter, and in particular
    // does not restart the wait." And: "The stored value is always a real
    // movement direction and is **never zero**, so **a new heading replaces
    // the cache rather than clearing it.**"
    //
    // An east wind against an easterly heading is the two-pass upwind wait of
    // `weather.md §5`, so the counter is observable between passes.
    let mut state = under_sail_world(5, 5, WindState::East);

    // The differing-heading step is the setter, and `weather.md §5` bounds
    // what it may do on the same command: "the ship turns and clears the
    // sailing counter; that action does not also move the ship."
    assert_eq!(state.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!(state.sail_cached_direction, Some(Direction::East));
    assert_eq!(state.sail_cadence, 0);
    assert_eq!((state.player.x, state.player.y), (5, 5));

    // Repeating the cached heading advances the wait instead of restarting
    // it.
    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);
    assert_eq!(state.sail_cached_direction, Some(Direction::East));
    assert_eq!(state.sail_cadence, 1);

    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);
    assert_eq!(state.sail_cached_direction, Some(Direction::East));
    assert_eq!(state.sail_cadence, 2);

    // A different heading replaces the cache and zeroes the counter on the
    // same step. The cache is never momentarily empty, and the ship stays put
    // on that step even though west is downwind of an east wind.
    assert_eq!(state.step(Direction::West), MoveOutcome::SailTurned);
    assert_eq!(state.sail_cached_direction, Some(Direction::West));
    assert_eq!(state.sail_cadence, 0);
    assert_eq!((state.player.x, state.player.y), (5, 5));

    // The restarted one-pass wait is then paid from zero.
    assert_eq!(state.step(Direction::West), MoveOutcome::SailStalled);
    assert_eq!(state.sail_cadence, 1);
    assert_eq!(state.step(Direction::West), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (4, 5));
    assert_eq!(state.sail_cadence, 0);
}

#[test]
fn a_wind_change_zeroes_the_sailing_counter_but_a_re_announcement_does_not() {
    // `weather.md §5.1`, "**A fourth reset of the sailing counter.**":
    // "**a wind change zeroes the sailing counter.** The routine that stores
    // a new wind state clears the counter immediately afterwards, and both
    // are skipped when the same routine is called only to re-announce the
    // wind already in force. So a ship one pass into a two-pass upwind wait
    // restarts that wait when the wind shifts."
    let mut state = under_sail_world(5, 5, WindState::East);
    // The heading-establishing turn leaves the counter at zero; the wait pass
    // after it is what puts the ship one pass into the two-pass upwind wait.
    assert_eq!(state.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!(state.sail_cadence, 0);
    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);
    assert_eq!(state.sail_cadence, 1);

    // Re-announcing the wind already in force leaves the wait alone.
    assert!(!state.apply_wind_state(WindState::East));
    assert_eq!(state.sail_cadence, 1);

    // An actual change restarts it.
    assert!(state.apply_wind_state(WindState::South));
    assert_eq!(state.sail_cadence, 0);
}

#[test]
fn the_post_command_marker_guard_owns_the_sail_cache_clears() {
    // `weather.md §5.1`, clear four: "After **every** outdoor command handler
    // returns, and before the next input is read, the loop clears the cache
    // unless the transport marker is still a hoisted frigate." That single
    // guard is why "Furling, boarding, disembarking, mounting a horse or a
    // carpet, and anything else that moves the transport marker out of the
    // hoisted-frigate run are covered ... on the same turn", and equally why
    // "a Look, a Ztats, or any other marker-neutral command under sail does
    // **not** interrupt sailing".
    let mut state = under_sail_world(5, 5, WindState::North);
    assert_eq!(state.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!(state.sail_cached_direction, Some(Direction::East));

    // Marker-neutral: the guard leaves the cache alone.
    state.apply_outdoor_sail_cache_marker_guard();
    assert_eq!(state.sail_cached_direction, Some(Direction::East));
    assert!(is_ship_transport_hoisted(state.player.transport.save_marker()));

    // Any command that moves the marker out of `0x20..=0x23` is covered,
    // whether or not its handler mentions the cache. A furled ship is the
    // simplest such marker move.
    state.player.transport = state.player.transport.with_ship_sails_furled();
    assert!(!is_ship_transport_hoisted(
        state.player.transport.save_marker()
    ));
    state.apply_outdoor_sail_cache_marker_guard();
    assert!(state.sail_cached_direction.is_none());

    // The guard is outdoor-only; it does not reach into a town scene.
    let mut town = test_state(open_grid(), 5, 5);
    town.sail_cached_direction = Some(Direction::East);
    town.apply_outdoor_sail_cache_marker_guard();
    assert_eq!(town.sail_cached_direction, Some(Direction::East));
}

#[test]
fn entering_outdoor_mode_clears_the_cached_sail_direction() {
    // `weather.md §5.1`, clear one: "Entering outdoor mode | The
    // outdoor-mode entry pass clears the cache in its opening block, so no
    // cached heading survives a scene change into the overworld."
    let mut state = under_sail_world(5, 5, WindState::North);
    assert_eq!(state.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!(state.sail_cached_direction, Some(Direction::East));

    state.clear_sail_cache_on_outdoor_mode_entry();

    assert!(state.sail_cached_direction.is_none());
}

#[test]
fn the_same_direction_key_is_swallowed_under_sail() {
    // `timing.md §8.2` / `RETRACTIONS.md` R372: "**One keyboard poll.** Every
    // pass polls exactly one key. No key runs the advance body. A key whose
    // translated code **equals the cached sail heading is swallowed** -
    // discarded, with the advance body running anyway - which is why pressing
    // the arrow you are already sailing does nothing. Any other key ends the
    // loop and becomes that turn's command."
    let mut state = under_sail_world(5, 5, WindState::North);
    // `weather.md §5`: the heading-establishing command "turns and clears the
    // sailing counter; that action does not also move the ship". It is the
    // command that fills the cache the swallow tests against.
    assert_eq!(state.step(Direction::East), MoveOutcome::SailTurned);
    assert_eq!(state.sail_cached_direction, Some(Direction::East));

    assert!(state.under_sail_key_is_swallowed(Some(Direction::East)));
    // Any other key ends the loop and becomes the turn's command.
    assert!(!state.under_sail_key_is_swallowed(Some(Direction::West)));
    assert!(!state.under_sail_key_is_swallowed(Some(Direction::North)));
    assert!(!state.under_sail_key_is_swallowed(None));

    // Nothing is swallowed when the auto-advance route does not apply, so a
    // steered ship that has furled reads its direction keys normally.
    let mut furled = under_sail_world(5, 5, WindState::North);
    assert_eq!(furled.step(Direction::East), MoveOutcome::SailTurned);
    furled.player.transport = furled.player.transport.with_ship_sails_furled();
    assert!(!furled.under_sail_key_is_swallowed(Some(Direction::East)));

    // End to end: the dispatcher discards the swallowed key, but "the advance
    // body [runs] anyway" - `timing.md §8.2` calls the swallow "observable
    // behaviour rather than a timing detail", and per pass the body costs
    // "**One world step, unconditionally**" and "**One fully consumed game
    // turn**". So the key is not a no-op: the auto-advance pass's synthesized
    // command in the cached heading is what lands.
    let turn_before = state.turn;
    let position_before = (state.player.x, state.player.y);
    assert_eq!(
        handle_play_key_input(
            &mut state,
            char::from(INPUT_CODE_EAST),
            "",
            Path::new("")
        )
        .unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(
        state.turn > turn_before,
        "the swallowed key must still run the advance body's consumed turn"
    );
    assert_eq!(
        (state.player.x, state.player.y),
        (position_before.0 + 1, position_before.1),
        "the advance body's synthesized command is the cached heading"
    );
    assert_eq!(state.sail_cached_direction, Some(Direction::East));

    // The advance body is the pass, not an extra movement command: a
    // swallowed key leaves the same world state as the idle pump's own pass
    // (`overworld.md §5` step 3 - the helper "returns the cached sail
    // direction ... which is how a ship keeps moving with no keypress").
    let mut pumped = under_sail_world(5, 5, WindState::North);
    assert_eq!(pumped.step(Direction::East), MoveOutcome::SailTurned);
    pumped.run_under_sail_advance_body(None).unwrap();
    assert_eq!((pumped.player.x, pumped.player.y), (state.player.x, state.player.y));
    assert_eq!(pumped.turn, state.turn);

    // A wind-stalled heading pays the wait pass rather than the step, and
    // that too is a consumed turn - the ship must not sit frozen because the
    // key was swallowed.
    let mut becalmed = under_sail_world(5, 5, WindState::Calm);
    becalmed.sail_cached_direction = Some(Direction::East);
    let becalmed_turn = becalmed.turn;
    let becalmed_position = (becalmed.player.x, becalmed.player.y);
    assert!(becalmed.under_sail_key_is_swallowed(Some(Direction::East)));
    assert_eq!(
        handle_play_key_input(
            &mut becalmed,
            char::from(INPUT_CODE_EAST),
            "",
            Path::new("")
        )
        .unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(becalmed.turn > becalmed_turn);
    assert_eq!(
        (becalmed.player.x, becalmed.player.y),
        becalmed_position,
        "a becalmed pass consumes its turn without releasing the step"
    );
}

