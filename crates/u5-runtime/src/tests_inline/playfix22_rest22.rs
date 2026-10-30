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

fn p22_karma_dat(dir: &std::path::Path) {
    let mut bytes = Vec::new();
    for record in ["low", "twenty", "forty", "sixty", "blackthorn-top", "camp-top"] {
        bytes.extend_from_slice(record.as_bytes());
        bytes.push(0);
    }
    fs::write(dir.join(KARMA_DAT_FILE), bytes).unwrap();
}

/// A one-member overworld party whose six-hour camp reaches the apparition
/// (prng seed 4 reaches the event), starting at `hour:minute`
/// with no food.
fn p22_apparition_camp(hour: u8, minute: u8) -> (PlayState, std::path::PathBuf) {
    let dir = debug_game_dir();
    p22_karma_dat(&dir);
    let mut state = britannia_state(open_world_grid(), 2, 0);
    state.clock = GameClock::new(hour, minute).unwrap();
    state.cleanup_previous_hour = hour;
    state.food = 0;
    state.prng_state = 4;
    state.party[0].hp = 10;
    state.party[0].max_hp = 30;
    state.party[0].climb_stat = 20;
    state.party_experience = vec![200];
    state.party_strengths = vec![20];
    state.party_intelligence = vec![18];
    state.moral_standing = 80;
    (state, dir)
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

/// Playtest P22-03. `time.md §12`: the status pass "is invoked from exactly
/// four places", and the camp apparition's closing turn-clock advance
/// (`rest-and-camp.md §7.1` step 7) is not one. Its hour crossing is then
/// erased by the outdoor turn's own advance (`time.md §5`, "Two
/// time-advancing calls between an hour crossing and the next upkeep pass
/// therefore erase the mismatch"). It used to run the pass, applying a
/// starvation roll after the event's heal with no `Starving!`.
#[test]
fn playtest_p22_03_apparition_clock_advance_runs_no_upkeep_pass() {
    let (mut state, dir) = p22_apparition_camp(12, 58);
    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    handle_play_key_input(&mut state, '6', "", &dir).unwrap();
    state.flush_text_hold();
    let rows = p22_rows(&state);
    assert!(rows.iter().any(|row| row == "An apparition!"), "{rows:?}");
    assert_eq!(state.clock, GameClock::new(19, 2).unwrap());
    assert_eq!(state.party[0].hp, state.party[0].max_hp, "no starvation after the heal");
    assert!(!rows.iter().any(|row| row == STARVATION_WARNING), "{rows:?}");
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P22-03, the turn's own crossing. When the outdoor turn's advance
/// after the event crosses the hour, the pass starves the party - and the
/// warning prints after the event's last held page, never between its pages.
#[test]
fn playtest_p22_03_apparition_turn_starvation_prints_after_the_event() {
    let (mut state, dir) = p22_apparition_camp(12, 56);
    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    handle_play_key_input(&mut state, '6', "", &dir).unwrap();
    assert!(state.text_hold_active());
    state.flush_text_hold();
    let rows = p22_rows(&state);
    assert_eq!(state.clock, GameClock::new(19, 0).unwrap());
    assert!(state.party[0].hp < state.party[0].max_hp);
    assert_eq!(
        rows.iter().rev().take(2).cloned().collect::<Vec<_>>(),
        [STARVATION_WARNING, "The strangely familiar old man vanishes..."],
        "{rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}
