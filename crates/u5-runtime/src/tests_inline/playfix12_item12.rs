// Regressions for the exploratory-playtest round-12 item findings
// (P12-10, P12-11, P12-13).

/// `catalogs/item-list.md §7.1`, scroll 1: the Wind Change scroll "applies
/// the wind only when the active scene byte is below 33"; combat is scene
/// byte 0xFF (`spell-list.md §4`), and `moons.md §3` says "Neither
/// deliberate wind change is available in combat". Playtest P12-10: the
/// scroll set the wind inside a town fight.
#[test]
fn playtest_p12_10_the_wind_change_scroll_leaves_the_wind_alone_in_combat() {
    let game_dir = std::path::Path::new("");
    let mut state = combat2b_two_member_state();
    state.wind = WindState::Calm;
    state.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 2;

    combat2b_use_row(
        &mut state,
        |request| {
            matches!(
                request,
                UseItemRequest::Scroll { index, .. } if index == SCROLL_WIND_CHANGE_INDEX
            )
        },
        game_dir,
    );
    assert!(state.active_use.is_some(), "the Direction- prompt opens first");
    handle_play_key_input(&mut state, '8', "", game_dir).unwrap();

    assert!(state.active_use.is_none());
    assert_eq!(state.wind, WindState::Calm);
    // The scroll is still spent: the debit precedes the gate.
    assert_eq!(state.scroll_stock[SCROLL_WIND_CHANGE_INDEX], 1);
    let rows = combat2b_rows_text(&state);
    assert!(
        !rows.iter().any(|row| row.contains("Not here")),
        "rows: {rows:?}"
    );
}

/// `catalogs/item-list.md §7.1`, scroll 1: "Dungeon-class scenes return
/// without changing wind after the prompt" - a silent return. Playtest
/// P12-11: the engine printed an unpublished `Not here!` after the
/// direction.
#[test]
fn playtest_p12_11_the_wind_change_scroll_returns_silently_in_a_dungeon() {
    let game_dir = std::path::Path::new("");
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.wind = WindState::Calm;
    state.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;

    handle_play_key_input(&mut state, 'U', "", game_dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();
    assert!(state.active_use.is_some(), "the Direction- prompt opens first");
    handle_play_key_input(&mut state, '8', "", game_dir).unwrap();

    assert!(state.active_use.is_none());
    assert_eq!(state.wind, WindState::Calm);
    assert_eq!(state.scroll_stock[SCROLL_WIND_CHANGE_INDEX], 0);
    let rows = combat2b_rows_text(&state);
    assert!(
        !rows.iter().any(|row| row.contains("Not here")),
        "rows: {rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.starts_with(SPELL_DIRECTION_PROMPT_PREFIX)),
        "the prompt still completes: {rows:?}"
    );
}

/// `inventory.md §4.7` makes the Z-stats Items page the 38-slot family, and
/// the only order the spec gives that family (§4.5's classification table,
/// §7's enumeration) puts the eight moonstones after the Sceptre and before
/// the shards. Playtest P12-13: the moonstones headed the page.
#[test]
fn playtest_p12_13_the_items_page_keeps_the_use_item_enumeration_order() {
    let mut state = test_state(open_grid(), 5, 5);
    state.scroll_stock[SCROLL_LIGHT_INDEX] = 2;
    state.potion_stock[POTION_BLUE_INDEX] = 2;
    state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 255;
    state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 255;
    open_z_stats_page_for_test(&mut state, ZStatsPage::SpecialUse);
    let rows = z_stats_page_rows_for_test(&state);
    // Scroll, potion, Sceptre, then the moonstones (the scroll row's rune
    // label is in the runic font, so compare positions, not text).
    assert!(rows[0].starts_with(" 2-"), "rows: {rows:?}");
    assert!(rows[1].ends_with("Blue"), "rows: {rows:?}");
    assert_eq!(rows[2], "Sceptre", "rows: {rows:?}");
    assert!(rows[3].starts_with(Z_STATS_MOONSTONE_LABEL), "rows: {rows:?}");
}
