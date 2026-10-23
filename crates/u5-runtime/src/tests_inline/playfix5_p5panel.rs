// Regressions for the panel/picker exploratory play-test findings (J##).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// A U-Use stock holding only what the test sets: no scrolls, potions or
/// special items, and every Moonstone still buried.
fn p5panel_empty_stock_state() -> PlayState {
    let mut state = test_state(open_grid(), 4, 4);
    state.scroll_stock = [0; SCROLL_COUNT];
    state.potion_stock = [0; POTION_COUNT];
    state.special_items = [0; SPECIAL_ITEM_COUNT];
    state.moonstone_slots = [MoonstoneGateSlot {
        scene: 0,
        x: 0,
        y: 0,
        z: 0,
    }; MOONSTONE_SLOT_COUNT];
    state
}

#[test]
fn playfix5_j10_plans_row_is_uncounted_for_any_nonzero_value() {
    // inventory.md §4.5: "the HMS Cape plans entry becomes the marker for any
    // nonzero carried value", and "Uncounted `HMS Cape Plan` | thirteen label
    // cells, columns 1 through 13, no padding" - one display line.
    for value in [1u8, 3, 99, 200, 255] {
        let mut state = p5panel_empty_stock_state();
        state.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = value;
        let rows = state.use_item_picker_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "HMS Cape Plan");
        assert_eq!(rows[0].quantity, None, "plans value {value}");
        let lines = crate::stats_panel::PanelPickerRow {
            quantity: rows[0].quantity,
            selector: crate::stats_panel::PANEL_PICKER_SELECTOR_BLANK,
            selector_runic: false,
            name: rows[0].decoration.decorate(&rows[0].label),
            zero_padded: false,
        }
        .lines();
        assert_eq!(lines.len(), 1, "plans value {value} takes one display line");
    }
}

#[test]
fn playfix5_j11_highlighting_the_plans_row_leaves_no_inverse_leak() {
    // inventory.md §4.4: only a highlighted entry that takes two display
    // lines leaks the inverse toggle into the panel. The plans row is
    // uncounted (§4.5) and never wraps, so accepting or cancelling on it
    // leaves the roster repaint normal.
    for finish in ['\u{1b}', '\r'] {
        let mut state = p5panel_empty_stock_state();
        state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 1;
        state.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = 1;
        state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;
        assert_eq!(state.start_use_item(), MoveOutcome::Observed);
        assert!(state.step_active_use('>', "", Path::new("")).unwrap());
        let _ = state.step_active_use(finish, "", Path::new(""));
        assert!(
            !state.stats_panel_window_inverse,
            "finishing on the plans row with {finish:?} must not leak"
        );
    }
}

#[test]
fn playfix5_j23_effect_slot_is_absent_while_a_picker_or_z_page_owns_the_panel() {
    // inventory.md §4.4 / §4.7 clear the panel for the picker frame and the
    // Z-stats page; stats-panel.md §8 draws the slot only from the
    // full-panel refresh. So the slot is gone until the panel is restored.
    let mut state = p5panel_empty_stock_state();
    state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 1;
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 20;
    assert_eq!(
        gameplay_chrome_content(&state).timing_glyph,
        Some(PROTECTION_ACTIVE_EFFECT_TAG),
        "the resting panel shows the slot"
    );

    let mut using = state.clone();
    assert_eq!(using.start_use_item(), MoveOutcome::Observed);
    assert_eq!(gameplay_chrome_content(&using).timing_glyph, None, "U-Use picker");
    assert!(using.step_active_use('\u{1b}', "", Path::new("")).unwrap());
    assert_eq!(
        gameplay_chrome_content(&using).timing_glyph,
        Some(PROTECTION_ACTIVE_EFFECT_TAG),
        "the slot returns with the roster redraw"
    );

    let mut z = state.clone();
    z.z_stats_for_party(0);
    assert!(z.active_z_stats.is_some());
    assert_eq!(gameplay_chrome_content(&z).timing_glyph, None, "Z-stats page");
}

/// An outdoor Britannia state with the Avatar active, so the Sextant reads.
fn p5panel_britannia_state() -> PlayState {
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.active_player = Some(0);
    state
}

