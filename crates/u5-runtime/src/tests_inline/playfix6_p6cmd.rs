// Regressions for the sixth-pass command exploratory-playtest findings
// (`p6cmd`, K04-K21).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `combat.md §8`: the arena's control codes are Escape, Ctrl-S, Ctrl-B,
/// Space, digits and directions, and "Anything else - the stock `What?`
/// refusal and a free re-prompt"; `commands.md §9`: "Combat replaces the
/// scheme entirely with its own parser". Playtest K15: Ctrl-V printed the
/// version banner and Ctrl-E opened `Exit to DOS?` mid-fight.
#[test]
fn playtest_k15_combat_refuses_the_exploration_control_bindings() {
    let game_dir = std::path::Path::new(".");
    for key in [
        PLAY_MORAL_STANDING_KEY,
        PLAY_VERSION_BANNER_KEY,
        PLAY_EXIT_TO_DOS_KEY,
    ] {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.active_player = Some(0);
        state.ensure_pending_combat_player_turn();
        let before = state.message_entries().len();

        let disposition = handle_play_key_input(&mut state, key, "", game_dir).unwrap();

        assert_eq!(disposition, PlayInputDisposition::Continue, "{key:?}");
        assert!(state.active_yes_no_prompt.is_none(), "{key:?}");
        assert!(state.combat_active, "{key:?}");
        assert_eq!(state.pending_combat_actor_slot, Some(0), "{key:?}");
        let rows: Vec<String> = state.message_entries()[before..]
            .iter()
            .map(|entry| entry.text.clone())
            .filter(|text| !text.is_empty())
            .collect();
        assert_eq!(rows, vec!["What?".to_string()], "{key:?}");
    }
}

/// `catalogs/item-list.md §7.2`: "fourteen chances in sixteen use the
/// selected colour's effect, one chance in sixteen forces the Orange sleep
/// effect, and one chance in sixteen replaces the effect with a random potion
/// id", and `prng.md §1` puts that roll on the shared range generator.
/// Playtest K21: the roll was a hash of turn, position and hour, so a Green
/// potion at one cell and time always gave `Slept!` whatever the generator
/// state.
#[test]
fn playtest_k21_potion_variation_is_drawn_from_the_shared_generator() {
    let state = combat2b_two_member_state();
    let mut counts = [0usize; POTION_COUNT];
    for seed in 1..=4096u16 {
        let mut probe = state.clone();
        probe.prng_state = seed;
        let effect = probe.potion_effect_index_after_variation_roll(POTION_GREEN_INDEX);
        counts[effect] += 1;
    }
    // Same turn, cell, clock and target for every seed: only the generator
    // varies, and the outcome must vary with it in roughly 14:1:1 shape.
    assert!(counts[POTION_GREEN_INDEX] > 4096 * 3 / 4, "{counts:?}");
    assert!(counts[POTION_ORANGE_INDEX] > 4096 / 32, "{counts:?}");
    assert!(
        counts
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != POTION_GREEN_INDEX && *index != POTION_ORANGE_INDEX)
            .any(|(_, count)| *count > 0),
        "the random-id branch reaches other colours: {counts:?}"
    );
}

/// `catalogs/item-list.md §7.2` Yellow: "Heal the selected target through the
/// shared small-heal helper" - a rolled amount, not a positional hash
/// (playtest K21).
#[test]
fn playtest_k21_yellow_potion_heal_amount_is_rolled() {
    let state = combat2b_two_member_state();
    let amounts: std::collections::BTreeSet<u16> = (1..=256u16)
        .map(|seed| {
            let mut probe = state.clone();
            probe.prng_state = seed;
            probe.potion_heal_amount()
        })
        .collect();
    assert!(amounts.len() > 4, "{amounts:?}");
}
