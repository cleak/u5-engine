// Conformance fixes from the 2026-10-16 audit against `systems/combat.md`
// and `formats/cbt.md` (group g1). Each test names the section it pins.

fn audit_g1_last_entry_texts(state: &PlayState, count: usize) -> Vec<String> {
    let entries = state.message_entries();
    entries[entries.len().saturating_sub(count)..]
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

#[test]
fn monster_teleport_line_continues_an_open_row_and_closes_it() {
    // `combat.md §11.1`: "Monster teleport | `<monster> teleports!\n`, no
    // leading newline of its own and **with** a trailing newline", and
    // "teleport can continue an already-open row before ending it".
    let mut state = combat_ai_turn_state(10, 10);
    state.combat_actors[8].owner_target_class = 13;
    state.combat_actors[8].hp_or_wound = combat_class_stats(13).unwrap().max_hp;
    state.combat_actors[0].x = 0;
    state.combat_actors[0].y = 0;
    state.active_objects[0].x = 0;
    state.active_objects[0].y = 0;
    state.emit_message_line("Open row ");
    assert!(state.message_row_open_mid_line);

    let application = state
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            255,
            255,
            &[],
            None,
            0,
            false,
            Some((9, 9)),
            true,
            &[1, 2, 3, 4],
            None,
        )
        .unwrap();

    assert_eq!(
        application.movement,
        Some(CombatAiMovementOutcome::Teleport { x: 9, y: 9 })
    );
    let name = combat_class_stats(13).unwrap().name;
    assert_eq!(
        audit_g1_last_entry_texts(&state, 1),
        vec![format!("Open row {name} teleports!")]
    );
    assert!(!state.message_row_open_mid_line);
}

#[test]
fn self_acting_arena_exit_plays_the_action_snap_and_runs_one_world_tick() {
    // `combat.md §11.1`: "**The cue is the 40-update action snap**" on the
    // arena-exit arm, and `§14` step 3: an ordinary escape "ends with
    // **exactly one** world tick".
    let mut state = combat_ai_turn_state(10, 10);
    state.combat_actors[8].owner_target_class = 13;
    state.combat_actors[0].x = 0;
    state.combat_actors[0].y = 0;
    state.active_objects[0].x = 0;
    state.active_objects[0].y = 0;
    // Same fixture as the Crown-bypass test: the Crown suppresses the
    // teleport arm, and the fleeing actor in the far corner steps off-grid.
    state.active_effect_tag = Some(CROWN_LB_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = u8::MAX;
    let frame_before = state.animation.frame;
    let sounds_before = state.sound_effect_history.len();

    let application = state
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            255,
            255,
            &[],
            None,
            0,
            false,
            Some((9, 9)),
            true,
            &[1, 2, 3, 4],
            None,
        )
        .unwrap();

    assert_eq!(
        application.movement,
        Some(CombatAiMovementOutcome::ArenaExit { direction_code: 2 })
    );
    let new_sounds: Vec<SoundEffect> = state.sound_effect_history[sounds_before..]
        .iter()
        .map(|(_, effect)| effect.clone())
        .collect();
    assert_eq!(new_sounds, vec![SoundEffect::ActionSnap]);
    assert_eq!(
        state.animation.frame,
        frame_before.wrapping_add(1),
        "exactly one world tick"
    );
    assert!(!combat_actor_is_active_not_dead(state.combat_actors[8]));
}

#[test]
fn wound_band_morale_draw_spans_zero_through_two_fifty_six() {
    // `combat.md §9` (`RETRACTIONS.md` R519): the morale check "sets
    // fleeing on only five of 257 possible results (a roll of 252 through
    // 256; the draw is over the inclusive range `0..256`, not a byte)".
    let seed = (0..=u16::MAX)
        .find(|&seed| {
            let mut probe = seed;
            u5_prng_range_u16(&mut probe, 0, 256) == 256
        })
        .expect("some seed draws 256");
    let mut state = combat_ai_turn_state(8, 5);
    let max_hp = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().max_hp;
    // First value of the quarter-to-under-half band.
    state.combat_actors[8].hp_or_wound = max_hp / 4;
    state.prng_state = seed;
    let mut expected_state = seed;
    let _ = u5_prng_range_u16(&mut expected_state, 0, 256);

    assert!(state.combat_ai_actor_fleeing(8));
    assert_eq!(state.prng_state, expected_state);
    assert!(monster_wound_sets_fleeing(30, 100, 256));
    assert_eq!(
        resolve_combat_wound_morale(30, 100, 256).fleeing,
        true
    );
}

