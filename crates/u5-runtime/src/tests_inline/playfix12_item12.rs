// Regressions for the exploratory-playtest round-12 item findings
// (P12-10, P12-11, P12-13).

/// `catalogs/item-list.md §7.1`, scroll 1: the Wind Change scroll "applies
/// the wind only when the active scene byte is below 33"; combat is scene
/// byte 0xFF (`spell-list.md §4`), and `moons.md §3` says "Neither
/// deliberate wind change is available in combat". Playtest P12-10: the
/// scroll set the wind inside a town fight. u5-spec#312 Q3 (R656) adds the
/// failure tail: "in combat it prints `Failed!` after its direction prompt
/// and the scroll is still spent", with the common cast-failure glissando.
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
    let serial = state.sound_effect_serial;
    handle_play_key_input(&mut state, '8', "", game_dir).unwrap();

    assert!(state.active_use.is_none());
    assert_eq!(state.wind, WindState::Calm);
    // The scroll is still spent: the debit precedes the gate.
    assert_eq!(state.scroll_stock[SCROLL_WIND_CHANGE_INDEX], 1);
    assert!(
        state
            .sound_effects_after(serial)
            .contains(&SoundEffect::CastFailure),
        "the common cast-failure glissando"
    );
    let rows = combat2b_rows_text(&state);
    assert!(
        !rows.iter().any(|row| row.contains("Not here")),
        "rows: {rows:?}"
    );
    let at = rows
        .iter()
        .rposition(|row| row == "Direction-North")
        .unwrap_or_else(|| panic!("rows: {rows:?}"));
    assert_eq!(rows.get(at + 1).map(String::as_str), Some("Failed!"), "rows: {rows:?}");
}

/// `catalogs/item-list.md §7.1`, scroll 1, as corrected by u5-spec#312 Q3
/// (`RETRACTIONS.md` R656, withdrawing the silent return that answered
/// playtest P12-11): in a dungeon the transcript is `Wind change!`,
/// `Direction-` with the direction word, then `Failed!` at column zero on
/// the next row, with the cast-failure glissando and no wind change. No
/// `Not here!` is printed.
#[test]
fn playtest_p12_11_the_wind_change_scroll_fails_in_a_dungeon() {
    let game_dir = std::path::Path::new("");
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.wind = WindState::Calm;
    state.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;

    handle_play_key_input(&mut state, 'U', "", game_dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();
    assert!(state.active_use.is_some(), "the Direction- prompt opens first");
    let serial = state.sound_effect_serial;
    handle_play_key_input(&mut state, '8', "", game_dir).unwrap();

    assert!(state.active_use.is_none());
    assert_eq!(state.wind, WindState::Calm);
    assert_eq!(state.scroll_stock[SCROLL_WIND_CHANGE_INDEX], 0);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::CastFailure]
    );
    state.flush_message_slot();
    let rows = combat2b_rows_text(&state);
    assert!(
        !rows.iter().any(|row| row.contains("Not here")),
        "rows: {rows:?}"
    );
    let at = rows
        .iter()
        .rposition(|row| row == "Wind change!")
        .unwrap_or_else(|| panic!("rows: {rows:?}"));
    assert_eq!(
        rows[at..at + 3].to_vec(),
        vec![
            "Wind change!".to_string(),
            "Direction-North".to_string(),
            "Failed!".to_string(),
        ],
        "rows: {rows:?}"
    );
    assert!(!state.message_row_open_mid_line, "`Failed!\\n` closes its row");
}

/// u5-spec#312 Q3: "`Pass` ends the same way there" - Space at the
/// dungeon's `Direction-` prompt still fails the gate and prints `Failed!`;
/// in the overworld the gate passes and nothing fails.
#[test]
fn spec312_q3_a_passed_wind_change_scroll_fails_in_a_dungeon_only() {
    let game_dir = std::path::Path::new("");
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;
    handle_play_key_input(&mut state, 'U', "", game_dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();
    let serial = state.sound_effect_serial;
    handle_play_key_input(&mut state, ' ', "", game_dir).unwrap();
    assert!(state.active_use.is_none());
    assert_eq!(state.scroll_stock[SCROLL_WIND_CHANGE_INDEX], 0);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::CastFailure]
    );
    state.flush_message_slot();
    let rows = combat2b_rows_text(&state);
    let at = rows
        .iter()
        .rposition(|row| row == "Direction-Pass")
        .unwrap_or_else(|| panic!("rows: {rows:?}"));
    assert_eq!(rows.get(at + 1).map(String::as_str), Some("Failed!"), "rows: {rows:?}");

    let mut world = britannia_state(open_world_grid(), 1, 1);
    world.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;
    handle_play_key_input(&mut world, 'U', "", game_dir).unwrap();
    handle_play_key_input(&mut world, '\r', "", game_dir).unwrap();
    let serial = world.sound_effect_serial;
    handle_play_key_input(&mut world, ' ', "", game_dir).unwrap();
    assert!(world.active_use.is_none());
    assert!(
        !world
            .sound_effects_after(serial)
            .contains(&SoundEffect::CastFailure)
    );
    world.flush_message_slot();
    let rows = combat2b_rows_text(&world);
    assert!(!rows.iter().any(|row| row == "Failed!"), "rows: {rows:?}");
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
