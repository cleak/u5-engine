// `text-output.md §10.4`/`§11` and `commands.md §5`: a command's own result
// continues the verb row its echo opened, and anything the end-of-turn work
// prints afterwards - here the outdoor swamp poisoning of `movement.md §8.2`
// - follows on a row of its own. The dispatcher's `handled!` arm runs that
// epilogue before it commits the echo, so the ordering has to hold however
// the handler reached the turn charge.

/// A Britannia state with every member certain to fail the outdoor poison
/// save (`movement.md §8.2`: the roll is `1..30` and a member is poisoned
/// when it exceeds the climb stat), standing on swamp when `swamp` is set.
fn swamp_poison_state(swamp: bool) -> PlayState {
    let mut grid = open_world_grid();
    if swamp {
        grid[world_cell_index(5, 5)] = BRIT_SWAMP_TILE;
    }
    let mut state = britannia_state(grid, 5, 5);
    state.party.truncate(1);
    state.party[0].status = b'G';
    state.party[0].climb_stat = 0;
    state
}

fn run_inline(key: char, swamp: bool, dir: &Path) -> PlayState {
    let mut state = swamp_poison_state(swamp);
    let turn_before = state.turn;
    assert!(
        state
            .handle_top_down_key_with_inline(key, dir, Some(Direction::North), None, None, None)
            .unwrap(),
        "{key}"
    );
    assert!(
        state.active_jimmy.is_none() && state.active_party_selector.is_none(),
        "{key}: a picker opened; {:?}",
        transcript_texts(&state)
    );
    assert_eq!(state.turn, turn_before + 1, "{key}");
    state
}

/// The rows a command prints are the same whether or not the end-of-turn
/// work prints after it; the epilogue line is only appended underneath.
fn assert_epilogue_line_only_appends(key: char, verb: &str, dir: &Path) {
    let quiet = transcript_texts(&run_inline(key, false, dir));
    let poisoned_state = run_inline(key, true, dir);
    assert_eq!(poisoned_state.party[0].status, b'P', "{key}");
    let poisoned = transcript_texts(&poisoned_state);
    assert!(quiet[0].starts_with(verb), "{key}: {quiet:?}");
    let mut expected = quiet.clone();
    expected.push(
        OUTDOOR_SWAMP_POISONED_LINE
            .trim_end_matches('\n')
            .to_string(),
    );
    assert_eq!(
        poisoned, expected,
        "{key}: the end-of-turn line follows the command's own rows"
    );
}

#[test]
fn inline_tile_commands_keep_their_result_on_the_verb_row_when_the_epilogue_prints() {
    let dir = debug_game_dir();
    // `commands.md §5.8`: Open and Jimmy results continue the echoed row.
    for (key, verb) in [('O', "Open-"), ('J', "Jimmy-")] {
        assert_epilogue_line_only_appends(key, verb, &dir);
        let rows = transcript_texts(&run_inline(key, true, &dir));
        assert!(
            rows[0].len() > verb.len(),
            "{key}: the result continues the echoed verb row: {rows:?}"
        );
    }
    // Get and Search results open with their own line feed (`commands.md
    // §5.8`), so they start under the verb row - and stay there.
    for (key, verb) in [('G', "Get-"), ('S', "Search-")] {
        assert_epilogue_line_only_appends(key, verb, &dir);
    }
}

/// The same ordering when the handler advances the turn itself, rather
/// than leaving it to the acted-exit charge: the commit used to be skipped
/// on that path, so the epilogue's flush split the result off the verb row.
#[test]
fn handler_charged_turn_keeps_the_result_on_the_verb_row_when_the_epilogue_prints() {
    let dir = debug_game_dir();
    let mut state = swamp_poison_state(true);
    let turn_before = state.turn;
    // Open the echo the dispatcher opens, then let a handler that charges
    // its own turn write a result and run the epilogue the way `handled!`
    // does, before the dispatcher's commit.
    state.begin_command_echo(top_down_command_echo('O').unwrap());
    state.advance_turn();
    state.message = "Nothing to open!".to_string();
    state
        .apply_post_turn_effects_after_outcome(turn_before, &dir, MoveOutcome::Blocked)
        .unwrap();
    state.commit_command_echo();
    state.flush_message_slot();
    assert_eq!(
        transcript_texts(&state),
        [
            "Open-Nothing to open!",
            OUTDOOR_SWAMP_POISONED_LINE.trim_end_matches('\n')
        ]
    );
}

/// `movement.md §8.2` / `time.md §5` (playtest P15-04): "Outdoor swamp
/// acquisition occurs before this upkeep pass, so a member poisoned by that
/// action's swamp save immediately takes the ordinary one-point poison hit."
/// A member already Poisoned is skipped by the save and takes only the pass's
/// one point; a Dead member takes nothing.
#[test]
fn swamp_poisoning_charges_the_same_action_poison_hit() {
    let dir = debug_game_dir();
    let mut state = swamp_poison_state(true);
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    assert!(
        state
            .handle_top_down_key_with_inline(' ', &dir, None, None, None, None)
            .unwrap()
    );
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[0].hp, 59, "{:?}", transcript_texts(&state));
    // The next action: already Poisoned, so no new save and one pass hit.
    assert!(
        state
            .handle_top_down_key_with_inline(' ', &dir, None, None, None, None)
            .unwrap()
    );
    assert_eq!(state.party[0].hp, 58);
    let _ = fs::remove_dir_all(dir);
}
