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
