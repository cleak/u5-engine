// Regressions for the pass-19 exploratory-playtest findings (ui19, P19-xx).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// P19-07: `stats-panel.md §6` draws the food counter "at its natural decimal
/// width, with no field padding" and the gold group as the 0..3-space ladder,
/// the literal ` G:` (its leading space is part of the label) and the value at
/// natural width. Neither is clamped, so an edited save keeps its digits.
#[test]
fn p19_counters_row_draws_edited_food_and_gold_unclamped() {
    let row = |food: u16, gold: u16| {
        let mut state = test_state(open_grid(), 1, 1);
        state.food = food;
        state.gold = gold;
        state.repaint_stats_panel();
        state.stats_panel.counter_row.clone()
    };
    assert_eq!(row(12000, 65535), "F:12000  G:65535");
    assert_eq!(row(12000, 9999), "F:12000  G:9999");
    // In-range rows keep the ladder and their last digit in column 38.
    assert_eq!(row(123, 456), "F:123     G:456");
    assert_eq!(row(9999, 5), "F:9999      G:5");
    assert_eq!(row(0, 150), "F:0       G:150");
}