#[test]
fn fleeing_turn_heal_draw_comes_first_and_has_no_ceiling() {
    // `combat.md §9.1` (issue #285): a monster-side fleeing actor "first
    // takes one draw over the four values zero through three on every
    // fleeing turn ... and on a three adds exactly one to the actor's
    // current health with **no ceiling**", before the classifier's own
    // conditional draw.
    let find_seed = |want_three: bool| {
        (0..=u16::MAX)
            .find(|&seed| {
                let mut probe = seed;
                (u5_prng_range_u16(&mut probe, 0, 3) == 3) == want_three
            })
            .unwrap()
    };
    let max_hp = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().max_hp;
    for (want_three, expected_hp) in [(true, max_hp + 1), (false, max_hp)] {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].hp_or_wound = max_hp;
        state.combat_actors[8].set_fleeing(true);
        let seed = find_seed(want_three);
        state.prng_state = seed;
        let mut expected_state = seed;
        let _ = u5_prng_range_u16(&mut expected_state, 0, 3);

        // At or above half health the classifier clears the bit and takes
        // no morale draw, so the heal draw is the only one.
        assert!(!state.combat_ai_actor_fleeing(8));
        assert_eq!(state.combat_actors[8].hp_or_wound, expected_hp);
        assert_eq!(state.prng_state, expected_state);
    }

    // A non-fleeing actor takes no heal draw.
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].hp_or_wound = max_hp;
    state.prng_state = 1234;
    assert!(!state.combat_ai_actor_fleeing(8));
    assert_eq!(state.prng_state, 1234);
    assert_eq!(state.combat_actors[8].hp_or_wound, max_hp);
}

fn audit_g1_blocking_arena_tile() -> u8 {
    (0..=u8::MAX)
        .find(|&tile| !is_combat_arena_tile_walkable(tile))
        .expect("some arena tile refuses a step")
}

#[test]
fn surrounded_actor_takes_no_step_and_no_movement_draws() {
    // `combat.md §9.1`: "An **early return** taken before target selection
    // and before either axis attempt: a surrounded actor takes no step at
    // all and the primitive reports that it did not move."
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].hp_or_wound = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().max_hp;
    state.combat_actors[0].x = 2;
    state.active_objects[0].x = 2;
    let wall = audit_g1_blocking_arena_tile();
    for (x, y) in [(7usize, 5usize), (9, 5), (8, 4), (8, 6)] {
        state.combat_terrain[y][x] = wall;
    }
    state.prng_state = 0x1357;

    let application = state.apply_combat_ai_turn(8).unwrap();

    assert_eq!(
        application.movement,
        Some(CombatAiMovementOutcome::Blocked {
            random_cardinal_attempts: 0,
            action_consumed: false,
        })
    );
    // No axis coin and no fallback draws.
    assert_eq!(state.prng_state, 0x1357);
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (8, 5));
}

fn audit_g1_fleeing_adjacent_state() -> PlayState {
    let mut state = combat_ai_turn_state(6, 5);
    // Below one quarter of the class maximum: the classifier always flees.
    state.combat_actors[8].hp_or_wound = 1;
    state.combat_actors[8].set_fleeing(true);
    state.party[0].status = b'G';
    state.party[0].hp = 20;
    state.party[0].max_hp = 20;
    state
}

fn audit_g1_forced_hit() -> CombatMonsterAttackInputs {
    CombatMonsterAttackInputs {
        forced_hit: Some(true),
        ..CombatMonsterAttackInputs::default()
    }
}

#[test]
fn fleeing_actor_in_reach_moves_first_and_does_not_attack() {
    // `combat.md §9.1`: a fleeing actor runs the movement primitive; only
    // "when the primitive reports that it did not move and the fleeing bit
    // is still set" does the driver dispatch it into the attack routine.
    let mut state = audit_g1_fleeing_adjacent_state();

    let application = state
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            255,
            255,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            Some(audit_g1_forced_hit()),
        )
        .unwrap();

    assert_eq!(application.attack_route, Some(CombatAiAttackRoute::Melee));
    assert_eq!(
        application.movement,
        Some(CombatAiMovementOutcome::Step {
            direction_code: 2,
            x: 7,
            y: 5,
        })
    );
    assert_eq!(application.monster_attack, None);
    assert_eq!(state.party[0].hp, 20);
}

