// Closed-audit cosmetic gaps (u5-spec#302 audit, cos-b): the dungeon chest
// scroll row's runic rune word, the wrap-aware printer's line feeds, and the
// sign stream's high-bit carriage return. The SHOPPE.DAT record count is
// pinned in `shoppe_bark.rs`'s own tests.

/// `containers.md §6` "What each row prints": "The rune word alone is drawn
/// from the game's **second character set** ... The surrounding `A scroll: `
/// and `!` are ordinary text."
#[test]
fn cosfix1_cos_b_dungeon_chest_scroll_rune_word_is_runic() {
    let dir = debug_game_dir();
    let mut checked = 0;
    for seed in 0u16..400 {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(7, 0, 2)] = 0x7c;
        let mut state = dungeon_state(grid, 7, 0, 2);
        state.prng_state = 0x1000 + seed;
        state.handle_dungeon_key('G', &dir).unwrap();
        state.flush_message_slot();
        let Some(entry) = state
            .message_entries()
            .iter()
            .find(|entry| entry.text.starts_with("A scroll: "))
        else {
            continue;
        };
        let word_len = entry.text.len() - "A scroll: ".len() - 1;
        let fonts: Vec<_> = entry.glyphs.iter().map(|glyph| glyph.font).collect();
        assert_eq!(fonts.len(), entry.text.len(), "{}", entry.text);
        let prefix = "A scroll: ".len();
        assert!(
            fonts[..prefix]
                .iter()
                .all(|font| *font == TlkGlyphFont::Ordinary),
            "{}",
            entry.text
        );
        assert!(
            fonts[prefix..prefix + word_len]
                .iter()
                .all(|font| *font == TlkGlyphFont::Runic),
            "{}",
            entry.text
        );
        assert_eq!(
            *fonts.last().unwrap(),
            TlkGlyphFont::Ordinary,
            "{}",
            entry.text
        );
        // The other rows of the chest stay in the ordinary font.
        for other in state.message_entries() {
            if !other.text.starts_with("A scroll: ") {
                assert!(
                    other
                        .glyphs
                        .iter()
                        .all(|glyph| glyph.font == TlkGlyphFont::Ordinary),
                    "{}",
                    other.text
                );
            }
        }
        assert!(state.pending_dungeon_scroll_rune_rows.is_empty());
        checked += 1;
        if checked == 3 {
            break;
        }
    }
    assert!(checked > 0, "no seed fired the scroll row");
    let _ = fs::remove_dir_all(dir);
}

/// The runic switch belongs to the dungeon chest's row only: a hidden
/// treasure's `A scroll: <rune>!` line is not in `containers.md §6`'s chest
/// table and keeps the ordinary font.
#[test]
fn cosfix1_cos_b_unqueued_scroll_line_stays_ordinary() {
    let mut state = dungeon_state(open_dungeon_record(), 7, 0, 2);
    state.push_message_entry("A scroll: AT!", false);
    let entry = state.message_entries().last().unwrap();
    assert!(
        entry
            .glyphs
            .iter()
            .all(|glyph| glyph.font == TlkGlyphFont::Ordinary)
    );
}

/// `text-output.md §5`/§6: "Line-feed and carriage-return bytes embedded in
/// the source string force an immediate flush at that point and pass through
/// to the per-cell emitter so the cursor moves accordingly" - a trailing feed
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

/// `formats/signs-dat.md §4` step 3: "`0x8D` reaches the cell emitter as a
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
