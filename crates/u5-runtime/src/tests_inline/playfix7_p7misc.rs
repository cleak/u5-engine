// Regressions for the seventh-pass miscellaneous exploratory-playtest
// findings (`p7misc`, M1-M5 and P27).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

fn p7misc_cancel_direction_prompt(state: &mut PlayState, letter: char) -> u64 {
    let dir = Path::new("");
    let turn_before = state.turn;
    handle_play_key_input(state, letter, "", dir).unwrap();
    assert!(
        state.active_direction_prompt.is_some(),
        "{letter}: the direction prompt is open"
    );
    assert_eq!(
        state.turn, turn_before,
        "{letter}: the prompt itself is free"
    );
    handle_play_key_input(state, ' ', "", dir).unwrap();
    assert!(
        state.active_direction_prompt.is_none(),
        "{letter}: Space cancels"
    );
    state.turn - turn_before
}

/// M1: `commands.md §3` - "`L` Look also retains the default acted result
/// in overworld, town and dungeon routes, including cancellation inside the
/// Look handler" - and `town-mode.md §10`: "Look consumes one ordinary town
/// action, including when its direction prompt is cancelled." `Look-Pass`
/// left the turn and clock alone in town and on the overworld.
#[test]
fn playtest_m1_a_cancelled_look_costs_its_turn() {
    let mut town = test_state(open_grid(), 1, 1);
    assert_eq!(p7misc_cancel_direction_prompt(&mut town, 'L'), 1, "town");

    let mut world = world_state(open_world_grid(), 5, 5);
    assert_eq!(
        p7misc_cancel_direction_prompt(&mut world, 'L'),
        1,
        "overworld"
    );
}

/// M2: `commands.md §3` - "Most letters discard whatever their handler
/// returned and report the default 'acted'. Exactly six routes forward the
/// handler's own value" - `T` is not one of them, and the town `A` row reads
/// "'Acted', except the on-foot refusal". A cancelled Talk (town and
/// overworld) and a cancelled town Attack were free. Overworld `A` is
/// "Always 'no action'" and stays free.
#[test]
fn playtest_m2_cancelled_talk_and_town_attack_cost_their_turn() {
    let mut town = test_state(open_grid(), 1, 1);
    assert_eq!(
        p7misc_cancel_direction_prompt(&mut town, 'T'),
        1,
        "town Talk"
    );

    let mut town = test_state(open_grid(), 1, 1);
    assert_eq!(
        p7misc_cancel_direction_prompt(&mut town, 'A'),
        1,
        "town Attack"
    );

    let mut world = world_state(open_world_grid(), 5, 5);
    assert_eq!(
        p7misc_cancel_direction_prompt(&mut world, 'T'),
        1,
        "overworld Talk"
    );

    let mut world = world_state(open_world_grid(), 5, 5);
    assert_eq!(
        p7misc_cancel_direction_prompt(&mut world, 'A'),
        0,
        "overworld Attack is always 'no action'"
    );
}

/// M4: `magic.md §6` step 6 prints `Mixing...` (with its line feed), and
/// step 7's wrong-mix branch "then emits a line break ... and invokes the
/// shared trap-effect resolver". That break leaves one blank row between
/// `Mixing...` and the trap word - the spacing `Done!` gets on success.
#[test]
fn playtest_m4_a_wrong_mix_leaves_a_blank_row_before_the_trap_word() {
    for suffix in ["AS/0x80/1", "IL/0x41/1"] {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [5; REAGENT_COUNT];
        assert_eq!(state.mix_reagents_from_suffix(suffix), MoveOutcome::Blocked);
        state.flush_message_slot();
        let rows: Vec<String> = state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect();
        let mixing = rows
            .iter()
            .position(|row| row == MMIX_MIXING_MESSAGE)
            .unwrap_or_else(|| panic!("{suffix}: {rows:?}"));
        assert_eq!(rows[mixing + 1], "", "{suffix}: one blank row: {rows:?}");
        assert!(
            matches!(
                rows[mixing + 2].as_str(),
                "ACID!" | "POISON!" | "BOMB!" | "GAS!"
            ),
            "{suffix}: then the trap word: {rows:?}"
        );
    }
}

/// M5: `input.md §2` step 3 - an extended key that does not classify "is
/// returned as 'no key'", and `§2`'s byte table: "the engine treats them as
/// 'no key' and continues polling". Nothing is printed; the engine wrote the
/// invented sentence `Input ignored.`.
#[test]
fn playtest_m5_an_unclassified_key_prints_nothing() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.message = "Previous line".to_string();
    let rows_before = state.message_transcript.len();
    let turn_before = state.turn;

    handle_play_key_input(&mut state, PLAY_IGNORED_INPUT_KEY, "escape", Path::new("")).unwrap();

    assert_eq!(state.turn, turn_before);
    assert_ne!(state.message, "Input ignored.");
    assert!(
        state.message_transcript[rows_before..]
            .iter()
            .all(|entry| entry.text != "Input ignored." && entry.text != "Function key ignored."),
        "no invented line is printed"
    );
}

