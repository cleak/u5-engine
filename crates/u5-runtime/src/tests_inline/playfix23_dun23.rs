// Playtest round 23 (dungeon): one dungeon command is one loop-head minute
// and one post-action pass (`dungeon-mode.md §4` steps 1 and 6) - the wake
// roll, the active-monster step, the underfoot reaction (room entry among its
// consequences, `§8.1`), then the upkeep pass of `time.md §5` last - with a
// room or camp-ambush fight nested inside it.

/// A `DUNGEON.CBT` whose every record seats one foe, so a room entry opens
/// a fight.
fn pf23_write_dungeon_cbt(dir: &Path) {
    let mut record = synthetic_combat_arena_record();
    let source_base =
        DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    for offset in 0..DUNGEON_ROOM_SOURCE_COUNT {
        record[source_base + offset] = 0;
    }
    record[source_base + 2] = 0x44;
    let mut dungeon_cbt = Vec::new();
    for _ in 0..DUNGEON_CBT_RECORDS {
        dungeon_cbt.extend_from_slice(&record);
    }
    fs::write(dir.join(DUNGEON_CBT_FILE), dungeon_cbt).unwrap();
}

/// P23-05: `dungeon-mode.md §4` step 1 makes one world-clock call per
/// iteration, `§13.1` has an applied climb report "acted" so the single
/// post-action pass runs, and `§8.1` puts room entry inside that pass. A K
/// down onto a room cell costs one minute, and its upkeep pass (the poison
/// point) waits for the room fight.
#[test]
fn playfix23_klimb_onto_a_room_cell_charges_one_minute_and_defers_the_upkeep_pass() {
    let dir = debug_game_dir();
    pf23_write_dungeon_cbt(&dir);
    let scene = DungeonScene::new(35).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x20;
    grid[dungeon_cell_index(1, 1, 1)] = 0xf7;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.area = Area::Dungeon { scene, level: 0 };
    state.party[0].status = b'P';
    state.party[0].max_hp = 200;
    state.party[0].hp = 100;
    state.clock = GameClock::new(10, 20).unwrap();
    state.cleanup_previous_hour = 10;

    handle_play_key_input(&mut state, 'K', "", &dir).unwrap();

    assert_eq!(state.area, Area::Dungeon { scene, level: 1 });
    assert!(state.combat_active, "the room fight is open");
    assert_eq!(state.clock, GameClock::new(10, 21).unwrap(), "one minute");
    assert_eq!(state.turn, 1, "one action");
    assert_eq!(state.party[0].hp, 100, "no poison point before the fight");
    assert!(state.pending_dungeon_status_provision_pass);
    let _ = fs::remove_dir_all(dir);
}

/// P23-06: `dungeon-mode.md §4` step 6 makes the wake roll the post-action
/// helper's "first act", and `§10` orders the pass "the wake roll, the
/// active-monster step, the encounter auto-face, the underfoot reaction".
/// Room entry is one of `§8.1`'s post-action underfoot consequences, so the
/// step that walks into a room still rolls the wake draw before the fight.
#[test]
fn playfix23_walking_into_a_room_rolls_the_wake_draw_before_the_fight() {
    let dir = debug_game_dir();
    pf23_write_dungeon_cbt(&dir);
    let scene = DungeonScene::new(35).unwrap();
    let mut woke = 0;
    for seed in 0..64u16 {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xf7;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.area = Area::Dungeon { scene, level: 0 };
        state.player.facing = Direction::East;
        state.party = vec![default_party()[0]; 2];
        state.party[1].status = b'S';
        state.party_equipment = vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]; 2];
        state.prng_state = seed.wrapping_mul(0x9e37);

        handle_play_key_input(&mut state, '.', "", &dir).unwrap();

        assert!(state.combat_active, "seed {seed}: the room fight is open");
        assert_eq!(state.turn, 1, "seed {seed}");
        if state.party[1].status == b'G' {
            woke += 1;
        }
    }
    assert!(
        woke > 0,
        "a 1-in-16 wake roll over 64 seeds woke nobody: the roll did not run"
    );
    let _ = fs::remove_dir_all(dir);
}

/// P23-07: `commands.md §3` has H spend a turn "Always", and
/// `dungeon-mode.md §11` step 4 nests the ambush arena inside the command,
/// so the post-action helper - its upkeep pass last - follows the fight. A
/// starving party's camp that crossed an hour before the ambush gets its
/// `Starving!` once the fight returns, not a command later.
#[test]
fn playfix23_ambushed_dungeon_camp_runs_the_post_action_pass_after_the_fight() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.player.facing = Direction::North;
    state.party[0].max_hp = 240;
    state.party[0].hp = 240;
    state.food = 0;
    state.clock = GameClock::new(8, 55).unwrap();
    state.cleanup_previous_hour = 8;
    // The first hour rollover's 1-in-64 interruption roll comes up zero.
    state.prng_state = (0..=u16::MAX)
        .find(|seed| {
            let mut scratch = *seed;
            u5_prng_range_u16(&mut scratch, 0, 63) == 0
        })
        .expect("some seed interrupts");

    handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
    handle_play_key_input(&mut state, '9', "", &dir).unwrap();
    assert!(state.combat_active, "the camp was ambushed");
    assert!(
        playtest_transcript_rows(&state)
            .iter()
            .any(|row| row == "Ambushed!")
    );
    assert!(state.pending_dungeon_post_action_helper);
    assert!(state.pending_dungeon_status_provision_pass);
    assert!(
        !playtest_transcript_rows(&state)
            .iter()
            .any(|row| row.contains("Starving"))
    );

    // The fight returns; the dispatcher's tail (`run_with_command_epilogue`)
    // then runs the owed helper and settles the upkeep pass.
    state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
    assert!(!state.combat_active);
    state
        .resume_dungeon_post_action_helper_after_combat(&dir)
        .unwrap();
    assert!(!state.pending_dungeon_post_action_helper);
    assert!(state.apply_pending_dungeon_status_provision_pass());
    state.append_pending_hourly_status_message();
    assert!(
        state.message.contains("Starving"),
        "the camp turn's hourly branch ran when the fight returned: {:?}",
        state.message
    );
    let _ = fs::remove_dir_all(dir);
}
