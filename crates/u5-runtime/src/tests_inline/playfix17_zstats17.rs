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

    /// `inventory.md §4.7`: the status name comes "from the five-entry status
    /// table keyed by the status letter", and §4.2 publishes that alphabet as
    /// `G P D S C`, looked up so "an out-of-range byte yields no letter rather
    /// than a wrong one". The class name is "looked up from the record's class
    /// letter through the nine-entry class table" and "the name is not
    /// repeated on that line". A stored `A` printed `Ashes`, and an unknown
    /// class byte printed the member's name (P17-08).
    #[test]
    fn playtest_p17_08_out_of_alphabet_status_and_class_print_nothing() {
        assert_eq!(z_stats_health_line(b'A'), "");
        assert_eq!(z_stats_health_line(b'G'), "Good Health");

        let mut state = test_state(open_grid(), 5, 5);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'Z',
            status: b'A',
            climb_stat: 15,
            mana: 7,
            hp: 240,
            max_hp: 300,
            level: 3,
        }];
        state.party_strengths = vec![16];
        state.party_intelligence = vec![12];
        state.party_experience = vec![123];
        let name = state.party_member_display_name(0);
        let rows = state.z_stats_panel_rows(&ZStatsSession::new(0));
        assert!(rows[0].contains("Lv-3"), "rows: {rows:?}");
        assert!(!name.is_empty() && !rows[0].contains(name.as_str()), "rows: {rows:?}");
        assert_eq!(rows[1].trim(), "", "rows: {rows:?}");
        assert!(!rows.iter().any(|row| row.contains("Ashes")), "rows: {rows:?}");
    }
}
