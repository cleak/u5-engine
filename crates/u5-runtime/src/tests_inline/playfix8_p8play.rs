// Regressions for the pass-8 exploratory-playtest findings (p8play, N02-N26).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `commands.md §3`: "`B` Board | \"Acted\" on every path - all four mounts
/// and both refusals - except the final unknown-target fallthrough". The
/// dungeon arm prints `Not here!` (`vehicles.md §4.1`), not the fallthrough,
/// so it spends the turn that drives the dungeon post-action pass (N06).
#[test]
fn playtest_n06_dungeon_board_refusal_reports_acted() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    let clock_before = state.clock;

    handle_play_key_input(&mut state, 'B', "", &dir).unwrap();

    assert_eq!(state.turn, 1, "dungeon Board refusal must report acted");
    let mut expected = clock_before;
    expected.advance_minutes(1);
    assert_eq!(state.clock, expected, "only the loop-head minute");
    let _ = fs::remove_dir_all(dir);
}
