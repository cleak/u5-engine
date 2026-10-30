// Regressions for the pass-22 exploratory playtest findings (misc22).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// Playtest P22-01. `inventory.md §4.5` quantity-case table: at the
/// "No quantity" marker (255) "Neither the quantity nor the selector cell is
/// emitted; the row prints only its name". §5.1/§4.7 put R-Ready and the
/// Z-stats inventory pages on that same row renderer. A counter of 255 printed
/// `255 Leath Helm` in R-Ready and `255-Leath Helm` on the Armaments page.
#[test]
fn playtest_p22_01_equipment_counter_at_the_no_quantity_marker_prints_only_its_name() {
    let helm = crate::EQUIPMENT_SHORT_LABELS
        .iter()
        .position(|label| *label == "Leath Helm")
        .expect("published short label");
    let coif = crate::EQUIPMENT_SHORT_LABELS
        .iter()
        .position(|label| *label == "Chain Coif")
        .expect("published short label");

    let mut state = test_state(open_grid(), 1, 1);
    state.equipment_stock = [0; EQUIPMENT_COUNT];
    state.equipment_stock[helm] = crate::commands::USE_PICKER_NO_QUANTITY;
    state.equipment_stock[coif] = 150;

    open_z_stats_page_for_test(&mut state, ZStatsPage::EquipmentStock);
    let panel = z_stats_page_rows_for_test(&state).join("|");
    assert!(panel.contains("Leath Helm"), "panel: {panel}");
    assert!(!panel.contains("255"), "panel: {panel}");
    assert!(panel.contains("150-Chain Coif"), "panel: {panel}");

    let marker = crate::stats_panel::PanelPickerRow {
        quantity: Some(crate::commands::USE_PICKER_NO_QUANTITY),
        selector: crate::stats_panel::PANEL_PICKER_SELECTOR_BLANK,
        selector_runic: true,
        zero_padded: false,
        name: "Leath Helm".to_string(),
    };
    assert_eq!(marker.text(), format!("{:<13}", "Leath Helm"));
    assert_eq!(marker.lines(), vec!["Leath Helm".to_string()]);
    let counted = crate::stats_panel::PanelPickerRow {
        quantity: Some(150),
        ..marker
    };
    let lines = counted.lines();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("150") && lines[0].ends_with("Leath Helm"));
}

/// Playtest P22-07. `formats/signs-dat.md §4` step 2: "Only raw `0x29..0x31`
/// are macros. Each switches to the runic font and prints one NUL-terminated
/// decoration fragment from a small resident pool; all nine fragments are
/// sixteen cells wide." A macro took one cell, so the Paws sign's framed rows
/// drifted (`**g  WELCOME@TO`). With two macros ahead of it, the first
/// authored row starts its own message-window row.
#[test]
fn playtest_p22_07_sign_macro_fills_a_sixteen_cell_row() {
    let mut payload = vec![0x29, 0x2a];
    payload.extend_from_slice(b"g  WELCOME@TO  g");
    payload.extend_from_slice(b"g     PAWS     g");
    payload.push(0x2b);
    let rendered = sign_payload_rendered(&payload);
    let macro_cells = rendered
        .glyphs
        .iter()
        .take(2 * SIGN_MACRO_FRAGMENT_CELLS)
        .all(|glyph| glyph.font == crate::tlk_runner::TlkGlyphFont::Runic);
    assert!(macro_cells);
    let rows = crate::play_state_impl::sign_stream_rows(&rendered.text);
    let lines: Vec<&str> = rows.split('\n').collect();
    assert_eq!(lines.len(), 5, "rows: {lines:?}");
    assert_eq!(lines[2], "g  WELCOME@TO  g");
    assert_eq!(lines[3], "g     PAWS     g");
    assert_eq!(lines[4].chars().count(), SIGN_MACRO_FRAGMENT_CELLS);
    assert!(lines.iter().all(|line| line.chars().count() <= 16), "rows: {lines:?}");
}

