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
