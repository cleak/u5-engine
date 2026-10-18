// Regressions for the systems exploratory play-test findings (F52..F58).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// F55: `inventory.md §4.7` publishes the Equipment screen as "(food, gold,
/// keys, gems, torches, grapple)" with the `\n_Grapple` label after
/// `\n_Torches....`; a party holding the grapple must see that row.
#[test]
fn playfix_f55_z_stats_equipment_page_lists_the_owned_grapple() {
    let mut state = world_state(open_world_grid(), 5, 5);
    let session = ZStatsSession {
        selected_party_index: 0,
        page: ZStatsPage::Counters,
        inventory_cursor: 0,
    };

    state.climbing_gear = 0;
    let rows = state.z_stats_panel_rows(&session);
    assert!(rows[6].starts_with(" Torches...."));
    assert!(!rows.iter().any(|row| row.contains("Grapple")));

    state.climbing_gear = 1;
    let rows = state.z_stats_panel_rows(&session);
    assert!(rows[6].starts_with(" Torches...."));
    assert_eq!(rows[7], " Grapple");
}
