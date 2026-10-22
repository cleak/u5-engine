// Regressions for the magic exploratory play-test findings (G05..G12).
// Each test names the finding it reproduces and the published sentence it
// rests on.

fn p3magic_transcript_rows(state: &PlayState) -> Vec<String> {
    state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

fn p3magic_three_member_world_state() -> PlayState {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0", *b"IOLO\0\0\0\0\0"];
    state.active_player = None;
    state
}

/// G05: `commands.md §5.8` - "With two or more, it prints `Player: ` and
/// opens the roster picker; a confirmed eligible pick completes that row
/// with the member's name. An ineligible pick prints `Disabled!\n\n` and
/// repeats the prompt", and "The direction form of Search always invokes
/// this acting-member selection before searching. Cast does too."
/// Eligible is Good or Poisoned (`traps.md §2.1`).
#[test]
fn playfix3_g05_cast_and_search_pickers_refuse_a_dead_or_sleeping_member() {
    for (status, target) in [
        (b'D', PartySelectorTarget::Cast),
        (b'S', PartySelectorTarget::Cast),
        (b'D', PartySelectorTarget::Search { direction: Direction::North }),
        (b'S', PartySelectorTarget::Search { direction: Direction::North }),
    ] {
        let mut state = p3magic_three_member_world_state();
        state.party[1].status = status;
        assert_eq!(state.start_party_selector(target), MoveOutcome::Observed);

        // Move to Shamino and commit.
        assert!(state.step_active_party_selector('2', ""));
        assert!(state.step_active_party_selector('\r', ""));
        assert!(
            state.active_party_selector.is_some(),
            "{target:?}/{}: the prompt repeats",
            status as char
        );
        assert!(state.active_cast.is_none(), "{target:?}: no cast opens");
        let rows = p3magic_transcript_rows(&state);
        assert!(
            !rows.iter().any(|row| row.contains("SHAMINO")),
            "{target:?}: the ineligible name is not echoed: {rows:?}"
        );
        let disabled = rows
            .iter()
            .rposition(|row| row == DUNGEON_ACTING_MEMBER_DISABLED.trim_end())
            .unwrap_or_else(|| panic!("{target:?}: `Disabled!` prints: {rows:?}"));
        assert_eq!(rows[disabled + 1], "", "{target:?}: {rows:?}");
        assert_eq!(state.message, PARTY_SELECTION_PROMPT);
        assert_eq!(state.turn, 0);

        // An eligible pick then proceeds as before.
        assert!(state.step_active_party_selector('3', ""));
        assert!(state.step_active_party_selector('\r', ""));
        assert!(state.active_party_selector.is_none());
        if target == PartySelectorTarget::Cast {
            assert_eq!(state.active_cast.as_ref().map(|cast| cast.caster_index), Some(2));
        }
    }
}

/// G08: `combat.md §15` - "Combat advances time at one specific point -
/// when the round counter wraps inside a round ... The wrap fires the
/// per-turn cleanup with a one-minute increment"; `§4`: "the only time a
/// fight adds is its round-wrap minutes". A combat cast spent the
/// overworld's two minutes and a turn.
#[test]
fn playfix3_g08_a_combat_cast_charges_no_clock_minutes_or_turn() {
    let spell = spell_index_from_code("IS").unwrap();
    for combat in [false, true] {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.combat_active = combat;
        state.party[0].mana = 50;
        state.party[0].level = 8;
        state.spell_charges[spell] = 1;
        state.active_player = Some(0);
        state.clock = GameClock::new(13, 0).unwrap();

        handle_play_key_input(&mut state, 'C', "1IS", Path::new("")).unwrap();
        assert_eq!(state.spell_charges[spell], 0, "combat {combat}: the cast resolved");
        let elapsed = (state.clock.hour, state.clock.minute, state.turn);
        if combat {
            assert_eq!(elapsed, (13, 0, 0), "a fight charges no per-cast time");
        } else {
            assert_eq!(elapsed, (13, 2, 1), "the overworld turn is two minutes");
        }
    }
}

/// G06: `magic.md §6` step 6 - the handler "prints `Mixing...`" once, and
/// step 7's wrong-mix branch "then emits a line break ... and invokes the
/// shared trap-effect resolver". Nothing prints `Mixing...` a second time.
#[test]
fn playfix3_g06_a_wrong_recipe_prints_the_mixing_beat_once() {
    for suffix in ["AS/0x80/1", "IL/0x41/1"] {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [5; REAGENT_COUNT];
        assert_eq!(state.mix_reagents_from_suffix(suffix), MoveOutcome::Blocked);
        state.flush_message_slot();
        let rows = p3magic_transcript_rows(&state);
        let mixing: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| *row == MMIX_MIXING_MESSAGE)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(mixing.len(), 1, "{suffix}: {rows:?}");
        assert!(
            matches!(
                rows[mixing[0] + 1].as_str(),
                "ACID!" | "POISON!" | "BOMB!" | "GAS!"
            ),
            "{suffix}: the trap word follows directly: {rows:?}"
        );
    }
}