/// The Get rows a plain world state prints, from its echo down. Get shares
/// Look's shape (`commands.md §5.2`/§5.8: echo, blank row, result) and needs
/// no look-text asset.
fn p5panel_plain_get_rows(dir: &Path) -> Vec<String> {
    let mut plain = p5panel_britannia_state();
    p4window_keys(&mut plain, dir, &['G', '\u{3}']);
    p4window_rows_from(&p4window_rows(&plain), ">Get-North")
}

#[test]
fn playfix5_j12_moonstone_and_sextant_results_close_their_row() {
    // inventory.md §7.1: the Moonstone completions are `buried!\n` and
    // `cannot be buried here!\n`; magic.md §8: the coordinate printer ends
    // with a newline. Each leaves the row closed, so the next command's
    // echo / blank / result (commands.md §5.8) is unchanged after it.
    let dir = debug_game_dir();
    let expected = p5panel_plain_get_rows(&dir);
    assert_eq!(expected.get(1).map(String::as_str), Some(""), "{expected:?}");

    let mut moonstone = p5panel_britannia_state();
    moonstone.scroll_stock = [0; SCROLL_COUNT];
    moonstone.potion_stock = [0; POTION_COUNT];
    moonstone.special_items = [0; SPECIAL_ITEM_COUNT];
    moonstone.moonstone_slots = [MoonstoneGateSlot {
        scene: 0,
        x: 0,
        y: 0,
        z: 0,
    }; MOONSTONE_SLOT_COUNT];
    moonstone.moonstone_slots[0] = MoonstoneGateSlot::invalid();
    p4window_keys(&mut moonstone, &dir, &['U', '\r']);
    assert!(
        moonstone.message.contains("buried"),
        "the Moonstone row ran: {:?}",
        moonstone.message
    );
    assert!(!moonstone.message_row_open_mid_line, "Moonstone result closes its row");
    p4window_keys(&mut moonstone, &dir, &['G', '\u{3}']);
    assert_eq!(p4window_rows_from(&p4window_rows(&moonstone), ">Get-North"), expected);

    let mut sextant = p5panel_britannia_state();
    sextant.scroll_stock = [0; SCROLL_COUNT];
    sextant.potion_stock = [0; POTION_COUNT];
    sextant.special_items = [0; SPECIAL_ITEM_COUNT];
    sextant.moonstone_slots = moonstone.moonstone_slots;
    sextant.moonstone_slots[0] = MoonstoneGateSlot {
        scene: 0,
        x: 0,
        y: 0,
        z: 0,
    };
    sextant.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 255;
    sextant.clock = GameClock::new(22, 7).unwrap();
    p4window_keys(&mut sextant, &dir, &['U', '\r']);
    assert!(
        sextant.message.starts_with(USE_SEXTANT_READING_LABEL),
        "the Sextant read: {:?}",
        sextant.message
    );
    assert!(!sextant.message_row_open_mid_line, "the readout closes its row");
    p4window_keys(&mut sextant, &dir, &['G', '\u{3}']);
    assert_eq!(p4window_rows_from(&p4window_rows(&sextant), ">Get-North"), expected);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn playfix5_j27_an_active_player_digit_repaints_a_stale_gold_counter() {
    // stats-panel.md §2.2.1: the active-player digit keys are an inline
    // whole-panel repaint; §2.4: a debit nothing repainted "stays invisible
    // until ... any inline repaint". The digit must reveal it.
    let dir = debug_game_dir();
    for key in ['1', '0'] {
        let mut state = p4window_world_state();
        state.active_player = None;
        state.gold = 150;
        state.repaint_stats_panel();
        // The wishing well's debit: no repaint and no request.
        state.gold = 149;
        assert!(state.displayed_stats_panel().counter_row.contains("G:150"));
        p4window_keys(&mut state, &dir, &[key]);
        assert!(
            state.displayed_stats_panel().counter_row.contains("G:149"),
            "digit {key}: {:?}",
            state.displayed_stats_panel().counter_row
        );
    }
    // A rejected digit selects nothing and repaints nothing.
    let mut state = p4window_world_state();
    state.gold = 150;
    state.repaint_stats_panel();
    state.gold = 149;
    p4window_keys(&mut state, &dir, &['9']);
    assert!(state.displayed_stats_panel().counter_row.contains("G:150"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn playfix5_j24_an_active_player_digit_with_a_three_digit_hull_lifts_the_history() {
    // stats-panel.md §2.1 / §6: with a hull of 100 or more "a repaint in this
    // state lifts the message window's painted history by one row every time
    // it runs"; §2.2.1 makes the digit key such a repaint.
    let dir = debug_game_dir();
    let rows_after_digit = |hull: u8| {
        let mut state = p4window_world_state();
        state.player.transport = TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull,
            skiffs: 1,
        };
        state.repaint_stats_panel();
        let before = state.message_entries().len();
        p4window_keys(&mut state, &dir, &['1']);
        let added = state.message_entries().len() - before;
        p4window_keys(&mut state, &dir, &[' ']);
        (added, p4window_rows_from(&p4window_rows(&state), ">Set Active Plr:"))
    };
    let (lifted, lifted_rows) = rows_after_digit(150);
    let (plain, plain_rows) = rows_after_digit(99);
    assert_eq!(lifted, plain + 1, "one strip scroll");
    assert_eq!(lifted_rows.len(), plain_rows.len() + 1, "{lifted_rows:?} vs {plain_rows:?}");
    let _ = fs::remove_dir_all(dir);
}

/// A world state with a three-member Good party, so the shared selector
/// prints `Player: ` (commands.md §5.8).
fn p5panel_three_member_state() -> PlayState {
    let mut state = p4window_world_state();
    state.active_player = None;
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0", *b"IOLO\0\0\0\0\0"];
    state
}

#[test]
fn playfix5_j26_z_then_zero_closes_the_player_row_before_status() {
    // commands.md §5.8: the no-selection result "completes the row as
    // `Player:_` plus a bare line feed"; inventory.md §4.7's `\nStatus:_`
    // then leaves one blank row, as it does after a named member.
    let dir = debug_game_dir();
    let mut named = p5panel_three_member_state();
    p4window_keys(&mut named, &dir, &['Z', '\r']);
    let named_rows = p4window_rows_from(&p4window_rows(&named), ">Z-stats...");
    assert_eq!(named_rows.get(2).map(String::as_str), Some(""), "{named_rows:?}");

    let mut zero = p5panel_three_member_state();
    p4window_keys(&mut zero, &dir, &['Z', '0']);
    assert!(zero.active_z_stats.is_some(), "0 opens shared Equipment");
    let rows = p4window_rows_from(&p4window_rows(&zero), ">Z-stats...");
    assert!(rows.get(1).is_some_and(|row| row.trim_end() == "Player:"), "{rows:?}");
    assert_eq!(rows.get(2).map(String::as_str), Some(""), "{rows:?}");
    assert_eq!(rows.len(), named_rows.len(), "{rows:?} vs {named_rows:?}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn playfix5_j28_zero_is_ignored_at_the_cast_search_and_new_order_selectors() {
    // inventory.md §4.3: "`0` commits the explicit 'no one' answer only in the
    // callers that allow it ... other callers ignore `0`"; commands.md §5.8:
    // where that answer is not wired "the digit `0` ... is simply ignored".
    let dir = debug_game_dir();
    for (keys, target) in [
        (vec!['C'], "Cast"),
        (vec!['S', '\u{3}'], "Search"),
        (vec!['N'], "New Order"),
    ] {
        let mut state = p5panel_three_member_state();
        p4window_keys(&mut state, &dir, &keys);
        assert!(state.active_party_selector.is_some(), "{target} opened the selector");
        let before = p4window_rows(&state);
        p4window_keys(&mut state, &dir, &['0']);
        assert!(state.active_party_selector.is_some(), "{target}: 0 does not close it");
        assert_eq!(p4window_rows(&state), before, "{target}: 0 prints nothing");
        // Escape is still the cancel.
        p4window_keys(&mut state, &dir, &['\u{1b}']);
        assert!(state.active_party_selector.is_none(), "{target}: Escape cancels");
    }
    let _ = fs::remove_dir_all(dir);
}
