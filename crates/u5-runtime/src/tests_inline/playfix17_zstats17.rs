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
        // u5-spec#311 Q3 publishes the five entries; `A` is not one of them.
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

    /// `inventory.md §4.4`: the loop "terminates on the cursor's row, not on
    /// an entry count", and "a wrapping entry that begins on window row 7
    /// puts its label on row 8, so a page can show eight rows of text and no
    /// bottom border"; §5.1 has the Z-stats carried-items page overflow
    /// exactly as U-Use does. The page left row 7 blank instead (P17-09).
    #[test]
    fn playtest_p17_09_a_wrapping_entry_starts_on_row_seven_and_overruns_the_border() {
        let mut state = test_state(open_grid(), 1, 1);
        state.potion_stock = [0; POTION_COUNT];
        for shard in [
            SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
            SPECIAL_ITEM_SHARD_HATRED_INDEX,
            SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
        ] {
            state.special_items[shard] = 1;
        }
        open_z_stats_page_for_test(&mut state, ZStatsPage::SpecialUse);
        state.active_z_stats.as_mut().unwrap().inventory_cursor = 2;
        let rows = z_stats_page_rows_for_test(&state);
        assert!(rows[5].starts_with(Z_STATS_MOONSTONE_LABEL), "rows: {rows:?}");
        assert_eq!(rows[6], " 1-", "rows: {rows:?}");
        assert_eq!(
            rows[7],
            format!("{Z_STATS_WRAPPED_LINE_MARKER}Shard/Falsehd"),
            "rows: {rows:?}"
        );

        // Window row 8 is screen row 9, the frame's bottom border: the label
        // opens at window column 0 over it.
        let system = painted_panel(&state);
        assert_eq!(cell_byte(&system, 26, 8), Some(b'1'));
        assert_eq!(cell_byte(&system, 24, 9), Some(b'S'));
        assert_eq!(cell_byte(&system, 36, 9), Some(b'd'));
    }

    /// u5-spec#311 Q3 (`inventory.md §4.7`): the status table is `G` `Good
    /// Health`, `P` `Poisoned`, `D` `Dead`, `S` `Asleep`, `C` `Charmed`, each
    /// "printed with centring on, at cursor column 0 of the window spanning
    /// screen columns 24 to 38 on screen row 2", starting at columns 26, 27,
    /// 29, 28 and 28.
    #[test]
    fn spec311_q3_status_names_sit_centred_on_screen_row_two() {
        for (letter, name, column) in [
            (b'G', "Good Health", 26u8),
            (b'P', "Poisoned", 27),
            (b'D', "Dead", 29),
            (b'S', "Asleep", 28),
            (b'C', "Charmed", 28),
        ] {
            assert_eq!(z_stats_health_line(letter), name);
            let mut state = test_state(open_grid(), 5, 5);
            state.party[0].status = letter;
            open_z_stats_page_for_test(&mut state, ZStatsPage::Stats);
            let system = painted_panel(&state);
            let painted: Vec<u8> = (24u8..=38)
                .map(|x| cell_byte(&system, x, 2).unwrap_or(b' '))
                .collect();
            let expected = format!(
                "{}{name}{}",
                " ".repeat(usize::from(column - 24)),
                " ".repeat(15 - usize::from(column - 24) - name.len())
            );
            assert_eq!(String::from_utf8_lossy(&painted), expected, "status {}", char::from(letter));
        }
    }
}
