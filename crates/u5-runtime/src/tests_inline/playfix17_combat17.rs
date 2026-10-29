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
