// Regressions for the third exploratory playtest's world/time findings
// (W1..W6). Each test names the finding and the published sentence it
// rests on.

#[test]
fn p3w_w1_arrest_wait_to_morning_runs_no_party_status_pass() {
    // `time.md §12`: the status/provision pass "is invoked from exactly four
    // places: the overworld per-turn epilogue, the town per-turn underfoot
    // handler, the dungeon post-action handler, and the town-bed rest loop's
    // ten-minute step". The arrest loop (`time.md §10`) is a twenty-minute
    // cleanup caller and not one of them, so a poisoned party crossing the
    // 06:00 meal takes no poison points and eats nothing.
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::new(0, 30).unwrap();
    state.party[0].status = b'P';
    state.party[0].hp = 12;
    state.food = 63;
    state.toll_progress = 0;
    let ticks = state.advance_town_arrest_wait_to_morning();
    assert_eq!(ticks, 23);
    assert_eq!(state.clock.hour, 8);
    assert_eq!(state.party[0].hp, 12, "no poison point per arrest step");
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.food, 63, "the 06:00 meal is not eaten in the loop");
    assert_eq!(
        state.toll_progress, 0,
        "the turn-step counter's only increment is the upkeep pass"
    );
}
