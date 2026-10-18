// Regressions for the dungeon exploratory playtest (findings E08..E20).
// Each test names the finding and the published sentence it rests on.

fn playtest_transcript_rows(state: &PlayState) -> Vec<String> {
    state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// E08: `dungeon-mode.md §13.1` - "A K that applies something - a climb
/// up, a climb down ... - reports \"acted\" to the dungeon loop, so the
/// turn's post-action pass runs", and `§8.1` lists the fall trap among the
/// post-action underfoot consequences. A climb down onto a fall trap falls
/// at once, not after the next command.
#[test]
fn playtest_e08_klimb_down_onto_a_fall_trap_falls_before_the_next_key() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(2, 1, 1)] = 0x20;
    grid[dungeon_cell_index(3, 1, 1)] = 0x69;
    let mut state = dungeon_state(grid, 2, 1, 1);

    handle_play_key_input(&mut state, 'k', "", Path::new("")).unwrap();

    assert_eq!(state.area, Area::Dungeon { scene, level: 4 });
    let rows = playtest_transcript_rows(&state);
    let down = rows
        .iter()
        .position(|row| row.ends_with("Down!"))
        .unwrap_or_else(|| panic!("the climb word prints: {rows:?}"));
    assert_eq!(
        rows[down + 1..]
            .iter()
            .filter(|row| !row.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            DUNGEON_PIT_TRAP_LINE.trim_end(),
            DUNGEON_FALLING_LINE.trim_end(),
            DUNGEON_SPLAT_LINE.trim_end()
        ],
        "{rows:?}"
    );
}

/// E08: the same pass enters a room the climb lands on (`§8.1`, "Room
/// trigger or room-helper cell | `Entering room...`"), on the one-direction
/// route and on the up-or-down prompt's answer alike.
#[test]
fn playtest_e08_klimb_onto_a_room_cell_enters_it_at_once() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(2, 1, 1)] = 0x10;
    grid[dungeon_cell_index(1, 1, 1)] = 0xF2;
    let mut up = dungeon_state(grid.clone(), 2, 1, 1);
    handle_play_key_input(&mut up, 'k', "", Path::new("")).unwrap();
    assert_eq!(up.area, Area::Dungeon { scene, level: 1 });
    assert_eq!(up.message, DUNGEON_ROOM_ENTRY_NARRATION);
    assert_eq!(up.dungeon_cell(1, 1, 1), 0xA2, "the room helper ran");

    grid[dungeon_cell_index(2, 1, 1)] = 0x30;
    let mut two_way = dungeon_state(grid, 2, 1, 1);
    handle_play_key_input(&mut two_way, 'k', "", Path::new("")).unwrap();
    handle_play_key_input(&mut two_way, '<', "", Path::new("")).unwrap();
    assert_eq!(two_way.area, Area::Dungeon { scene, level: 1 });
    assert_eq!(two_way.message, DUNGEON_ROOM_ENTRY_NARRATION);
    assert_eq!(two_way.dungeon_cell(1, 1, 1), 0xA2, "the room helper ran");
}

/// E09: `dungeon-mode.md §13.1` - "Up or Down spell, destination refused |
/// the climb word (already printed), then `Failed!`", and an ordinary level
/// change prints "the same `Up!` or `Down!`, and only that". The word is
/// printed once by the shared apply routine.
#[test]
fn playtest_e09_level_change_spell_prints_the_climb_word_once() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(1, 1, 1)] = 0xC0;
    let mut refused = dungeon_state(grid, 0, 1, 1);
    arm_spell(&mut refused, DES_POR_SPELL_INDEX);
    handle_play_key_input(&mut refused, 'C', "1DP", Path::new("")).unwrap();
    assert_eq!(refused.area, Area::Dungeon { scene, level: 0 });
    let rows = playtest_transcript_rows(&refused);
    let down = rows
        .iter()
        .position(|row| row == "Down!")
        .expect("climb word");
    assert_eq!(rows[down..], ["Down!", "Failed!"], "{rows:?}");

    let mut accepted = dungeon_state(open_dungeon_record(), 0, 1, 1);
    arm_spell(&mut accepted, DES_POR_SPELL_INDEX);
    handle_play_key_input(&mut accepted, 'C', "1DP", Path::new("")).unwrap();
    assert_eq!(accepted.area, Area::Dungeon { scene, level: 1 });
    let rows = playtest_transcript_rows(&accepted);
    assert_eq!(
        rows.iter().filter(|row| *row == "Down!").count(),
        1,
        "{rows:?}"
    );
    assert!(!accepted.message_slot_needs_flush());
}