#[test]
fn fleeing_actor_that_did_not_move_falls_into_the_attack_routine() {
    let mut state = audit_g1_fleeing_adjacent_state();
    let wall = audit_g1_blocking_arena_tile();
    for (x, y) in [(7usize, 5usize), (6, 4), (6, 6)] {
        state.combat_terrain[y][x] = wall;
    }

    let application = state
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            255,
            255,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            Some(audit_g1_forced_hit()),
        )
        .unwrap();

    assert_eq!(
        application.movement,
        Some(CombatAiMovementOutcome::Blocked {
            random_cardinal_attempts: 0,
            action_consumed: false,
        })
    );
    assert!(application.monster_attack.is_some());
    assert_eq!(application.command_key, Some(COMBAT_AI_ATTACK_COMMAND_KEY));
    assert!(state.party[0].hp < 20);
}

#[test]
fn narrating_a_graded_wound_line_runs_the_flee_classifier() {
    // `combat.md §11.1`: "Because this is the same classifier, a score of 1
    // - and a score of 2 on the morale draw described in Section 9 - also
    // raises the target's fleeing bit as a side effect of narrating the
    // hit."
    let max_hp = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().max_hp;
    let rat = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().name;

    // Score 1: critical, fleeing raised, no draw.
    let mut state = combat_ai_turn_state(6, 5);
    state.combat_actors[8].hp_or_wound = 2;
    let application = state
        .apply_combat_weapon_damage_to_target(Some(0), 8, 1, true)
        .unwrap();
    assert_eq!(state.combat_actors[8].hp_or_wound, 1);
    assert!(!state.combat_actors[8].is_fleeing());
    let prng_before = state.prng_state;
    let line = crate::input_dispatch::combat_landed_damage_result_line(
        &mut state,
        8,
        Some(0),
        application,
    );
    assert_eq!(line, Some(format!("{rat} critical!")));
    assert!(state.combat_actors[8].is_fleeing());
    assert_eq!(state.prng_state, prng_before);

    // Score 2: the morale draw is taken at narration time, over `0..=256`.
    let seed = (0..=u16::MAX)
        .find(|&seed| {
            let mut probe = seed;
            u5_prng_range_u16(&mut probe, 0, 256) >= 252
        })
        .unwrap();
    let mut state = combat_ai_turn_state(6, 5);
    state.combat_actors[8].hp_or_wound = max_hp / 4 + 1;
    let application = state
        .apply_combat_weapon_damage_to_target(Some(0), 8, 1, true)
        .unwrap();
    assert_eq!(state.combat_actors[8].hp_or_wound, max_hp / 4);
    state.prng_state = seed;
    let mut expected_state = seed;
    let _ = u5_prng_range_u16(&mut expected_state, 0, 256);
    let line = crate::input_dispatch::combat_landed_damage_result_line(
        &mut state,
        8,
        Some(0),
        application,
    );
    assert_eq!(line, Some(format!("{rat} heavily wounded!")));
    assert!(state.combat_actors[8].is_fleeing());
    assert_eq!(state.prng_state, expected_state);
}

#[test]
fn active_player_digit_needs_an_eligible_party_actor_with_that_roster_slot() {
    // `combat.md §8.4` (issue #293): "A member is eligible when it is a
    // party-side actor in the arena with that roster slot and none of the
    // disabled, asleep or dead-marked bits".
    let mut state = world_state(open_world_grid(), 10, 20);
    for slot in 1..3 {
        let mut member = state.party[0];
        member.slot = slot;
        state.party.push(member);
    }
    // Roster [0, 1, 2] seated packed: roster 2 sits in descriptor 1 and
    // roster 1 is not in the arena at all.
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 2, 1, 0, 6, 5]);

    assert_eq!(
        state.apply_combat_active_player_digit('2'),
        CombatActivePlayerSelectionOutcome::Invalid
    );
    assert_eq!(state.active_player, None);
    assert_eq!(
        state.apply_combat_active_player_digit('3'),
        CombatActivePlayerSelectionOutcome::SelectPartySlot(2)
    );
    assert_eq!(state.active_player, Some(2));

    for bit in [COMBAT_ACTOR_FLAG_STATUS_DISABLED, COMBAT_ACTOR_FLAG_MARKED_DEAD] {
        state.active_player = None;
        state.combat_actors[1].flags = COMBAT_ACTOR_FLAG_SELECTABLE_80 | bit;
        assert_eq!(
            state.apply_combat_active_player_digit('3'),
            CombatActivePlayerSelectionOutcome::Invalid
        );
        assert_eq!(state.active_player, None);
    }
}

