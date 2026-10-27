// Regressions for the queued conformance items FC1, Z1 and R14a. Each test
// names the item and the published sentence (or clean runtime observation)
// it rests on.

/// One-deep fall onto an ordinary passage, by a step (the path that spends
/// the step's turn inside the fall handler). The member wears the Ring of
/// Regeneration, whose 1-in-8 draw sits in the status/provision pass, so
/// the HP it ends on shows which side of the monster setup that pass ran.
fn fc1_fall_fixture(seed: u16, landing: u8) -> PlayState {
    let mut grid = vec![0x90; DUNGEON_RECORD_LEN];
    grid[dungeon_cell_index(0, 2, 1)] = 0x61;
    grid[dungeon_cell_index(1, 2, 1)] = landing;
    let mut state = dungeon_state(grid, 0, 2, 1);
    state.prng_state = seed;
    state.party.truncate(1);
    state.party[0].status = b'G';
    state.party[0].max_hp = 200;
    state.party[0].hp = 100;
    state
        .party_equipment
        .resize(state.party.len(), [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]);
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    state
}

/// FC1. `dungeon-mode.md §8` (u5-spec#304 Q1): "The active dungeon monster
/// gets its fresh setup (Section 4.1) exactly once per chain, after the last
/// descent's sweep ... The stats redraw and the hourly status tick follow
/// the chain." The step's turn - which carries that status pass - was spent
/// before the setup, so the ring's draw came ahead of the setup's draws.
#[test]
fn fc1_fall_chain_runs_the_status_tick_after_the_monster_setup() {
    let scene = DungeonScene::new(33).unwrap();
    let mut order_was_observable = false;
    for seed in 0..256u16 {
        let mut state = fc1_fall_fixture(seed, 0x00);
        let outcome = state.resolve_dungeon_fall_trap(scene, 0, 2, 1, None).unwrap();
        assert_eq!(
            outcome,
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 1 })
        );

        let replay = |setup_first: bool| {
            let mut expected = fc1_fall_fixture(seed, 0x00);
            expected.area = Area::Dungeon { scene, level: 1 };
            expected.sync_player_object();
            expected.apply_dungeon_floor_trap_damage();
            if setup_first {
                expected.setup_dungeon_active_monster_fresh();
                expected.advance_turn();
            } else {
                expected.advance_turn();
                expected.setup_dungeon_active_monster_fresh();
            }
            expected
        };
        let expected = replay(true);
        assert_eq!(state.turn, 1, "seed {seed}: the step spends exactly one turn");
        assert_eq!(state.clock, expected.clock, "seed {seed}");
        assert_eq!(state.prng_state, expected.prng_state, "seed {seed}");
        assert_eq!(state.party[0].hp, expected.party[0].hp, "seed {seed}");
        assert_eq!(
            state.active_objects.get(DUNGEON_ACTIVE_MONSTER_SLOT),
            expected.active_objects.get(DUNGEON_ACTIVE_MONSTER_SLOT),
            "seed {seed}"
        );
        let wrong = replay(false);
        if wrong.party[0].hp != expected.party[0].hp {
            order_was_observable = true;
        }
    }
    assert!(
        order_was_observable,
        "no seed separates the two orders, so this test pins nothing"
    );
}

/// FC1, room landing: moving the step's turn to the end of the chain must
/// not charge it twice. The room-entry helper spends the turn before it
/// hands off to the fight, so the chain's tail adds none of its own.
#[test]
fn fc1_fall_into_a_room_spends_the_step_turn_once() {
    let scene = DungeonScene::new(33).unwrap();
    let mut state = fc1_fall_fixture(0x1234, 0xF0);
    let start = state.clock;
    state.resolve_dungeon_fall_trap(scene, 0, 2, 1, None).unwrap();
    assert_eq!(state.area, Area::Dungeon { scene, level: 1 });
    assert_eq!(state.turn, 1, "one turn for the step, not one per stage");
    let mut one_minute = start;
    one_minute.advance_minutes(1);
    assert_eq!(state.clock, one_minute);
}

/// Z1. `inventory.md §4.7` publishes the `Arms\n\n` heading and the
/// `(None ready)` placeholder but no row for the placeholder. Clean runtime
/// observation (paired `reseed-falsehood-theft`, beat `p2`, 2026-10-26): with
/// nothing readied the stock game draws `(None ready)` on panel row 5,
/// same column and text as the engine, which drew it on row 3. A readied list
/// still starts on row 3 (`zstats-pages/p2`, 2026-09-10).
#[test]
fn z1_none_ready_placeholder_sits_on_panel_row_five() {
    let mut state = test_state(open_grid(), 1, 1);
    state
        .party_equipment
        .resize(state.party.len(), [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]);
    state.party_equipment[0] = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
    let session = ZStatsSession {
        selected_party_index: 0,
        page: ZStatsPage::Equipment,
        inventory_cursor: 0,
    };
    let rows = state.z_stats_panel_rows(&session);
    assert_eq!(rows[0].trim(), "Arms", "rows: {rows:?}");
    assert_eq!(rows[4], format!(" {Z_STATS_NONE_READY_PLACEHOLDER}"), "rows: {rows:?}");
    for (index, row) in rows.iter().enumerate() {
        if index != 0 && index != 4 {
            assert!(row.is_empty(), "row {index} should be blank: {rows:?}");
        }
    }

    // A readied item still lists from row 3.
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    let rows = state.z_stats_panel_rows(&session);
    assert!(rows[2].starts_with(' ') && !rows[2].trim().is_empty(), "rows: {rows:?}");
    assert!(!rows.iter().any(|row| row.contains(Z_STATS_NONE_READY_PLACEHOLDER)));
}