/// E10: `dungeon-mode.md §11` - "The rest concludes either with `Party
/// rested!\n` or `Ambushed!\n\n` (and an immediate combat)". An interrupted
/// dungeon camp prints the ambush line alone, once, with no leading space.
#[test]
fn playtest_e10_dungeon_camp_ambush_prints_only_the_ambush_line() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.clock = GameClock::new(8, 55).unwrap();
    // The first hour rollover's 1-in-64 interruption roll comes up zero.
    state.prng_state = (0..=u16::MAX)
        .find(|seed| {
            let mut scratch = *seed;
            u5_prng_range_u16(&mut scratch, 0, 63) == 0
        })
        .expect("some seed interrupts");

    let _ = state.hole_up_command(
        &dir,
        InlineRestRequest {
            hours: Some(9),
            watcher: None,
        },
    );

    let rows = playtest_transcript_rows(&state);
    let sleep = rows
        .iter()
        .position(|row| row == CAMP_BODY_SLEEP_LINE.trim_end())
        .unwrap_or_else(|| panic!("the camp body prints: {rows:?}"));
    assert_eq!(rows[sleep + 1], "", "{rows:?}");
    assert_eq!(rows[sleep + 2], "Ambushed!", "{rows:?}");
    assert_eq!(
        rows.iter().filter(|row| row.contains("Ambushed")).count(),
        1,
        "{rows:?}"
    );
    assert!(
        !rows.iter().any(|row| row.contains("rested")),
        "the result line is replaced, not followed: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// E11: `formats/saved-gam.md §12.1` - "`0x105D` | Dungeon facing" - and
/// `dungeon-mode.md §4.1`: "a saved facing survives untouched and the party
/// resumes facing whatever the save recorded ... a save whose facing field is
/// zero starts the party facing north".
#[test]
fn playtest_e11_dungeon_facing_round_trips_through_the_save() {
    for (facing, code) in [
        (Direction::North, 0u8),
        (Direction::East, 1),
        (Direction::South, 2),
        (Direction::West, 3),
    ] {
        let mut template = saved_game_seed_bytes(33, 0, 1, 1);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        let dir = save_fidelity_game_dir(template);
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = facing;
        state.write_save_files(&dir).unwrap();

        let bytes = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(
            bytes[SAVE_NPC_BAND_DUNGEON_FACING_OFFSET], code,
            "{facing:?}"
        );
        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.facing, Some(facing));
        let loaded = PlayState::load_scene(&dir, options).unwrap();
        assert_eq!(loaded.player.facing, facing);
        let _ = fs::remove_dir_all(dir);
    }
}

/// E12: `dungeon-mode.md §8.1`, "Who acts" - "Dungeon L-Look, Search, Jimmy
/// and Open all run the shared acting-member prompt before anything else".
/// With three eligible members `Player: ` comes before the cell's `What?`.
#[test]
fn playtest_e12_dungeon_open_asks_who_acts_on_any_cell() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));

    handle_play_key_input(&mut state, 'o', "", Path::new("")).unwrap();
    assert!(
        state.active_surface_chest.is_some(),
        "the member prompt is open"
    );
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert_eq!(state.turn, 0);

    handle_play_key_input(&mut state, '2', "\r", Path::new("")).unwrap();
    assert!(state.active_surface_chest.is_none());
    assert_eq!(state.message, DUNGEON_CHEST_OPEN_WHAT);

    // The already-open cell keeps its own answer behind the prompt.
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x70;
    let mut open = dungeon_state(grid, 0, 1, 1);
    open.party.push(audit_g5_companion(1));
    handle_play_key_input(&mut open, 'o', "", Path::new("")).unwrap();
    assert_eq!(open.message, PARTY_SELECTION_PROMPT);
    handle_play_key_input(&mut open, '1', "\r", Path::new("")).unwrap();
    assert_eq!(open.message, DUNGEON_CHEST_OPEN_ALREADY_OPEN);

    // "a party with no eligible member at all, answers `None!\n` and the
    // command aborts" - before the cell is read, so not `What?`.
    for tile in [0x00, 0x40] {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = tile;
        let mut nobody = dungeon_state(grid, 0, 1, 1);
        nobody.party[0].status = b'S';
        handle_play_key_input(&mut nobody, 'o', "", Path::new("")).unwrap();
        assert!(nobody.active_surface_chest.is_none(), "{tile:#04x}");
        assert_eq!(nobody.message, DUNGEON_ACTING_MEMBER_NONE, "{tile:#04x}");
        assert_eq!(
            nobody.grid[dungeon_cell_index(0, 1, 1)],
            tile,
            "the cell is untouched"
        );
    }
}

