// Regressions for the pass-22 exploratory playtest findings (rest22).
//
// All of them concern where a Hole-up or an inn rest runs the party
// status/provision pass (`time.md §5`) and where its `Starving!` prints.
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

fn p22_rows(state: &PlayState) -> Vec<String> {
    let mut rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    if !state.message.is_empty() && state.message != state.message_flushed {
        rows.extend(state.message.lines().map(str::to_string));
    }
    rows
}

/// Playtest P22-02. `rest-and-camp.md §4`: at the town bed "Space or `0`
/// cancels ... The command still reports acted, so a cancel costs the
/// ordinary one-minute town turn and its epilogue", and `town-mode.md §7`
/// step 4 ends that epilogue with the shared status pass. The cancel turn's
/// poison point used to wait for the next command, which then took two.
#[test]
fn playtest_p22_02_town_bed_cancel_takes_its_own_poison_point() {
    let (mut state, dir) = p3w_town_bed_state();
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.party[0].status = b'P';

    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    handle_play_key_input(&mut state, '0', "", &dir).unwrap();
    assert!(state.active_rest.is_none());
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(10, 1).unwrap());
    assert_eq!(state.party[0].hp, 59, "the cancel turn's own poison point");
    assert!(!state.pending_town_status_provision_pass);

    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert_eq!(state.party[0].hp, 58, "one point per turn, never two");
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P22-02, the hour branch. `time.md §5`: the hour-gated branch
/// prints the starvation warning and then applies the damage, on the
/// invocation that sees the crossing - here the cancel turn's.
#[test]
fn playtest_p22_02_town_bed_cancel_starves_on_its_own_turn() {
    let (mut state, dir) = p3w_town_bed_state();
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.food = 0;
    state.clock = GameClock::new(12, 59).unwrap();
    state.cleanup_previous_hour = 12;

    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    handle_play_key_input(&mut state, '0', "", &dir).unwrap();
    assert_eq!(state.clock, GameClock::new(13, 0).unwrap());
    assert!(state.party[0].hp < 60);
    assert!(state.pending_hourly_status_message.is_none());
    let rows = p22_rows(&state);
    assert_eq!(
        rows.iter().filter(|row| *row == STARVATION_WARNING).count(),
        1,
        "{rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P22-05. `dungeon-mode.md §4` step 6: after a command that acted
/// the post-action helper runs, and "The helper's last act is the party
/// upkeep pass of `systems/time.md` Section 5". `§11` exempts only the camp's
/// own five-minute loop. An accepted dungeon camp skipped the pass; the
/// loop-head minute stays the turn's only minute.
#[test]
fn playtest_p22_05_dungeon_camp_turn_runs_the_upkeep_pass() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.party[0].status = b'P';
    state.clock = GameClock::new(10, 20).unwrap();
    state.cleanup_previous_hour = 10;

    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    assert_eq!(state.clock, GameClock::new(10, 21).unwrap(), "loop-head minute");
    handle_play_key_input(&mut state, '2', "", &dir).unwrap();
    assert!(state.active_rest.is_none());
    assert!(!state.combat_active);
    assert_eq!(state.clock, GameClock::new(12, 21).unwrap(), "no second minute");
    assert_eq!(state.party[0].hp, 59, "the camp turn's poison point");
    let _ = fs::remove_dir_all(dir);
}
