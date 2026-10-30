// Regressions for the pass-22 exploratory playtest findings (rest22).
//
// All of them concern where a Hole-up or an inn rest runs the party
// status/provision pass (`time.md §5`) and where its `Starving!` prints.
// The P22-04 inn-night warnings were withdrawn by u5-spec#314 Q3 (paid inn
// rest runs no pass); the spec314 tests at the end pin that answer.
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

/// u5-spec#314 Q3. `shops.md §8.4`: "The rest runs **no** party
/// status/provision pass ... no provisions are spent, no `Starving!` warning
/// prints, no starvation damage is rolled and no per-turn poison point is
/// taken. After the innkeeper's farewell the message window shows only
/// `Zzzzzz....`, nothing at all while the clock runs, then `Morning!`".
/// Playtest P22-04's night warnings are withdrawn.
#[test]
fn spec314_inn_rest_prints_nothing_between_sleep_and_morning() {
    let mut state = pf2_inn_state();
    state.food = 0;
    state.clock = GameClock::new(22, 30).unwrap();
    state.cleanup_previous_hour = 22;
    pf2_keys(&mut state, &['R', 'Y']);
    state.flush_message_slot();
    let rows = pf2_rows(&state);
    let sleep = pf2_row_index(&rows, crate::commands::PAID_INN_REST_SLEEP_LINE);
    let morning = pf2_row_index(&rows, crate::commands::PAID_INN_REST_MORNING_LINE);
    assert_eq!(morning, sleep + 2, "{rows:#?}");
    assert_eq!(rows[sleep + 1], "", "{rows:#?}");
    assert!(
        !rows.iter().any(|row| row == STARVATION_WARNING),
        "{rows:#?}"
    );
    assert!(state.pending_hourly_status_message.is_none());
    assert_eq!(state.food, 0);
    assert!(state.party.iter().all(|member| member.status != b'D'));
}

/// u5-spec#314 Q3. `shops.md §8.4`: "with provisions on hand the 06:00 meal
/// is skipped ... the first hour charged is 07:00", and `time.md §5`: the
/// Talk turn's "ordinary one-minute advance then rewrites the snapshot to 6
/// before that turn's upkeep pass runs, so the pass sees no hour change".
#[test]
fn spec314_inn_rest_charges_no_hour_it_crossed() {
    let mut state = pf2_inn_state();
    state.food = 50;
    state.clock = GameClock::new(22, 30).unwrap();
    state.cleanup_previous_hour = 22;
    pf2_keys(&mut state, &['R', 'Y']);
    state.flush_message_slot();
    assert_eq!(state.food, 50);
    // Whatever turn the conversation's close spends, 06:00 stays uncharged.
    state.advance_turn();
    state.apply_pending_town_status_provision_pass();
    assert_eq!(state.clock.hour, 6);
    assert_eq!(state.food, 50);
}

/// u5-spec#314 Q3. `shops.md §8.4`: "a party resting from 22:30 with no
/// food and a Poisoned member printed no `Starving!`, the member was still
/// Poisoned when the clock stopped and died only in the recovery pass".
#[test]
fn spec314_inn_rest_takes_no_poison_point_while_the_clock_runs() {
    let mut state = test_state(open_grid(), 5, 5);
    state.food = 0;
    state.clock = GameClock::new(22, 30).unwrap();
    state.cleanup_previous_hour = 22;
    state.party[0].status = b'P';
    state.party[0].hp = 3;
    state.advance_inn_rest_clock_to_morning();
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[0].hp, 3);
    assert!(state.pending_hourly_status_message.is_none());
}

/// u5-spec#314 Q3. `time.md §5`: "Paid inn rest likewise issues the roll
/// directly, once per nine-minute step after its first sixty minutes, and
/// never through this pass". From 01:00 the rest takes sixty minutes to
/// 02:00, then 27 nine-minute steps to 06:03, crossing no midnight (whose
/// hideout reroll draws too).
#[test]
fn spec314_inn_rest_rolls_the_ring_once_per_nine_minute_step() {
    let start = |ring: bool| {
        let mut state = test_state(open_grid(), 5, 5);
        state.clock = GameClock::new(1, 0).unwrap();
        state.cleanup_previous_hour = 1;
        state.prng_state = 0x1234;
        state.party[0].hp = 1;
        if ring {
            state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
        }
        state
    };
    let mut plain = start(false);
    plain.advance_inn_rest_clock_to_morning();
    assert_eq!((plain.clock.hour, plain.clock.minute), (6, 3));
    assert_eq!(plain.prng_state, 0x1234, "no ring, no draws");

    let mut worn = start(true);
    worn.advance_inn_rest_clock_to_morning();
    let mut expected = 0x1234;
    let mut gains = 0;
    for _ in 0..27 {
        if u5_prng_range_u16(&mut expected, 0, 7) == 0 {
            gains += 1;
        }
    }
    assert_eq!(worn.prng_state, expected);
    assert_eq!(worn.party[0].hp, (1 + gains).min(worn.party[0].max_hp));
}