/// E13: `dungeon-mode.md §8.1`, "Who acts" - the shared acting-member prompt
/// "prints `Player: ` otherwise, echoing the chosen member's name", and the
/// paragraph names Jimmy among its users.
#[test]
fn playtest_e13_dungeon_jimmy_echoes_the_chosen_member() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"SHAMINO\0\0"];
    state.keys = 1;
    let name = party_name_to_string(&state.party_names[1]).expect("named member");

    handle_play_key_input(&mut state, 'j', "", Path::new("")).unwrap();
    assert!(state.active_jimmy.is_some(), "the member prompt is open");
    handle_play_key_input(&mut state, '2', "\r", Path::new("")).unwrap();

    let rows = playtest_transcript_rows(&state);
    let prompt = rows
        .iter()
        .rposition(|row| row.ends_with(PARTY_SELECTION_PROMPT))
        .unwrap_or_else(|| panic!("the prompt row: {rows:?}"));
    assert!(
        rows[prompt].ends_with(&name) || rows.get(prompt + 1) == Some(&name),
        "the chosen member's name follows `Player: `: {rows:?}"
    );
}

/// E14: `dungeon-mode.md §12` L-Look - "1. Prompts for a party-member slot
/// ... 2. Checks the lighting gate: if both torch and light-spell counters
/// are zero, prints exactly `You see:\ndarkness.\n` and returns. 3. Invokes
/// the shared dungeon relative-focus helper". Unlit, no `Dir-` row opens.
#[test]
fn playtest_e14_unlit_dungeon_look_refuses_before_the_dir_chooser() {
    let mut solo = dungeon_state(open_dungeon_record(), 0, 1, 1);
    solo.torch_counter = 0;
    solo.light_spell_counter = 0;
    handle_play_key_input(&mut solo, 'l', "", Path::new("")).unwrap();
    assert!(solo.active_direction_prompt.is_none(), "no chooser opens");
    let rows = playtest_transcript_rows(&solo);
    assert!(!rows.iter().any(|row| row.contains("Dir-")), "{rows:?}");
    assert!(rows.iter().any(|row| row == "darkness."), "{rows:?}");

    let mut party = dungeon_state(open_dungeon_record(), 0, 1, 1);
    party.party.push(audit_g5_companion(1));
    party.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
    party.torch_counter = 0;
    party.light_spell_counter = 0;
    handle_play_key_input(&mut party, 'l', "", Path::new("")).unwrap();
    assert!(
        party.active_party_selector.is_some(),
        "the member prompt opens"
    );
    handle_play_key_input(&mut party, '2', "", Path::new("")).unwrap();
    handle_play_key_input(&mut party, '\r', "", Path::new("")).unwrap();
    assert!(party.active_direction_prompt.is_none(), "no chooser opens");
    let rows = playtest_transcript_rows(&party);
    assert!(!rows.iter().any(|row| row.contains("Dir-")), "{rows:?}");
    let name = rows
        .iter()
        .position(|row| row.ends_with("IOLO"))
        .unwrap_or_else(|| panic!("the pick is echoed: {rows:?}"));
    assert_eq!(rows[name + 1..], ["You see:", "darkness."], "{rows:?}");
}

/// E15: `dungeon-mode.md §11` - the hours prompt "loops until a digit or
/// Space and echoes the key plus a line feed - Space or `0` aborts with no
/// further output". Escape is neither, so the prompt keeps waiting.
#[test]
fn playtest_e15_camp_hours_prompt_zero_is_silent_and_escape_is_reread() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    handle_play_key_input(&mut state, 'h', "", Path::new("")).unwrap();
    assert!(state.active_rest.is_some());

    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    assert!(state.active_rest.is_some(), "Escape is re-read");
    assert!(
        !playtest_transcript_rows(&state)
            .iter()
            .any(|row| row.contains("None!")),
        "{:?}",
        playtest_transcript_rows(&state)
    );

    handle_play_key_input(&mut state, '0', "", Path::new("")).unwrap();
    assert!(state.active_rest.is_none(), "`0` aborts");
    let rows = playtest_transcript_rows(&state);
    assert_eq!(
        rows.last().map(String::as_str),
        Some(format!("{REST_HOURS_PROMPT}0").as_str()),
        "the key is echoed and nothing follows: {rows:?}"
    );
    assert!(!state.message_slot_needs_flush());
}

