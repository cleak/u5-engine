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
