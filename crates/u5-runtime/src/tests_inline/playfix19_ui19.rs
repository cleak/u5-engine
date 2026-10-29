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

/// P19-08: `inventory.md §7.1` completes a potion's `Item:_` with exactly
/// `Potion\n` and binds the combat target "without this prompt"; §7.2's
/// `Healed!\n`, `Failed!\n`, `Poof!\n` and `Invisible!\n` carry no leading
/// feed, so in combat the result sits on the row right under `Item: Potion`.
#[test]
fn p19_combat_potion_result_follows_item_potion_with_no_blank_row() {
    let game_dir = std::path::Path::new(".");
    for colour in [POTION_YELLOW_INDEX, POTION_BLUE_INDEX, POTION_PURPLE_INDEX] {
        let mut state = combat2b_two_member_state();
        state.party[1].hp = 5;
        state.potion_stock[colour] = 1;
        let seed = (1..=u16::MAX)
            .find(|seed| {
                let mut probe = state.clone();
                probe.prng_state = *seed;
                probe.potion_variation_roll() >= POTION_VARIATION_SELECTED_FIRST
            })
            .expect("some seed keeps the selected colour");
        state.prng_state = seed;
        combat2b_use_row(
            &mut state,
            |request| matches!(request, UseItemRequest::Potion { index, .. } if index == colour),
            game_dir,
        );
        state.flush_staged_narration();
        let rows = combat2b_rows_text(&state);
        let item = rows
            .iter()
            .rposition(|row| row == "Item: Potion")
            .unwrap_or_else(|| panic!("rows: {rows:?}"));
        let expected = match colour {
            POTION_YELLOW_INDEX => "Healed!",
            POTION_BLUE_INDEX => "Failed!",
            _ => "Poof!",
        };
        assert_eq!(
            rows.get(item + 1).map(String::as_str),
            Some(expected),
            "colour {colour}, rows: {rows:?}"
        );
    }
}

/// P19-08 review: the same single-feed rule covers `inventory.md §7.1`'s
/// other single-feed family. `Skull Key\n` is followed in dungeon exploration
/// by `Not here!\n` with no leading feed, so the refusal sits on the row right
/// under `Item: Skull Key`.
#[test]
fn p19_dungeon_skull_key_refusal_follows_item_skull_key_with_no_blank_row() {
    let game_dir = std::path::Path::new(".");
    let mut state = dungeon_state(open_dungeon_record(), 2, 1, 1);
    state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] = 2;
    assert!(state.skull_key_refused_here());
    combat2b_use_row(
        &mut state,
        |request| request == UseItemRequest::SkullKey,
        game_dir,
    );
    state.flush_staged_narration();
    let rows = combat2b_rows_text(&state);
    let item = rows
        .iter()
        .rposition(|row| row == "Item: Skull Key")
        .unwrap_or_else(|| panic!("rows: {rows:?}"));
    assert_eq!(
        rows.get(item + 1).map(String::as_str),
        Some("Not here!"),
        "rows: {rows:?}"
    );
    assert_eq!(state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 1);
}
