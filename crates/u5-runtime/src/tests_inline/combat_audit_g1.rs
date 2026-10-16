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
