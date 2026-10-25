// Regressions for the pass-9 exploratory-playtest findings (p9combat).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `audio.md §6.1` "Cast rejection sounds": "a spell rejected by the
/// castability mask prints `Not here!\n` and plays the 50-update
/// cast-failure glissando before any resource debit, with no additional
/// Failed line" (`magic.md §7` result table). The dispatcher's scene gate
/// printed the line and played nothing (Q17).
#[test]
fn playtest_q17_scene_gate_not_here_plays_the_failure_glissando() {
    let mut state = test_state(open_grid(), 5, 5);
    state.spell_charges[UUS_POR_SPELL_INDEX] = 1;
    state.party[0].mana = 4;
    state.party[0].level = 4;
    let serial = state.sound_effect_serial;

    handle_play_key_input(&mut state, 'C', "1PU", Path::new("")).unwrap();

    assert_eq!(state.message, "Not here!");
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::CastFailure]
    );
    assert_eq!(state.spell_charges[UUS_POR_SPELL_INDEX], 1, "nothing spent");
    assert_eq!(state.party[0].mana, 4);
}

/// `combat.md §8.2`: a member with no qualifying item "makes a single
/// bare-handed attempt, which behaves as melee with range one", and the
/// `§11.1` census row "Swing begins | party melee" plays the rising sweep
/// "unconditionally, before the roll" (`audio.md §7.4`). Bare-handed attacks
/// were silent; only an armed melee attack swung (Q18).
#[test]
fn playtest_q18_bare_handed_attack_plays_the_party_melee_swing() {
    for forced_hit in [true, false] {
        let mut state = combat_ai_turn_state(6, 5);
        let serial = state.sound_effect_serial;
        let application = state
            .resolve_and_apply_combat_bare_handed_attack(0, 8, 30, 10, 0, Some(forced_hit))
            .expect("an adjacent bare-handed attempt resolves");
        assert!(!matches!(
            application.resolution,
            CombatWeaponAttackResolution::OutOfRange { .. }
        ));
        assert_eq!(
            state.sound_effects_after(serial).first(),
            Some(&SoundEffect::PartyMeleeAttackSwing),
            "the swing precedes the to-hit branch (forced_hit {forced_hit})"
        );
    }

    // Out of reach the attempt reaches no attack application and stays
    // silent, exactly like the equipment path's `OutOfRange` arm.
    let mut far = combat_ai_turn_state(9, 5);
    let serial = far.sound_effect_serial;
    let application = far
        .resolve_and_apply_combat_bare_handed_attack(0, 8, 30, 10, 0, Some(true))
        .expect("the attempt resolves");
    assert!(matches!(
        application.resolution,
        CombatWeaponAttackResolution::OutOfRange { .. }
    ));
    assert!(far.sound_effects_after(serial).is_empty());
}

/// `combat.md §7` restarts the table walk at slot 32 while actors remain, and
/// `§8.1`'s dragged-under arm ends each held turn "with no command read", so
/// the walk carries on until a keyboard-driven actor actually needs a key.
/// The driver's non-termination guard counted every table walk, so a victim
/// held for a few turns exhausted it and the walk stopped with no prompt
/// open - the round's remaining output (a last foe's `escapes!` and
/// `VICTORY!`) then printed only after the next key (Q02).
#[test]
fn playtest_q02_dragged_under_turns_do_not_stall_the_round_walk() {
    let mut longest_hold = 0;
    for seed in 0..64u16 {
        let mut state = worked_bat_arena(&[], 0);
        state.prng_state = seed.wrapping_mul(2_654).wrapping_add(1);
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_DRAGGED_UNDER;
        state.active_objects[0].tile = 0;

        open_owed_combat_player_turn(&mut state);

        let held_turns = state
            .message_entries()
            .iter()
            .filter(|entry| entry.text.starts_with(COMBAT_DRAGGED_UNDER_TURN_LINE))
            .count();
        longest_hold = longest_hold.max(held_turns);
        assert!(
            !state.combat_actors[0].is_dragged_under(),
            "seed {seed}: the walk stopped while the victim was still held \
             after {held_turns} ARGH! turns"
        );
        assert_eq!(
            state.pending_combat_actor_slot,
            Some(0),
            "seed {seed}: the released victim's prompt is open"
        );
    }
    assert!(
        longest_hold >= 3,
        "some seed must hold the victim past the old thirty-seven-walk guard"
    );
}
