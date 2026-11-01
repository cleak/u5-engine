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

/// carriage return (column reset, no glyph), not a pause".
#[test]
fn cosfix1_cos_b_sign_high_bit_carriage_return_resets_the_column() {
    let rendered = sign_payload_rendered(&[0xC1, 0xC1, 0x8D, 0xC2, 0x00]);
    assert_eq!(rendered.text, "BA");
    assert_eq!(rendered.glyphs.len(), 2);
    assert!(
        rendered
            .glyphs
            .iter()
            .all(|glyph| glyph.font == TlkGlyphFont::Ordinary)
    );
    // The reset is to the row's left edge: after a line feed it overwrites
    // only the new row.
    let rendered = sign_payload_rendered(&[0xC1, 0x8A, 0xC1, 0xC1, 0x8D, 0xC2]);
    assert_eq!(rendered.text, "A\nBA");
    // Raw `0x0D` is still the pause, not a carriage return.
    let rendered = sign_payload_rendered(&[0xC1, 0x0D, 0xC2]);
    assert_eq!(
        rendered.text.as_bytes(),
        &[b'A', crate::text_hold::TEXT_HOLD_WAIT_KEY_MARKER, b'B']
    );
}
