// Dungeon post-action order: `dungeon-mode.md §4` step 6 makes the wake
// roll the helper's "first act" and the party upkeep pass of `time.md §5`
// its "last act"; `time.md §12` puts that pass in "the dungeon post-action
// handler". The turn's charge used to run the pass first, so its draws came
// ahead of the wake roll's and the monster step's on every dungeon command.

/// A one-level open dungeon with an active monster, a Ring of Regeneration
/// wearer (the pass's 1-in-8 draw) in slot 0 and a Sleeping member (the
/// wake roll's draw) in slot 1.
fn dto_fixture(seed: u16, ring: bool) -> PlayState {
    let mut state = dungeon_state(vec![0x00; DUNGEON_RECORD_LEN], 0, 3, 3);
    state.prng_state = 0x5151;
    assert!(state.setup_dungeon_active_monster_fresh());
    state.prng_state = seed;
    state.party = vec![default_party()[0]; 2];
    state.party[0].status = b'G';
    state.party[0].max_hp = 200;
    state.party[0].hp = 100;
    state.party[1].status = b'S';
    state.party_equipment = vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]; 2];
    if ring {
        state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    }
    state.clock = GameClock::new(10, 20).unwrap();
    state.cleanup_previous_hour = 10;
    state
}

/// The dispatcher's acted charge (`settle_dungeon_acted_charge`): the turn,
/// then the post-action helper.
fn dto_acted_turn(state: &mut PlayState, dir: &Path) {
    let turn_before = state.turn;
    state.advance_turn();
    state
        .apply_top_down_post_turn_effects_after_turn(turn_before, dir)
        .unwrap();
}

#[test]
fn dungeon_upkeep_pass_draws_after_the_wake_roll_and_monster_step() {
    let dir = debug_game_dir();
    let mut order_was_observable = false;
    for seed in 0..64u16 {
        let seed = seed.wrapping_mul(0x9e37);
        let mut state = dto_fixture(seed, true);
        dto_acted_turn(&mut state, &dir);
        assert!(!state.pending_dungeon_status_provision_pass);

        // The same turn without the ring: the wake roll and the monster
        // step alone. With the order of §4 step 6 the ring's roll is the
        // turn's last draw, taken from the stream those two left.
        let mut plain = dto_fixture(seed, false);
        dto_acted_turn(&mut plain, &dir);
        let mut after = plain.prng_state;
        let gain = u16::from(u5_prng_range_u16(&mut after, 0, 7) == 0);
        assert_eq!(state.prng_state, after, "seed {seed:#06x}");
        assert_eq!(state.party[0].hp, 100 + gain, "seed {seed:#06x}");
        assert_eq!(state.party[1].status, plain.party[1].status);
        assert_eq!(
            state.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT],
            plain.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT],
            "seed {seed:#06x}: the monster walked the same stream"
        );

        // The withdrawn order: the ring first, then the helper's draws.
        let mut first = seed;
        let _ = u5_prng_range_u16(&mut first, 0, 7);
        let mut withdrawn = dto_fixture(first, false);
        dto_acted_turn(&mut withdrawn, &dir);
        if withdrawn.prng_state != state.prng_state
            || withdrawn.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT]
                != state.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT]
        {
            order_was_observable = true;
        }
    }
    assert!(
        order_was_observable,
        "no seed separates the two orders, so this test pins nothing"
    );
    let _ = fs::remove_dir_all(dir);
}

/// `dungeon-mode.md §4` step 6: the pass is owed by the turn's charge and
/// runs once, at the end of the helper; the poison point still lands on
/// the acted turn, before the next key.
#[test]
fn dungeon_upkeep_pass_is_owed_to_the_end_of_the_helper() {
    let dir = debug_game_dir();
    let mut state = dto_fixture(0x1234, false);
    state.party[0].status = b'P';
    let turn_before = state.turn;
    state.advance_turn();
    assert!(state.pending_dungeon_status_provision_pass);
    assert_eq!(state.party[0].hp, 100, "no poison point inside the charge");
    state
        .apply_top_down_post_turn_effects_after_turn(turn_before, &dir)
        .unwrap();
    assert!(!state.pending_dungeon_status_provision_pass);
    assert_eq!(state.party[0].hp, 99, "the helper's last act");
    let _ = fs::remove_dir_all(dir);
}

/// Playtest P22-05's dungeon camp keeps its upkeep pass on the camp turn,
/// now after the helper's draws; nothing is left owed once the key returns.
#[test]
fn dungeon_camp_turn_settles_its_upkeep_pass_before_the_next_key() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.party[0].status = b'P';
    state.clock = GameClock::new(10, 20).unwrap();
    state.cleanup_previous_hour = 10;
    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    handle_play_key_input(&mut state, '2', "", &dir).unwrap();
    assert!(!state.pending_dungeon_status_provision_pass);
    assert_eq!(state.party[0].hp, 59);
    let _ = fs::remove_dir_all(dir);
}
