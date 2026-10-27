// The R-Ready picker's highlight covers the row's padding (`inventory.md
// §4.5`: "The selected item is drawn with its ordinary label and padding in
// inverted glyph pixels"; the renderer "pads a short row until the column
// count reaches fourteen").
mod ready_picker_highlight_pad {
    use super::*;

    /// Measured 2026-10-27 (`hut-ready-refusals`, beat `first-weapon`, every
    /// paired run back to September): the stock bar on `6 Dagger` runs to the
    /// frame's right rule; the engine's stopped after the label.
    #[test]
    fn the_ready_highlight_runs_to_the_frame_rule() {
        let mut state = test_state(open_grid(), 1, 1);
        state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
        state.equipment_stock[EQUIPMENT_ID_ARROWS] = 3;
        for key in ['R', '1', '\r'] {
            handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        }
        assert!(state.active_ready.is_some(), "the item picker is open");
        let mut system = TextWindowSystem::new();
        configure_play_text_windows(&mut system);
        paint_stats_panel_text_window(&mut system, &state, state.active_player);

        let inverse =
            |column: u8, row: u8| system.cell(column, row).is_some_and(|cell| cell.inverse);
        let highlight_row = (0u8..=10)
            .find(|row| (25u8..=37).any(|column| inverse(column, *row)))
            .expect("one picker row is highlighted");
        // Window columns 1..=13 are the frame's interior: label and padding.
        for column in 25u8..=37 {
            assert!(
                inverse(column, highlight_row),
                "screen column {column} of the highlighted row is not inverted"
            );
        }
        // The rules either side are frame glyphs, not part of the bar.
        assert!(!inverse(24, highlight_row));
        assert!(!inverse(38, highlight_row));
        // The next entry's padding is drawn, but not inverted.
        let next = highlight_row + 1;
        assert!((25u8..=37).all(|column| !inverse(column, next)));
        assert_eq!(system.cell(37, next).map(|cell| cell.byte), Some(b' '));
    }
}