/// A synthetic `DATA.OVL` image: filler, then nine made-up sixteen-cell
/// strings separated by NUL pairs, then the published placard rows
/// (`formats/signs-dat.md §6`). The fragment cells are invented test data,
/// not the shipped strings.
fn spec314_synthetic_data_ovl() -> (Vec<u8>, Vec<[u8; 16]>) {
    let mut image = b"\0unrelated text\0\0".to_vec();
    let mut fragments = Vec::new();
    for index in 0..SIGN_MACRO_FRAGMENT_COUNT {
        let mut cells = [b'l'; 16];
        cells[0] = b'0' + index as u8;
        cells[15] = b'g';
        image.extend_from_slice(&cells);
        image.extend_from_slice(&[0, 0]);
        fragments.push(cells);
    }
    for row in SIGN_NO_RECORD_PLACARD_ROWS {
        image.extend_from_slice(row.as_bytes());
        image.extend_from_slice(b"\n\0");
    }
    (image, fragments)
}

/// u5-spec#314 Q1. `formats/signs-dat.md §9`: the nine fragments "are the
/// nine sixteen-cell strings stored in selector order (`0x29` first, `0x31`
/// last) directly before the no-record placard's first row, and each holds
/// runic-font code points printed one glyph per cell". Playtest P22-07's
/// blank cells remain only as the fallback when no pool is located.
#[test]
fn spec314_sign_macro_fragments_are_read_before_the_placard() {
    let (image, fragments) = spec314_synthetic_data_ovl();
    let pool = locate_sign_macro_fragments(&image).expect("fragment pool");
    for (index, cells) in fragments.iter().enumerate() {
        assert_eq!(pool.fragment(SIGN_BODY_MACRO_FIRST + index as u8), Some(cells));
    }
    assert_eq!(pool.fragment(SIGN_BODY_MACRO_FIRST - 1), None);
    assert_eq!(pool.fragment(SIGN_BODY_MACRO_LAST + 1), None);

    let mut payload = vec![0x2d];
    payload.extend_from_slice(b"g  WELCOME@TO  g");
    payload.push(0x2c);
    let rendered = sign_payload_rendered_with_fragments(&payload, Some(&pool));
    let rows = crate::play_state_impl::sign_stream_rows(&rendered.text);
    let lines: Vec<&str> = rows.split('\n').collect();
    assert_eq!(lines, vec![
        std::str::from_utf8(&fragments[4]).unwrap(),
        "g  WELCOME@TO  g",
        std::str::from_utf8(&fragments[3]).unwrap(),
    ]);
    assert!(
        rendered
            .glyphs
            .iter()
            .all(|glyph| glyph.font == crate::tlk_runner::TlkGlyphFont::Runic)
    );
}

/// The locator rejects an image it cannot read unambiguously, and the
/// renderer then keeps P22-07's sixteen blank cells.
#[test]
fn spec314_sign_macro_fragment_locator_rejects_malformed_images() {
    let (image, _) = spec314_synthetic_data_ovl();
    // A short fragment.
    let mut short = image.clone();
    let at = short.windows(2).position(|pair| pair == [b'g', 0]).unwrap();
    short.remove(at - 1);
    assert_eq!(locate_sign_macro_fragments(&short), None);
    // A second placard anchor.
    let mut doubled = image.clone();
    doubled.extend_from_slice(SIGN_NO_RECORD_PLACARD_ROWS[0].as_bytes());
    assert_eq!(locate_sign_macro_fragments(&doubled), None);
    // No placard at all.
    assert_eq!(locate_sign_macro_fragments(b"\0abc\0"), None);

    let rendered = sign_payload_rendered_with_fragments(&[0x29], None);
    assert_eq!(rendered.text, " ".repeat(SIGN_MACRO_FRAGMENT_CELLS));
}

/// Against the configured original assets (skipped when
/// `U5_TEST_ASSET_DIR` is unset): the locator finds all nine fragments, and
/// every cell is a space or one of the ten runic glyphs §9 lists
/// (`g l h i j k 8 9 m n`). Nothing of the shipped strings is asserted
/// beyond that vocabulary.
#[test]
fn spec314_original_data_ovl_yields_nine_fragments_in_the_published_vocabulary() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let pool = load_sign_macro_fragments(&dir)
        .unwrap()
        .expect("DATA.OVL fragment pool");
    for selector in SIGN_BODY_MACRO_FIRST..=SIGN_BODY_MACRO_LAST {
        let cells = pool.fragment(selector).unwrap();
        assert!(
            cells.iter().all(|cell| b" glhijk89mn".contains(cell)),
            "selector {selector:#04x} uses a glyph outside the published vocabulary"
        );
    }
}
