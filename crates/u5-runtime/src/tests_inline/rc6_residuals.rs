// Regressions for the residual items found while assembling `rc6` from the
// fourth-pass playtest fixes (u5-engine#49).
//
// Each test names the published sentence it pins.

/// `dungeon-mode.md §10`: the dungeon A-Attack handler "calls the combat
/// framer" from inside the dispatch; `combat.md §4`: "combat is a function
/// call" and "the only time a fight adds is its round-wrap minutes";
/// `dungeon-mode.md §15`: the dungeon's one minute is charged at the head of
/// the loop iteration, and (u5-spec#300 Q10) the post-action pass does not
/// run for A-Attack at all. The engine's dungeon arm advanced the turn (and its
/// upkeep) before framing the arena, the sibling of playtest H21's overworld
/// arm.
#[test]
fn rc6_dungeon_attack_spends_no_turn_before_the_arena() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 0,
        tile: 0,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 20,
        aux3: 0,
    });
    let turn = state.turn;
    let clock = state.clock;

    assert!(state.handle_dungeon_key('A', Path::new("")).unwrap());

    assert!(state.combat_active, "the attack frames the arena");
    assert_eq!(state.turn, turn, "no turn is spent before the arena");
    let mut loop_head = clock;
    loop_head.advance_minutes(1);
    assert_eq!(state.clock, loop_head, "only the loop-head minute has run");

    // A round-wrap inside the fight adds its own minute (`combat.md §4`);
    // the loop-head flag still held for the exit does not swallow it.
    state.combat_round_counter = COMBAT_ROUND_COUNTER_WRAP - 1;
    state.advance_combat_round_counter();
    let mut after_wrap = loop_head;
    after_wrap.advance_minutes(COMBAT_ROUND_WRAP_TIME_ADVANCE_MINUTES);
    assert_eq!(state.clock, after_wrap);
    let turn_in_arena = state.turn;

    // u5-spec#300 Q10: A-Attack "reports \"no action\" and calls no clock
    // before or after the fight", and "must not run the post-action pass
    // after the fight" - the exit spends no turn and no minute.
    state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
    assert!(!state.combat_active);
    assert_eq!(state.turn, turn_in_arena, "no turn is spent after the arena");
    assert_eq!(state.clock, after_wrap);
}

/// `inventory.md §4.5`, "Which rows take a second display line": measured
/// "at one-, two- and three-digit quantities", the spell-charge vocabulary
/// has **none** that does. The Spells page printed the engine's own
/// `<code> <name>` label, so a ` 5-GP Magic Missile` charge wrapped
/// `Missile` onto a second line and cost the page a row.
#[test]
fn rc6_z_stats_spell_charge_entries_never_take_a_second_row() {
    let mut state = test_state(open_grid(), 1, 1);
    for count in [1u8, 5, 99, 100, 255] {
        state.spell_charges = [count; SPELL_COUNT];
        if state.active_z_stats.is_none() {
            assert_eq!(state.z_stats(), MoveOutcome::Observed);
        }
        let session = state.active_z_stats.as_mut().unwrap();
        session.page = ZStatsPage::Spells;
        for cursor in 0..SPELL_COUNT {
            state.active_z_stats.as_mut().unwrap().inventory_cursor = cursor;
            let session = state.active_z_stats.clone().unwrap();
            let rows = state.z_stats_panel_rows(&session);
            let listed: Vec<&String> = rows.iter().filter(|row| !row.trim().is_empty()).collect();
            assert!(
                !listed
                    .iter()
                    .any(|row| row.starts_with(Z_STATS_WRAPPED_LINE_MARKER)),
                "count {count}, cursor {cursor}: {rows:?}"
            );
            // Seven entries on every full page (`§4.4`: "A page shows seven
            // entries only when no entry takes two display lines").
            let expected = (SPELL_COUNT - cursor.min(SPELL_COUNT - 7)).min(7);
            assert_eq!(
                listed.len(),
                expected,
                "count {count}, cursor {cursor}: {rows:?}"
            );
            for row in &listed {
                // §4.5: the label ends before window column 15, so the whole
                // counted row fits the fifteen writable cells after column 0
                // with at least the closing cell to spare.
                assert!(
                    row.chars().count() < crate::stats_panel::PANEL_PICKER_ROW_CELLS,
                    "count {count}: {row:?}"
                );
            }
        }
    }
    state.spell_charges = [0; SPELL_COUNT];
    state.spell_charges[1] = 5;
    let session = state.active_z_stats.clone().unwrap();
    let rows = state.z_stats_panel_rows(&ZStatsSession {
        inventory_cursor: 0,
        ..session
    });
    assert_eq!(rows[0], " 5-GP", "rows: {rows:?}");
}

/// `inventory.md §4.4`: the render loop "terminates on the cursor's row, not
/// on an entry count, so an entry that wraps onto a second display line costs
/// a row: an overflowing page shows six entries". The row cursor's scan limit
/// counted entries (`len - 7`), so a band whose last seven entries held a
/// wrapped row stopped one entry short and never drew its final entry.
#[test]
fn rc6_z_stats_scan_limit_counts_rendered_rows() {
    let mut state = test_state(open_grid(), 1, 1);
    state.special_items.fill(0);
    state.scroll_stock.fill(0);
    state.potion_stock.fill(0);
    // Eight Moonstones (one line each), a counted Shard of Falsehood (two
    // lines: §4.5 "two quantity cells and the selector on the first display
    // line; the thirteen-cell label ... on the second"), then the Sextant.
    state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 5;
    state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 255;
    assert_eq!(state.z_stats(), MoveOutcome::Observed);
    state.active_z_stats.as_mut().unwrap().page = ZStatsPage::SpecialUse;
    state.active_z_stats.as_mut().unwrap().inventory_cursor = 0;

    for _ in 0..20 {
        assert!(state.step_active_z_stats(char::from(INPUT_CODE_EAST), ""));
    }
    let session = state.active_z_stats.clone().unwrap();
    assert_eq!(session.page, ZStatsPage::SpecialUse);
    // From the end: Sextant (1) + shard (2) + Moonstones 7..4 (4) = 7 rows.
    assert_eq!(session.inventory_cursor, 4);
    let rows = state.z_stats_panel_rows(&session);
    assert!(rows.iter().any(|row| row == "Sextant"), "rows: {rows:?}");
    let listed = rows.iter().filter(|row| !row.trim().is_empty()).count();
    assert_eq!(
        listed,
        crate::stats_panel::PANEL_PICKER_ROWS,
        "rows: {rows:?}"
    );
}
