// Regressions for the Z-stats exploratory-playtest findings (zstats17,
// P17-xx).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.
mod playfix17_zstats17 {
    use super::*;

    fn painted_panel(state: &PlayState) -> TextWindowSystem {
        let mut system = TextWindowSystem::new();
        configure_play_text_windows(&mut system);
        paint_stats_panel_text_window(&mut system, state, state.active_player);
        system
    }

    fn cell_byte(system: &TextWindowSystem, column: u8, row: u8) -> Option<u8> {
        system.cell(column, row).map(|cell| cell.byte)
    }

    /// `inventory.md §4.4`: each pass "homes the cursor to window column 1 of
    /// window row 1, and renders rows", and §4.7 gives the party-wide pages
    /// "the same eight-row frame and row renderer as the R-Ready picker". Only
    /// the empty page's `(None owned!)` placeholder stands alone; a one-entry
    /// list was drawn on interior row 4, where the placeholder goes (P17-07).
    #[test]
    fn playtest_p17_07_a_one_entry_list_starts_on_the_first_interior_row() {
        let mut state = test_state(open_grid(), 1, 1);
        state.spell_charges = [0; SPELL_COUNT];
        state.spell_charges[IN_LOR_SPELL_INDEX] = 3;
        open_z_stats_page_for_test(&mut state, ZStatsPage::Spells);
        let system = painted_panel(&state);
        // Panel window (24,1)-(39,9): frame on screen row 1, interior row 1
        // on screen row 2; ` 3-In Lor` opens at window column 2.
        assert_eq!(cell_byte(&system, 26, 2), Some(b'3'));
        assert_eq!(cell_byte(&system, 28, 2), Some(b'I'));
        assert!((25u8..=37).all(|column| cell_byte(&system, column, 5) != Some(b'3')));

        // The empty page keeps its placeholder on interior row 4.
        state.spell_charges = [0; SPELL_COUNT];
        open_z_stats_page_for_test(&mut state, ZStatsPage::Spells);
        let system = painted_panel(&state);
        assert_eq!(cell_byte(&system, 25, 5), Some(b'('));
        assert_ne!(cell_byte(&system, 25, 2), Some(b'('));
    }
}