fn audit_g1_flame_wind_state(with_targets: bool) -> PlayState {
    let spell = spell_index_from_code("FHI").unwrap();
    let mut state = combat_ai_turn_state(5, 4);
    state.party[0].status = b'G';
    state.party[0].mana = 99;
    state.party[0].level = 8;
    state.party_intelligence = vec![31; state.party.len()];
    state.spell_charges[spell] = 1;
    state.combat_actors[8] = CombatActorDescriptor::empty();
    state.active_objects[8] = ActiveObject::empty();
    if with_targets {
        let max_hp = combat_class_stats(COMBAT_CLASS_DRAGON).unwrap().max_hp;
        for (slot, y) in [(8usize, 4u8), (9, 3)] {
            state.active_objects[slot] = ActiveObject {
                type_byte: 0x90,
                tile: 0x90,
                x: 5,
                y: usize::from(y),
                ..ActiveObject::empty()
            };
            state.combat_actors[slot] = CombatActorDescriptor::from_row([
                10,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_40,
                COMBAT_CLASS_DRAGON,
                slot as u8,
                0,
                5,
                y,
            ]);
            state.combat_actors[slot].hp_or_wound = max_hp / 2 + 10;
        }
    }
    state
}

/// The reference order: roll, apply and narrate each target in turn
/// (`interleaved`), or every roll first (`!interleaved`, the old batching).
fn audit_g1_flame_wind_model(seed: u16, interleaved: bool) -> PlayState {
    let mut state = audit_g1_flame_wind_state(true);
    state.prng_state = seed;
    let cells = state
        .directed_combat_spell_target_cells(0, Direction::North, CombatDirectedSpellEffect::FlameWind)
        .unwrap();
    let slots = collect_directed_spell_actor_slots(&state.combat_actors, &cells);
    assert_eq!(slots.len(), 2);
    let mut rolls = Vec::new();
    if !interleaved {
        for _ in &slots {
            rolls.push(state.combat_spell_damage_roll_for_kind(CombatSpellDamageKind::FlameWind));
        }
    }
    for (index, slot) in slots.iter().copied().enumerate() {
        let roll = if interleaved {
            state.combat_spell_damage_roll_for_kind(CombatSpellDamageKind::FlameWind)
        } else {
            rolls[index]
        };
        let raw = resolve_directed_spell_raw_damage(CombatDirectedSpellEffect::FlameWind, roll)
            .unwrap();
        let application = state
            .apply_combat_weapon_damage_to_target(Some(0), slot, raw, true)
            .unwrap();
        let _ = crate::input_dispatch::combat_landed_damage_result_line(
            &mut state,
            slot,
            Some(0),
            application,
        );
    }
    state
}

#[test]
fn flame_wind_rolls_damage_and_narrates_one_target_at_a_time() {
    // `magic.md §8`: Flame Wind's per-effect branch "rolls raw damage in
    // `[1, 30]` before the same damage/status path", and each accepted target
    // "reach[es] the shared result narrator after [its] effect". Since the
    // graded wound line runs the flee classifier and its morale draw
    // (`combat.md §11.1`), batching every damage roll ahead of the first
    // narration would hand the second target a different roll.
    let spell = spell_index_from_code("FHI").unwrap();

    // Draws the cast takes outside the per-target loop, measured on an empty
    // cone; the turn advance in this fixture takes none, so all of them come
    // before the loop.
    let mut probe = audit_g1_flame_wind_state(false);
    let mut ticked = probe.clone();
    ticked.advance_turn();
    assert_eq!(ticked.prng_state, probe.prng_state);
    probe.prng_state = 0x2468;
    let _ = probe.cast_directed_combat_spell(
        0,
        spell,
        CombatDirectedSpellEffect::FlameWind,
        Some(Direction::North),
    );
    let mut outside_draws = 0usize;
    let mut stepped = 0x2468u16;
    while stepped != probe.prng_state {
        stepped = u5_prng_advance_state(stepped);
        outside_draws += 1;
        assert!(outside_draws < 64);
    }

    let max_hp = combat_class_stats(COMBAT_CLASS_DRAGON).unwrap().max_hp;
    let quarter = max_hp / 4;
    let seed = (0..=u16::MAX)
        .find(|&seed| {
            let mut pre = seed;
            for _ in 0..outside_draws {
                pre = u5_prng_advance_state(pre);
            }
            let interleaved = audit_g1_flame_wind_model(pre, true);
            let batched = audit_g1_flame_wind_model(pre, false);
            let first_hp = interleaved.combat_actors[8].hp_or_wound;
            // The first target lands in the morale band, so its narration
            // draws between the two damage rolls.
            (quarter..2 * quarter).contains(&first_hp)
                && interleaved.combat_actors[9].hp_or_wound
                    != batched.combat_actors[9].hp_or_wound
        })
        .expect("some seed separates the two orders");

    let mut pre = seed;
    for _ in 0..outside_draws {
        pre = u5_prng_advance_state(pre);
    }
    let expected = audit_g1_flame_wind_model(pre, true);
    let mut state = audit_g1_flame_wind_state(true);
    state.prng_state = seed;
    assert_eq!(
        state.cast_directed_combat_spell(
            0,
            spell,
            CombatDirectedSpellEffect::FlameWind,
            Some(Direction::North),
        ),
        MoveOutcome::Cast
    );
    for slot in [8, 9] {
        assert_eq!(
            state.combat_actors[slot].hp_or_wound,
            expected.combat_actors[slot].hp_or_wound,
            "slot {slot}"
        );
        assert_eq!(
            state.combat_actors[slot].is_fleeing(),
            expected.combat_actors[slot].is_fleeing(),
            "slot {slot}"
        );
    }
    assert_eq!(state.prng_state, expected.prng_state);
}

