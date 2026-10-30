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
