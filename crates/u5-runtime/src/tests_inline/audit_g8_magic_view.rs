// Conformance tests for the 2026-10-16 audit, cluster g8: `systems/magic.md`,
// `catalogs/spell-list.md`, `systems/view.md` and `systems/visibility.md`.

fn g8_heal_caster_state(mana: u8) -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party = vec![
        PartyMember {
            slot: 0,
            class_byte: b'M',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana,
            hp: 20,
            max_hp: 20,
            level: HEAL_COST,
        },
        PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 3,
            max_hp: 20,
            level: 1,
        },
    ];
    state.spell_charges[HEAL_SPELL_INDEX] = 1;
    state.active_player = Some(0);
    state
}

#[test]
fn audit_a118_cancelled_heal_prompt_keeps_the_spent_charge_and_mana() {
    // `magic.md §5` (issue #277): "Measured: with exactly one Heal charge,
    // Escape at `On who: ` and a second cast gives `None mixed!`."
    let mut state = g8_heal_caster_state(HEAL_COST);
    assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
    assert!(state.step_active_cast('M', "", Path::new("")).unwrap().is_none());
    assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_none());
    assert_eq!(state.message, USE_POTION_TARGET_PROMPT);
    assert_eq!(state.spell_charges[HEAL_SPELL_INDEX], 0);
    assert_eq!(state.party[0].mana, 0);

    assert!(state
        .step_active_cast_followup('\u{1b}', "", Path::new(""))
        .unwrap()
        .is_none());
    assert!(state.active_cast_followup.is_none());
    assert_eq!(state.spell_charges[HEAL_SPELL_INDEX], 0);
    assert_eq!(state.party[0].mana, 0);
    assert_eq!(state.party[1].hp, 3);

    assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
    assert!(state.step_active_cast('M', "", Path::new("")).unwrap().is_none());
    assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_some());
    assert!(state.active_cast_followup.is_none());
    assert!(state.message.contains("None mixed!"), "{:?}", state.message);
}

#[test]
fn audit_a118_mana_refusal_precedes_the_handler_prompt() {
    // `magic.md §5`: "No later gate can fail after a prompt opens" - the
    // mana gate's `M.P. too low!` / `Failed!` comes before `On who: `.
    let mut state = g8_heal_caster_state(0);
    assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
    assert!(state.step_active_cast('M', "", Path::new("")).unwrap().is_none());
    assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_some());
    assert!(state.active_cast_followup.is_none());
    assert_eq!(state.message, "M.P. too low!\nFailed!\n");
    assert_eq!(state.spell_charges[HEAL_SPELL_INDEX], 0);
}