/// E16: `dungeon-mode.md §11` - "after `Yes`, `Who will stand guard? `,
/// where a cancel or an ineligible pick prints `None posted!\n\n` and
/// re-asks the watch question."
#[test]
fn playtest_e16_cancelled_guard_pick_reasks_the_watch_question() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party[2].status = b'P';
    for key in ['h', '3', 'y'] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
    }
    assert_eq!(state.message, REST_WATCH_MEMBER_PROMPT);

    // A cancel.
    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    assert!(state.active_rest.is_some(), "the rest has not begun");
    assert_eq!(state.message, REST_WATCH_PROMPT);
    assert!(
        playtest_transcript_rows(&state)
            .iter()
            .any(|row| row == REST_NO_WATCH_LITERAL.trim_end()),
        "{:?}",
        playtest_transcript_rows(&state)
    );

    // An ineligible (poisoned) pick does the same.
    handle_play_key_input(&mut state, 'y', "", Path::new("")).unwrap();
    assert_eq!(state.message, REST_WATCH_MEMBER_PROMPT);
    handle_play_key_input(&mut state, '3', "", Path::new("")).unwrap();
    assert!(state.active_rest.is_some(), "the rest has not begun");
    assert_eq!(state.message, REST_WATCH_PROMPT);
    assert_eq!(state.turn, 0);
}

/// E17: `dungeon-mode.md §15` - the minute advance "sits at the head of each
/// iteration ... so a command the dispatcher reports as \"no action\" (a
/// digit solo-select, a refused Push, the typeahead toggle) still costs a
/// minute underground"; `§4` step 3 routes the Control bindings through the
/// same iteration. None of them takes a turn.
#[test]
fn playtest_e17_dungeon_control_bindings_cost_the_loop_minute() {
    for key in [
        PLAY_TYPEAHEAD_TOGGLE_KEY,
        PLAY_MUSIC_TOGGLE_KEY,
        PLAY_MORAL_STANDING_KEY,
        PLAY_VERSION_BANNER_KEY,
        PLAY_EXIT_TO_DOS_KEY,
    ] {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        let before = state.clock;
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert_eq!(
            (state.clock.hour, state.clock.minute),
            (before.hour, before.minute + 1),
            "key {:#04x}",
            key as u32
        );
        assert_eq!(state.turn, 0, "key {:#04x} takes no turn", key as u32);
    }
}

/// E18: `dungeon-mode.md §8.1`, "Fountain drink flow" - "`Will you
/// drink?\n` blocks until `Y` or `N`". Escape is re-read, not taken for `N`.
#[test]
fn playtest_e18_fountain_drink_prompt_ignores_escape() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x50;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;
    state.torch_counter = 5;
    assert_eq!(state.look_dungeon(), MoveOutcome::Observed);
    assert!(state.active_yes_no_prompt.is_some());

    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    assert!(state.active_yes_no_prompt.is_some(), "Escape is re-read");
    assert!(
        !playtest_transcript_rows(&state)
            .iter()
            .any(|row| row.contains("No.")),
        "{:?}",
        playtest_transcript_rows(&state)
    );

    handle_play_key_input(&mut state, 'n', "", Path::new("")).unwrap();
    assert!(state.active_yes_no_prompt.is_none(), "`N` still answers");
}

/// E19: `input.md §5` - dungeon mode "does not run the digit-to-direction
/// translation at all" - and `dungeon-mode.md §12`: the `Dir-` chooser takes
/// Up/Left/Right/Down and Space, other keys "ignored without echo; the
/// chooser remains open".
#[test]
fn playtest_e19_dungeon_dir_chooser_ignores_top_row_digits() {
    for digit in ['8', '4', '6', '2'] {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.torch_counter = 5;
        handle_play_key_input(&mut state, 'l', "", Path::new("")).unwrap();
        assert!(state.active_direction_prompt.is_some());
        handle_play_key_input(&mut state, digit, "", Path::new("")).unwrap();
        assert!(
            state.active_direction_prompt.is_some(),
            "`{digit}` is not a direction underground"
        );
        let rows = playtest_transcript_rows(&state);
        assert!(!rows.iter().any(|row| row == "You see:"), "{rows:?}");

        handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", Path::new("")).unwrap();
        assert!(state.active_direction_prompt.is_none(), "Up still answers");
    }
}

/// E20: `commands.md §5`, the digit row - "Digits `1` upward ... reject Dead
/// or Asleep members with `Invalid!`"; `dungeon-mode.md §4` step 3 routes
/// dungeon digits to that selector.
#[test]
fn playtest_e20_digit_solo_select_rejects_a_sleeping_member() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party[1].status = b'S';
    state.party[2].status = b'D';

    for digit in ['2', '3'] {
        handle_play_key_input(&mut state, digit, "", Path::new("")).unwrap();
        assert_eq!(state.active_player, None, "`{digit}`");
        assert!(
            state
                .message
                .contains(SET_ACTIVE_PLAYER_INVALID_REPLY.trim_end()),
            "`{digit}`: {:?}",
            state.message
        );
    }
    handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();
    assert_eq!(state.active_player, Some(0));
}
