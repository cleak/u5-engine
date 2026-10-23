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
