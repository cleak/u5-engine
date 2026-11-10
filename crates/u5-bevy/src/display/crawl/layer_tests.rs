//! The crawl's lines (design 2.2.4, 2.3.8): echo and game rows, the live
//! row with what is being typed, a keyword exchange before and after a
//! submit (no prompt row survives it), a bare `:`, a cursor on its own row,
//! a centred row and a wrapped echo.

use u5_runtime::{MESSAGE_WINDOW_LEFT, MessageWindowRow, TlkRenderedGlyph};

use super::*;
use crate::display::crawl::{TypedLog, classify_rows};

fn row(row: u8, text: &str, prefixed: bool) -> MessageWindowRow {
    MessageWindowRow {
        row,
        column: MESSAGE_WINDOW_LEFT + u8::from(prefixed),
        text: text.to_string(),
        glyphs: text.bytes().map(TlkRenderedGlyph::ordinary).collect(),
        prefixed,
        trailing_spaces: 0,
    }
}

const CURSOR: (u8, [u8; 8]) = (5, [0x5a; 8]);

/// The lines for `placed`, classified as the sync classifies them.
fn lines(
    placed: &PlacedRows,
    echo: &[bool],
    ages: &[u64],
    typed: &TypedLog,
) -> (Vec<CrawlLine>, Option<CrawlCell>) {
    let classes = classify_rows(&placed.rows, typed, echo, placed.last_row_live());
    crawl_lines(
        placed,
        &classes,
        ages,
        typed.current(),
        |row, at| [row.glyphs[at].byte; 8],
        CURSOR,
    )
}

fn summary(lines: &[CrawlLine]) -> Vec<(LineKind, u64, String, Vec<bool>)> {
    lines
        .iter()
        .map(|line| {
            (
                line.kind,
                line.age,
                line.text(),
                line.cells.iter().map(|cell| cell.typed).collect(),
            )
        })
        .collect()
}

#[test]
fn a_walk_gives_typed_echoes_game_text_and_the_live_row() {
    let placed = PlacedRows {
        rows: vec![
            row(11, "North", true),
            row(12, "Slow progress!", false),
            row(13, "", true),
        ],
        live_row_placed: true,
        cursor: Some((25, 13)),
    };
    let (got, cursor) = lines(&placed, &[true, false, false], &[3, 1, 0], &TypedLog::new());
    let summary = summary(&got);
    assert_eq!(
        summary[0],
        (LineKind::Text, 3, "North".into(), vec![true; 5])
    );
    assert_eq!(
        summary[1],
        (LineKind::Text, 1, "Slow progress!".into(), vec![false; 14])
    );
    assert_eq!(summary[2], (LineKind::Live, 0, String::new(), vec![]));
    // An echo row's text starts in the first cell: the end cap is not a
    // text cell.
    assert_eq!(got[0].cells[0].cell, 0);
    assert_eq!(got[0].cells[0].mask, [b'N'; 8]);
    let cursor = cursor.expect("the cursor");
    assert_eq!((cursor.cell, cursor.byte, cursor.mask), (0, 5, [0x5a; 8]));
}

#[test]
fn the_keyword_prompt_shows_on_its_own_line_while_its_row_is_live() {
    let mut typed = TypedLog::new();
    typed.observe("jo");
    let placed = PlacedRows {
        rows: vec![
            row(11, "Talk-North", true),
            row(12, "A baker waves.", false),
            row(13, "Your interest?", false),
            row(14, ":jo", false),
        ],
        live_row_placed: false,
        cursor: Some((27, 14)),
    };
    let (got, cursor) = lines(&placed, &[true, false, false, false], &[0; 4], &typed);
    assert_eq!(
        summary(&got),
        [
            (LineKind::Text, 0, "Talk-North".into(), vec![true; 10]),
            (LineKind::Text, 0, "A baker waves.".into(), vec![false; 14]),
            (
                LineKind::Prompt,
                0,
                "Your interest?".into(),
                vec![false; 14]
            ),
            (LineKind::Live, 0, ":jo".into(), vec![false, true, true]),
        ]
    );
    assert_eq!(cursor.map(|cell| cell.cell), Some(3));
}

#[test]
fn no_prompt_row_survives_a_submit() {
    let mut typed = TypedLog::new();
    typed.observe("bread");
    typed.observe("");
    let placed = PlacedRows {
        rows: vec![
            row(11, "A baker waves.", false),
            row(12, "Your interest?", false),
            row(13, ":bread", false),
            row(14, "\"Rye and honey!\"", false),
            row(15, "Your interest?", false),
            row(16, ":", false),
        ],
        live_row_placed: false,
        cursor: Some((25, 16)),
    };
    let (got, _) = lines(&placed, &[false; 6], &[0; 6], &typed);
    assert_eq!(
        summary(&got),
        [
            (LineKind::Text, 0, "A baker waves.".into(), vec![false; 14]),
            // The committed input keeps only the typed word, from the
            // first cell; the colon belonged to the prompt.
            (LineKind::Text, 0, "bread".into(), vec![true; 5]),
            (
                LineKind::Text,
                0,
                "\"Rye and honey!\"".into(),
                vec![false; 16]
            ),
            (
                LineKind::Prompt,
                0,
                "Your interest?".into(),
                vec![false; 14]
            ),
            (LineKind::Live, 0, ":".into(), vec![false]),
        ]
    );
    assert_eq!(got[1].cells[0].cell, 0);
    // Once the exchange is over (the live row gone), no prompt row at all.
    let mut ended = placed.clone();
    ended.rows.truncate(5);
    ended.cursor = None;
    let (got, cursor) = lines(&ended, &[false; 5], &[0; 5], &typed);
    assert!(
        got.iter().all(|line| line.kind == LineKind::Text),
        "{got:?}"
    );
    assert!(cursor.is_none());
}

#[test]
fn a_bare_committed_colon_is_not_drawn() {
    let placed = PlacedRows {
        rows: vec![row(11, "Your interest?", false), row(12, ":", false)],
        live_row_placed: false,
        cursor: None,
    };
    let (got, _) = lines(&placed, &[false; 2], &[0; 2], &TypedLog::new());
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_cursor_below_the_text_gets_a_live_row_of_its_own() {
    // A trailing space that filled the row: the cursor waits in column 0 of
    // the next row (`text-output.md §6`).
    let placed = PlacedRows {
        rows: vec![row(21, "Sleep for how", false), row(22, "long?", false)],
        live_row_placed: false,
        cursor: Some((24, 23)),
    };
    let (got, cursor) = lines(&placed, &[false; 2], &[0; 2], &TypedLog::new());
    assert_eq!(got.len(), 3);
    assert_eq!(got[2].kind, LineKind::Live);
    assert_eq!(got[2].frame_row, 23);
    assert_eq!(cursor.map(|cell| cell.cell), Some(0));
}

#[test]
fn a_centred_row_keeps_its_offset_and_a_wrapped_echo_is_typed() {
    let mut bats = row(12, "BATS", false);
    bats.column = 30;
    let placed = PlacedRows {
        rows: vec![
            row(11, "Look-a long", true),
            row(12, "description", false),
            bats.clone(),
        ],
        live_row_placed: false,
        cursor: None,
    };
    let (got, _) = lines(&placed, &[true, true, false], &[0; 3], &TypedLog::new());
    assert_eq!(
        got[0].cells.iter().map(|c| c.typed).collect::<Vec<_>>(),
        [true; 11]
    );
    assert_eq!(
        got[1].cells.iter().map(|c| c.typed).collect::<Vec<_>>(),
        [true; 11]
    );
    assert_eq!(got[2].cells[0].cell, 6);
}
