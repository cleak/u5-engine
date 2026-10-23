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

/// Playtest J16. `town-mode.md §7` step 4: the clock minute, underfoot and
/// status pass and schedule pass run "when the dispatcher returns and the
/// action consumed a turn", and the Talk dispatcher runs the whole shop arm.
/// `shops.md §2`: explicit Talk "receives normal town clock/scheduler
/// processing". So a shop opened at 11:59 greets at 11:59 with the 12:00
/// meal not yet eaten, and the turn is spent when the shop closes.
#[test]
fn p5world_talk_entered_shop_spends_its_turn_when_the_shop_closes() {
    let dialogue = HashMap::new();
    let raw = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(21).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.clock = GameClock::new(11, 59).unwrap();
    state.cleanup_previous_hour = 11;
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x84,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.food = 63;

    assert_eq!(
        state.talk_direction_with_dialogue_and_keyword_raw(Direction::East, &dialogue, &raw, None),
        MoveOutcome::Talked
    );
    assert!(state.active_shop.is_some());
    assert_eq!(state.clock, GameClock::new(11, 59).unwrap());
    assert_eq!(state.food, 63);
    assert_eq!(state.turn, 0);

    let dir = debug_game_dir();
    for key in ['N', '\u{1b}', ' ', 'N', '\u{1b}', ' '] {
        if state.active_shop.is_none() {
            break;
        }
        handle_play_key_input(&mut state, key, "", &dir).unwrap();
    }
    let _ = fs::remove_dir_all(dir);
    assert!(state.active_shop.is_none(), "the shop closed");
    assert!(!state.pending_talk_shop_turn);
    assert_eq!(state.turn, 1, "the Talk turn is spent once, at close");
    assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
    assert!(state.food < 63, "the 12:00 meal is eaten after the shop");
}

/// Playtest J18. `town-mode.md §7`/§7.1: drunkenness is a pre-dispatch stage
/// that runs after the command is read; a firing draw "prints the fixed line
/// `Hic!` followed by a line break" and substitutes a cardinal step. So
/// `Hic!` sits on the read command's own prompt row, after its marker, and
/// the substituted step's echo follows on the next row with no marker and no
/// blank of its own.
#[test]
fn p5world_hiccup_takes_the_command_prompt_row_and_the_step_echo_follows() {
    let gate_seed = (0..=u16::MAX)
        .find(|seed| {
            let mut state = *seed;
            u5_prng_range_u16(&mut state, 0, 1) == 1
        })
        .unwrap();
    let mut state = test_state(open_grid(), 15, 15);
    state.town_drunkenness_counter = 25;
    state.prng_state = gate_seed;
    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert_eq!(state.town_drunkenness_counter, 24);
    assert!(!state.town_hiccup_echo_continues);

    let rows = p4window_rows(&state);
    let hic = rows
        .iter()
        .position(|row| row == ">Hic!")
        .unwrap_or_else(|| panic!("no `>Hic!` row in {rows:?}"));
    let step = &rows[hic + 1];
    assert!(
        ["North", "South", "East", "West"].contains(&step.as_str()),
        "the substituted step's echo follows `Hic!` directly: {rows:?}"
    );
}
