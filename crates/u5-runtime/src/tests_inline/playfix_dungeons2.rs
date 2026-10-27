// Regressions for the second dungeon exploratory playtest (findings
// F08..F17). Each test names the finding and the published sentence it
// rests on.

fn playfix2_transcript_rows(state: &PlayState) -> Vec<String> {
    state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// A `DUNGEON.CBT` whose every record seats one monster, in a fresh debug
/// game directory.
fn playfix2_room_game_dir() -> std::path::PathBuf {
    let dir = debug_game_dir();
    let mut record = synthetic_combat_arena_record();
    let source_base =
        DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    for offset in 0..DUNGEON_ROOM_SOURCE_COUNT {
        record[source_base + offset] = 0;
    }
    record[source_base + 2] = 0x44;
    let mut dungeon_cbt = Vec::new();
    for _ in 0..DUNGEON_CBT_RECORDS {
        dungeon_cbt.extend_from_slice(&record);
    }
    fs::write(dir.join(DUNGEON_CBT_FILE), dungeon_cbt).unwrap();
    dir
}

/// F08: `dungeon-mode.md §4` Initialisation reads the underfoot tile and
/// the underfoot reaction runs the room-entry helper on a cached `0xA`/`0xF`
/// nibble before the inner loop reads a key; `§5` says room effects "fire
/// before the player can act". A save loaded onto Doom's `0xF0` entry cell
/// enters the room at the gate, with no key pressed.
#[test]
fn playfix2_f08_a_party_arriving_on_a_room_cell_enters_it_before_any_key() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xf0;
    let mut state = dungeon_state(grid, 0, 1, 1);

    let gate = state.apply_exploration_turn_gate(Path::new("")).unwrap();

    assert!(matches!(gate, ExplorationTurnGateOutcome::Ready { .. }));
    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xa0, "the room helper ran");
    assert_eq!(state.turn, 1);
    let rows = playfix2_transcript_rows(&state);
    assert_eq!(
        rows.last().map(String::as_str),
        Some(DUNGEON_ROOM_ENTRY_NARRATION.trim_end()),
        "{rows:?}"
    );
}

/// F08: a command that leaves the party on a room cell - the surface entry
/// onto Shame's or Doom's entry cell - is followed by the same reaction, so
/// the next key is not spent as a dungeon command.
#[test]
fn playfix2_f08_a_command_ending_on_a_room_cell_enters_it_before_the_next_key() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xf0;
    let mut state = dungeon_state(grid, 0, 1, 1);
    // A dungeon arrival that left the room unresolved, as a surface entry
    // did: the room cell is underfoot and the loop is about to read a key.
    assert!(run_dungeon_entry_underfoot_reaction(&mut state, Path::new("")).unwrap());
    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xa0);

    // Nothing is owed on an ordinary floor cell.
    let mut plain = dungeon_state(open_dungeon_record(), 0, 1, 1);
    assert!(!run_dungeon_entry_underfoot_reaction(&mut plain, Path::new("")).unwrap());
    assert_eq!(plain.turn, 0);
}

/// F09: `combat.md §8.1` prints the turn banner "before any key is read",
/// and `dungeon-mode.md §8.1`'s verified rows for a room entry are
/// `Entering room...`, combat's blank row, the wrapped banner and the
/// prompt. Stepping onto a room trigger leaves the first actor's banner on
/// the screen, with that actor's turn open.
#[test]
fn playfix2_f09_room_entry_prints_the_first_turn_banner_before_any_key() {
    let dir = playfix2_room_game_dir();
    let scene = DungeonScene::new(35).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xf7;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.area = Area::Dungeon { scene, level: 0 };

    handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", &dir).unwrap();

    assert!(state.combat_active);
    assert!(
        state.pending_combat_actor_slot.is_some(),
        "the first keyboard-driven turn is open"
    );
    let rows = playfix2_transcript_rows(&state);
    let entering = rows
        .iter()
        .position(|row| row == DUNGEON_ROOM_ENTRY_NARRATION.trim_end())
        .unwrap_or_else(|| panic!("the room narration prints: {rows:?}"));
    let banner = state
        .combat_turn_banner_for_actor(state.pending_combat_actor_slot.unwrap())
        .unwrap();
    let after: String = rows[entering + 1..].join("\n");
    let first_banner_word = banner.trim_start().split(',').next().unwrap().to_string();
    assert!(
        after.contains(&first_banner_word),
        "the banner follows `Entering room...`: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// A lit three-member dungeon party, everyone able to act.
fn playfix2_lit_party_state() -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"SHAMINO\0\0"];
    state.torch_counter = 5;
    state
}

