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
