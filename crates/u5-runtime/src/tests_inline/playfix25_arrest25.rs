// Regressions for the twenty-fifth exploratory playtest's arrest findings
// (P25-01, P25-02). Each test names the finding and the published sentence
// it rests on.

/// Playtest P25-01. `town-mode.md §14` "Surrender, in order": the clock runs
/// forward "(ten-minute steps under Quickness ... and the jail loop does not
/// age the effect) until a step lands in the 08:00 hour ... always between
/// 08:00 and 08:19". A 96-step cap woke a Quickness party at 04:32 and aged
/// the effect once per step.
#[test]
fn playtest_p25_01_quickness_jail_wait_reaches_eight_without_ageing() {
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::new(12, 32).unwrap();
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 0xFE;

    let steps = state.advance_town_arrest_wait_to_morning();

    assert_eq!((state.clock.hour, state.clock.minute), (8, 2));
    assert_eq!(steps, 117, "ten-minute steps from 12:32 to 08:02");
    assert_eq!(state.active_effect_tag, Some(QUICKNESS_ACTIVE_EFFECT_TAG));
    assert_eq!(state.active_effect_counter, 0xFE);
}

/// P25-01 with the spell's own short duration: the effect must not expire
/// inside the wait, so the steps stay ten minutes long to the end.
#[test]
fn playtest_p25_01_short_quickness_survives_the_jail_wait() {
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::new(10, 6).unwrap();
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 0x1E;

    state.advance_town_arrest_wait_to_morning();

    assert_eq!((state.clock.hour, state.clock.minute), (8, 6));
    assert_eq!(state.active_effect_tag, Some(QUICKNESS_ACTIVE_EFFECT_TAG));
    assert_eq!(state.active_effect_counter, 0x1E);
}

/// P25-01, variant (B): Protection is not aged by the twenty-minute jail
/// steps either.
#[test]
fn playtest_p25_01_protection_is_not_aged_by_the_jail_wait() {
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::new(12, 31).unwrap();
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 0xFE;

    state.advance_town_arrest_wait_to_morning();

    assert_eq!((state.clock.hour, state.clock.minute), (8, 11));
    assert_eq!(state.active_effect_tag, Some(PROTECTION_ACTIVE_EFFECT_TAG));
    assert_eq!(state.active_effect_counter, 0xFE);
}

/// Playtest P25-02. `blackthorn.md §3`: "The handoff writes the scene byte,
/// the floor ..., the local position, and zeroes the carried-key counter" -
/// no transport marker - and `town-mode.md §15.1`'s entry setup does not
/// "separate the party from a vehicle". A party captured on a horse woke in
/// the palace cell on foot and the horse was gone.
#[test]
fn playtest_p25_02_captive_cell_handoff_keeps_the_party_mounted() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap();
    state.area = Area::Town { scene, floor: 0 };
    let horse = TransportState::Horse {
        type_byte: 0xA0,
        tile: 0xA0,
    };
    state.player.transport = horse;

    state
        .apply_blackthorn_captive_cell_handoff(&dir, "test")
        .unwrap();

    assert_eq!(
        (state.player.x, state.player.y),
        (
            BLACKTHORN_CAPTIVE_CELL_X as usize,
            BLACKTHORN_CAPTIVE_CELL_Y as usize
        )
    );
    assert_eq!(state.player.transport, horse);
    let _ = fs::remove_dir_all(dir);
}
