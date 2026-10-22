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
