// u5-spec#309 (`combat.md §11`, "The three keyless arms enter the endpoint";
// `RETRACTIONS.md` R651): the asleep (`Zzzzz...`), dragged-under (`ARGH!`)
// and Sword of Chaos command-handler arms all enter the combat maintenance
// endpoint - ring hook, field pass, timed-counter ageing - on every
// dispatch, after the arm's own draws and before the contact hook. Playtest
// P17-14 (a lone member asleep on a Sleep Field softlocking the fight) is the
// black-box consequence.

/// A one-member arena: party slot 0 at (5, 5), its sprite in record 0, and an
/// optional field marker in record 1 at `field_cell`.
fn spec309_lone_member_arena(flags: u8, field: Option<(u8, (usize, usize))>) -> PlayState {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party.truncate(1);
    state.party_combat_defense = vec![0; state.party.len()];
    state.combat_active = true;
    state.combat_round_loop_prologue_ran = true;
    state.combat_terrain = [[0x05; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
    state.active_objects[0] = ActiveObject {
        type_byte: 0x80,
        tile: 0x80,
        x: 5,
        y: 5,
        ..ActiveObject::empty()
    };
    if let Some((kind, (x, y))) = field {
        state.active_objects[1] = ActiveObject {
            type_byte: kind,
            tile: kind,
            x,
            y,
            ..ActiveObject::empty()
        };
    }
    if flags & COMBAT_ACTOR_FLAG_STATUS_DISABLED != 0 {
        state.party[0].status = b'S';
    }
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        20,
        20,
        COMBAT_ACTOR_FLAG_SELECTABLE_80 | flags,
        0,
        0,
        0,
        5,
        5,
    ]);
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 20;
    state.pending_combat_actor_slot = None;
    state.next_combat_actor_slot = 0;
    state.active_player = None;
    state
}

fn spec309_dispatch(state: &mut PlayState, slot: usize) -> CombatActorDispatchAction {
    let application = state.apply_combat_actor_slot_dispatch_with_inputs(
        slot,
        30,
        false,
        0,
        false,
        1,
        1,
        &[],
        None,
        0,
        false,
        None,
        true,
        &[1, 2, 3, 4],
        &[],
    );
    let CombatActorSlotDispatchApplication::Slot { action, .. } = application else {
        panic!("a live slot should produce a slot dispatch");
    };
    action
}

/// `§11`: "Asleep party member: the wake draw (one inclusive `0..255`
/// gameplay draw ...); `Zzzzz...` ...; the maintenance endpoint, in which the
/// worn-ring hook does nothing ..., the field pass makes one `0..255` draw
/// per field marker in the arena, and ageing makes none".
#[test]
fn spec309_the_asleep_arm_runs_the_maintenance_endpoint() {
    // A Poison Field away from the sleeper, so the contact hook stays inert.
    let field = Some((COMBAT_FIELD_KIND_POISON, (1, 1)));
    let mut state = spec309_lone_member_arena(COMBAT_ACTOR_FLAG_STATUS_DISABLED, field);
    let mut without = spec309_lone_member_arena(COMBAT_ACTOR_FLAG_STATUS_DISABLED, None);
    // A seed whose field draw keeps the marker, so the record survives.
    let mut seed = 0u16;
    loop {
        let mut probe = seed;
        let _wake = u5_prng_range_u16(&mut probe, 0, 255);
        if u5_prng_range_u16(&mut probe, 0, 255) >= 16 {
            break;
        }
        seed += 1;
    }
    state.prng_state = seed;
    without.prng_state = seed;

    let action = spec309_dispatch(&mut state, 0);
    let _ = spec309_dispatch(&mut without, 0);

    assert!(matches!(
        action,
        CombatActorDispatchAction::StatusDisabledWake { .. }
    ));
    let mut expected = without.prng_state;
    let _ = u5_prng_range_u16(&mut expected, 0, 255);
    assert_eq!(state.prng_state, expected, "one field draw per marker");
    assert_eq!(
        state.active_effect_counter, 19,
        "one ageing per asleep turn"
    );
    assert_eq!(without.active_effect_counter, 19);
}

