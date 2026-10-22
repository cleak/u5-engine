// Regressions for the fourth-pass combat exploratory-playtest findings
// (`p4combat`, H02-H32).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

fn p4combat_rows_text(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// `combat.md §8`, the `P` row: "Prints the `Push-` label and calls the
/// shared movable-static-tile handler directly", and `commands.md §8.1` row
/// C's exact text `Push-<Dir>\nWon't budge!\n`. Playtest H23: the world
/// dispatcher's verb echo was opened as well, so `Push-` stood on two rows.
#[test]
fn playtest_h23_combat_push_prints_its_label_once() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.ensure_pending_combat_player_turn();
    let before = state.message_entries().len();

    handle_play_key_input(&mut state, 'P', "", game_dir).unwrap();

    let rows = p4combat_rows_text(&state);
    let labels = rows[before..]
        .iter()
        .filter(|row| row.starts_with("Push"))
        .count();
    assert_eq!(labels, 1, "rows: {rows:?}");
    assert_eq!(
        rows.last().map(String::as_str),
        Some("Push-"),
        "rows: {rows:?}"
    );
}

/// The same `P` row in a dungeon-room arena: the dungeon-mode `Push` echo
/// (whose `Not here!` refusal never runs in combat) must not open above the
/// arena's `Push-` prompt. Playtest H27.
#[test]
fn playtest_h27_dungeon_room_combat_push_opens_no_stray_echo_row() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.area = Area::Dungeon {
        scene: DungeonScene::new(0x1B).unwrap_or(DungeonScene {
            byte: 0x1B,
            record: 0,
        }),
        level: 1,
    };
    state.ensure_pending_combat_player_turn();
    let before = state.message_entries().len();

    handle_play_key_input(&mut state, 'P', "", game_dir).unwrap();

    let rows = p4combat_rows_text(&state);
    let new_rows: Vec<&String> = rows[before..]
        .iter()
        .filter(|row| !row.is_empty())
        .collect();
    assert_eq!(new_rows, [&"Push-".to_string()], "rows: {rows:?}");
}
