// Regressions for the seventeenth exploratory playtest's combat findings
// (P17-12, P17-13). Each test names the finding and the published sentence
// it rests on.

/// `magic.md §8` creature-prompt targeters: Polymorph has only the
/// protected-class and resistance gates, and Clone only the table-capacity
/// failure. Neither refuses a sleeper; the engine rejected the asleep bit
/// `0x08` and the cast ended silently (playtest P17-12).
#[test]
fn playtest_p17_12_polymorph_and_clone_accept_a_sleeping_target() {
    let polymorph = spell_index_from_code("BRX").unwrap();
    let mut state = spells_combat_audio_state(polymorph);
    state.party_intelligence = vec![u8::MAX];
    let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_DAEMON);
    state.combat_actors[target].flags |= COMBAT_ACTOR_FLAG_STATUS_DISABLED;

    assert_eq!(
        state.cast_combat_polymorph_spell(0, polymorph, target),
        MoveOutcome::Cast
    );
    assert_eq!(state.message, crate::commands::SPELL_SUCCESS_LINE);
    assert_eq!(
        state.combat_actors[target].owner_target_class,
        COMBAT_CLASS_GIANT_RAT
    );

    let clone = spell_index_from_code("IQX").unwrap();
    let mut state = spells_combat_audio_state(clone);
    let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
    state.combat_actors[target].flags |= COMBAT_ACTOR_FLAG_STATUS_DISABLED;
    let occupied_before = state.combat_actors.iter().filter(|a| !a.is_empty()).count();

    assert_eq!(
        state.cast_combat_clone_spell(0, clone, target),
        MoveOutcome::Cast
    );
    assert_eq!(state.message, crate::commands::SPELL_SUCCESS_LINE);
    assert_eq!(
        state.combat_actors.iter().filter(|a| !a.is_empty()).count(),
        occupied_before + 1,
        "the copy is placed"
    );
}

/// `combat.md §11` (field dissolution endpoint) and `§12` "Active-effect
/// display counter": "On every skipped turn while an active player is
/// selected ... that denial enters the same endpoint directly", whose step 3
/// is "The timed-magic counter ageing". With the Avatar selected, the second
/// member's denied turn ages Protection by one, as a committed action would
/// (playtest P17-11; fixed by the u5-spec#308 base, pinned here).
#[test]
fn playtest_p17_11_active_player_denied_turn_ages_the_timed_effect_counter() {
    let mut state = combat2b_two_member_state();
    state.pending_combat_actor_slot = None;
    state.active_player = Some(0);
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 20;
    // Slot 1's phase counter is due, so its turn reaches the handler gate.
    state.combat_actors[1].phase_counter = 0;

    let walk = state.apply_combat_round_walk_from_slot(1, COMBAT_PHASE_REFRESH_CONSTANT);

    assert!(walk.applications.iter().any(|application| matches!(
        application,
        CombatActorSlotDispatchApplication::Slot {
            action: CombatActorDispatchAction::ActivePlayerSentinelSkipped,
            ..
        }
    )));
    assert_eq!(state.active_effect_tag, Some(PROTECTION_ACTIVE_EFFECT_TAG));
    assert_eq!(
        state.active_effect_counter, 19,
        "one denied turn, one ageing"
    );
}