/// `§11`: "Maintenance runs on each of the sleeper's own asleep dispatches,
/// so the field under it gets one removal draw per such dispatch, after that
/// dispatch's wake draw and before the contact hook; a wake on the dispatch
/// in which the field dissolves therefore sticks."
#[test]
fn spec309_a_wake_on_the_dispatch_the_field_dissolves_sticks() {
    // Record 1 is a Poison Field decoy away from the sleeper and record 2 the
    // Sleep Field under her, so each asleep dispatch draws wake, decoy, then
    // the Sleep Field's removal.
    let arena = || {
        let mut state = spec309_lone_member_arena(
            COMBAT_ACTOR_FLAG_STATUS_DISABLED,
            Some((COMBAT_FIELD_KIND_POISON, (1, 1))),
        );
        state.active_objects[2] = ActiveObject {
            type_byte: COMBAT_FIELD_KIND_SLEEP,
            tile: COMBAT_FIELD_KIND_SLEEP,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };
        state
    };
    let seed_where = |field_goes: bool| {
        (0u16..=u16::MAX)
            .find(|&seed| {
                let mut probe = seed;
                let wake = u5_prng_range_u16(&mut probe, 0, 255) < 16;
                let decoy_stays = u5_prng_range_u16(&mut probe, 0, 255) >= 16;
                let dissolves = u5_prng_range_u16(&mut probe, 0, 255) < 16;
                wake && decoy_stays && dissolves == field_goes
            })
            .expect("some seed")
    };

    let mut sticks = arena();
    sticks.prng_state = seed_where(true);
    let _ = spec309_dispatch(&mut sticks, 0);
    assert!(sticks.active_objects[2].is_empty(), "the field dissolved");
    assert!(
        !sticks.combat_actors[0].is_status_disabled(),
        "the wake sticks"
    );
    assert_eq!(sticks.party[0].status, b'G');

    let mut resleeps = arena();
    resleeps.prng_state = seed_where(false);
    let _ = spec309_dispatch(&mut resleeps, 0);
    assert!(!resleeps.active_objects[2].is_empty(), "the field holds");
    assert!(
        resleeps.combat_actors[0].is_status_disabled(),
        "the live field re-sleeps the woken member"
    );
}

/// `§11`: "Dragged-under party member: the `1..30` release draw; the ring
/// hook, which does run here ...; the field pass; ageing".
#[test]
fn spec309_the_dragged_under_arm_runs_the_maintenance_endpoint() {
    let field = Some((COMBAT_FIELD_KIND_POISON, (1, 1)));
    let mut state = spec309_lone_member_arena(COMBAT_ACTOR_FLAG_DRAGGED_UNDER, field);
    // Base step 1: the hold is never released (`§8.1`, release when the
    // `1..30` draw is below the base step).
    state.combat_actors[0].base_step = 1;
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    // Freeze the per-dispatch world ticks (u5-spec#310 Q2) so the stream
    // holds only the draws counted here.
    state.time_stop_counter = u8::MAX;
    let seed = state.prng_state;

    let action = spec309_dispatch(&mut state, 0);

    assert_eq!(action, CombatActorDispatchAction::DraggedUnderTurn);
    assert!(
        state.combat_actors[0].is_dragged_under(),
        "base step 1 never releases"
    );
    let mut expected = seed;
    let _release = u5_prng_range_u16(&mut expected, 1, 30);
    let _regeneration = u5_prng_range_u16(&mut expected, 0, 7);
    let _field = u5_prng_range_u16(&mut expected, 0, 255);
    assert_eq!(
        state.prng_state, expected,
        "release draw, one regeneration roll, one field draw"
    );
    assert_eq!(state.active_effect_counter, 19);
}

/// `§11`: "Sword of Chaos party member: the automatic driver's own draws
/// (Section 9), then the ring hook, the field pass, ageing". Only the
/// dispatch that takes the Chaos arm enters it; the walker sends the slot to
/// the driver directly once the controlled bit is set (`§11`: automatic
/// party-side slots "are also routed to the automatic driver, except for the
/// forwarding arm").
#[test]
fn spec309_the_sword_of_chaos_arm_runs_the_maintenance_endpoint() {
    let mut state = spec309_lone_member_arena(0, None);
    state.party_equipment[0][EQUIP_SLOT_WEAPON] = EQUIPMENT_SWORD_OF_CHAOS as u8;
    // A second member keeps the post-dispatch recount off its faint arm.
    state.party.push(state.party[0].clone());
    state.party_combat_defense = vec![0; 2];
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 6, 5]);

    let _ = spec309_dispatch(&mut state, 0);
    assert!(
        state.combat_actors[0].is_controlled(),
        "the Chaos arm fired"
    );
    assert_eq!(state.active_effect_counter, 19, "the hand-off ages once");

    // The next dispatch is the walker's own driver routing, not the arm.
    let _ = spec309_dispatch(&mut state, 0);
    assert_eq!(state.active_effect_counter, 19);
}

