// Regressions for the p5world exploratory play-test findings (J15..J22).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// Playtest J20. `time.md §5`: the party status/provision pass "does **not**
/// run in combat mode"; its caller census: "Combat mode and the wilderness
/// camp elapse loop never invoke it, so poison damage and provision
/// consumption do not accrue there." `combat.md §15`: the round wrap fires
/// only "the per-turn cleanup with a one-minute increment".
#[test]
fn p5world_combat_round_wrap_minute_runs_no_status_provision_pass() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.party[0].status = b'P';
    let hp = state.party[0].hp;
    state.food = 63;
    state.clock = GameClock::new(17, 59).unwrap();
    state.cleanup_previous_hour = 17;
    state.combat_round_counter = COMBAT_ROUND_COUNTER_WRAP - 1;

    let tick = state.advance_combat_round_counter();

    assert!(tick.wrapped);
    assert_eq!(state.clock, GameClock::new(18, 0).unwrap());
    assert_eq!(state.party[0].hp, hp, "no poison tick inside the arena");
    assert_eq!(state.food, 63, "no 18:00 meal inside the arena");
    assert!(!state.pending_town_status_provision_pass);
}
