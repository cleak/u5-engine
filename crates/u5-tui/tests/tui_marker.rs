//! Terminal transcription of the stats-panel active-player marker.
//!
//! `stats-panel.md §4` party-row field table, column 33: "Active-player
//! marker | The fixed-cell font's right-pointing arrow, glyph code
//! `0x1A`, or a space." The emitted cell surface therefore carries the
//! byte `0x1A`, and `PlayState::render_text_window_frame` reads that
//! surface back verbatim. A terminal has no `IBM.CH` glyph table, so the
//! TUI shell must transcribe that cell to the engine-local plain-text
//! stand-in `STATS_PANEL_ACTIVE_MARKER_ASCII` (`'>'`) before printing -
//! the same convention that leaves the arms-browser page badges
//! (`0x01`/`0x02`/`0x19`) out of the plain-text view.

use u5_runtime::stats_panel::{
    STATS_PANEL_ACTIVE_MARKER_ASCII, STATS_PANEL_ACTIVE_MARKER_COLUMN,
    STATS_PANEL_ACTIVE_MARKER_GLYPH,
};
use u5_runtime::test_fixtures::*;
use u5_runtime::*;
use u5_tui::play_loop::{write_play_frame, write_play_script_snapshot};

fn roster_row(text: &str) -> &str {
    text.lines()
        .nth(usize::from(STATS_ROSTER_TOP))
        .expect("frame carries the first roster row")
}

#[test]
fn terminal_play_frame_prints_ascii_marker_not_the_raw_glyph_byte() {
    let mut state = test_state(open_grid(), 1, 1);
    state.active_player = Some(0);

    let mut out = Vec::new();
    write_play_frame(&mut state, None, &mut out).unwrap();
    let printed = String::from_utf8(out).expect("terminal frame is UTF-8");

    // The regression this guards: a raw `0x1A` (SUB) control byte
    // reaching the terminal in place of the arrow.
    assert!(
        !printed.contains(char::from(STATS_PANEL_ACTIVE_MARKER_GLYPH)),
        "terminal output must not carry the raw glyph byte 0x1A"
    );

    // The printed frame leads with the top-down text view, so find the
    // panel row by the marker column rather than by a fixed line number.
    let marker_index = usize::from(STATS_PANEL_ACTIVE_MARKER_COLUMN);
    assert!(
        printed
            .lines()
            .any(|line| line.chars().nth(marker_index) == Some(STATS_PANEL_ACTIVE_MARKER_ASCII)),
        "no printed row carries the ASCII marker in column 33: {printed:?}"
    );
}

#[test]
fn terminal_script_snapshot_prints_ascii_marker_not_the_raw_glyph_byte() {
    let mut state = test_state(open_grid(), 1, 1);
    state.active_player = Some(0);

    let mut out = Vec::new();
    write_play_script_snapshot(&mut state, None, &mut out).unwrap();
    let printed = String::from_utf8(out).expect("script snapshot is UTF-8");

    assert!(!printed.contains(char::from(STATS_PANEL_ACTIVE_MARKER_GLYPH)));
    assert!(printed.contains(STATS_PANEL_ACTIVE_MARKER_ASCII));
}

/// The transcription is terminal-local: the cell surface the pixel
/// renderers consume still carries glyph `0x1A` in column 33.
#[test]
fn cell_surface_keeps_the_fixed_cell_font_glyph() {
    let mut state = test_state(open_grid(), 1, 1);
    state.active_player = Some(0);

    let system = render_play_text_window_system(&state, state.active_player, None);
    assert_eq!(
        system
            .cell(STATS_PANEL_ACTIVE_MARKER_COLUMN, STATS_ROSTER_TOP)
            .unwrap()
            .byte,
        STATS_PANEL_ACTIVE_MARKER_GLYPH
    );

    let frame = state.render_text_window_frame(None);
    assert!(roster_row(&frame).contains(char::from(STATS_PANEL_ACTIVE_MARKER_GLYPH)));
}