/// `§11`: "A charmed monster asleep on its keyboard turn takes the asleep arm
/// and so also gets maintenance."
#[test]
fn spec309_an_asleep_charmed_monster_gets_maintenance() {
    let mut state = spec309_lone_member_arena(0, None);
    let slot = COMBAT_PARTY_ACTOR_SLOTS;
    state.combat_actors[slot] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40
            | COMBAT_ACTOR_FLAG_CONTROLLED
            | COMBAT_ACTOR_FLAG_STATUS_DISABLED,
        COMBAT_CLASS_GIANT_RAT,
        2,
        0,
        7,
        7,
    ]);

    let action = spec309_dispatch(&mut state, slot);

    assert!(matches!(
        action,
        CombatActorDispatchAction::StatusDisabledWake { .. }
    ));
    assert_eq!(state.active_effect_counter, 19);
}

/// `§8` (u5-spec#309): "Eligible means party-side and neither asleep nor
/// removed, so an asleep member's maintenance pass makes no regeneration
/// roll, while a dragged-under wearer's does."
#[test]
fn spec309_the_ring_hook_skips_an_asleep_wearer() {
    let mut asleep = spec309_lone_member_arena(COMBAT_ACTOR_FLAG_STATUS_DISABLED, None);
    asleep.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    let seed = asleep.prng_state;
    assert!(
        asleep
            .apply_visible_combat_magic_ring_pass_to_slot(0)
            .is_none()
    );
    assert_eq!(asleep.prng_state, seed, "no regeneration roll");

    let mut held = spec309_lone_member_arena(COMBAT_ACTOR_FLAG_DRAGGED_UNDER, None);
    held.combat_actors[0].base_step = 1;
    held.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    let seed = held.prng_state;
    let _ = held.apply_visible_combat_magic_ring_pass_to_slot(0);
    let mut expected = seed;
    let _regeneration = u5_prng_range_u16(&mut expected, 0, 7);
    assert_eq!(held.prng_state, expected, "one regeneration roll");
}

/// Playtest P17-14, end to end: the only party member in the arena is asleep
/// on a Sleep Field. `§11`: "An actor alone and asleep on a Sleep Field is
/// not stuck: it needs about 31 asleep dispatches on average (about 16 for
/// the field to go, then about 15 more for a wake) before it gets a keyboard
/// turn." One drive of the round walker must reach her prompt on every seed,
/// with the field gone and the member awake.
#[test]
fn spec309_p17_14_a_lone_sleeper_on_a_sleep_field_reaches_a_keyboard_turn() {
    let field = Some((COMBAT_FIELD_KIND_SLEEP, (5, 5)));
    for seed in 0..120u16 {
        let mut state = spec309_lone_member_arena(COMBAT_ACTOR_FLAG_STATUS_DISABLED, field);
        state.prng_state = seed.wrapping_mul(40_503).wrapping_add(7);

        open_owed_combat_player_turn(&mut state);

        assert!(state.combat_active, "seed {seed}: the fight is still on");
        assert_eq!(
            state.pending_combat_actor_slot,
            Some(0),
            "seed {seed}: the sleeper's keyboard turn opens"
        );
        assert!(
            state.active_objects[1].is_empty(),
            "seed {seed}: the field dissolved"
        );
        assert!(
            !state.combat_actors[0].is_status_disabled(),
            "seed {seed}: awake"
        );
        assert_eq!(state.party[0].status, b'G', "seed {seed}");
    }
}

/// `§11`: "Executed multi-turn runs of the original chain (world ticks
/// stubbed) gave a mean of 31.6 over 120 seeds, median 29, range 3 to 98,
/// none stuck". Counted here as the sleeper's own asleep dispatches until
/// she wakes with the field gone.
#[test]
fn spec309_p17_14_asleep_dispatch_count_is_finite_and_near_the_published_mean() {
    let field = Some((COMBAT_FIELD_KIND_SLEEP, (5, 5)));
    let seeds = 120u16;
    let mut total = 0usize;
    for seed in 0..seeds {
        let mut state = spec309_lone_member_arena(COMBAT_ACTOR_FLAG_STATUS_DISABLED, field);
        state.prng_state = seed.wrapping_mul(40_503).wrapping_add(7);
        let mut asleep_dispatches = 0usize;
        for _ in 0..100_000 {
            if !state.combat_actors[0].is_status_disabled() {
                break;
            }
            if matches!(
                spec309_dispatch(&mut state, 0),
                CombatActorDispatchAction::StatusDisabledWake { .. }
            ) {
                asleep_dispatches += 1;
            }
        }
        assert!(
            !state.combat_actors[0].is_status_disabled(),
            "seed {seed}: stuck asleep after {asleep_dispatches} asleep dispatches"
        );
        assert!(state.active_objects[1].is_empty(), "seed {seed}");
        total += asleep_dispatches;
    }
    let mean = total as f64 / f64::from(seeds);
    assert!(
        (20.0..=45.0).contains(&mean),
        "mean asleep dispatches {mean:.1}, the spec's executed mean is 31.6"
    );
}
