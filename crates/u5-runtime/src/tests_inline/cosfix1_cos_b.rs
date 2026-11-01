// Closed-audit cosmetic gaps (u5-spec#302 audit, cos-b): the dungeon chest
// scroll row's runic rune word, the wrap-aware printer's line feeds, and the
// sign stream's high-bit carriage return. The SHOPPE.DAT record count is
// pinned in `shoppe_bark.rs`'s own tests.

/// and every feed of a feed-only run spend their row.
#[test]
fn cosfix1_cos_b_wrap_printer_passes_trailing_and_feed_only_line_feeds() {
    let cursor_after = |text: &str| {
        let mut system = TextWindowSystem::new();
        system.set_window_rect(0, 24, 11, 39, 23);
        system.set_active_window(0);
        system.print_wrapped_string(text);
        system.active_cursor()
    };
    assert_eq!(cursor_after("ABC\n"), (0, 1));
    assert_eq!(cursor_after("\n\n"), (0, 2));
    assert_eq!(cursor_after("\n"), (0, 1));
    assert_eq!(cursor_after("ABCDEFGHIJKLMNO\n"), (0, 1));
    assert_eq!(cursor_after("A\n\nB"), (1, 2));
    // §6 "Line feeds and full rows": a chunk that fills the row has already
    // wrapped, so its own trailing feed is consumed rather than spending a
    // blank row.
    assert_eq!(cursor_after("ABCDEFGHIJKLMNOP\n"), (0, 1));
    // A carriage return is the column-only half: it resets the column and
    // leaves the row.
    assert_eq!(cursor_after("ABC\r"), (0, 0));
    assert_eq!(cursor_after("ABC\rD"), (1, 0));
}