#[test]
fn an_active_target_spell_narrates_before_the_turn_advance() {
    // `magic.md §8`: a resolved Magic Missile runs "impact presentation, a
    // newline, shared damage resolution and the shared target-result
    // narrator", and that narrator takes the morale draw (`combat.md §11.1`).
    // A day rollover's own draws must not slide in front of the morale
    // draw: the target's fate is the same whether or not the clock sits at
    // midnight. Inside a fight the cast charges no minute at all
    // (`combat.md §15`, "the only time a fight adds is its round-wrap
    // minutes"; playtest G08), so the day does not roll and no extra draw
    // follows.
    let spell = spell_index_from_code("GP").unwrap();
    let max_hp = combat_class_stats(COMBAT_CLASS_DRAGON).unwrap().max_hp;
    let run = |seed: u16, cross_midnight: bool| {
        let mut state = audit_g1_flame_wind_state(true);
        state.spell_charges[spell] = 1;
        state.combat_actors[9] = CombatActorDescriptor::empty();
        state.active_objects[9] = ActiveObject::empty();
        // Every Magic Missile roll (at most 16) leaves the Dragon in the
        // quarter-to-under-half band, where narration takes the morale draw.
        state.combat_actors[8].hp_or_wound = max_hp / 4 + 16;
        state.clock.day = 3;
        (state.clock.hour, state.clock.minute) = if cross_midnight { (23, 59) } else { (12, 0) };
        state.prng_state = seed;
        let outcome = state.cast_active_target_combat_spell(
            0,
            spell,
            CombatSpellDamageKind::MagicMissile,
            8,
        );
        (
            outcome,
            state.combat_actors[8].hp_or_wound,
            state.combat_actors[8].is_fleeing(),
            state.prng_state,
            state.clock.day,
        )
    };
    let mut saw_flee = false;
    for seed in (0..=u16::MAX).step_by(97) {
        let plain = run(seed, false);
        let crossing = run(seed, true);
        assert_eq!(plain.0, MoveOutcome::Cast);
        assert_eq!(crossing.4, 3, "a combat cast charges no clock minute");
        assert_eq!((plain.1, plain.2), (crossing.1, crossing.2), "seed {seed}");
        assert_eq!(plain.3, crossing.3, "no day-rollover draws: seed {seed}");
        saw_flee |= plain.2;
    }
    assert!(saw_flee, "some seed must take the fleeing morale result");
}