/// F10: `inventory.md §4.3` - "**A digit moves the indicator; it does not
/// commit.** ... Only Return or Space commits the indicated row, Escape
/// cancels, and `0` commits the explicit 'no one' answer only in the
/// callers that allow it ... This is one shared routine, so the rule is the
/// same for ... Search and every other caller." `dungeon-mode.md §8.1` puts
/// dungeon Look and Search on that shared prompt.
#[test]
fn playfix2_f10_dungeon_look_and_search_pickers_move_on_digits_and_commit_on_return() {
    for verb in ['l', 'S'] {
        let mut state = playfix2_lit_party_state();
        handle_play_key_input(&mut state, verb, "", Path::new("")).unwrap();
        assert!(state.active_party_selector.is_some(), "{verb}: the picker opens");

        for key in ['2', '0'] {
            handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
            assert!(
                state.active_party_selector.is_some(),
                "{verb}: `{key}` does not commit"
            );
            assert!(state.active_direction_prompt.is_none());
        }
        // `2` moved the indicator to the second row; a direction key moves
        // it on to the third, and Return commits that row.
        handle_play_key_input(&mut state, char::from(INPUT_CODE_SOUTH), "", Path::new(""))
            .unwrap();
        assert_eq!(state.selector_highlight(), Some(2), "{verb}");
        handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();

        assert!(state.active_party_selector.is_none(), "{verb}");
        let rows = playfix2_transcript_rows(&state);
        assert!(rows.iter().any(|row| row == "Player: SHAMINO"), "{verb}: {rows:?}");
        let party_index = state
            .active_direction_prompt
            .map(|session| match session.kind {
                DirectionPromptKind::DungeonLook { party_index, .. }
                | DirectionPromptKind::DungeonSearch { party_index } => party_index,
                other => panic!("{verb}: unexpected prompt {other:?}"),
            });
        assert_eq!(party_index, Some(Some(2)), "{verb}: the `Dir-` chooser opens");
    }
    // The chest sites' picker is the same routine.
    assert_eq!(acting_member_picker_key('2', 0, 3), ActingMemberPickerKey::Move(1));
}

