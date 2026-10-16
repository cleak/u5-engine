// `magic.md §8` (`RETRACTIONS.md` R446): Kill has no protected-class
// rejection - "Classes 14, 15 and 47 can reach Kill's shared damage endpoint
// after an admitted hit; this does not remove their existing class-specific
// death behavior." For the Shadow Lord that behaviour is `combat.md §6.3`'s
// vanish-on-death branch, whose `0x02` result bit makes the shared narrator
// skip the generic `killed!` line.

#[test]
fn an_admitted_kill_on_the_shadow_lord_vanishes_it_without_a_killed_line() {
    let spell_index = spell_index_from_code("CX").unwrap();
    let mana_cost = (spell_index / 6 + 1) as u8;
    let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.party[0].mana = mana_cost;
    state.party[0].level = mana_cost;
    state.spell_charges[spell_index] = 1;
    // A caster rating that makes `combat.md §11`'s hit check certain.
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([99, 99, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5]);
    // `combat.md §6.1`: a placed monster carries the hostile tag, never the
    // party-side bit, and owns its linked active-object record.
    let stats = combat_class_stats(COMBAT_CLASS_SHADOW_LORD).unwrap();
    state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
        stats,
        target_slot as u8,
        5,
        5,
        combat_monster_placement_flags(COMBAT_CLASS_SHADOW_LORD),
        0,
    );
    if state.active_objects.len() <= target_slot {
        state
            .active_objects
            .resize(target_slot + 1, ActiveObject::empty());
    }
    state.active_objects[target_slot] =
        summoned_active_object_record(COMBAT_CLASS_SHADOW_LORD, 5, 5, 0).unwrap();

    let _ = state.cast_active_target_combat_spell(
        0,
        spell_index,
        CombatSpellDamageKind::Kill,
        target_slot,
    );

    let rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    assert!(
        rows.iter().any(|row| row == "Shadow Lord vanishes!"),
        "the vanish branch narrates the death: {rows:?} message {:?} actor {:?}",
        state.message,
        state.combat_actors[target_slot]
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.contains("killed!") || row.contains("Failed!")),
        "the narrator skips the generic chain after a vanish: {rows:?}"
    );
    assert!(!state.message.contains("killed!"), "{:?}", state.message);
    // `§6.3` step 5 releases the descriptor; "clears `0x02` in cleanup".
    assert!(state.combat_actors[target_slot].is_free_for_allocation());
    assert_eq!(
        state.combat_action_result & COMBAT_ACTION_RESULT_VANISH_NARRATED,
        0
    );
}