#[test]
fn party_direction_melee_takes_the_flee_classifier_at_hit_time_not_in_the_narrator() {
    // `combat.md §11.1`: the graded wound line shares the flee classifier,
    // and "a score of 1 - and a score of 2 on the morale draw described in
    // Section 9 - also raises the target's fleeing bit". The narration runs
    // inside the swing, before the committed-action tail (field contact,
    // magic-ring pass and its Regeneration roll), so the morale draw belongs
    // to the swing entry and the later transcript assembly must not take it
    // a second time (audit A042 follow-up).
    //
    // `combat.md §8.1` (R310) gives a bare direction key no bump attack, so
    // the StepOrAttack `Attack` outcome is driven through the swing entry the
    // dispatcher calls for it.
    let inputs = CombatPlayerWeaponAttackInputs {
        hit_raw_roll_0_to_60: 0,
        damage_roll: Some(1),
        forced_hit: Some(true),
    };
    let action = CombatPlayerCommandAction::StepOrAttack {
        direction_code: COMBAT_DIRECTION_EAST,
        outcome: CombatStepOrAttackPrimitiveOutcome::Attack { target_slot: 8 },
    };
    let max_hp = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().max_hp;
    let rat = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().name;

    // Score 1: the bit is already raised when the swing returns.
    let mut state = combat_player_command_state(6, 5);
    state.combat_actors[8].hp_or_wound = 2;
    let attack = state
        .apply_combat_player_weapon_attack_for_action(0, &action, inputs)
        .unwrap();
    assert_eq!(state.combat_actors[8].hp_or_wound, 1, "{attack:?}");
    assert!(
        state.combat_actors[8].is_fleeing(),
        "the classifier runs at hit time, before the post-dispatch tail"
    );
    let line =
        crate::input_dispatch::combat_weapon_attack_narrated_result_message(&mut state, 8, attack);
    assert_eq!(line, Some(format!("{rat} critical!")));

    // Score 2: the morale draw is spent by the swing; narrating the line
    // afterwards takes no further draw.
    let seed = (0..=u16::MAX)
        .find(|&seed| {
            let mut probe = seed;
            u5_prng_range_u16(&mut probe, 0, 256) >= 252
        })
        .unwrap();
    let mut state = combat_player_command_state(6, 5);
    state.combat_actors[8].hp_or_wound = max_hp / 4 + 1;
    state.prng_state = seed;
    let mut expected_prng = seed;
    let _ = u5_prng_range_u16(&mut expected_prng, 0, 256);
    let attack = state
        .apply_combat_player_weapon_attack_for_action(0, &action, inputs)
        .unwrap();
    assert_eq!(state.combat_actors[8].hp_or_wound, max_hp / 4);
    assert_eq!(state.prng_state, expected_prng, "the swing takes the morale draw");
    assert!(state.combat_actors[8].is_fleeing());
    let line =
        crate::input_dispatch::combat_weapon_attack_narrated_result_message(&mut state, 8, attack);
    assert_eq!(line, Some(format!("{rat} heavily wounded!")));
    assert_eq!(
        state.prng_state, expected_prng,
        "the narrator must not take the morale draw a second time"
    );
}

/// A two-member arena with the second member standing in the caster's
/// north cone, for the friendly-fire Sleep and Poison Wind arms.
fn p10_friendly_fire_state(spell: usize) -> PlayState {
    let mut state = combat_ai_turn_state(12, 12);
    state.party[0].status = b'G';
    state.party[0].mana = 99;
    state.party[0].level = 8;
    let mut second = state.party[0].clone();
    second.slot = 1;
    state.party.push(second);
    state.party_intelligence = vec![31, 1];
    state.party_combat_defense = vec![0; 2];
    state.spell_charges[spell] = 9;
    state.combat_actors[8] = CombatActorDescriptor::empty();
    state.active_objects[8] = ActiveObject::empty();
    state.active_objects[1] = ActiveObject {
        type_byte: 0x80,
        tile: 0x80,
        x: 5,
        y: 4,
        ..ActiveObject::empty()
    };
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 5, 4]);
    state
}

/// p10 R09. `combat.md §6.2`'s party row: In Zu on a party member writes
/// the `'S'` letter **and** the descriptor's disabled bit, which is what
/// routes the sleeper to `§8.1`'s asleep arm. The production cast wrote only
/// the letter, so the sleeper kept being prompted and never printed
/// `Zzzzz...`.
#[test]
fn in_zu_friendly_fire_sets_the_party_members_asleep_bit() {
    let spell = spell_index_from_code("IZ").unwrap();
    let mut slept = false;
    for seed in 0..256u16 {
        let mut state = p10_friendly_fire_state(spell);
        state.prng_state = seed.wrapping_mul(0x9E37);
        let _ = state.cast_directed_combat_spell(
            0,
            spell,
            CombatDirectedSpellEffect::Sleep,
            Some(Direction::North),
        );
        if state.party[1].status != b'S' {
            continue;
        }
        slept = true;
        assert!(
            state.combat_actors[1].is_status_disabled(),
            "seed {seed}: the slept member's descriptor carries bit 0x08"
        );
        break;
    }
    assert!(slept, "no seed let In Zu land on the party member");
}
