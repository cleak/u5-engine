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