/// F11, F12: `dungeon-mode.md §8.1` "Who acts" - "a cancel ... answers
/// `None!\n` and the command aborts"; `commands.md §5.8` - "Escape prints
/// `None!\n` on the prompt row". Search ignored Escape, and Look put the
/// answer on a row of its own.
#[test]
fn playfix2_f11_f12_escape_cancels_dungeon_look_and_search_on_the_prompt_row() {
    for verb in ['l', 'S'] {
        let mut state = playfix2_lit_party_state();
        handle_play_key_input(&mut state, verb, "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();

        assert!(state.active_party_selector.is_none(), "{verb}");
        assert!(state.active_direction_prompt.is_none(), "{verb}");
        let rows = playfix2_transcript_rows(&state);
        assert_eq!(
            rows.last().map(String::as_str),
            Some("Player: None!"),
            "{verb}: {rows:?}"
        );
    }
}

/// F13: `dungeon-mode.md §8.1` - "A pick whose status is neither Good nor
/// Poisoned answers `Disabled!\n\n` and re-prompts from `Player: `"; and
/// while the picker waits the question is the line being edited
/// (`inventory.md §4.3`), not a fresh command row under it.
#[test]
fn playfix2_f13_dungeon_jimmy_picker_keeps_its_prompt_open_and_reprompts() {
    let mut state = playfix2_lit_party_state();
    state.party[1].status = b'D';
    state.keys = 1;

    handle_play_key_input(&mut state, 'J', "", Path::new("")).unwrap();
    assert!(state.active_jimmy.is_some());
    let open = state.open_prompt_line().expect("the member prompt is open");
    assert!(open.ends_with(PARTY_SELECTION_PROMPT), "{open:?}");

    handle_play_key_input(&mut state, '2', "\r", Path::new("")).unwrap();
    assert!(state.active_jimmy.is_some(), "a disabled pick re-prompts");
    let rows = playfix2_transcript_rows(&state);
    let disabled = rows
        .iter()
        .rposition(|row| row == DUNGEON_ACTING_MEMBER_DISABLED.trim_end())
        .unwrap_or_else(|| panic!("`Disabled!` prints: {rows:?}"));
    assert_eq!(rows[disabled + 1..], ["", PARTY_SELECTION_PROMPT], "{rows:?}");
    assert_eq!(state.open_prompt_line().as_deref(), Some(PARTY_SELECTION_PROMPT));

    handle_play_key_input(&mut state, '1', "\r", Path::new("")).unwrap();
    assert!(state.active_jimmy.is_none());
    let rows = playfix2_transcript_rows(&state);
    assert!(rows.iter().any(|row| row == "Player: AVATAR"), "{rows:?}");
}

/// F13 follow-up: `commands.md §5.8` - "An ineligible pick prints
/// `Disabled!\n\n` and repeats the prompt; Escape prints `None!\n` on the
/// prompt row and aborts." After the re-prompt that row is a bare
/// `Player: `, so the cancel reads `Player: None!` on one row, as it already
/// does at the shared Look and Search picker.
#[test]
fn playfix2_f13_escape_after_a_disabled_reprompt_answers_on_the_player_row() {
    let mut state = playfix2_lit_party_state();
    state.party[1].status = b'D';
    state.keys = 1;

    handle_play_key_input(&mut state, 'J', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '2', "\r", Path::new("")).unwrap();
    assert!(state.active_jimmy.is_some(), "a disabled pick re-prompts");

    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    assert!(state.active_jimmy.is_none());
    let rows = playfix2_transcript_rows(&state);
    let disabled = rows
        .iter()
        .rposition(|row| row == DUNGEON_ACTING_MEMBER_DISABLED.trim_end())
        .unwrap_or_else(|| panic!("`Disabled!` prints: {rows:?}"));
    assert_eq!(rows[disabled + 1..], ["", "Player: None!"], "{rows:?}");
    assert!(!state.message_row_open_mid_line, "{rows:?}");
}

/// F17: `dungeon-mode.md §12` - each Look class line ends "in one line
/// feed", so a Look leaves the row closed; `§13.2`'s line-feed-led
/// `Exit to ` then gets its blank row. A multi-member Look left the row
/// marked open from its `Dir-` prompt.
#[test]
fn playfix2_f17_a_party_look_closes_its_row() {
    let mut state = playfix2_lit_party_state();
    handle_play_key_input(&mut state, 'l', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert!(state.active_direction_prompt.is_some());
    handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", Path::new("")).unwrap();

    let rows = playfix2_transcript_rows(&state);
    assert!(rows.iter().any(|row| row == "You see:"), "{rows:?}");
    assert!(!state.message_row_open_mid_line, "{rows:?}");
}

/// F15: `dungeon-mode.md §8.1` - "Search, unlit | `\nYou find:\ndarkness.\n`
/// - a *find* line, with a leading blank row", and `§12` ends every `Dir-`
/// completion with its own line feed. Whether a member prompt ran first
/// does not change the blank row.
#[test]
fn playfix2_f15_unlit_party_search_keeps_the_blank_row_before_you_find() {
    let rows_after_dir = |party: bool| {
        let mut state = playfix2_lit_party_state();
        state.torch_counter = 0;
        state.light_spell_counter = 0;
        if !party {
            state.party.truncate(1);
        }
        handle_play_key_input(&mut state, 'S', "", Path::new("")).unwrap();
        if party {
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
        }
        handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", Path::new(""))
            .unwrap();
        let rows = playfix2_transcript_rows(&state);
        let dir = rows
            .iter()
            .rposition(|row| row.starts_with("Dir-"))
            .unwrap_or_else(|| panic!("the chooser row: {rows:?}"));
        rows[dir + 1..].to_vec()
    };
    let solo = rows_after_dir(false);
    assert_eq!(solo[..3], ["", "You find:", "darkness."], "{solo:?}");
    assert_eq!(rows_after_dir(true), solo);
}

/// P13-06: `dungeon-mode.md §13.4` - "Losing a fight in a dungeon room
/// reaches the same place indirectly: combat returns to the dungeon loop
/// that framed it, and the loop's next capability check sees the result".
/// That check (`§4` step 7) ends the loop when nobody can act, so the party
/// still standing on the room cell is not run through the room-entry helper
/// a second time, and no second `Entering room...` prints.
#[test]
fn playtest_p13_06_a_lost_room_fight_does_not_re_enter_the_room() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xa0;
    let mut state = dungeon_state(grid, 0, 1, 1);
    for member in &mut state.party {
        member.status = b'D';
        member.hp = 0;
    }
    let rows_before = playfix2_transcript_rows(&state).len();

    assert!(!run_dungeon_entry_underfoot_reaction(&mut state, Path::new("")).unwrap());

    assert!(!state.combat_active);
    assert_eq!(state.turn, 0);
    assert_eq!(playfix2_transcript_rows(&state).len(), rows_before);
}
