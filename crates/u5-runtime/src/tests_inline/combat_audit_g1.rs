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
